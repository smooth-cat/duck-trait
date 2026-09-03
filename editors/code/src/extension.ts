import * as vscode from 'vscode';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { matchDuckTraitDiag } from './diagnose';
import { resolveFieldName } from './field';
import {
  appendedFieldsBlock,
  fieldsImportInsertion,
  findFirstFieldsBlock,
  hasFieldsImport,
  inferIndent,
  inlineModuleSpan,
  insertionFor,
  modInsertion,
  newFieldsFileContent,
  InlineModule,
} from './fieldsFile';
import { declaredFields, hasDependency, scanPropsStructs, workspaceMembers } from './scan';

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.languages.registerCodeActionsProvider(
      { language: 'rust', scheme: 'file' },
      new DuckTraitFixProvider(),
      { providedCodeActionKinds: [vscode.CodeActionKind.QuickFix] },
    ),
    // the whole fix lives in this command (not in CodeAction.edit): the
    // command is the single execution entry, and it saves every file it
    // touches right after applying the edit, so rust-analyzer re-analyzes
    // without a manual Cmd+S
    vscode.commands.registerCommand('duck-trait.applyFix', async (plan: FixPlan) => {
      const edit = new vscode.WorkspaceEdit();
      if (plan.create) {
        const uri = vscode.Uri.file(plan.create.path);
        edit.createFile(uri, { overwrite: false });
        edit.insert(uri, new vscode.Position(0, 0), plan.create.content);
      }
      for (const ins of plan.edits) {
        edit.insert(
          vscode.Uri.file(ins.path),
          new vscode.Position(ins.line, ins.character),
          ins.snippet,
        );
      }
      if (!(await vscode.workspace.applyEdit(edit))) {
        void vscode.window.showErrorMessage('duck-trait: failed to apply the fix');
        return;
      }
      const touched = new Set(plan.edits.map(e => e.path));
      if (plan.create) {
        touched.add(plan.create.path);
      }
      for (const file of touched) {
        const uri = vscode.Uri.file(file);
        const doc = vscode.workspace.textDocuments.find(d => d.uri.toString() === uri.toString());
        if (doc?.isDirty) {
          await doc.save();
        }
      }
    }),
    vscode.commands.registerCommand('duck-trait.declareFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || !editor.document.uri.fsPath.endsWith('.rs')) {
        void vscode.window.showInformationMessage('duck-trait: open a Rust file first');
        return;
      }
      const crate = await resolveCrate();
      if (!crate) {
        return;
      }
      await declareMissing(crate, [editor.document.uri.fsPath], 'in this file');
    }),
    vscode.commands.registerCommand('duck-trait.declareCrate', async () => {
      const crate = await resolveCrate();
      if (!crate) {
        return;
      }
      await declareMissing(crate, rustFiles(crate.srcDir), 'in this crate');
    }),
    vscode.commands.registerCommand('duck-trait.createFieldsFile', async () => {
      const crate = await resolveCrate();
      if (!crate) {
        return;
      }
      await createFieldsFile(crate);
    }),
    vscode.workspace.onDidSaveTextDocument(doc => {
      void maybeAutoDeclare(doc);
    }),
    // no pending debounce may outlive the extension
    new vscode.Disposable(() => {
      for (const { timer } of autoDeclarePending.values()) {
        clearTimeout(timer);
      }
      autoDeclarePending.clear();
    }),
  );
}

/** How long a file's save waits for more saves of the same file before running. */
const AUTO_DECLARE_DEBOUNCE_MS = 300;

/** Debounce bookkeeping: saved file -> pending run (crate + its timer). */
const autoDeclarePending = new Map<string, { crate: Crate; timer: NodeJS.Timeout }>();

/**
 * Save-triggered auto-declaration: content-diff the saved Rust file against
 * its fields module and fill the gap, silently. Runs only when the saved
 * file's crate — the nearest member manifest, so monorepo siblings do not
 * leak in — actually depends on `duck-trait`.
 */
