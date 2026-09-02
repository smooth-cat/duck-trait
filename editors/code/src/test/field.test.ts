import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fieldForTrait, resolveFieldName, scanPropFields, traitNameFor } from '../field';

test('traitNameFor mirrors the macro convention', () => {
  assert.equal(traitNameFor('value'), '_Value');
  assert.equal(traitNameFor('my_field'), '_MyField');
  assert.equal(traitNameFor('r#type'), '_Type');
});

test('fieldForTrait inverts simple names', () => {
  assert.equal(fieldForTrait('_Value'), 'value');
  assert.equal(fieldForTrait('_MyField'), 'my_field');
});

test('fieldForTrait writes keywords raw', () => {
  assert.equal(fieldForTrait('_Type'), 'r#type');
});

test('fieldForTrait refuses non-round-tripping guesses', () => {
  // `_A_B` cannot come from any field: every snake_case guess fails to map back
  assert.equal(fieldForTrait('_A_B'), undefined);
  assert.equal(fieldForTrait('Value'), undefined);
  assert.equal(fieldForTrait('_'), undefined);
});

test('fieldForTrait accepts guesses that regenerate the trait', () => {
  // `h_t_t_p` is unusual but `fields!(h_t_t_p)` really does generate `_HTTP`
  assert.equal(fieldForTrait('_HTTP'), 'h_t_t_p');
});

test('scanPropFields reads marked fields', () => {
  const text = `
    #[props]
    struct S {
      #[prop]
      value: i32,
      #[doc = "x"]
      #[prop]
      r#type: u8,
      #[prop] inline: bool,
      plain: (),
    }
  `;
  assert.deepEqual(scanPropFields(text), [
    { field: 'value', trait: '_Value' },
    { field: 'r#type', trait: '_Type' },
    { field: 'inline', trait: '_Inline' },
  ]);
});

test('resolveFieldName prefers the scanned spelling over the guess', () => {
  assert.equal(resolveFieldName('_Value', 'struct S { #[prop] value: i32 }'), 'value');
  assert.equal(resolveFieldName('_Value', 'struct S { other: i32 }'), 'value');
  assert.equal(resolveFieldName('_Type', 'struct S { #[prop] r#type: u8 }'), 'r#type');
});
