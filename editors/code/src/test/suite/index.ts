/**
 * Integration tests executed inside a real VS Code workbench
 * (`runTests` from @vscode/test-electron). They assert the two scenarios from
 * the auto-save report: applying `duck-trait.applyFix` must write to disk both
 * for files that are not open and for files open with unsaved changes.
 */

import * as assert from 'assert';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import {
  appendedFieldsBlock,
  findFirstFieldsBlock,
  inferIndent,
  insertionFor,
} from '../../fieldsFile';
import { positionAt } from '../../extension';

function positionOf(text: string, offset: number): { line: number; character: number } {
  const before = text.slice(0, offset);
  const line = (before.match(/\n/g) ?? []).length;
  const lastNl = before.lastIndexOf('\n');
  return { line, character: offset - (lastNl + 1) };
}

export async function run(): Promise<void> {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'duck-trait-it-'));
  fs.mkdirSync(path.join(dir, 'src'));

  // -- scenario 1: the target file is NOT open -> edit must land on disk ----
  const fieldsFile = path.join(dir, 'src/_fields.rs');
  fs.writeFileSync(fieldsFile, 'use duck_trait::fields;\n\nfields! {\n    other,\n}\n');
  const diskText = fs.readFileSync(fieldsFile, 'utf8');
  const diskBlock = findFirstFieldsBlock(diskText);
  assert.ok(diskBlock, 'fields! block found');
  const diskIns = insertionFor(diskText, diskBlock, ['value']);
  await vscode.commands.executeCommand('duck-trait.applyFix', {
    edits: [{ path: fieldsFile, ...positionOf(diskText, diskIns.offset), snippet: diskIns.snippet }],
  });
  assert.ok(
    fs.readFileSync(fieldsFile, 'utf8').includes('value,'),
    'unopened file must be updated on disk',
  );

  // -- scenario 2: the target file IS open with unsaved changes -> saved ----
  const libFile = path.join(dir, 'src/lib.rs');
  fs.writeFileSync(libFile, 'use duck_trait::fields;\n\nfields! {\n    other,\n}\n');
  const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(libFile));
  await vscode.window.showTextDocument(doc);
  const dirtyEdit = new vscode.WorkspaceEdit();
  dirtyEdit.insert(doc.uri, new vscode.Position(0, 0), 'X');
  assert.ok(await vscode.workspace.applyEdit(dirtyEdit), 'buffer edit applied');
  assert.strictEqual(doc.isDirty, true, 'document must be dirty before the fix');

  const bufferText = doc.getText();
  const bufferBlock = findFirstFieldsBlock(bufferText);
  assert.ok(bufferBlock, 'fields! block found in buffer');
  const bufferIns = insertionFor(bufferText, bufferBlock, ['value']);
  await vscode.commands.executeCommand('duck-trait.applyFix', {
    edits: [
      { path: libFile, ...positionOf(bufferText, bufferIns.offset), snippet: bufferIns.snippet },
    ],
  });
  assert.strictEqual(doc.isDirty, false, 'document must be saved by the fix');
  const onDisk = fs.readFileSync(libFile, 'utf8');
  assert.ok(onDisk.includes('value,'), 'opened file must be updated on disk');
  assert.ok(onDisk.startsWith('X'), 'the unsaved buffer change must survive the save');

  // -- scenario 3: the plan survives the code action menu (JSON transport) --
  // replicating the provider: positionAt() -> plan -> serialized arguments.
  // A class-based position would lose line/character here and land at (0,0).
  const posFile = path.join(dir, 'src/position.rs');
  fs.writeFileSync(posFile, 'fields! {\n    other,\n}\n');
  const posText = fs.readFileSync(posFile, 'utf8');
  const posBlock = findFirstFieldsBlock(posText);
  assert.ok(posBlock, 'fields! block found');
  const posIns = insertionFor(posText, posBlock, ['value']);
  const plan = JSON.parse(
    JSON.stringify({
      edits: [{ path: posFile, ...positionAt(posText, posIns.offset), snippet: posIns.snippet }],
    }),
  );
  await vscode.commands.executeCommand('duck-trait.applyFix', plan);
  const posOnDisk = fs.readFileSync(posFile, 'utf8');
  assert.ok(
    posOnDisk.indexOf('value,') > posOnDisk.indexOf('other,'),
    'declaration must land inside the fields! block, not at the top of the file',
  );
  assert.ok(!posOnDisk.startsWith('value,'), 'declaration must not land at (0,0)');

  // -- scenario 4: empty declaration file -> import + block in the crate's --
  // indentation (no rustfmt.toml here, so the root file's style is inferred)
  const crate2 = path.join(dir, 'crate2');
  fs.mkdirSync(path.join(crate2, 'src'), { recursive: true });
  const root2 = path.join(crate2, 'src/lib.rs');
  fs.writeFileSync(root2, 'mod x {\n  pub fn f() {}\n}\n'); // 2-space reference
  const emptyFile = path.join(crate2, 'src/_fields.rs');
  fs.writeFileSync(emptyFile, '');
  const emptyText = fs.readFileSync(emptyFile, 'utf8');
  const emptyIns = appendedFieldsBlock(emptyText, ['value'], inferIndent(emptyText) ?? inferIndent(fs.readFileSync(root2, 'utf8')));
  await vscode.commands.executeCommand('duck-trait.applyFix', {
    edits: [{ path: emptyFile, ...positionAt(emptyText, emptyIns.offset), snippet: emptyIns.snippet }],
  });
  const emptyOnDisk = fs.readFileSync(emptyFile, 'utf8');
  assert.ok(
    emptyOnDisk.includes('use duck_trait::fields;'),
    'the missing fields import must be added',
  );
  assert.ok(
    emptyOnDisk.includes('fields! {\n  value,\n}'),
    "indentation must match the crate's style (2 spaces)",
  );

  // -- scenario 5: one fix declares every missing field of a file at once ---
  const batchFile = path.join(dir, 'crate2/src/batch_fields.rs');
  fs.writeFileSync(batchFile, 'use duck_trait::fields;\n\nfields! {\n  other,\n}\n');
  const batchText = fs.readFileSync(batchFile, 'utf8');
  const batchBlock = findFirstFieldsBlock(batchText)!;
  const batchIns = insertionFor(batchText, batchBlock, ['value', 'count']);
  await vscode.commands.executeCommand('duck-trait.applyFix', {
    edits: [
      { path: batchFile, ...positionAt(batchText, batchIns.offset), snippet: batchIns.snippet },
    ],
  });
  const batchOnDisk = fs.readFileSync(batchFile, 'utf8');
  assert.ok(batchOnDisk.includes('  value,'), 'first field must land on disk');
  assert.ok(batchOnDisk.includes('  count,'), 'second field must land on disk');
  assert.ok(
    batchOnDisk.indexOf('value,') > batchOnDisk.indexOf('other,') &&
      batchOnDisk.indexOf('count,') > batchOnDisk.indexOf('value,'),
    'batch declarations keep the insertion order',
  );

  // -- scenario 6: palette command declares the active file's fields --------
  const crate2Fields = path.join(crate2, 'src/_fields.rs');
  const declFile = path.join(crate2, 'src/decl_file.rs');
  fs.writeFileSync(
    declFile,
    '#[props]\nstruct DeclSource {\n  #[prop]\n  count: u8,\n  #[prop]\n  alpha: String,\n}\n',
  );
  await vscode.window.showTextDocument(vscode.Uri.file(declFile));
  await vscode.commands.executeCommand('duck-trait.declareFile');
  const afterDecl = fs.readFileSync(crate2Fields, 'utf8');
  assert.ok(afterDecl.includes('  count,'), 'declareFile must add count');
  assert.ok(afterDecl.includes('  alpha,'), 'declareFile must add alpha');

  // -- scenario 7: declareCrate scans a crate whose entry is not src/ -------
  const crate3 = path.join(dir, 'crate3');
  fs.mkdirSync(path.join(crate3, 'source'), { recursive: true });
  fs.writeFileSync(
    path.join(crate3, 'Cargo.toml'),
    '[package]\nname = "crate3"\nversion = "0.0.0"\nedition = "2021"\n\n[lib]\npath = "source/lib.rs"\n',
  );
  fs.writeFileSync(
    path.join(crate3, 'source/lib.rs'),
    '#[props]\nstruct A {\n  #[prop]\n  beta: u8,\n}\n',
  );
  fs.writeFileSync(
    path.join(crate3, 'source/deep.rs'),
    '#[props]\nstruct B {\n  #[prop]\n  gamma: u8,\n}\n',
  );
  await vscode.window.showTextDocument(vscode.Uri.file(path.join(crate3, 'source/lib.rs')));
  await vscode.commands.executeCommand('duck-trait.declareCrate');
  const fields3 = path.join(crate3, 'source/_fields.rs');
  const onDisk3 = fs.readFileSync(fields3, 'utf8');
  assert.ok(onDisk3.includes('use duck_trait::fields;'), 'import must be written');
  assert.ok(onDisk3.includes('  beta,'), 'crate scan must reach the entry file');
  assert.ok(onDisk3.includes('  gamma,'), 'crate scan must reach nested files');
  assert.ok(
    fs.readFileSync(path.join(crate3, 'source/lib.rs'), 'utf8').includes('mod _fields;'),
    'the mod declaration must be wired into the entry file',
  );

  // -- scenario 8: createFieldsFile scaffolds import + mod without a block --
  const crate4 = path.join(dir, 'crate4');
  fs.mkdirSync(path.join(crate4, 'src'), { recursive: true });
  fs.writeFileSync(
    path.join(crate4, 'Cargo.toml'),
    '[package]\nname = "crate4"\nversion = "0.0.0"\nedition = "2021"\n',
  );
  const crate4Lib = path.join(crate4, 'src/lib.rs');
  fs.writeFileSync(crate4Lib, 'pub fn nothing() {}\n');
  await vscode.window.showTextDocument(vscode.Uri.file(crate4Lib));
  await vscode.commands.executeCommand('duck-trait.createFieldsFile');
  const fields4 = fs.readFileSync(path.join(crate4, 'src/_fields.rs'), 'utf8');
  assert.ok(fields4.includes('use duck_trait::fields;'), 'the import must be scaffolded');
  assert.ok(!fields4.includes('fields! {'), 'an empty fields! block would not compile');
  assert.ok(
    fs.readFileSync(crate4Lib, 'utf8').includes('mod _fields;'),
    'the mod declaration must be added',
  );

  // -- scenario 9: an inline declaration module gets no file and no dup mod --
  const crate5 = path.join(dir, 'crate5');
  fs.mkdirSync(path.join(crate5, 'src'), { recursive: true });
  fs.writeFileSync(
    path.join(crate5, 'Cargo.toml'),
    '[package]\nname = "crate5"\nversion = "0.0.0"\nedition = "2021"\n',
  );
  const crate5Lib = path.join(crate5, 'src/lib.rs');
  fs.writeFileSync(
    crate5Lib,
    'use duck_trait::{fields, props};\n\nmod override_fields {\n  use duck_trait::fields;\n\n  fields! {\n    pub tag,\n  }\n}\n\n#[props(path = crate::override_fields)]\nstruct T {\n  #[prop]\n  tag: String,\n  #[prop]\n  extra: u8,\n}\n',
  );
  await vscode.window.showTextDocument(vscode.Uri.file(crate5Lib));
  await vscode.commands.executeCommand('duck-trait.declareCrate');
  const lib5 = fs.readFileSync(crate5Lib, 'utf8');
  assert.ok(
    !fs.existsSync(path.join(crate5, 'src/override_fields.rs')),
    'an inline module must not be turned into a file',
  );
  assert.ok(
    (lib5.match(/mod override_fields/g) ?? []).length === 1,
    'the inline module must not get a duplicate mod declaration',
  );
  assert.ok(
    lib5.includes('    extra,') || lib5.includes('  extra,') || lib5.includes('extra,'),
    'the missing field must be appended inside the inline module',
  );
  const tagDecls = (lib5.match(/^\s*pub tag,$/gm) ?? []).length;
  assert.strictEqual(tagDecls, 1, 'the already-declared tag must not be duplicated');
}