function maybeAutoDeclare(doc: vscode.TextDocument): void {
  if (doc.languageId !== 'rust' || doc.uri.scheme !== 'file' || !doc.uri.fsPath.endsWith('.rs')) {
    return;
  }
  if (path.basename(doc.uri.fsPath) === '_fields.rs') {
    return; // declaration files hold no #[props] structs — a save here is always a no-op
  }
  if (!vscode.workspace.getConfiguration('duck-trait').get('autoDeclareOnSave', true)) {
    return;
  }
  const crate = locateCrate(path.dirname(doc.uri.fsPath));
  if (!crate || !entryFileOf(crate)) {
    return;
  }
  const manifest = path.join(crate.root, 'Cargo.toml');
  try {
    if (!hasDependency(fs.readFileSync(manifest, 'utf8'), 'duck-trait')) {
      return; // the crate does not use duck-trait — nothing to auto-generate
    }
  } catch {
    return;
  }
  scheduleAutoDeclare(crate, doc.uri.fsPath);
}

/**
 * Per-file debounce: the debounce window restarts on every save of the same
 * file, so a burst of saves collapses into one run — a newer save supersedes
 * the older pending one instead of queueing a second (already no-op) task.
 * Runs that already started are never interrupted.
 */
function scheduleAutoDeclare(crate: Crate, file: string): void {
  const existing = autoDeclarePending.get(file);
  if (existing) {
    clearTimeout(existing.timer);
  }
  const timer = setTimeout(() => {
    autoDeclarePending.delete(file);
    void enqueueAutoDeclare(crate, file);
  }, AUTO_DECLARE_DEBOUNCE_MS);
  autoDeclarePending.set(file, { crate, timer });
}

/** Save events run one at a time so concurrent plans never race on offsets. */
let autoDeclareChain: Promise<void> = Promise.resolve();
function enqueueAutoDeclare(crate: Crate, file: string): Promise<void> {
  const run = (): Promise<void> => declareMissing(crate, [file], 'in this file', { silent: true });
  const own = autoDeclareChain.then(run, run).catch(() => {});
  autoDeclareChain = own;
  return own;
}

/**
 * The crate to work on: the active file's crate when possible, otherwise the
 * workspace members (monorepo) offered as a picker.
 */
async function resolveCrate(): Promise<Crate | undefined> {
  const active = vscode.window.activeTextEditor?.document.uri.fsPath;
  const direct = active ? locateCrate(path.dirname(active)) : undefined;
  if (direct) {
    return direct;
  }
  const workspaceRoot =
    (active ? locateWorkspace(path.dirname(active)) : undefined) ??
    vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceRoot) {
    void vscode.window.showInformationMessage(
      'duck-trait: open a file inside a crate (a directory with Cargo.toml and sources)',
    );
    return undefined;
  }
  const manifest = path.join(workspaceRoot, 'Cargo.toml');
  if (!fs.existsSync(manifest)) {
    void vscode.window.showInformationMessage('duck-trait: no Cargo.toml found');
    return undefined;
  }
  const crates = memberCrates(workspaceRoot, fs.readFileSync(manifest, 'utf8'));
  if (crates.length === 0) {
    void vscode.window.showInformationMessage(
      'duck-trait: no workspace member with a source directory found',
    );
    return undefined;
  }
  const picked = await vscode.window.showQuickPick(
    crates.map(c => ({ label: path.basename(c.root), description: c.srcDir, crate: c })),
    { placeHolder: 'duck-trait: select a crate' },
  );
  return picked?.crate;
}

