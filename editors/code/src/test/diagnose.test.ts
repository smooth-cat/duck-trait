import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matchDuckTraitDiag } from '../diagnose';

test('matches the rustc E0405 message', () => {
  const hit = matchDuckTraitDiag(
    "cannot find trait `_Value` in module `crate::_fields`\n\n --> src/foo.rs:7:22",
  );
  assert.deepEqual(hit, { traitName: '_Value', modulePath: 'crate::_fields' });
});

test('matches an overridden module path', () => {
  const hit = matchDuckTraitDiag('cannot find trait `_Tag` in module `crate::override_fields`');
  assert.deepEqual(hit, { traitName: '_Tag', modulePath: 'crate::override_fields' });
});

test('matches the `type` wording', () => {
  const hit = matchDuckTraitDiag('cannot find type `_V` in module `crate::_fields`');
  assert.deepEqual(hit, { traitName: '_V', modulePath: 'crate::_fields' });
});

test('matches the missing module declaration (E0433)', () => {
  const hit = matchDuckTraitDiag(
    "could not find `_fields` in the crate root\n\n --> src/lib.rs:3:1",
  );
  assert.deepEqual(hit, { modName: '_fields' });
});

test('matches a trait referenced in this scope without an import (E0405)', () => {
  const hit = matchDuckTraitDiag(
    "cannot find trait `_Value` in this scope\n\n --> src/api.rs:4:12",
  );
  assert.deepEqual(hit, { traitName: '_Value' });
});

test('rejects non-underscored traits and unrelated messages', () => {
  assert.equal(
    matchDuckTraitDiag('cannot find trait `Display` in module `core::fmt`'),
    undefined,
  );
  assert.equal(matchDuckTraitDiag('cannot find trait `Display` in this scope'), undefined);
  assert.equal(matchDuckTraitDiag('borrowed value does not implement any call'), undefined);
  assert.equal(matchDuckTraitDiag(''), undefined);
});
