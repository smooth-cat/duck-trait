/**
 * Maps between field names and duck-trait accessor trait names, mirroring the
 * naming convention of the `fields!` macro:
 *
 *   field `value`    -> trait `_Value<T>`   (getter `value`)
 *   field `my_field` -> trait `_MyField<T>` (getter `my_field`)
 *   field `r#type`   -> trait `_Type<T>`    (getter `r#type`)
 */

/** Rust strict keywords — a derived field name equal to one of these must be written raw (`r#type`). */
const STRICT_KEYWORDS = new Set([
  'as', 'break', 'const', 'continue', 'crate', 'else', 'enum', 'extern', 'false', 'fn', 'for',
  'if', 'impl', 'in', 'let', 'loop', 'match', 'mod', 'move', 'mut', 'pub', 'ref', 'return',
  'self', 'static', 'struct', 'super', 'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while',
  'async', 'await', 'dyn',
]);

/** `value` -> `_Value`, `my_field` -> `_MyField`, `r#type` -> `_Type`. */
export function traitNameFor(field: string): string {
  const name = field.replace(/^r#/, '');
  let out = '';
  for (const segment of name.split('_')) {
    if (!segment) {
      continue;
    }
    out += segment[0].toUpperCase() + segment.slice(1);
  }
  return out ? `_${out}` : '';
}

/** `_Value` -> `value`, `_MyField` -> `my_field`, `_Type` -> `r#type`. */
export function fieldForTrait(traitName: string): string | undefined {
  if (!traitName.startsWith('_') || traitName.length < 2) {
    return undefined;
  }
  const pascal = traitName.slice(1);
  const snake = pascal.replace(
    /[A-Z]/g,
    (c, offset) => (offset === 0 ? c.toLowerCase() : `_${c.toLowerCase()}`),
  );
  // a lossy guess (`_HTTP` -> `h_t_t_p`) would not round-trip; refuse it
  if (traitNameFor(snake) !== traitName) {
    return undefined;
  }
  return STRICT_KEYWORDS.has(snake) ? `r#${snake}` : snake;
}

export interface PropField {
  field: string;
  trait: string;
}

/**
 * Scans Rust source for `#[prop]`-marked named fields and returns their names
 * together with the accessor trait each one maps to.
 */
export function scanPropFields(text: string): PropField[] {
  const re = /#\[prop\]\s*(?:#\[[^\]]*\]\s*)*(r#)?([A-Za-z_][A-Za-z0-9_]*)\s*:/g;
  const out: PropField[] = [];
  for (const m of text.matchAll(re)) {
    const field = (m[1] ?? '') + m[2];
    const trait = traitNameFor(field);
    if (trait) {
      out.push({ field, trait });
    }
  }
  return out;
}

/**
 * Determines the field name to declare for `traitName`: the real spelling of a
 * `#[prop]` field found in `docText` wins; otherwise the trait name is guessed
 * back into a snake_case field name.
 */
export function resolveFieldName(traitName: string, docText: string): string | undefined {
  const hit = scanPropFields(docText).find(f => f.trait === traitName);
  if (hit) {
    return hit.field;
  }
  return fieldForTrait(traitName);
}