/** The Cargo.toml of a `[workspace]` above `fromDir`, if any. */
function locateWorkspace(fromDir: string): string | undefined {
  let dir = fromDir;
  for (let i = 0; i < 20; i++) {
    const manifest = path.join(dir, 'Cargo.toml');
    if (fs.existsSync(manifest) && /^\[workspace\]/m.test(fs.readFileSync(manifest, 'utf8'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return undefined;
    }
    dir = parent;
  }
  return undefined;
}

/** The workspace members that have a source directory; supports a single `*` level. */
function memberCrates(workspaceRoot: string, manifestText: string): Crate[] {
  const crates: Crate[] = [];
  for (const member of workspaceMembers(manifestText)) {
    const candidates: string[] = [];
    if (member.endsWith('/*')) {
      const base = path.join(workspaceRoot, member.slice(0, -2));
      for (const entry of fs.existsSync(base) ? fs.readdirSync(base) : []) {
        candidates.push(path.join(base, entry));
      }
    } else {
      candidates.push(path.join(workspaceRoot, member));
    }
    for (const dir of candidates) {
      const manifest = path.join(dir, 'Cargo.toml');
      if (!fs.existsSync(manifest)) {
        continue;
      }
      const srcDir = entryDirOf(dir, fs.readFileSync(manifest, 'utf8'));
      if (srcDir) {
        crates.push({ root: dir, srcDir });
      }
    }
  }
  return crates;
}

/** Every `.rs` file under `dir`, recursively. */
function rustFiles(dir: string): string[] {
  const out: string[] = [];
  const walk = (current: string): void => {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (entry.name.endsWith('.rs')) {
        out.push(full);
      }
    }
  };
  walk(dir);
  return out;
}

/**
 * Shared flow of the palette commands: scan the given files for `#[props]`
 * structs, diff their `#[prop]` fields against the declarations, and apply one
 * plan that fills every gap (missing import, `mod` declaration, new file).
 */
async function declareMissing(
  crate: Crate,
  files: string[],
  scopeLabel: string,
  opts: { silent?: boolean } = {},
): Promise<void> {
  const show = (message: string): void => {
    if (!opts.silent) {
      void vscode.window.showInformationMessage(message);
    }
  };
  // module path -> every `#[prop]` field of the structs targeting it
  const byModule = new Map<string, string[]>();
  for (const file of files) {
    const text = liveText(file);
    if (text === undefined) {
      continue;
    }
    for (const struct of scanPropsStructs(text)) {
      if (struct.fields.length === 0) {
        continue;
      }
      const modulePath = struct.modulePath ?? 'crate::_fields';
      const list = byModule.get(modulePath) ?? [];
      for (const field of struct.fields) {
        if (!list.includes(field)) {
          list.push(field);
        }
      }
      byModule.set(modulePath, list);
    }
  }
  if (byModule.size === 0) {
    show(`duck-trait: no #[props] structs with #[prop] fields found ${scopeLabel}`);
    return;
  }

  const plan: FixPlan = { edits: [] };
  let declared = 0;
  let createdModules = 0;
  for (const [modulePath, fields] of byModule) {
    const segments = modulePath.replace(/^crate::/, '').split('::');
    const target = path.join(crate.srcDir, ...segments) + '.rs';
    const rootFile = entryFileOf(crate);
    if (!rootFile) {
      void vscode.window.showErrorMessage(
        'duck-trait: no lib.rs/main.rs entry file found in the crate',
      );
      return;
    }
    const rootText = liveText(rootFile);
    if (rootText === undefined) {
      return;
    }

    if (fs.existsSync(target)) {
      const targetText = liveText(target);
      if (targetText === undefined) {
        return;
      }
      declared += appendFileDeclarations(plan, crate, target, targetText, fields);
    } else {
      // the module may be declared inline in the entry file — extend it
      // instead of creating an orphan declaration file it cannot be wired to
      const inline = inlineModuleSpan(rootText, segments[segments.length - 1]);
      if (inline) {
        declared += appendInlineDeclarations(
          plan,
          crate,
          { rootFile, rootText, span: inline },
          fields,
        );
      } else {
        // one new declaration file per run — run the command again for others
        if (plan.create) {
          continue;
        }
        createdModules++;
        plan.create = {
          path: target,
          content: newFieldsFileContent(fields, crateIndent(crate, rootText)),
        };
        declared += fields.length;
      }
    }

    // the declaration may exist without its `mod` wiring
    const mod = modInsertion(rootText, segments[segments.length - 1]);
    if (mod) {
      plan.edits.push({ path: rootFile, ...positionAt(rootText, mod.offset), snippet: mod.snippet });
    }
  }

  if (!plan.create && plan.edits.length === 0) {
    show('duck-trait: nothing to declare — every field is already declared');
    return;
  }
  await vscode.commands.executeCommand('duck-trait.applyFix', plan);
  const parts: string[] = [];
  if (declared > 0) {
    parts.push(`declared ${declared} field(s)`);
  }
  if (plan.create) {
    parts.push(`created ${path.relative(crate.root, plan.create.path)}`);
  }
  show(`duck-trait: ${parts.join(', ')} ${scopeLabel}`);
}

async function createFieldsFile(crate: Crate): Promise<void> {
  const target = path.join(crate.srcDir, '_fields.rs');
  if (fs.existsSync(target)) {
    await vscode.window.showTextDocument(vscode.Uri.file(target));
    return;
  }
  const rootFile = entryFileOf(crate);
  if (!rootFile) {
    void vscode.window.showErrorMessage(
      'duck-trait: no lib.rs/main.rs entry file found in the crate',
    );
    return;
  }
  const rootText = liveText(rootFile);
  if (rootText === undefined) {
    return;
  }
  const plan: FixPlan = {
    create: { path: target, content: newFieldsFileContent([], crateIndent(crate, rootText)) },
    edits: [],
  };
  const mod = modInsertion(rootText, '_fields');
  if (mod) {
    plan.edits.push({ path: rootFile, ...positionAt(rootText, mod.offset), snippet: mod.snippet });
  }
  await vscode.commands.executeCommand('duck-trait.applyFix', plan);
  const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(target));
  await vscode.window.showTextDocument(doc);
  void vscode.window.showInformationMessage(
    `duck-trait: created ${path.relative(crate.root, target)}`,
  );
}

