/**
 * Matches duck-trait-related unresolved-trait diagnostics.
 *
 * rustc (E0405) and rust-analyzer render the failure of a `#[props]`-generated
 * impl identically, e.g.:
 *
 *   cannot find trait `_Value` in module `crate::_fields`
 */

export interface UnresolvedFieldTrait {
  /** The trait name, including the leading underscore: `_Value`. */
  traitName: string;
  /** The module path where the trait was expected: `crate::_fields`. */
  modulePath: string;
}

/** A missing `mod <name>;` declaration in the crate root (rustc E0433). */
export interface MissingModule {
  modName: string;
}

export type DuckTraitDiag = UnresolvedFieldTrait | MissingModule;

const UNRESOLVED_RE =
  /cannot find (?:trait|type) `(_[A-Za-z_][A-Za-z0-9_]*)` in module `([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)`/;

const MISSING_MOD_RE = /could not find `([A-Za-z_][A-Za-z0-9_]*)` in the crate root/;

export function matchDuckTraitDiag(message: string): DuckTraitDiag | undefined {
  const unresolved = UNRESOLVED_RE.exec(message);
  if (unresolved) {
    return { traitName: unresolved[1], modulePath: unresolved[2] };
  }
  const missingMod = MISSING_MOD_RE.exec(message);
  if (missingMod) {
    return { modName: missingMod[1] };
  }
  return undefined;
}
