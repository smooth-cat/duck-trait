import { test } from 'node:test';
import assert from 'node:assert/strict';
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
} from '../fieldsFile';
import { declaredFields } from '../scan';

test('finds a multi-line brace block and its entry indentation', () => {
  const text = 'use duck_trait::fields;\n\nfields! {\n    value,\n}\n';
  const block = findFirstFieldsBlock(text);
  assert.ok(block);
  assert.equal(block.indent, '    ');
  const at = insertionFor(text, block, ['name']);
  assert.equal(at.offset, text.indexOf('}'));
  assert.equal(at.snippet, '    name,\n');
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'use duck_trait::fields;\n\nfields! {\n    value,\n    name,\n}\n');
});

test('inserts several fields at once', () => {
  const text = 'fields! {\n    value,\n}\n';
  const block = findFirstFieldsBlock(text)!;
  const at = insertionFor(text, block, ['a', 'b']);
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'fields! {\n    value,\n    a,\n    b,\n}\n');
});

test('finds a single-line paren block', () => {
  const text = 'fields!(value)\n';
  const block = findFirstFieldsBlock(text);
  assert.ok(block);
  const at = insertionFor(text, block, ['name']);
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'fields!(value, name)\n');
});

test('handles single-line trailing comma and empty parens', () => {
  const withComma = 'fields!(value,)';
  let block = findFirstFieldsBlock(withComma)!;
  let at = insertionFor(withComma, block, ['name']);
  assert.equal(withComma.slice(0, at.offset) + at.snippet + withComma.slice(at.offset),
    'fields!(value, name)');

  const empty = 'fields!()';
  block = findFirstFieldsBlock(empty)!;
  at = insertionFor(empty, block, ['name']);
  assert.equal(empty.slice(0, at.offset) + at.snippet + empty.slice(at.offset),
    'fields!(name)');
});

test('handles an empty multi-line block', () => {
  const text = 'fields! {\n}\n';
  const block = findFirstFieldsBlock(text)!;
  const at = insertionFor(text, block, ['value']);
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'fields! {\n    value,\n}\n');
});

test('skips comments while balancing and picks the first complete block', () => {
  const text = 'fields! {\n    // braces } in a comment\n    value,\n}\n';
  const block = findFirstFieldsBlock(text)!;
  const at = insertionFor(text, block, ['name']);
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'fields! {\n    // braces } in a comment\n    value,\n    name,\n}\n');
});

test('returns undefined without a fields! block', () => {
  assert.equal(findFirstFieldsBlock('fn main() {}'), undefined);
});

test('appendedFieldsBlock writes import and block into an empty file', () => {
  const at = appendedFieldsBlock('', ['value']);
  assert.equal(at.offset, 0);
  assert.equal(at.snippet, 'use duck_trait::fields;\n\nfields! {\n    value,\n}\n');
});

test('appendedFieldsBlock appends after existing content', () => {
  const text = 'use duck_trait::fields;\n';
  const at = appendedFieldsBlock(text, ['value']);
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'use duck_trait::fields;\n\nfields! {\n    value,\n}\n');
});

test('appendedFieldsBlock skips the import when present in a brace list', () => {
  const text = 'use duck_trait::{fields, props};\n';
  const at = appendedFieldsBlock(text, ['value']);
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'use duck_trait::{fields, props};\n\nfields! {\n    value,\n}\n');
});

test('appendedFieldsBlock adds the import when missing', () => {
  const text = 'use duck_trait::props;\n';
  const at = appendedFieldsBlock(text, ['value']);
  // the two use declarations stay grouped, rustfmt-style
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'use duck_trait::props;\nuse duck_trait::fields;\n\nfields! {\n    value,\n}\n');
});

test('appendedFieldsBlock normalizes a missing trailing newline', () => {
  const at = appendedFieldsBlock('//! doc', ['value']);
  assert.equal(at.snippet, '\nuse duck_trait::fields;\n\nfields! {\n    value,\n}\n');
});

test('appendedFieldsBlock honors a custom indent and several fields', () => {
  const at = appendedFieldsBlock('', ['value', 'name'], '  ');
  assert.equal(at.snippet, 'use duck_trait::fields;\n\nfields! {\n  value,\n  name,\n}\n');
});

test('fieldsImportInsertion places the import in front of a bare fields! block', () => {
  const text = 'fields! {\n    value,\n}\n';
  const at = fieldsImportInsertion(text)!;
  assert.equal(at.offset, 0);
  assert.equal(at.snippet, 'use duck_trait::fields;\n\n');
});

test('fieldsImportInsertion groups with the leading docs and existing imports', () => {
  const text = '//! module docs\nuse duck_trait::props;\n\nfields! {\n    value,\n}\n';
  const at = fieldsImportInsertion(text)!;
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    '//! module docs\nuse duck_trait::props;\nuse duck_trait::fields;\n\nfields! {\n    value,\n}\n');
});