/**
 * Everything the `duck-trait.applyFix` command needs, as plain JSON so it
 * survives transport through the code action menu. Insertion points are
 * computed against the file's live state (open editor buffer, else disk).
 */
interface FixPlan {
  create?: { path: string; content: string };
  edits: { path: string; line: number; character: number; snippet: string }[];
}

interface Crate {
  root: string;
  srcDir: string;
}

class DuckTraitFixProvider implements vscode.CodeActionProvider {
  provideCodeActions(
    document: vscode.TextDocument,
    _range: vscode.Range,
    context: vscode.CodeActionContext,
  ): vscode.CodeAction[] {
    // context.diagnostics only covers the cursor's range, but the errors live
    // on the generated impls while users typically stand on the struct —
    // collect every diagnostic of the file so fixes are offered anywhere in it
    const diagnostics = collectFileDiagnostics(document.uri, context);
    const actions: vscode.CodeAction[] = [];
    const unresolved: { diag: vscode.Diagnostic; traitName: string; modulePath: string }[] = [];
    for (const diag of diagnostics) {
      const hit = matchDuckTraitDiag(diag.message);
      if (!hit) {
        continue;
      }
      if ('traitName' in hit) {
        unresolved.push({ diag, traitName: hit.traitName, modulePath: hit.modulePath });
        const action = this.buildTraitAction(document.uri, diag, hit.traitName, hit.modulePath);
        if (action) {
          actions.push(action);
        }
      } else {
        const action = this.buildModuleAction(document.uri, diag, hit.modName);
        if (action) {
          actions.push(action);
        }
      }
    }
    const batch = this.buildBatchAction(document.uri, unresolved);
    if (batch) {
      // more than one field missing: the all-in-one fix takes precedence and
      // the per-field fixes step back
      actions.forEach(a => (a.isPreferred = false));
      actions.push(batch);
    }
    return actions;
  }

