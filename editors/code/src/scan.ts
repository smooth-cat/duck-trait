/**
 * Pure-text scanning for the palette commands: which `#[prop]` fields a file
 * declares through `#[props]` structs, which fields a `fields!` module already
 * declares, and which crates a workspace lists.
 *
 * No Rust parser — the shapes involved (`#[props]` on structs, `fields!`
 * arguments) are simple enough for balanced-bracket scanning, exactly like the
 * quick fix path.
 */

import { findFirstFieldsBlock } from './fieldsFile';

export interface PropsStruct {
  /** `path = ..` from `#[props(path = crate::x)]`, e.g. `crate::x`. */
  modulePath?: string;
  /** The `#[prop]`-marked fields of the struct, raw spellings (`r#type` kept). */
  fields: string[];
}

const PROPS_STRUCT_RE =
  /#\[props(?:\([^)]*\))?\]\s*(?:#[^\n]*\s*)*(?:pub(?:\([^)]*\))?\s+)?struct\s+/;
const PATH_ARG_RE = /(?:^|[{,\s])path\s*=\s*([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)/;

/** Matches the balanced-bracket span starting at `text[opener]` ('(' or '{'). */
function matchBracket(text: string, opener: number): number {
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

/**
 * Scans Rust source for structs annotated with `#[props]` / `#[props(path = ..)]`
 * and collects their fields. Every named field counts (they generate accessors
 * by default) except the ones ignored with `#[_prop]`. `#[props(name: Type)]`
 * on **traits** is ignored — traits do not declare fields.
 */
export function scanPropsStructs(text: string): PropsStruct[] {
  const out: PropsStruct[] = [];
  const re = new RegExp(PROPS_STRUCT_RE.source, 'g');
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    // the attribute may sit on a trait (`#[props(value: i32)]`) — only structs count
    const attrArgs = /#\[props(?:\(([^)]*)\))?\]/.exec(m[0]);
    const brace = text.indexOf('{', m.index + m[0].length);
    if (brace === -1) {
      continue; // unbalanced / mid-typing
    }
    const closer = matchBracket(text, brace);
    if (closer === -1) {
      continue;
    }
    const body = text.slice(brace + 1, closer);
    const modulePath = attrArgs?.[1] ? PATH_ARG_RE.exec(attrArgs[1])?.[1] : undefined;
    out.push({ modulePath, fields: scanFields(body) });
    re.lastIndex = closer; // continue scanning after the struct body
  }
  return out;
}

/**
 * Named fields of a struct body: every `name:` entry — the inverse logic of
 * the old opt-in api, `#[_prop]`-ignored fields are excluded. Attribute lines
 * and comments keep the pending ignore state, so a `#[_prop]` followed by doc
 * comments still excludes its field.
 */
function scanFields(body: string): string[] {
  const fields: string[] = [];
  let ignore = false;
  const collect = (line: string): void => {
    const m = line.match(/^(?:pub(?:\([^)]*\))?\s+)?(r#)?([A-Za-z_][A-Za-z0-9_]*)\s*:/);
    if (m) {
      if (!ignore) {
        fields.push((m[1] ?? '') + m[2]);
      }
      ignore = false;
    }
  };
  for (const raw of body.split('\n')) {
    const line = raw.trim();
    if (line.startsWith('#[')) {
      if (/#\[_prop\]/.test(line)) {
        ignore = true;
      }
      // the field may share the line with its attributes: `#[prop] inline: bool`
      const rest = line.replace(/#\[[^\]]*\]/g, '').trim();
      if (rest) {
        collect(rest);
      }
      continue;
    }
    if (line.startsWith('//') || line === '') {
      continue;
    }
    collect(line);
  }
  return fields;
}

/**
 * Every field name already declared by the `fields!` blocks of `text`
 * (all blocks, comments skipped, raw identifiers kept as `r#type`).
 */
export function declaredFields(text: string): string[] {
  const out: string[] = [];
  let from = 0;
  for (;;) {
    const block = findFirstFieldsBlock(text.slice(from));
    if (!block) {
      break;
    }
    const opener = from + block.opener;
    const closer = from + block.closer;
    const inner = text.slice(opener + 1, closer).replace(/\/\/[^\n]*/g, '');
    // entries are comma separated in both layouts — single-line blocks
    // (`fields!(inner, r#type)`) put several of them on one line
    for (const entry of inner.split(',')) {
      const name = entry.trim().match(/^(?:pub(?:\([^)]*\))?\s+)?(r#)?([A-Za-z_][A-Za-z0-9_]*)$/);
      if (name) {
        out.push((name[1] ?? '') + name[2]);
      }
    }
    from = closer + 1;
  }
  return out;
}

/** The `members` entries of a `[workspace]` Cargo.toml (literal paths and `*` globs). */
export function workspaceMembers(cargoTomlText: string): string[] {
  const workspaceStart = /^\[workspace\]/m.exec(cargoTomlText);
  if (!workspaceStart) {
    return [];
  }
  const rest = cargoTomlText.slice(workspaceStart.index + workspaceStart[0].length);
  const nextSection = /^\[/m.exec(rest);
  const section = nextSection ? rest.slice(0, nextSection.index) : rest;

  const array = /members\s*=\s*\[([\s\S]*?)\]/.exec(section);
  if (!array) {
    return [];
  }
  const members: string[] = [];
  for (const item of array[1].matchAll(/"([^"]+)"/g)) {
    if (!members.includes(item[1])) {
      members.push(item[1]);
    }
  }
  return members;
}
