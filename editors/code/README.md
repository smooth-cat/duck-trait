# duck-trait — VS Code quick fixes

Quick fixes for the [`duck-trait`](https://github.com/smooth-cat/duck-trait) field-based api
(`fields!` + `#[props]`).

When rust-analyzer or rustc reports

```
cannot find trait `_Value` in module `crate::_fields`
```

this extension adds a `Cmd+.` (Quick Fix) entry that declares the missing field in the crate's
`fields!` module:

- **`duck-trait: declare \`value\` in src/_fields.rs`** — inserts the field name into the first
  `fields!` block of the target file (both `fields!(..)` and `fields! { .. }` layouts, comments and
  trailing commas handled).
- **`duck-trait: create src/_fields.rs and declare \`value\`** — offered when the target file does
  not exist yet: creates it with a starter `fields!` block and writes `mod _fields;` into the crate
  root (`src/lib.rs` / `src/main.rs`).

A related failure, `could not find \`_fields\` in the crate root`, gets a fix that adds the missing
`mod _fields;` declaration to the crate root when the declaration file already exists.

When more than one field of the current file is missing, the menu also offers
**`duck-trait: declare all missing fields (N) ...`** — a single click declares every missing field
of the file at once (deduplicated and grouped per declaration module), and it sorts above the
per-field fixes.

All fixes are offered from anywhere in the file: the cursor does not have to sit on the failing
`impl` (the diagnostics live on generated code that is not visible in the editor).

## Command palette (`Cmd+Shift+P`)

| Command | Behavior |
| --- | --- |
| `duck-trait: Declare missing fields of this file in _fields.rs` | Scans the active file for `#[props]` structs and declares every field that is missing from its declaration module (`#[_prop]`-ignored fields excluded, honors `#[props(path = ..)]` overrides). |
| `duck-trait: Declare all missing fields of the crate in _fields.rs` | Same, recursively for every `.rs` file under the crate's source directory. |
| `duck-trait: Create _fields.rs` | Scaffolds the declaration file (header comment + `use duck_trait::fields;`, no empty `fields!` block — that would not compile) and writes `mod _fields;` into the entry file. Opens the file when it already exists. |

Both declare commands reuse the quick fix pipeline: missing imports are added, indentation follows
the crate's `rustfmt.toml` (or inferred from existing files), and edited files are saved
immediately.

Declaration modules declared **inline** in the entry file (`mod override_fields { fields! { .. } }`)
are supported everywhere: fields are appended inside the inline module's `fields!` block, no
declaration file is created and no duplicate `mod` declaration is inserted.

When the active file does not belong to a crate (e.g. a monorepo root), the commands list the
workspace members (`[workspace] members`, single-level `crates/*` globs supported) as a picker.
Crates whose entry file is not `src/lib.rs` are supported via the `[lib] path` / `[[bin]] path`
keys in their Cargo.toml — sources and `_fields.rs` then live next to the entry file.

Each fix is carried entirely by a command (`duck-trait.applyFix`): the edit is applied through a
workspace edit and every touched file is saved immediately, so rust-analyzer re-analyzes without a
manual `Cmd+S`. Insertion points are computed against the live state of the target file — the
editor buffer when the file is open (unsaved changes survive), otherwise disk.

The field name is resolved from the real field spelling in the erroneous file when possible
(this handles raw identifiers such as `r#type`); otherwise the trait name is guessed back into
snake_case (`_MyField` -> `my_field`).

## Install

Build the extension package from this directory:

```sh
npm install
npm run package      # produces duck-trait-<version>.vsix
```

Then install it:

```sh
code --install-extension duck-trait-<version>.vsix
```

or via the VS Code UI: *Extensions* → *…* → *Install from VSIX…*.

## Prerequisites

- rust-analyzer with proc-macro expansion enabled (default):
  - `rust-analyzer.procMacro.enable = true`
  - `rust-analyzer.procMacro.attributes.enable = true`

## Limitations

- Overridden module paths (`#[props(path = crate::a::b)]`) resolve to `src/a/b.rs` only when that
  file already exists; creating missing files is supported for single-segment modules only
  (`crate::_fields`).
- Multiple missing fields produce one diagnostic each — every one gets its own quick fix, plus an
  all-in-one fix when more than one field is missing.
- The declaration is inserted into the first `fields!` block of the target file; a file without any
  `fields!` block (e.g. an empty one) gets a fresh block appended at the end.
- `mod.rs` layouts and `#[path = ..]` module redirections are not resolved.
- Workspace member globs support a single `*` level; nested globs and `{a,b}` alternatives are not
  expanded.

## License

Dual-licensed under [MIT](../LICENSE) or [Apache-2.0](../LICENSE-APACHE), at your option.