  /**
   * One click declares every missing field of this file: the unresolved-trait
   * diagnostics are deduplicated, resolved to their `#[prop]` spellings and
   * grouped per declaration module.
   */
  private buildBatchAction(
    docUri: vscode.Uri,
    unresolved: { diag: vscode.Diagnostic; traitName: string; modulePath: string }[],
  ): vscode.CodeAction | undefined {
    if (unresolved.length < 2) {
      return undefined; // the per-field fix already covers it
    }
    const crate = locateCrate(path.dirname(docUri.fsPath));
    if (!crate) {
      return undefined;
    }
    const docText = liveText(docUri.fsPath);
    if (docText === undefined) {
      return undefined;
    }

    const seen = new Set<string>();
    const fields: string[] = [];
    const byModule = new Map<string, string[]>();
    for (const { traitName, modulePath } of unresolved) {
      const key = `${modulePath}::${traitName}`;
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      const field = resolveFieldName(traitName, docText);
      if (!field) {
        return undefined;
      }
      fields.push(field);
      const list = byModule.get(modulePath) ?? [];
      list.push(field);
      byModule.set(modulePath, list);
    }
    if (fields.length < 2) {
      return undefined;
    }

    const plan: FixPlan = { edits: [] };
    let declared = 0;
    for (const [modulePath, fields] of byModule) {
      const segments = modulePath.replace(/^crate::/, '').split('::');
      const site = declarationSite(crate, segments);
      if (!site) {
        return undefined;
      }
      declared += appendDeclarations(crate, site, segments, fields, plan);
    }

    const rel =
      byModule.size === 1
        ? `in ${path.relative(crate.root, [...byModule.keys()][0].replace(/^crate::/, crate.srcDir) + '.rs')}`
        : 'across the declaration files';
    const action = new vscode.CodeAction(
      `duck-trait: declare all missing fields (${fields.length}) ${rel}`,
      vscode.CodeActionKind.QuickFix,
    );
    action.isPreferred = true;
    action.diagnostics = unresolved.map(u => u.diag);
    action.command = { title: action.title, command: 'duck-trait.applyFix', arguments: [plan] };
    return action;
  }

  /** `cannot find trait \`_Value\` in module \`crate::_fields\`` — declare the field. */
  private buildTraitAction(
    docUri: vscode.Uri,
    diag: vscode.Diagnostic,
    traitName: string,
    modulePath: string,
  ): vscode.CodeAction | undefined {
    const crate = locateCrate(path.dirname(docUri.fsPath));
    if (!crate) {
      return undefined;
    }

    const docText = liveText(docUri.fsPath);
    if (docText === undefined) {
      return undefined;
    }
    const field = resolveFieldName(traitName, docText);
    if (!field) {
      return undefined;
    }

    const segments = modulePath.replace(/^crate::/, '').split('::');
    const site = declarationSite(crate, segments);
    if (!site) {
      return undefined;
    }
    const plan: FixPlan = { edits: [] };
    if (appendDeclarations(crate, site, segments, [field], plan) === 0 && plan.edits.length === 0) {
      return undefined; // already declared — a stale diagnostic
    }
    const targetRel =
      site.kind === 'missing'
        ? path.relative(crate.root, site.target)
        : path.relative(crate.root, site.kind === 'file' ? site.target : site.rootFile);
    return this.planAction(
      site.kind === 'missing'
        ? `duck-trait: create ${targetRel} and declare \`${field}\``
        : `duck-trait: declare \`${field}\` in ${targetRel}`,
      plan,
      diag,
    );
  }

