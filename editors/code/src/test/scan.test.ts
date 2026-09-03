import { test } from 'node:test';
import assert from 'node:assert/strict';
import { declaredFields, hasDependency, scanPropsStructs, workspaceMembers } from '../scan';

test('scanPropsStructs collects every named field of structs', () => {
  const text = `
    #[props]
    struct A {
      value: i32,
      plain: u8,
    }

    #[props]
    pub struct B<T: Clone> where T: Send {
      my_field: T,
    }
  `;
  assert.deepEqual(scanPropsStructs(text), [
    { modulePath: undefined, fields: ['value', 'plain'] },
    { modulePath: undefined, fields: ['my_field'] },
  ]);
});

test('scanPropsStructs reads the path override', () => {
  const text = `
    #[props(path = crate::override_fields)]
    struct T {
      tag: String,
    }
  `;
  assert.deepEqual(scanPropsStructs(text), [{ modulePath: 'crate::override_fields', fields: ['tag'] }]);
});

test('scanPropsStructs excludes #[_prop]-ignored fields and traits', () => {
  const text = `
    #[props(value: i32)]
    trait Show {}

    #[props]
    struct S {
      value: u8,
      #[_prop]
      ignored: u8,
    }
  `;
  assert.deepEqual(scanPropsStructs(text), [{ modulePath: undefined, fields: ['value'] }]);
});

test('scanPropsStructs handles raw identifiers and keeps ignore across attributes', () => {
  const text = `
    #[props]
    #[serde(rename_all = "snake_case")]
    struct K {
      /// doc comment
      #[prop]
      legacy: u8,
      r#type: u8,
      #[doc = "x"]
      #[_prop]
      hidden: String,
      name: String,
    }
  `;
  const structs = scanPropsStructs(text);
  assert.equal(structs.length, 1);
  // `#[prop]` is a stale marker: the field still counts (it compiles as an error anyway)
  assert.deepEqual(structs[0].fields, ['legacy', 'r#type', 'name']);
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

test('hasDependency detects plain, table and path dependencies', () => {
  assert.equal(hasDependency('[dependencies]\nduck-trait = "0.13.0"\nserde = "1"\n', 'duck-trait'), true);
  assert.equal(hasDependency('[dependencies]\nduck-trait = { path = "../duck-trait" }\n', 'duck-trait'), true);
  assert.equal(hasDependency('[package]\nname = "x"\n\n[dependencies]\nserde = "1"\n', 'duck-trait'), false);
});

test('hasDependency detects workspace inheritance and dotted keys', () => {
  assert.equal(hasDependency('[dependencies]\nduck-trait.workspace = true\n', 'duck-trait'), true);
  assert.equal(hasDependency('[dependencies]\nduck-trait = { workspace = true }\n', 'duck-trait'), true);
});

test('hasDependency detects renamed packages, single- and multi-line', () => {
  assert.equal(
    hasDependency('[dependencies]\ndt = { path = "..", package = "duck-trait" }\n', 'duck-trait'),
    true,
  );
  assert.equal(
    hasDependency(
      '[dependencies]\ndt = {\n  path = "..",\n  package = "duck-trait",\n}\n',
      'duck-trait',
    ),
    true,
  );
});

test('hasDependency ignores prefix names, comments and unrelated crates', () => {
  const text = '[dependencies]\nduck-trait-lite = "1"\n# duck-trait = "0.1"\nserde = "1"\n';
  assert.equal(hasDependency(text, 'duck-trait'), false);
});

test('hasDependency covers dev- and build-dependencies', () => {
  assert.equal(hasDependency('[dev-dependencies]\nduck-trait = { path = "../duck-trait" }\n', 'duck-trait'), true);
  assert.equal(hasDependency('[build-dependencies]\nduck-trait = "1"\n', 'duck-trait'), true);
});

test('hasDependency ignores workspace.dependencies the member does not opt into', () => {
  const text = [
    '[workspace.dependencies]',
    'duck-trait = { path = "crates/duck-trait" }',
    '',
    '[dependencies]',
    'serde = "1"',
  ].join('\n');
  assert.equal(hasDependency(text, 'duck-trait'), false);
});

test('hasDependency reads the verify manifest as a duck-trait consumer', () => {
  const verify = '[dependencies]\nduck-trait = { path = "../duck-trait" }\n';
  assert.equal(hasDependency(verify, 'duck-trait'), true);
  assert.equal(hasDependency('', 'duck-trait'), false);
});
