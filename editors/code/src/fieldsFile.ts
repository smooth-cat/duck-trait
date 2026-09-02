/**
 * Locates `fields!` declarations and computes precise insertion points inside
 * them, without a Rust parser: `fields!` arguments are plain names and `pub`
 * qualifiers, so bracket balancing with comment/string skipping is sufficient.
 */

export interface FieldsBlock {
  /** Index of the `(` or `{` right after `fields!`. */
  opener: number;
  /** Index of the matching `)` or `}`. */
  closer: number;
  /** Indentation used by the block's entries (multi-line blocks only). */
  indent: string;
}

/** Finds the first `fields!(..)` / `fields! { .. }` block in `text`. */
export function findFirstFieldsBlock(text: string, indentHint = '    '): FieldsBlock | undefined {
  const re = /fields!\s*[({]/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    const opener = m.index + m[0].length - 1;
    const closer = matchCloser(text, opener);
    if (closer === -1) {
      continue; // unbalanced (mid-typing) — try the next block
    }
    const inner = text.slice(opener + 1, closer);
    let indent = indentHint;
    if (inner.includes('\n')) {
      const firstEntry = inner.slice(inner.indexOf('\n') + 1).match(/^[ \t]*\S/);
      if (firstEntry) {
        indent = firstEntry[0].slice(0, firstEntry[0].length - 1);
      }
    }
    return { opener, closer, indent };
  }
  return undefined;
}

function matchCloser(text: string, opener: number): number {
  const open = text[opener];
  const close = open === '(' ? ')' : '}';
  let depth = 0;
  let i = opener;
  while (i < text.length) {
    const c = text[i];
    if (c === '/' && text[i + 1] === '/') {
      while (i < text.length && text[i] !== '\n') {
        i++;
      }
      continue;
    }
    if (c === '/' && text[i + 1] === '*') {
      i += 2;
      while (i < text.length && !(text[i] === '*' && text[i + 1] === '/')) {
        i++;
      }
      i += 2;
      continue;
    }
    if (c === '"') {
      i++;
      while (i < text.length && text[i] !== '"') {
        if (text[i] === '\\') {
          i++;
        }
        i++;
      }
      i++;
      continue;
    }
    if (c === open) {
      depth++;
    } else if (c === close) {
      depth--;
      if (depth === 0) {
        return i;
      }
    }
    i++;
  }
  return -1;
}

export interface Insertion {
  /** Absolute offset into the file text where `snippet` goes. */
  offset: number;
  snippet: string;
}

/**
 * Computes where to insert field declarations so that both layouts stay
 * valid:
 *
 *   fields!(value)          -> fields!(value, name)
 *   fields!(value,)         -> fields!(value, name)
 *   fields! {\n  value,\n}  -> fields! {\n  value,\n  name,\n}
 */
export function insertionFor(text: string, block: FieldsBlock, fields: string[]): Insertion {
  const inner = text.slice(block.opener + 1, block.closer);
  if (!inner.includes('\n')) {
    const trimmedEnd = inner.replace(/[ \t]+$/, '');
    const sep = trimmedEnd.length === 0 ? '' : trimmedEnd.endsWith(',') ? ' ' : ', ';
    return { offset: block.closer, snippet: `${sep}${fields.join(', ')}` };
  }
  const prev = text[block.closer - 1];
  const lead = prev === '\n' ? '' : '\n';
  const body = fields.map(f => `${block.indent}${f},\n`).join('');
  return { offset: block.closer, snippet: `${lead}${body}` };
}

/**
 * The indentation style used by `text`: the most common leading whitespace
 * width, tabs when tab-indented lines dominate, undefined when the text has no
 * indented lines at all.
 */
export function inferIndent(text: string): string | undefined {
  let tabLines = 0;
  const widths = new Map<number, number>();
  for (const m of text.matchAll(/^([ \t]+)\S/gm)) {
    if (m[1].includes('\t')) {
      tabLines++;
    } else {
      widths.set(m[1].length, (widths.get(m[1].length) ?? 0) + 1);
    }
  }
  const spaceLines = [...widths.values()].reduce((a, b) => a + b, 0);
  if (tabLines > 0 && tabLines >= spaceLines) {
    return '\t';
  }
  let best: number | undefined;
  let bestCount = 0;
  for (const [width, count] of widths) {
    if (count > bestCount || (count === bestCount && best !== undefined && width < best)) {
      best = width;
      bestCount = count;
    }
  }
  return best === undefined ? undefined : ' '.repeat(best);
}

/**
 * Whether the file already imports the `fields!` macro (directly, in a brace
 * list, or via a glob). Renames are honored: `fields as f` does not make
 * `fields!` available, while `x as fields` does:
 *
 *   use duck_trait::fields;            -> true
 *   use duck_trait::{fields, props};   -> true
 *   use duck_trait::*;                 -> true
 *   use duck_trait::props;             -> false
 *   use duck_trait::fields as f;       -> false (the macro is renamed)
 *   use duck_trait::fields_macro as fields; -> true
 */
export function hasFieldsImport(text: string): boolean {
  const re = /^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?use[ \t]+([^;]+);/gm;
  for (const m of text.matchAll(re)) {
    const stmt = m[1].trim();
    // part of the duck_trait crate path, but not a longer identifier such as `x_duck_trait`
    if (!/(?<!\w)duck_trait(?!\w)/.test(stmt)) {
      continue;
    }
    const brace = stmt.indexOf('{');
    if (brace !== -1) {
      const inner = stmt.slice(brace + 1, stmt.lastIndexOf('}'));
      for (const item of inner.split(',')) {
        if (importedName(item.trim()) === 'fields') {
          return true;
        }
      }
    } else if (importedName(stmt) === 'fields') {
      return true;
    } else if (stmt.endsWith('.*') || stmt.endsWith('::*')) {
      return true;
    }
  }
  return false;
}

/** The name a `use` item (or statement) makes available, honoring `as` renames. */
function importedName(item: string): string {
  const [path, alias] = item.split(/\s+as\s+/);
  return (alias ?? path.split('::').pop()).trim();
}

/**
 * Computes where to append a fresh `fields!` block when the file has none
 * (e.g. it is empty or only holds comments/imports). The `fields` import is
 * added along with the block when the file does not have it yet:
 *
 *   (empty)                            -> use duck_trait::fields;\n\nfields! {..}
 *   use duck_trait::props;             -> use duck_trait::props;\n\nuse duck_trait::fields;\n\nfields! {..}
 *   use duck_trait::{fields, props};   -> fields! {..} (import already present)
 */
/**
 * Computes where to append a fresh `fields!` block when the file has none
 * (e.g. it is empty or only holds comments/imports). The `fields` import is
 * added along with the block when the file does not have it yet:
 *
 *   (empty)                            -> use duck_trait::fields;\n\nfields! {..}
 *   use duck_trait::props;             -> use duck_trait::props;\nuse duck_trait::fields;\n\nfields! {..}
 *   use duck_trait::{fields, props};   -> fields! {..} (import already present)
 */
export function appendedFieldsBlock(
  text: string,
  fields: string[],
  indent = '    ',
): Insertion {
  const atLineEnd = text.length === 0 || text.endsWith('\n');
  const tail = atLineEnd ? '' : '\n';
  const use = hasFieldsImport(text) ? '' : 'use duck_trait::fields;\n\n';
  // keep one blank line between existing content and the appended block
  const gap = use || text.length === 0 ? '' : atLineEnd ? '\n' : '';
  const body = fields.map(f => `${indent}${f},\n`).join('');
  return { offset: text.length, snippet: `${tail}${gap}${use}fields! {\n${body}}\n` };
}

/**
 * Content written when the target declaration file does not exist yet.
 * Without fields the file holds only the import (an empty `fields!` block
 * would not compile — the macro requires at least one name).
 */
export function newFieldsFileContent(fields: string[], indent = '    '): string {
  const lines = [
    '//! Field-based api declarations — every accessor trait of this crate,',
    '//! declared once here and implemented by `#[props]` structs anywhere.',
    '',
    'use duck_trait::fields;',
  ];
  if (fields.length > 0) {
    lines.push('', 'fields! {', ...fields.map(f => `${indent}${f},`), '}');
  } else {
    lines.push('', '// declare fields here, e.g. fields!(value)');
  }
  return lines.join('\n') + '\n';
}

export interface ModInsertion {
  offset: number;
  snippet: string;
}

/**
 * Computes where to insert `mod <name>;` into the crate root file: right after
 * an existing `use duck_trait::..` line when present, otherwise at the end of
 * the file. Returns undefined if the module is already declared — as a file
 * module (`mod x;`) **or** as an inline module (`mod x { .. }`).
 */
export function modInsertion(crateRootText: string, name: string): ModInsertion | undefined {
  if (
    new RegExp(`^\\s*(?:pub(?:\\([^)]*\\))?\\s+)?mod\\s+${name}\\s*(;|\\{)`, 'm').test(
      crateRootText,
    )
  ) {
    return undefined;
  }
  const useLine = /^use duck_trait::.*$/m.exec(crateRootText);
  if (useLine) {
    const lineEnd = crateRootText.indexOf('\n', useLine.index);
    const offset = lineEnd === -1 ? crateRootText.length : lineEnd;
    return { offset, snippet: `\nmod ${name};` };
  }
  const tail = crateRootText.endsWith('\n') ? '' : '\n';
  return { offset: crateRootText.length, snippet: `${tail}mod ${name};\n` };
}

export interface InlineModule {
  /** Index of the `{` opening the inline module body. */
  opener: number;
  /** Index of the matching `}`. */
  closer: number;
}

/** The body span of an inline `mod <name> { .. }` in `text`, if declared there. */
export function inlineModuleSpan(text: string, name: string): InlineModule | undefined {
  const m = new RegExp(`^\\s*(?:pub(?:\\([^)]*\\))?\\s+)?mod\\s+${name}\\s*\\{`, 'm').exec(text);
  if (!m) {
    return undefined;
  }
  const opener = m.index + m[0].length - 1;
  const closer = matchCloser(text, opener);
  if (closer === -1) {
    return undefined;
  }
  return { opener, closer };
}