  /**
   * `could not find \`_fields\` in the crate root` — the declaration file may
   * exist without its `mod` declaration; offer to wire it up. Nothing to offer
   * when the file itself is missing (no field name is known yet).
   */
  private buildModuleAction(
    docUri: vscode.Uri,
    diag: vscode.Diagnostic,
    modName: string,
  ): vscode.CodeAction | undefined {
    const crate = locateCrate(path.dirname(docUri.fsPath));
    if (!crate) {
      return undefined;
    }
    const target = path.join(crate.srcDir, `${modName}.rs`);
    if (!fs.existsSync(target)) {
      return undefined;
    }
    const rootFile = entryFileOf(crate);
    if (!rootFile) {
      return undefined;
    }
    const rootText = liveText(rootFile);
    if (rootText === undefined) {
      return undefined;
    }
    const mod = modInsertion(rootText, modName);
    if (!mod) {
      return undefined;
    }

    return this.planAction(
      `duck-trait: add \`mod ${modName};\` to ${path.relative(crate.root, rootFile)}`,
      { edits: [{ path: rootFile, ...positionAt(rootText, mod.offset), snippet: mod.snippet }] },
      diag,
    );
  }

  private planAction(title: string, plan: FixPlan, diag: vscode.Diagnostic): vscode.CodeAction {
    const action = new vscode.CodeAction(title, vscode.CodeActionKind.QuickFix);
    action.isPreferred = true;
    action.diagnostics = [diag];
    action.command = { title, command: 'duck-trait.applyFix', arguments: [plan] };
    return action;
  }
}

/**
 * Every duck-trait-relevant diagnostic of the file: the cursor-scoped ones
 * from the quick fix context plus all file-wide diagnostics (rustc flycheck
 * and rust-analyzer), deduplicated by message + range.
 */
function collectFileDiagnostics(uri: vscode.Uri, context: vscode.CodeActionContext): vscode.Diagnostic[] {
  const all = [...context.diagnostics];
  for (const diag of vscode.languages.getDiagnostics(uri)) {
    if (
      !matchDuckTraitDiag(diag.message) ||
      all.some(e => e.message === diag.message && e.range.isEqual(diag.range))
    ) {
      continue;
    }
    all.push(diag);
  }
  return all;
}

/**
 * The file's current state: the open editor buffer when the file is open
 * (so insertion points stay aligned with unsaved changes), otherwise disk.
 */
function liveText(file: string): string | undefined {
  const uri = vscode.Uri.file(file);
  const doc = vscode.workspace.textDocuments.find(d => d.uri.toString() === uri.toString());
  if (doc) {
    return doc.getText();
  }
  try {
    return fs.readFileSync(file, 'utf8');
  } catch {
    return undefined;
  }
}

/**
 * Where the fields of the module `segments` are declared: the module file when
 * it exists, the entry file's inline `mod <name> { .. }` when the module is
 * declared inline, or a missing declaration file.
 */
type DeclarationSite =
  | { kind: 'file'; target: string; rootFile: string; rootText: string; text: string }
  | { kind: 'inline'; target: string; rootFile: string; rootText: string; span: InlineModule }
  | { kind: 'missing'; target: string; rootFile: string; rootText: string };

function declarationSite(crate: Crate, segments: string[]): DeclarationSite | undefined {
  const target = path.join(crate.srcDir, ...segments) + '.rs';
  const rootFile = entryFileOf(crate);
  if (!rootFile) {
    return undefined;
  }
  const rootText = liveText(rootFile);
  if (rootText === undefined) {
    return undefined;
  }
  if (fs.existsSync(target)) {
    const text = liveText(target);
    return text === undefined ? undefined : { kind: 'file', target, rootFile, rootText, text };
  }
  // the module may live inline in the entry file — no declaration file at all
  const inline = inlineModuleSpan(rootText, segments[segments.length - 1]);
  if (inline) {
    return { kind: 'inline', target, rootFile, rootText, span: inline };
  }
  return { kind: 'missing', target, rootFile, rootText };
}

