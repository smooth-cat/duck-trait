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

/**
 * The trait is referenced in the file's own scope (generic bound, impl, ...)
 * without being imported — rustc E0405:
 *
 *   cannot find trait `_Value` in this scope
 *
 * Unlike the generated-impl errors above, no module path is reported; the
 * declaration module is inferred from the file's `#[props]` structs.
 */
export interface UnresolvedScopeTrait {
  traitName: string;
}

export type DuckTraitDiag = UnresolvedFieldTrait | MissingModule | UnresolvedScopeTrait;

const UNRESOLVED_RE =
  /cannot find (?:trait|type) `(_[A-Za-z_][A-Za-z0-9_]*)` in module `([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)`/;

const MISSING_MOD_RE = /could not find `([A-Za-z_][A-Za-z0-9_]*)` in the crate root/;

const SCOPE_TRAIT_RE = /cannot find trait `(_[A-Za-z_][A-Za-z0-9_]*)` in this scope/;

export function matchDuckTraitDiag(message: string): DuckTraitDiag | undefined {
  const unresolved = UNRESOLVED_RE.exec(message);
  if (unresolved) {
    return { traitName: unresolved[1], modulePath: unresolved[2] };
  }
  const missingMod = MISSING_MOD_RE.exec(message);
  if (missingMod) {
    return { modName: missingMod[1] };
  }
  const scopeTrait = SCOPE_TRAIT_RE.exec(message);
  if (scopeTrait) {
    return { traitName: scopeTrait[1] };
  }
  return undefined;
}