test('fieldsImportInsertion keeps an existing import intact (no double import)', () => {
  assert.equal(fieldsImportInsertion('use duck_trait::fields;\n\nfields! {\n    value,\n}\n'), undefined);
  assert.equal(fieldsImportInsertion('use duck_trait::{fields, props};\nfields!(value)'), undefined);
  assert.equal(fieldsImportInsertion('use duck_trait::*;\nfields!(value)'), undefined);
  // renamed imports do not make `fields!` available — the insert is still offered
  assert.ok(fieldsImportInsertion('use duck_trait::fields as f;\nfields!(value)'));
});

test('fieldsImportInsertion fixes a missing trailing newline before the block', () => {
  const text = 'use duck_trait::props;';
  const at = fieldsImportInsertion(text)!;
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'use duck_trait::props;\nuse duck_trait::fields;\n');
});

test('fieldsImportInsertion indents imports meant for an inline module body', () => {
  const at = fieldsImportInsertion('fields! {\n  value,\n}\n', '  ')!;
  assert.equal(at.snippet, '  use duck_trait::fields;\n\n');
});

test('a fields! block without the import ends up with both after the fix', () => {
  const text = 'fields! {\n    value,\n}\n';
  const block = findFirstFieldsBlock(text)!;
  const at = insertionFor(text, block, ['name']);
  const afterField = text.slice(0, at.offset) + at.snippet + text.slice(at.offset);
  const imp = fieldsImportInsertion(afterField)!;
  const fixed = afterField.slice(0, imp.offset) + imp.snippet + afterField.slice(imp.offset);
  assert.equal(fixed, 'use duck_trait::fields;\n\nfields! {\n    value,\n    name,\n}\n');
  // and a repeat run adds nothing — import present, field already declared
  assert.equal(fieldsImportInsertion(fixed), undefined);
  assert.deepEqual(declaredFields(fixed), ['value', 'name']);
});

test('hasFieldsImport detects direct, brace-list and glob imports', () => {
  assert.equal(hasFieldsImport('use duck_trait::fields;'), true);
  assert.equal(hasFieldsImport('use duck_trait::{fields, props};'), true);
  assert.equal(hasFieldsImport('use duck_trait::*;'), true);
  assert.equal(hasFieldsImport('pub use duck_trait::fields;'), true);
  assert.equal(hasFieldsImport('use duck_trait::props;'), false);
  assert.equal(hasFieldsImport('use duck_trait::fields as f;'), false);
  assert.equal(hasFieldsImport('use duck_trait::fields_macro as fields;'), true);
  assert.equal(hasFieldsImport('use std::fields;'), false);
  assert.equal(hasFieldsImport(''), false);
});

test('inferIndent picks the dominant style', () => {
  assert.equal(inferIndent('struct A {\n  a: u8,\n  b: u8,\n}\n'), '  ');
  assert.equal(inferIndent('struct A {\n    a: u8,\n    b: u8,\n}\n'), '    ');
  assert.equal(inferIndent('fn f() {\n\tx();\n\ty();\n}\n'), '\t');
  assert.equal(inferIndent('fn main() {}\n'), undefined);
  assert.equal(inferIndent(''), undefined);
});

test('newFieldsFileContent honors the indent', () => {
  assert.ok(newFieldsFileContent(['value'], '  ').includes('fields! {\n  value,\n}'));
  assert.ok(newFieldsFileContent(['value']).includes('fields! {\n    value,\n}'));
  // no fields -> only the import, an empty fields! block would not compile
  const empty = newFieldsFileContent([]);
  assert.ok(empty.includes('use duck_trait::fields;'));
  assert.ok(!empty.includes('fields! {'));
});

test('modInsertion goes after the duck_trait use line', () => {
  const text = '//! doc\n\nuse duck_trait::props;\nuse duck_trait::fields;\n\nfn main() {}\n';
  const at = modInsertion(text, '_fields')!;
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    '//! doc\n\nuse duck_trait::props;\nmod _fields;\nuse duck_trait::fields;\n\nfn main() {}\n');
});

test('modInsertion appends without a duck_trait use line', () => {
  const text = 'fn main() {}\n';
  const at = modInsertion(text, '_fields')!;
  assert.equal(text.slice(0, at.offset) + at.snippet + text.slice(at.offset),
    'fn main() {}\nmod _fields;\n');
});

test('modInsertion is a no-op when already declared', () => {
  assert.equal(modInsertion('mod _fields;\n', '_fields'), undefined);
});

test('modInsertion is a no-op for inline module declarations', () => {
  const text = 'mod override_fields {\n  fields!(tag,)\n}\n';
  assert.equal(modInsertion(text, 'override_fields'), undefined);
  assert.equal(modInsertion('pub mod _fields {\n}\n', '_fields'), undefined);
});

test('inlineModuleSpan locates the balanced module body', () => {
  const text = 'use duck_trait::fields;\n\nmod _fields {\n  fields! {\n    value,\n  }\n}\n';
  const span = inlineModuleSpan(text, '_fields');
  assert.ok(span);
  const body = text.slice(span.opener + 1, span.closer);
  assert.ok(body.includes('fields! {'), 'body must contain the fields! block');
  assert.equal(text[span.closer], '}');
  assert.equal(inlineModuleSpan(text, 'other'), undefined);
});