/**
 * Appends `fields` to the declaration site of the module `segments` (skipping
 * ones already declared there) and adds the `mod` wiring when needed. Returns
 * the number of fields actually declared.
 */
function appendDeclarations(
  crate: Crate,
  site: DeclarationSite,
  segments: string[],
  fields: string[],
  plan: FixPlan,
): number {
  let declared = 0;
  if (site.kind === 'file') {
    declared = appendFileDeclarations(plan, crate, site.target, site.text, fields);
  } else if (site.kind === 'inline') {
    declared = appendInlineDeclarations(plan, crate, site, fields);
  } else if (!plan.create) {
    // one created declaration file per plan
    plan.create = {
      path: site.target,
      content: newFieldsFileContent(fields, crateIndent(crate, site.rootText)),
    };
    declared = fields.length;
  }

  // the module may still need its `mod` wiring — modInsertion skips modules
  // that are already declared, inline ones included
  const mod = modInsertion(site.rootText, segments[segments.length - 1]);
  if (mod) {
    plan.edits.push({
      path: site.rootFile,
      ...positionAt(site.rootText, mod.offset),
      snippet: mod.snippet,
    });
  }
  return declared;
}

/**
 * Declares the `fields` missing from an existing declaration file. Returns
 * the number of fields added. A `fields!` block without the macro import
 * never compiles, so when the file calls the macro but does not import it
 * the `use duck_trait::fields;` line is wired in alongside the new entry.
 */
function appendFileDeclarations(
  plan: FixPlan,
  crate: Crate,
  target: string,
  text: string,
  fields: string[],
): number {
  const known = new Set(declaredFields(text));
  const missing = fields.filter(f => !known.has(f));
  const indent = crateIndent(crate, text);
  const block = findFirstFieldsBlock(text, indent);
  if (missing.length > 0) {
    const ins = block
      ? insertionFor(text, block, missing)
      : appendedFieldsBlock(text, missing, indent);
    plan.edits.push({ path: target, ...positionAt(text, ins.offset), snippet: ins.snippet });
  }
  // only the block path needs the import here — appendedFieldsBlock (no
  // block) already appends the import together with the new block
  if (block && !hasFieldsImport(text)) {
    const ins = fieldsImportInsertion(text);
    if (ins) {
      plan.edits.push({ path: target, ...positionAt(text, ins.offset), snippet: ins.snippet });
    }
  }
  return missing.length;
}

/** The inline-module counterpart of {@link appendFileDeclarations}. */
function appendInlineDeclarations(
  plan: FixPlan,
  crate: Crate,
  site: { rootFile: string; rootText: string; span: InlineModule },
  fields: string[],
): number {
  const bodyText = site.rootText.slice(site.span.opener + 1, site.span.closer);
  const known = new Set(declaredFields(bodyText));
  const missing = fields.filter(f => !known.has(f));
  const step = crateIndent(crate, site.rootText) ?? '    ';
  const block = findFirstFieldsBlock(bodyText, step);
  if (missing.length > 0) {
    const ins = block
      ? insertionFor(bodyText, block, missing)
      : appendedFieldsBlock(bodyText, missing, step + step);
    plan.edits.push({
      path: site.rootFile,
      ...positionAt(site.rootText, site.span.opener + 1 + ins.offset),
      snippet: ins.snippet,
    });
  }
  if (block && !hasFieldsImport(bodyText)) {
    const ins = fieldsImportInsertion(bodyText, step);
    if (ins) {
      plan.edits.push({
        path: site.rootFile,
        ...positionAt(site.rootText, site.span.opener + 1 + ins.offset),
        snippet: ins.snippet,
      });
    }
  }
  return missing.length;
}

