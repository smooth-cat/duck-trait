import { test } from 'node:test';
import assert from 'node:assert/strict';
import { declaredFields, scanPropsStructs, workspaceMembers } from '../scan';

test('scanPropsStructs collects marked fields from structs', () => {
  const text = `
    #[props]
    struct A {
      #[prop]
      value: i32,
      plain: u8,
    }

    #[props]
    pub struct B<T: Clone> where T: Send {
      #[prop]
      my_field: T,
    }
  `;
  assert.deepEqual(scanPropsStructs(text), [
    { modulePath: undefined, fields: ['value'] },
    { modulePath: undefined, fields: ['my_field'] },
  ]);
});

test('scanPropsStructs reads the path override', () => {
  const text = `
    #[props(path = crate::override_fields)]
    struct T {
      #[prop]
      tag: String,
    }
  `;
  assert.deepEqual(scanPropsStructs(text), [{ modulePath: 'crate::override_fields', fields: ['tag'] }]);
});

test('scanPropsStructs ignores traits and bare #[prop] structs', () => {
  const text = `
    #[props(value: i32)]
    trait Show {}

    struct Plain {
      #[prop]
      orphan: u8,
    }
  `;
  assert.deepEqual(scanPropsStructs(text), []);
});

test('scanPropsStructs handles raw identifiers and other attributes', () => {
  const text = `
    #[props]
    #[serde(rename_all = "snake_case")]
    struct K {
      /// doc comment
      #[prop]
      r#type: u8,
      #[prop]
      #[doc = "x"]
      name: String,
    }
  `;
  const structs = scanPropsStructs(text);
  assert.equal(structs.length, 1);
  assert.deepEqual(structs[0].fields, ['r#type', 'name']);
});

test('declaredFields reads every fields! block', () => {
  const text = `
    fields! {
      value,
      pub name, // trailing comment
    }

    fields!(inner, r#type);
  `;
  assert.deepEqual(declaredFields(text), ['value', 'name', 'inner', 'r#type']);
});

test('declaredFields skips comments and non-entry lines', () => {
  const text = 'fields! {\n    // value,\n    count,\n}\n';
  assert.deepEqual(declaredFields(text), ['count']);
});

test('declaredFields is empty without blocks', () => {
  assert.deepEqual(declaredFields('fn main() {}'), []);
});

test('workspaceMembers lists literals and globs', () => {
  const manifest = [
    '[workspace]',
    'resolver = "2"',
    'members = [',
    '  "crates/duck-trait",',
    '  "crates/*",',
    '  "editors/code",',
    ']',
    '',
    '[profile.dev]',
    'debug = 0',
  ].join('\n');
  assert.deepEqual(workspaceMembers(manifest), [
    'crates/duck-trait',
    'crates/*',
    'editors/code',
  ]);
});

test('workspaceMembers is empty without a workspace section', () => {
  assert.deepEqual(workspaceMembers('[package]\nname = "x"\n'), []);
});