function locateCrate(fromDir: string): Crate | undefined {
  let dir = fromDir;
  for (let i = 0; i < 20; i++) {
    const manifest = path.join(dir, 'Cargo.toml');
    if (fs.existsSync(manifest)) {
      // the entry file decides where sources (and `_fields.rs`) live:
      // `[lib] path = "source/lib.rs"` puts them next to `source/`, not `src/`
      const srcDir = entryDirOf(dir, fs.readFileSync(manifest, 'utf8'));
      return srcDir ? { root: dir, srcDir } : undefined;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return undefined;
    }
    dir = parent;
  }
  return undefined;
}

/** The directory of the crate's entry file (`[lib]`/`[[bin]]` `path`), defaulting to `src/`. */
function entryDirOf(crateRoot: string, manifestText: string): string | undefined {
  const src = path.join(crateRoot, 'src');
  const libSection = cargoSection(manifestText, 'lib');
  const explicit = /^\s*path\s*=\s*"([^"]+)"/m.exec(libSection)?.[1];
  if (explicit) {
    return path.normalize(path.join(crateRoot, explicit, '..'));
  }
  if (libSection.trim() !== '' || fs.existsSync(path.join(src, 'lib.rs'))) {
    return src;
  }
  const binSection = cargoSection(manifestText, 'bin');
  const binPath = /^\s*path\s*=\s*"([^"]+)"/m.exec(binSection)?.[1];
  if (binPath) {
    return path.normalize(path.join(crateRoot, binPath, '..'));
  }
  return fs.existsSync(src) ? src : undefined;
}

/** The body of a `[section]` / `[[section]]` header in a Cargo.toml. */
function cargoSection(manifestText: string, name: string): string {
  const re = new RegExp(`^\\[{1,2}${name}\\]{1,2}[^\\n]*\\n([\\s\\S]*?)(?=^\\[|$(?!\\n))`, 'm');
  return re.exec(manifestText)?.[1] ?? '';
}

/** `lib.rs` / `main.rs` inside the crate's source directory, whichever exists. */
function entryFileOf(crate: Crate): string | undefined {
  return ['lib.rs', 'main.rs']
    .map(f => path.join(crate.srcDir, f))
    .find(f => fs.existsSync(f));
}

/**
 * The indentation generated code should use: the crate's rustfmt.toml
 * (`tab_spaces` / `hard_tabs`), then the target file's own content, then the
 * crate root file.
 */
function crateIndent(crate: Crate, targetText: string): string | undefined {
  return rustfmtIndent(crate.root) ?? inferIndent(targetText) ?? rootFileIndent(crate);
}

function rustfmtIndent(crateRoot: string): string | undefined {
  for (const name of ['rustfmt.toml', '.rustfmt.toml']) {
    const file = path.join(crateRoot, name);
    if (!fs.existsSync(file)) {
      continue;
    }
    const text = fs.readFileSync(file, 'utf8');
    if (/^\s*hard_tabs\s*=\s*true/m.exec(text)) {
      return '\t';
    }
    const spaces = /^\s*tab_spaces\s*=\s*(\d+)/m.exec(text);
    return spaces ? ' '.repeat(Number(spaces[1])) : undefined;
  }
  return undefined;
}

function rootFileIndent(crate: Crate): string | undefined {
  const rootFile = ['lib.rs', 'main.rs']
    .map(f => path.join(crate.srcDir, f))
    .find(f => fs.existsSync(f));
  if (!rootFile) {
    return undefined;
  }
  return inferIndent(fs.readFileSync(rootFile, 'utf8'));
}

/**
 * A plain-object position. Must NOT be a `vscode.Position` instance: the plan
 * travels through the code action menu, and spreading a class instance would
 * silently drop its prototype getters (`line`/`character`), landing the edit
 * at the top of the file.
 */
export function positionAt(text: string, offset: number): { line: number; character: number } {
  const before = text.slice(0, offset);
  const line = (before.match(/\n/g) ?? []).length;
  const lastNl = before.lastIndexOf('\n');
  return { line, character: offset - (lastNl + 1) };
}
