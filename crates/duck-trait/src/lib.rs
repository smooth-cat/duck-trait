//! `duck-trait` — stop repeating `get`/`set`/`get_mut` declarations in traits.
//!
//! Mark struct fields with `#[duck]` (public) or `#[_duck]` (private) inside a
//! scope wrapped in `ducks! { .. }` (or annotated with `#[duck_mod]`), and the
//! macros generate one accessor trait per field name together with the impls
//! for every marked struct:
//!
//! ```rust
//! use duck_trait::ducks;
//!
//! ducks! {
//!     pub struct Player {
//!         #[duck]
//!         name: String,
//!     }
//! }
//!
//! let mut player = Player { name: "duck".to_owned() };
//! player.name_set("silly duck".to_owned());
//! assert_eq!(player.name(), "silly duck");
//! ```
//!
//! Structs sharing a field name share the same generated trait, regardless of
//! the field type — that is the whole point: write traits once, implement for
//! every type that "has a `name`".
//!
//! ## Custom traits
//!
//! `#[duck(MyTrait(..))]` additionally generates `impl MyTrait(..) for the
//! struct` right after the accessor impl. A bare `_` inside the parentheses
//! stands for the marked field's type; all other arguments are kept verbatim
//! and may reference the struct's own generics:
//!
//! ```rust
//! use duck_trait::ducks;
//!
//! ducks! {
//!     pub struct B {
//!         #[duck(MyValue<_>)] // also generates: impl MyValue<String> for B
//!         value: String,
//!     }
//!
//!     trait MyValue<T>: _Value<T> {
//!         fn my_get(&self) -> &T {
//!             self.value()
//!         }
//!     }
//! }
//!
//! let b = B { value: String::from("hi") };
//! assert_eq!(b.my_get(), "hi");
//! ```
//!
//! ## Props: write the trait first
//!
//! `#[props(..)]` declares the data a trait needs on the trait itself, so it
//! no longer has to wait for a struct to exist. The macro generates a shadow
//! trait `_Show` with `xxx()` / `xxx_set()` / `xxx_mut()` accessors for every
//! prop and binds `Show` to it as a supertrait:
//!
//! ```rust
//! use duck_trait::{duck, props};
//!
//! #[props(name: String, score: i32)]
//! pub trait Show {
//!     fn show(&self) {
//!         println!("{}: {}", self.name(), self.score());
//!     }
//! }
//!
//! #[duck(_Show{name, score})]
//! struct Player {
//!     name: String,
//!     score: i32,
//! }
//!
//! impl Show for Player {}
//!
//! Player { name: "duck".to_owned(), score: 7 }.show();
//! ```
//!
//! The shadow trait copies visibility, generics and where clauses verbatim
//! from the annotated trait. A prop type may reference a same-named generic
//! of the trait; the struct side then spells the arguments out explicitly
//! (struct generics may be used as well):
//!
//! ```rust
//! use duck_trait::{duck, props};
//!
//! #[props(inner: T)]
//! trait Has<T> {
//!     fn get(&self) -> &T {
//!         self.inner()
//!     }
//! }
//!
//! #[duck(_Has<T>{inner})]
//! struct Wrapper<T> {
//!     inner: T,
//! }
//!
//! impl<T> Has<T> for Wrapper<T> {}
//!
//! assert_eq!(Wrapper { inner: 7 }.get(), &7);
//! ```
//!
//! Rules enforced at compile time:
//!
//! - every prop listed in `#[duck(_Trait{..})]` must match a same-named field:
//!
//! ```compile_fail
//! use duck_trait::{duck, props};
//!
//! #[props(name: String)]
//! trait Show {}
//!
//! #[duck(_Show{name, score})]
//! struct A {
//!     name: String,
//! }
//! ```
//!
//! - generated accessor names must not collide (a prop named `a_set` clashes
//!   with the setter of a prop named `a`):
//!
//! ```compile_fail
//! use duck_trait::props;
//!
//! #[props(a: String, a_set: i32)]
//! trait Conflicting {}
//! ```
//!
//! - props not listed in `#[duck(..)]` leave the trait unimplemented (the
//!   compiler reports the missing method on the generated impl), and the impl
//!   method signatures are built from the field types, so a field whose type
//!   differs from the prop type is reported there as well.
//!
//! ## `#[ducky]`: brace-less entries in a module scope
//!
//! Inside a `#[ducky]` module, `#[props]` traits are registered, so
//! struct-level `#[duck(_Show)]` entries may omit the props list — it is
//! derived from the registered trait and the struct's own fields. Generic
//! arguments are inferred while every prop is a bare generic parameter;
//! otherwise write them explicitly.
//!
//! ```rust
//! use duck_trait::ducky;
//!
//! #[ducky]
//! mod duckied {
//!     // `props`/`duck` markers are consumed by `#[ducky]`, no import needed
//!
//!     #[props(name: String, score: i32)]
//!     pub trait Show {
//!         fn show(&self) {
//!             println!("{}: {}", self.name(), self.score());
//!         }
//!     }
//!
//!     // props derived from `Show`; extra fields are fine
//!     #[duck(_Show)]
//!     pub struct Player {
//!         pub name: String,
//!         pub score: i32,
//!     }
//!
//!     impl Show for Player {}
//! }
//!
//! use duckied::{Player, Show};
//!
//! Player { name: "duck".to_owned(), score: 7 }.show();
//! ```
//!
//! Generic arguments may be written explicitly (their count is checked
//! against the registered trait) or inferred for generic structs:
//!
//! ```rust
//! use duck_trait::ducky;
//!
//! #[ducky]
//! mod duckied {
//!     #[props(inner: T)]
//!     pub trait Has<T> {
//!         fn get(&self) -> &T {
//!             self.inner()
//!         }
//!     }
//!
//!     // generic arguments written explicitly; props still derived
//!     #[duck(_Has<String>)]
//!     pub struct Wrapper {
//!         inner: String,
//!     }
//!
//!     impl Has<String> for Wrapper {}
//!
//!     // generic structs infer `impl<T> _Has<T> for W<T>`
//!     #[duck(_Has)]
//!     pub struct W<T> {
//!         inner: T,
//!     }
//!
//!     impl<T> Has<T> for W<T> {}
//!
//!     pub fn check() {
//!         let w = Wrapper { inner: "duck".to_owned() };
//!         assert_eq!(w.get(), "duck");
//!         let n = W { inner: 7 };
//!         assert_eq!(n.get(), &7);
//!     }
//! }
//!
//! duckied::check();
//! ```
//!
//! Traits registered in enclosing `#[ducky]` scopes are visible to nested
//! modules. Field-level `#[duck]` markers are not touched inside `#[ducky]`:
//! the old flow belongs to `#[duck_mod]`/`ducks!`.
//!
//! Rules enforced at compile time:
//!
//! - a brace-less entry must reference a `#[props]` trait of the scope:
//!
//! ```compile_fail
//! use duck_trait::ducky;
//!
//! #[ducky]
//! mod duckied {
//!     #[duck(_Nope)]
//!     struct A {
//!         a: u8,
//!     }
//! }
//! ```
//!
//! - inference requires every generic parameter to be the bare type of some
//!   prop (`items: Vec<T>` cannot be inferred):
//!
//! ```compile_fail
//! use duck_trait::ducky;
//!
//! #[ducky]
//! mod duckied {
//!     #[props(items: Vec<T>)]
//!     trait Bag<T> {}
//!
//!     #[duck(_Bag)]
//!     struct B {
//!         items: Vec<u8>,
//!     }
//! }
//! ```
//!
//! - explicitly written arguments are checked against the trait's generics:
//!
//! ```compile_fail
//! use duck_trait::ducky;
//!
//! #[ducky]
//! mod duckied {
//!     #[props(a: T, b: U)]
//!     trait Two<T, U> {}
//!
//!     #[duck(_Two<String>)]
//!     struct S {
//!         a: String,
//!         b: u8,
//!     }
//! }
//! ```
//!
//! - outside `#[ducky]`, the props list is required:
//!
//! ```compile_fail
//! use duck_trait::duck;
//!
//! #[duck(_Show)]
//! struct A {
//!     name: String,
//! }
//! ```
//!
//! ## Trait visibility
//!
//! `#[duck]` defaults to a `pub(crate)` accessor trait — usable anywhere in
//! the crate. `#[_duck]` keeps the trait private to the declaring scope, and
//! `#[duck(pub)]` / `#[duck(pub = ..)]` widen or restrict it on demand:
//!
//! | Marker                              | Generated trait visibility     |
//! | ----------------------------------- | ------------------------------ |
//! | `#[duck]`                           | `pub(crate)` (default)         |
//! | `#[duck(pub)]`                      | `pub`                          |
//! | `#[duck(pub = crate)]`              | `pub(crate)`                   |
//! | `#[duck(pub = super)]`              | `pub(super)`                   |
//! | `#[duck(pub = self)]`               | `pub(self)`                    |
//! | `#[duck(pub = crate::foo)]`         | `pub(in crate::foo)`           |
//! | `#[_duck]`                          | private to the declaring scope |
//!
//! The visibility item may sit anywhere in the argument list, next to custom
//! trait paths: `#[duck(MyValue<_>, pub = super)]`. Because the default trait
//! is `pub(crate)`, in-crate callers can use it across module boundaries:
//!
//! ```rust
//! use duck_trait::duck_mod;
//!
//! #[duck_mod]
//! mod model {
//!     pub struct Player {
//!         #[duck] // generates: pub(crate) trait _Name<T>
//!         name: String,
//!     }
//!
//!     pub fn make() -> Player {
//!         Player { name: "duck".to_owned() }
//!     }
//! }
//!
//! // the pub(crate) trait is reachable from any other module of the crate
//! fn shout(player: &impl model::_Name<String>) {
//!     println!("{}", player.name());
//! }
//!
//! shout(&model::make());
//! ```
//!
//! Rules enforced at compile time:
//!
//! - All structs sharing one trait must declare the same visibility:
//!
//! ```compile_fail
//! use duck_trait::ducks;
//!
//! ducks! {
//!     pub struct A {
//!         #[duck(pub)]
//!         value: String,
//!     }
//!
//!     struct B {
//!         #[duck] // error: `pub` vs `pub(crate)` for the shared `_Value` trait
//!         value: String,
//!     }
//! }
//! ```
//!
//! - Block scopes (function bodies, closures, ...) cannot carry visibility
//!   qualifiers, so they only accept `#[_duck]`:
//!
//! ```compile_fail
//! use duck_trait::ducks;
//!
//! ducks! {
//!     fn make() -> u8 {
//!         struct Local {
//!             #[duck] // error: use `#[_duck]` inside a block scope
//!             v: u8,
//!         }
//!         0
//!     }
//! }
//! ```
//!
//! - `#[_duck]` always generates a private trait and rejects `pub` items:
//!
//! ```compile_fail
//! use duck_trait::ducks;
//!
//! ducks! {
//!     struct A {
//!         #[_duck(pub)] // error: `#[_duck]` generates a private trait
//!         value: String,
//!     }
//! }
//! ```
//!
//! - A `#[duck(..)]` list accepts at most one visibility item:
//!
//! ```compile_fail
//! use duck_trait::ducks;
//!
//! ducks! {
//!     struct A {
//!         #[duck(pub, pub = crate)] // error: at most one `pub` item
//!         value: String,
//!     }
//! }
//! ```
//!
//! ## The `#[duck_mod]` attribute form
//!
//! `ducks!` places items directly into the enclosing scope and is the
//! recommended entry point. `#[duck_mod]` is the equivalent attribute form for
//! an inline module; the generated traits live in the module's namespace
//! (defaulting to `pub(crate)`, see [Trait visibility](#trait-visibility)):
//!
//! ```rust
//! use duck_trait::duck_mod;
//!
//! #[duck_mod]
//! mod model {
//!     #![allow(private_bounds)]
//!
//!     pub struct Player {
//!         #[duck]
//!         name: String,
//!     }
//!
//!     pub trait Show: _Name<String> {
//!         fn show(&self) {
//!             println!("{}", self.name());
//!         }
//!     }
//!
//!     impl Show for Player {}
//!
//!     pub fn new_player(name: &str) -> Player {
//!         let mut player = Player { name: name.to_owned() };
//!         player.name_set("silly ".to_owned());
//!         player
//!     }
//! }
//!
//! use model::{Player, Show};
//!
//! let player = model::new_player("duck");
//! player.show(); // silly duck
//! ```
//!
//! ## Block scopes are scanned too
//!
//! Structs declared inside function bodies — or any other block: closures,
//! loops, match arms, method bodies — get their traits generated in that same
//! block. Items in a block cannot carry visibility qualifiers, so these
//! scopes only accept the private `#[_duck]` marker:
//!
//! ```rust
//! use duck_trait::ducks;
//!
//! ducks! {
//!     fn make() -> u8 {
//!         struct Local {
//!             #[_duck]
//!             v: u8,
//!         }
//!         let mut local = Local { v: 1 };
//!         local.v_set(2);
//!         *local.v()
//!     }
//! }
//!
//! assert_eq!(make(), 2);
//! ```

use std::collections::{BTreeMap, BTreeSet, btree_map};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{
  Attribute, Block, Fields, FieldsNamed, File, GenericArgument, GenericParam, Generics, Ident,
  Item, ItemEnum, ItemMod, ItemStruct, ItemTrait, ItemUnion, Meta, Path, PathArguments, Stmt,
  Token, Type, TypeParamBound, Visibility, parse::Parse, parse::ParseStream, parse_quote, parse2,
  visit_mut::VisitMut,
};

/// Attribute form: `#[duck_mod] mod name { .. }`.
///
/// Scans the module (recursively into nested inline modules) for struct fields
/// marked with `#[duck]`/`#[_duck]`, strips the markers and generates the
/// accessor traits plus their impls into each scope.
#[proc_macro_attribute]
pub fn duck_mod(attr: TokenStream, item: TokenStream) -> TokenStream {
  expand_attr(attr.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Function-like form: `ducks! { .. }`.
///
/// Same transformation as `#[duck_mod]`, but the wrapped items are placed
/// directly into the enclosing scope without introducing a module.
#[proc_macro]
pub fn ducks(item: TokenStream) -> TokenStream {
  expand_bang(item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Trait-side marker: `#[props(name: String, score: i32)]` on a trait.
///
/// Generates the shadow trait `_<Trait>` declaring `xxx()` / `xxx_set()` /
/// `xxx_mut()` accessors for every prop, and binds the annotated trait to it
/// as a supertrait so default methods can call the accessors directly. The
/// shadow trait copies visibility, generics and where clauses verbatim from
/// the annotated trait; the struct side opts in with
/// `#[duck(_Show{field, ..})]` (see the crate docs).
#[proc_macro_attribute]
pub fn props(attr: TokenStream, item: TokenStream) -> TokenStream {
  expand_props(attr.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Marker stub and struct-level entry. `#[duck]` on a struct field is consumed
/// (stripped) by `#[duck_mod]`/`ducks!` before the compiler ever resolves it,
/// so this macro only runs when the marker is used outside a duck_mod scope,
/// or on something other than a struct field. On a struct,
/// `#[duck(_Show{field, ..}, ..)]` implements the listed shadow traits for it:
/// one accessor method triple per listed prop, reading the same-named fields.
#[proc_macro_attribute]
pub fn duck(attr: TokenStream, item: TokenStream) -> TokenStream {
  let attr = TokenStream2::from(attr);
  let item = TokenStream2::from(item);
  if !attr.is_empty() && parse2::<ItemStruct>(item.clone()).is_ok() {
    return expand_struct_duck(attr, item).unwrap_or_else(syn::Error::into_compile_error).into();
  }
  syn::Error::new_spanned(
    item,
    "`#[duck]` must be applied to a named struct field inside a scope \
         annotated with `#[duck_mod]` or wrapped in `ducks! { .. }`, or to a \
         struct with shadow-trait entries: `#[duck(_Show{field, ..})]`",
  )
  .into_compile_error()
  .into()
}

/// Private-marker stub. `#[_duck]` is consumed (stripped) by
/// `#[duck_mod]`/`ducks!` before the compiler ever resolves it, so this macro
/// only runs when the private marker is used outside a duck_mod scope, or on
/// something other than a struct field.
#[proc_macro_attribute]
pub fn _duck(_attr: TokenStream, item: TokenStream) -> TokenStream {
  syn::Error::new_spanned(
    TokenStream2::from(item),
    "`#[_duck]` must be applied to a named struct field inside a scope \
         annotated with `#[duck_mod]` or wrapped in `ducks! { .. }`",
  )
  .into_compile_error()
  .into()
}

// ---------------------------------------------------------------------------
// expansion entry points
// ---------------------------------------------------------------------------

fn expand_attr(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
  if !attr.is_empty() {
    return Err(syn::Error::new(Span::call_site(), "`#[duck_mod]` does not take any arguments"));
  }

  let mut module: ItemMod = parse2(item).map_err(|_| {
    syn::Error::new(
      Span::call_site(),
      "`#[duck_mod]` can only be applied to an inline module: `#[duck_mod] mod name { .. }`",
    )
  })?;

  let Some((_, items)) = &mut module.content else {
    return Err(syn::Error::new(
      module.ident.span(),
      "`#[duck_mod]` cannot scan a file-based module (`mod name;`); \
             inline the module or use `ducks! { .. }` instead",
    ));
  };

  process_scope(items)?;
  Ok(quote! { #module })
}

fn expand_bang(item: TokenStream2) -> syn::Result<TokenStream2> {
  if item.is_empty() {
    return Err(syn::Error::new(Span::call_site(), "`ducks! { .. }` requires at least one item"));
  }

  let mut file: File = parse2(item)
    .map_err(|_| syn::Error::new(Span::call_site(), "`ducks!` expects a list of items"))?;

  process_scope(&mut file.items)?;
  let items = file.items;
  Ok(quote! { #(#items)* })
}

// ---------------------------------------------------------------------------
// scope processing
// ---------------------------------------------------------------------------

/// One `#[duck]`/`#[_duck]`-marked field, waiting to be grouped into a trait.
struct DuckField {
  struct_ident: Ident,
  generics: Generics,
  field_ident: Ident,
  field_ty: Type,
  /// Visibility of the generated accessor trait; `Visibility::Inherited` for
  /// the private `#[_duck]` marker.
  vis: Visibility,
  /// Trait paths from `#[duck(MyTrait, ..)]` to additionally implement for the
  /// struct.
  custom_impls: Vec<Path>,
}

fn process_scope(items: &mut Vec<Item>) -> syn::Result<()> {
  let mut collected: Vec<DuckField> = Vec::new();
  scan_items(items, &mut collected)?;

  let generated = generate(&collected)?;
  reject_conflicts(items.iter().filter_map(item_ident), &generated)?;
  items.extend(generated);
  Ok(())
}

/// One block scope: collects `#[duck]`-marked structs declared directly inside
/// `block` and appends the generated traits and impls to the block's own
/// statements, so they stay visible exactly where the struct is declared.
///
/// Every block in Rust (function bodies, closures, `unsafe`/`async`/`const`
/// blocks, loop/`if`/`match` branch blocks, method bodies) is a scope of its
/// own; nested blocks are processed recursively the same way.
fn process_block(block: &mut Block) -> syn::Result<()> {
  let mut collected: Vec<DuckField> = Vec::new();
  scan_stmts(&mut block.stmts, &mut collected)?;

  // items inside a block cannot carry visibility qualifiers (E0449)
  if let Some(field) = collected.iter().find(|field| !matches!(field.vis, Visibility::Inherited)) {
    return Err(syn::Error::new(
      field.field_ident.span(),
      "`#[duck]` (or `#[duck(pub ..)]`) cannot be used inside a block scope: \
       visibility qualifiers are not permitted on the generated trait here; \
       use `#[_duck]` for a private accessor trait instead",
    ));
  }

  let generated = generate(&collected)?;
  reject_conflicts(block.stmts.iter().filter_map(stmt_ident), &generated)?;
  // a trailing tail expression (`x` without `;`) must stay last, so insert
  // right before it
  let end = match block.stmts.last() {
    Some(Stmt::Expr(_, None)) => block.stmts.len() - 1,
    _ => block.stmts.len(),
  };
  block.stmts.splice(end..end, generated.into_iter().map(Stmt::Item));
  Ok(())
}

/// Walks `items`, collecting `#[duck]`-marked fields into `collected` while
/// handing every nested scope (inline modules, function bodies and any other
/// block) to its own processing.
fn scan_items(items: &mut [Item], collected: &mut Vec<DuckField>) -> syn::Result<()> {
  let mut err = None;
  {
    let mut scanner = ScopeScanner { collected, err: &mut err };
    for item in items.iter_mut() {
      scanner.visit_item_mut(item);
    }
  }
  if let Some(err) = err {
    return Err(err);
  }
  Ok(())
}

/// Same as [`scan_items`] for the statement list of one block scope.
fn scan_stmts(stmts: &mut [Stmt], collected: &mut Vec<DuckField>) -> syn::Result<()> {
  let mut err = None;
  {
    let mut scanner = ScopeScanner { collected, err: &mut err };
    for stmt in stmts.iter_mut() {
      scanner.visit_stmt_mut(stmt);
    }
  }
  if let Some(err) = err {
    return Err(err);
  }
  Ok(())
}

/// `syn::visit_mut` walker that collects `#[duck]`-marked struct fields into
/// `collected`. Inline modules, function bodies and every other block are
/// delegated to [`process_scope`] / [`process_block`] as scopes of their own
/// and are *not* traversed again from here.
struct ScopeScanner<'a> {
  collected: &'a mut Vec<DuckField>,
  err: &'a mut Option<syn::Error>,
}

impl ScopeScanner<'_> {
  /// Stores the first error; traversal keeps running but does no further work.
  fn record(&mut self, result: syn::Result<()>) {
    if self.err.is_none()
      && let Err(err) = result
    {
      *self.err = Some(err);
    }
  }
}

impl VisitMut for ScopeScanner<'_> {
  fn visit_item_struct_mut(&mut self, node: &mut ItemStruct) {
    if self.err.is_some() {
      return;
    }
    let result = collect_from_struct(node, self.collected);
    self.record(result);
    // struct bodies hold no block scopes, so there is nothing to descend into
  }

  fn visit_item_enum_mut(&mut self, node: &mut ItemEnum) {
    if self.err.is_some() {
      return;
    }
    for variant in &node.variants {
      let result = reject_duck_on_foreign_fields(variant.fields.iter());
      self.record(result);
    }
  }

  fn visit_item_union_mut(&mut self, node: &mut ItemUnion) {
    if self.err.is_some() {
      return;
    }
    let result = reject_duck_on_foreign_fields(node.fields.named.iter());
    self.record(result);
  }

  fn visit_item_mod_mut(&mut self, node: &mut ItemMod) {
    if self.err.is_some() {
      return;
    }
    if let Some((_, inner)) = &mut node.content {
      let result = process_scope(inner);
      self.record(result);
      // the inline module is a scope of its own; do not descend again
    }
    // file-based modules have no token content to scan
  }

  fn visit_block_mut(&mut self, node: &mut Block) {
    if self.err.is_some() {
      return;
    }
    let result = process_block(node);
    self.record(result);
    // `process_block` covered everything inside; do not descend again
  }
}

fn collect_from_struct(item_struct: &mut ItemStruct, out: &mut Vec<DuckField>) -> syn::Result<()> {
  match &mut item_struct.fields {
    Fields::Named(fields_named) => {
      for field in fields_named.named.iter_mut() {
        let duck_pos = field.attrs.iter().position(is_duck);
        let private_pos = field.attrs.iter().position(is_private_duck);
        let Some(pos) = duck_pos.or(private_pos) else {
          continue;
        };
        if duck_pos.is_some() && private_pos.is_some() {
          return Err(syn::Error::new(
            field.ident.as_ref().expect("named field has an ident").span(),
            "a field cannot be marked with both `#[duck]` and `#[_duck]`",
          ));
        }
        let public_marker = duck_pos.is_some();
        let attr = field.attrs.remove(pos);
        let (vis, custom_impls) = parse_marker(&attr, public_marker)?;
        out.push(DuckField {
          struct_ident: item_struct.ident.clone(),
          generics: item_struct.generics.clone(),
          field_ident: field.ident.clone().expect("named field has an ident"),
          field_ty: field.ty.clone(),
          vis,
          custom_impls,
        });
      }
    }
    Fields::Unnamed(_) | Fields::Unit => {
      for field in item_struct.fields.iter() {
        if let Some(attr) = field.attrs.iter().find(|attr| is_duck(attr) || is_private_duck(attr)) {
          return Err(syn::Error::new_spanned(
            attr,
            "`#[duck]`/`#[_duck]` only supports named struct fields",
          ));
        }
      }
    }
  }
  Ok(())
}

fn reject_duck_on_foreign_fields<'a>(
  fields: impl IntoIterator<Item = &'a syn::Field>,
) -> syn::Result<()> {
  for field in fields {
    if let Some(attr) = field.attrs.iter().find(|attr| is_duck(attr) || is_private_duck(attr)) {
      return Err(syn::Error::new_spanned(
        attr,
        "`#[duck]`/`#[_duck]` only supports named struct fields",
      ));
    }
  }
  Ok(())
}

fn is_duck(attr: &Attribute) -> bool {
  attr.path().is_ident("duck")
}

fn is_private_duck(attr: &Attribute) -> bool {
  attr.path().is_ident("_duck")
}

/// Trait visibility used when the marker declares none: `#[duck]` defaults to
/// `pub(crate)`, `#[_duck]` to private.
fn default_vis(public_marker: bool) -> Visibility {
  if public_marker {
    parse2::<Visibility>(quote!(pub(crate))).expect("`pub(crate)` is a valid visibility")
  } else {
    Visibility::Inherited
  }
}

/// Parses a `#[duck]` / `#[_duck]` field marker into the trait visibility and
/// the trait paths of the `impl`s to additionally generate.
///
/// `#[duck(..)]` items are comma-separated and may appear in any order:
///
/// - at most one visibility item: bare `pub` (fully public) or `pub = <value>`
///   where value is `crate`, `super`, `self` or a path rendered as
///   `pub(in path)`,
/// - any number of trait paths, additionally implemented for the struct.
///
/// `#[_duck]` always generates a private trait and rejects visibility items.
fn parse_marker(attr: &Attribute, public_marker: bool) -> syn::Result<(Visibility, Vec<Path>)> {
  let marker = if public_marker { "`#[duck]`" } else { "`#[_duck]`" };
  match &attr.meta {
    Meta::Path(_) => Ok((default_vis(public_marker), Vec::new())),
    Meta::NameValue(_) => {
      Err(syn::Error::new_spanned(attr, format!("{marker} does not support name-value arguments")))
    }
    Meta::List(_) => {
      let mut vis: Option<Visibility> = None;
      let mut custom_impls: Vec<Path> = Vec::new();
      attr.parse_args_with(|input: ParseStream| -> syn::Result<()> {
        let mut expect_comma = false;
        while !input.is_empty() {
          if expect_comma {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
              break; // trailing comma
            }
          }
          expect_comma = true;
          if input.peek(Token![pub]) {
            if !public_marker {
              return Err(input.error(format!(
                "{marker} always generates a private trait; \
                 use `#[duck(pub ..)]` for a public accessor trait"
              )));
            }
            if vis.is_some() {
              return Err(input.error(format!("{marker} accepts at most one `pub` item")));
            }
            vis = Some(parse_vis_item(input)?);
          } else {
            custom_impls.push(input.parse::<Path>().map_err(|_| {
              input.error(format!(
                "{marker} expects `pub` items or trait paths, \
                 e.g. `#[duck(pub = crate, MyValue<_>)]`"
              ))
            })?);
            if input.peek(syn::token::Brace) {
              return Err(input.error(
                "`#[duck(_Trait{..})]` is the struct-level shadow-trait form \
                 and cannot be used on a field",
              ));
            }
          }
        }
        Ok(())
      })?;
      Ok((vis.unwrap_or_else(|| default_vis(public_marker)), custom_impls))
    }
  }
}

/// Parses one `pub` (fully public) or `pub = <value>` visibility item from a
/// `#[duck(..)]` argument list.
fn parse_vis_item(input: ParseStream) -> syn::Result<Visibility> {
  input.parse::<Token![pub]>()?;
  if !input.peek(Token![=]) {
    return Ok(parse2::<Visibility>(quote!(pub)).expect("`pub` is a valid visibility"));
  }
  input.parse::<Token![=]>()?;

  // `crate`/`super`/`self` directly followed by `::` starts a path instead
  let restricted: Option<&str> = if input.peek(Token![crate]) && !input.peek2(Token![::]) {
    input.parse::<Token![crate]>()?;
    Some("crate")
  } else if input.peek(Token![super]) && !input.peek2(Token![::]) {
    input.parse::<Token![super]>()?;
    Some("super")
  } else if input.peek(Token![self]) && !input.peek2(Token![::]) {
    input.parse::<Token![self]>()?;
    Some("self")
  } else {
    None
  };

  if let Some(kw) = restricted {
    let tokens = match kw {
      "crate" => quote!(pub(crate)),
      "super" => quote!(pub(super)),
      _ => quote!(pub(self)),
    };
    return Ok(parse2::<Visibility>(tokens).expect("keyword restrictions are valid visibilities"));
  }

  let path: Path = input.parse()?;
  let first = path.segments.first().map(|seg| seg.ident.to_string());
  let valid_in_path =
    path.leading_colon.is_none() && matches!(first.as_deref(), Some("crate" | "super" | "self"));
  if !valid_in_path {
    return Err(input.error(
      "`pub = <path>` is rendered as `pub(in path)`, whose path \
       must start with `crate`, `super`, or `self`",
    ));
  }
  Ok(parse2::<Visibility>(quote!(pub(in #path))).expect("keyword-led paths are valid in-paths"))
}

// ---------------------------------------------------------------------------
// trait / impl generation
// ---------------------------------------------------------------------------

/// Replaces every bare `_` placeholder among `path`'s generic arguments with the
/// marked field's type; all other arguments are kept verbatim.
fn fill_placeholders(path: &mut Path, field_ty: &Type) -> syn::Result<()> {
  for segment in &mut path.segments {
    match &mut segment.arguments {
      PathArguments::None => {}
      PathArguments::Parenthesized(args) => {
        return Err(syn::Error::new_spanned(
          args,
          "`#[duck(...)]` does not support parenthesized generic arguments",
        ));
      }
      PathArguments::AngleBracketed(args) => {
        for arg in &mut args.args {
          match arg {
            GenericArgument::Type(ty) => {
              if matches!(*ty, Type::Infer(_)) {
                *ty = field_ty.clone();
              }
            }
            GenericArgument::AssocType(assoc) => {
              if matches!(assoc.ty, Type::Infer(_)) {
                assoc.ty = field_ty.clone();
              }
            }
            _ => {}
          }
        }
      }
    }
  }
  Ok(())
}

/// One accessor trait and every field sharing it.
struct TraitGroup<'a> {
  base: String,
  first_field: String,
  vis: Visibility,
  fields: Vec<&'a DuckField>,
}

fn generate(fields: &[DuckField]) -> syn::Result<Vec<Item>> {
  // trait name (without leading `_`) -> grouped fields sharing it
  let mut groups: BTreeMap<String, TraitGroup> = BTreeMap::new();
  // (struct, rendered trait path) of custom impls already emitted
  let mut emitted_custom: BTreeSet<(String, String)> = BTreeSet::new();

  for field in fields {
    let trait_name = trait_name_for(&field.field_ident)?;
    let base = method_base(&field.field_ident);
    match groups.entry(trait_name) {
      btree_map::Entry::Occupied(mut occupied) => {
        let trait_name = occupied.key().clone();
        let group = occupied.get_mut();
        if group.base != base {
          return Err(syn::Error::new(
            field.field_ident.span(),
            format!(
              "field `{}` and field `{}` produce the same trait `_{}` \
                             but require different method names",
              field.field_ident, group.first_field, trait_name,
            ),
          ));
        }
        let label = vis_label(&field.vis);
        if vis_label(&group.vis) != label {
          return Err(syn::Error::new(
            field.field_ident.span(),
            format!(
              "field `{field}` declares visibility `{label}` for the accessor trait \
               `_{name}`, but field `{existing}` declares `{existing_label}`; \
               all fields sharing one trait must declare the same visibility",
              field = field.field_ident,
              name = trait_name,
              existing = group.first_field,
              existing_label = vis_label(&group.vis),
            ),
          ));
        }
        group.fields.push(field);
      }
      btree_map::Entry::Vacant(vacant) => {
        vacant.insert(TraitGroup {
          base,
          first_field: field.field_ident.to_string(),
          vis: field.vis.clone(),
          fields: vec![field],
        });
      }
    }
  }

  let mut tokens = TokenStream2::new();
  for (trait_name, group) in groups {
    let TraitGroup { base, vis, fields: group, .. } = group;
    let trait_ident = format_ident!("_{}", trait_name);
    // getter keeps the original spelling (raw idents such as `r#type` stay raw)
    let get_ident = &group[0].field_ident;
    let set_ident = format_ident!("{}_set", base);
    let mut_ident = format_ident!("{}_mut", base);

    tokens.extend(quote! {
        #vis trait #trait_ident<T> {
            fn #get_ident(&self) -> &T;
            fn #set_ident(&mut self, v: T);
            fn #mut_ident(&mut self) -> &mut T;
        }
    });

    for field in group {
      let field_ident = &field.field_ident;
      let struct_ident = &field.struct_ident;
      let field_ty = &field.field_ty;
      let (impl_generics, ty_generics, where_clause) = field.generics.split_for_impl();

      tokens.extend(quote! {
          impl #impl_generics #trait_ident<#field_ty> for #struct_ident #ty_generics #where_clause {
              fn #get_ident(&self) -> &#field_ty {
                  &self.#field_ident
              }
              fn #set_ident(&mut self, v: #field_ty) {
                  self.#field_ident = v;
              }
              fn #mut_ident(&mut self) -> &mut #field_ty {
                  &mut self.#field_ident
              }
          }
      });

      for custom in &field.custom_impls {
        let mut custom = custom.clone();
        fill_placeholders(&mut custom, field_ty)?;
        let rendered = quote!(#custom).to_string();
        if !emitted_custom.insert((struct_ident.to_string(), rendered)) {
          continue;
        }
        tokens.extend(quote! {
            impl #impl_generics #custom for #struct_ident #ty_generics #where_clause {}
        });
      }
    }
  }

  let file: File = parse2(tokens).expect("duck-trait generated syntactically invalid items");
  Ok(file.items)
}

/// Human-readable visibility rendering for error messages.
fn vis_label(vis: &Visibility) -> String {
  match vis {
    Visibility::Inherited => "private".to_owned(),
    Visibility::Public(_) => "pub".to_owned(),
    Visibility::Restricted(restricted) => {
      let path = &restricted.path;
      let rendered = quote!(#path).to_string();
      if restricted.in_token.is_some() {
        format!("pub(in {rendered})")
      } else {
        format!("pub({rendered})")
      }
    }
  }
}

/// `value` -> `Value`, `my_field` -> `MyField`, `r#type` -> `Type`.
fn trait_name_for(field_ident: &Ident) -> syn::Result<String> {
  let raw = field_ident.to_string();
  let name = raw.strip_prefix("r#").unwrap_or(&raw);
  let mut out = String::with_capacity(name.len());
  for segment in name.split('_') {
    let mut chars = segment.chars();
    if let Some(first) = chars.next() {
      out.extend(first.to_uppercase());
      out.push_str(chars.as_str());
    }
  }
  if out.is_empty() {
    return Err(syn::Error::new(
      field_ident.span(),
      "cannot derive an accessor trait name from this field",
    ));
  }
  Ok(out)
}

/// `value` -> `value`, `r#type` -> `type` (used to compose `*_set` / `*_mut`).
fn method_base(field_ident: &Ident) -> String {
  let raw = field_ident.to_string();
  match raw.strip_prefix("r#") {
    Some(stripped) => stripped.to_owned(),
    None => raw,
  }
}

fn reject_conflicts<'a>(
  existing: impl IntoIterator<Item = &'a Ident>,
  generated: &[Item],
) -> syn::Result<()> {
  let generated_names: Vec<&Ident> = generated.iter().filter_map(item_ident).collect();
  for ident in existing {
    if generated_names.iter().any(|name| **name == *ident) {
      return Err(syn::Error::new(
        ident.span(),
        format!(
          "`{ident}` conflicts with an accessor trait generated by duck-trait; \
                     rename the field or this item"
        ),
      ));
    }
  }
  Ok(())
}

fn stmt_ident(stmt: &Stmt) -> Option<&Ident> {
  match stmt {
    Stmt::Item(item) => item_ident(item),
    _ => None,
  }
}

fn item_ident(item: &Item) -> Option<&Ident> {
  match item {
    Item::Const(item) => Some(&item.ident),
    Item::Enum(item) => Some(&item.ident),
    Item::Fn(item) => Some(&item.sig.ident),
    Item::Macro(item) => item.ident.as_ref(),
    Item::Mod(item) => Some(&item.ident),
    Item::Static(item) => Some(&item.ident),
    Item::Struct(item) => Some(&item.ident),
    Item::Trait(item) => Some(&item.ident),
    Item::TraitAlias(item) => Some(&item.ident),
    Item::Type(item) => Some(&item.ident),
    Item::Union(item) => Some(&item.ident),
    _ => None,
  }
}

// ---------------------------------------------------------------------------
// props: trait-first shadow traits
// ---------------------------------------------------------------------------

/// One `#[props(..)]` entry: `name: Type`.
struct Prop {
  name: Ident,
  ty: Type,
}

impl Parse for Prop {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let name = input.parse::<Ident>()?;
    input.parse::<Token![:]>()?;
    let ty = input.parse::<Type>()?;
    Ok(Prop { name, ty })
  }
}

/// Parsed `#[props(..)]` arguments: at least one prop, and every generated
/// method name must be unique across props (`a` vs `a_set`, duplicates, ...).
struct PropsAttr(Vec<Prop>);

impl Parse for PropsAttr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let props: Vec<Prop> =
      Punctuated::<Prop, Token![,]>::parse_terminated(input)?.into_iter().collect();
    if props.is_empty() {
      return Err(
        input.error("`#[props(..)]` requires at least one prop: `#[props(name: String)]`"),
      );
    }

    let mut seen: BTreeMap<String, (String, &'static str)> = BTreeMap::new();
    for prop in &props {
      let base = method_base(&prop.name);
      let generated = [
        (prop.name.to_string(), "getter"),
        (format!("{}_set", base), "setter"),
        (format!("{}_mut", base), "mut accessor"),
      ];
      for (method, kind) in generated {
        if let Some((previous, previous_kind)) =
          seen.insert(method.clone(), (prop.name.to_string(), kind))
        {
          return Err(syn::Error::new(
            prop.name.span(),
            format!(
              "prop `{previous}` ({previous_kind}) and prop `{}` ({kind}) both \
               generate the method `{method}`; rename one of the props",
              prop.name,
            ),
          ));
        }
      }
    }
    Ok(PropsAttr(props))
  }
}

/// Parses the arguments of a `#[props(..)]` attribute.
fn parse_props_attribute(attr: &Attribute) -> syn::Result<Vec<Prop>> {
  match &attr.meta {
    Meta::List(_) => Ok(attr.parse_args::<PropsAttr>()?.0),
    _ => Err(syn::Error::new_spanned(
      attr,
      "`#[props(..)]` requires at least one prop: `#[props(name: String)]`",
    )),
  }
}

/// `_Show` for `Show`; a raw identifier cannot carry the `_` prefix.
fn shadow_trait_ident(trait_ident: &Ident) -> syn::Result<Ident> {
  let trait_str = trait_ident.to_string();
  if trait_str.starts_with("r#") {
    return Err(syn::Error::new(
      trait_ident.span(),
      format!(
        "cannot derive a shadow trait name from the raw identifier `{trait_str}`; \
         rename the trait"
      ),
    ));
  }
  Ok(format_ident!("_{}", trait_str))
}

/// Builds the shadow trait `_Show` and rewrites `trait_item` to depend on it
/// as a supertrait. Visibility, generics and where clauses are copied
/// verbatim from the annotated trait.
fn build_shadow_items(mut trait_item: ItemTrait, props: &[Prop]) -> syn::Result<(Item, Item)> {
  if trait_item.auto_token.is_some() {
    return Err(syn::Error::new(
      trait_item.ident.span(),
      "`#[props(..)]` does not support auto traits",
    ));
  }
  let shadow_ident = shadow_trait_ident(&trait_item.ident)?;

  let vis = &trait_item.vis;
  let params = &trait_item.generics.params;
  let where_clause = &trait_item.generics.where_clause;
  let generics_head = if params.is_empty() { quote!() } else { quote!(<#params>) };

  let mut accessors = TokenStream2::new();
  for prop in props {
    let name = &prop.name;
    let ty = &prop.ty;
    let base = method_base(name);
    let set_ident = format_ident!("{}_set", base);
    let mut_ident = format_ident!("{}_mut", base);
    accessors.extend(quote! {
        fn #name(&self) -> &#ty;
        fn #set_ident(&mut self, v: #ty);
        fn #mut_ident(&mut self) -> &mut #ty;
    });
  }

  let shadow_doc: Attribute = parse_quote!(#[doc = "Accessor trait generated by `duck-trait` for the props of the annotated trait."]);

  // supertrait reference: the trait's own generic parameters as arguments
  let args: Vec<TokenStream2> = trait_item
    .generics
    .params
    .iter()
    .map(|param| match param {
      GenericParam::Lifetime(lifetime) => {
        let lifetime = &lifetime.lifetime;
        quote!(#lifetime)
      }
      GenericParam::Type(ty) => {
        let ident = &ty.ident;
        quote!(#ident)
      }
      GenericParam::Const(const_) => {
        let ident = &const_.ident;
        quote!(#ident)
      }
    })
    .collect();
  let shadow_path =
    if args.is_empty() { quote!(#shadow_ident) } else { quote!(#shadow_ident<#(#args),*>) };

  // bind the annotated trait to the shadow trait as a supertrait
  if trait_item.supertraits.is_empty() {
    trait_item.colon_token = Some(Token![:](Span::call_site()));
  } else {
    trait_item.supertraits.push_punct(Token![+](Span::call_site()));
  }
  trait_item.supertraits.push_value(parse2::<TypeParamBound>(shadow_path)?);

  let shadow_tokens = quote! {
      #shadow_doc
      #vis trait #shadow_ident #generics_head #where_clause {
          #accessors
      }
  };
  let shadow_trait: ItemTrait = parse2(shadow_tokens).expect("generated shadow trait is valid");
  Ok((Item::Trait(shadow_trait), Item::Trait(trait_item)))
}

/// Expands `#[props(name: String, ..)] trait Show { .. }` into the shadow
/// trait `_Show` plus the original trait bound to it as a supertrait.
fn expand_props(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
  let props = parse2::<PropsAttr>(attr)?.0;
  let trait_item: ItemTrait = parse2(item).map_err(|_| {
    syn::Error::new(Span::call_site(), "`#[props(..)]` can only be applied to a trait")
  })?;
  let (shadow, modified) = build_shadow_items(trait_item, &props)?;
  Ok(quote! { #shadow #modified })
}

// ---------------------------------------------------------------------------
// struct-level `#[duck(_Trait{props})]`: shadow-trait impls
// ---------------------------------------------------------------------------

/// One struct-level `#[duck(..)]` entry: `_Show`, `_Has<String>` or
/// `_Show{field, ..}`.
struct DuckEntry {
  path: Path,
  /// `None` when the braces are omitted; only resolvable inside `#[ducky]`.
  props: Option<Vec<Ident>>,
}

impl Parse for DuckEntry {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let path: Path = input.parse()?;
    let props = if input.peek(syn::token::Brace) {
      let content;
      syn::braced!(content in input);
      let props = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
      if props.is_empty() {
        return Err(content.error("expected at least one prop: `_Show{field, ..}`"));
      }
      Some(props.into_iter().collect())
    } else {
      None
    };
    Ok(DuckEntry { path, props })
  }
}

/// Parsed `#[duck(..)]` arguments: at least one entry.
struct DuckEntries(Vec<DuckEntry>);

impl Parse for DuckEntries {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let entries: Vec<DuckEntry> =
      Punctuated::<DuckEntry, Token![,]>::parse_terminated(input)?.into_iter().collect();
    if entries.is_empty() {
      return Err(input.error("`#[duck(..)]` requires at least one shadow-trait entry"));
    }
    Ok(DuckEntries(entries))
  }
}

/// Parses the arguments of a struct-level `#[duck(..)]` attribute.
fn parse_duck_entries_attribute(attr: &Attribute) -> syn::Result<Vec<DuckEntry>> {
  match &attr.meta {
    Meta::List(_) => Ok(attr.parse_args::<DuckEntries>()?.0),
    _ => Err(syn::Error::new_spanned(
      attr,
      "`#[duck(..)]` requires at least one shadow-trait entry: `#[duck(_Show{field, ..})]`",
    )),
  }
}

/// One fully resolved entry: the complete trait path of the generated `impl`
/// and the props it provides.
struct ResolvedDuckEntry {
  trait_path: Path,
  props: Vec<Ident>,
}

/// Expands `#[duck(_Show{name, ..}, ..)] struct A { .. }` into the struct plus
/// one `impl _Show for A` per entry, with accessor methods reading the listed
/// fields. The trait path (including any generic arguments) is used verbatim;
/// method signatures are built from the field types.
fn expand_struct_duck(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
  let entries = parse2::<DuckEntries>(attr)?.0;
  let struct_item: ItemStruct = parse2(item).map_err(|_| {
    syn::Error::new(
      Span::call_site(),
      "`#[duck(..)]` with shadow-trait entries can only be applied to a struct",
    )
  })?;

  let mut resolved = Vec::new();
  for entry in entries {
    let Some(props) = entry.props else {
      return Err(syn::Error::new_spanned(
        &entry.path,
        "the brace-less form `#[duck(_Show)]` only works inside `#[ducky]`; \
         write the props explicitly: `#[duck(_Show{field, ..})]`",
      ));
    };
    validate_impl_path(&entry.path)?;
    resolved.push(ResolvedDuckEntry { trait_path: entry.path, props });
  }

  let impls = build_duck_impls(&struct_item, &resolved)?;
  Ok(quote! { #struct_item #(#impls)* })
}

/// Builds one accessor impl per entry: every prop must match a same-named
/// field, and the method signatures are built from the field types.
fn build_duck_impls(
  struct_item: &ItemStruct,
  entries: &[ResolvedDuckEntry],
) -> syn::Result<Vec<Item>> {
  let fields = named_fields(struct_item)?;
  let struct_ident = &struct_item.ident;
  let (impl_generics, ty_generics, where_clause) = struct_item.generics.split_for_impl();

  let mut impls = Vec::new();
  for entry in entries {
    let trait_path = &entry.trait_path;
    let mut listed: BTreeSet<String> = BTreeSet::new();
    let mut methods = TokenStream2::new();
    for prop in &entry.props {
      if !listed.insert(prop.to_string()) {
        return Err(syn::Error::new(
          prop.span(),
          format!("prop `{prop}` is listed twice in `#[duck(..)]`"),
        ));
      }
      let field = find_field(fields, prop, struct_ident)?;
      let field_ident = field.ident.as_ref().expect("named field has an ident");
      let ty = &field.ty;
      let base = method_base(prop);
      let set_ident = format_ident!("{}_set", base);
      let mut_ident = format_ident!("{}_mut", base);
      methods.extend(quote! {
          fn #prop(&self) -> &#ty {
              &self.#field_ident
          }
          fn #set_ident(&mut self, v: #ty) {
              self.#field_ident = v;
          }
          fn #mut_ident(&mut self) -> &mut #ty {
              &mut self.#field_ident
          }
      });
    }
    let tokens = quote! {
        impl #impl_generics #trait_path for #struct_ident #ty_generics #where_clause {
            #methods
        }
    };
    impls.push(parse2(tokens).expect("generated duck impl is valid"));
  }
  Ok(impls)
}

/// The named fields of `struct_item`, or an error for unit/tuple structs.
fn named_fields(struct_item: &ItemStruct) -> syn::Result<&FieldsNamed> {
  match &struct_item.fields {
    Fields::Named(fields) => Ok(fields),
    _ => Err(syn::Error::new(
      struct_item.ident.span(),
      "`#[duck(_Trait{..})]` requires a struct with named fields",
    )),
  }
}

/// The field matching `prop`, or a clear error.
fn find_field<'a>(
  fields: &'a FieldsNamed,
  prop: &Ident,
  struct_ident: &Ident,
) -> syn::Result<&'a syn::Field> {
  fields.named.iter().find(|field| field.ident.as_ref() == Some(prop)).ok_or_else(|| {
    syn::Error::new(
      prop.span(),
      format!(
        "no field `{prop}` on struct `{struct_ident}`; \
         every prop in `#[duck(_Trait{{..}})]` must match a field name"
      ),
    )
  })
}

/// The shadow-trait path is used verbatim in the generated `impl`, so
/// parenthesized arguments are never valid and `_` cannot be inferred there.
fn validate_impl_path(path: &Path) -> syn::Result<()> {
  for segment in &path.segments {
    match &segment.arguments {
      PathArguments::None => {}
      PathArguments::Parenthesized(args) => {
        return Err(syn::Error::new_spanned(
          args,
          "`#[duck(_Trait{..})]` does not support parenthesized generic arguments",
        ));
      }
      PathArguments::AngleBracketed(args) => {
        for arg in &args.args {
          if let GenericArgument::Type(Type::Infer(_)) = arg {
            return Err(syn::Error::new_spanned(
              arg,
              "`_` cannot be used here: the shadow-trait arguments of \
               `#[duck(_Trait{..})]` must be written out in full",
            ));
          }
        }
      }
    }
  }
  Ok(())
}

// ---------------------------------------------------------------------------
// `#[ducky]`: module scope with brace-less `#[duck(_Show)]` entries
// ---------------------------------------------------------------------------

/// Registered `#[props]` trait of a `#[ducky]` scope.
struct PropsTraitInfo {
  /// `Show` — for error messages.
  trait_ident: Ident,
  /// `_Show` — the lookup key of brace-less `#[duck(_Show)]` entries.
  shadow_ident: Ident,
  /// Declared props; the declared types drive the bare-parameter matching.
  props: Vec<Prop>,
  /// Idents of the trait's generic parameters, in declaration order.
  param_idents: Vec<Ident>,
}

/// The `#[props]` traits visible to a `#[ducky]` scope: its own traits plus
/// those of the enclosing `#[ducky]` scopes.
struct PropsRegistry<'a> {
  local: Vec<PropsTraitInfo>,
  parent: Option<&'a PropsRegistry<'a>>,
}

impl PropsRegistry<'_> {
  /// The registered trait plus the number of `super::` hops from the
  /// referencing scope to the scope that declared it.
  fn find(&self, shadow: &str) -> Option<(&PropsTraitInfo, usize)> {
    if let Some(info) = self.local.iter().find(|info| info.shadow_ident == shadow) {
      return Some((info, 0));
    }
    let (info, level) = self.parent?.find(shadow)?;
    Some((info, level + 1))
  }
}

/// Attribute form: `#[ducky] mod name { .. }`.
///
/// Module scope for the props flow: `#[props]` traits declared inside are
/// expanded in place and registered, so struct-level `#[duck(_Show)]` entries
/// may omit the props list — it is derived from the registered trait and the
/// struct's own fields. The shadow-trait arguments are inferred while every
/// generic parameter is the bare type of some prop (`inner: T`); otherwise
/// write them explicitly: `#[duck(_Has<String>)]`. Entries with an explicit
/// props list (`#[duck(_Show{field, ..})]`) keep the standalone semantics.
#[proc_macro_attribute]
pub fn ducky(attr: TokenStream, item: TokenStream) -> TokenStream {
  expand_ducky(attr.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

fn expand_ducky(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
  if !attr.is_empty() {
    return Err(syn::Error::new(Span::call_site(), "`#[ducky]` does not take any arguments"));
  }
  let mut module: ItemMod = parse2(item).map_err(|_| {
    syn::Error::new(
      Span::call_site(),
      "`#[ducky]` can only be applied to an inline module: `#[ducky] mod name { .. }`",
    )
  })?;
  let Some((_, items)) = &mut module.content else {
    return Err(syn::Error::new(
      module.ident.span(),
      "`#[ducky]` cannot scan a file-based module (`mod name;`); inline the module instead",
    ));
  };
  process_ducky_scope(items, None)?;
  Ok(quote! { #module })
}

/// Processes one `#[ducky]` scope: expands and registers `#[props]` traits
/// (pass one), then resolves the struct-level `#[duck(..)]` entries against
/// them (pass two). Nested inline modules are processed as scopes of their
/// own, seeing the enclosing scopes' registered traits. Field-level `#[duck]`
/// markers are not touched: the old flow belongs to `#[duck_mod]`/`ducks!`.
fn process_ducky_scope(
  items: &mut Vec<Item>,
  parent: Option<&PropsRegistry<'_>>,
) -> syn::Result<()> {
  // pass one: expand `#[props]` traits and register them
  let mut registry = PropsRegistry { local: Vec::new(), parent };
  let mut expanded: Vec<Item> = Vec::with_capacity(items.len());
  for item in items.drain(..) {
    let Item::Trait(mut trait_item) = item else {
      expanded.push(item);
      continue;
    };
    let Some(pos) = trait_item.attrs.iter().position(|attr| attr.path().is_ident("props")) else {
      expanded.push(Item::Trait(trait_item));
      continue;
    };
    let attr = trait_item.attrs.remove(pos);
    let props = parse_props_attribute(&attr)?;
    let param_idents = trait_item
      .generics
      .params
      .iter()
      .map(|param| match param {
        GenericParam::Lifetime(lifetime) => lifetime.lifetime.ident.clone(),
        GenericParam::Type(ty) => ty.ident.clone(),
        GenericParam::Const(const_) => const_.ident.clone(),
      })
      .collect();
    let trait_ident = trait_item.ident.clone();
    let shadow_ident = shadow_trait_ident(&trait_item.ident)?;
    let (shadow, modified) = build_shadow_items(trait_item, &props)?;
    registry.local.push(PropsTraitInfo { trait_ident, shadow_ident, props, param_idents });
    expanded.push(shadow);
    expanded.push(modified);
  }

  // pass two: resolve struct-level `#[duck(..)]` entries; recurse into
  // nested inline modules
  let mut result: Vec<Item> = Vec::with_capacity(expanded.len() + 1);
  for item in expanded {
    match item {
      Item::Struct(mut struct_item) => {
        let Some(pos) = struct_item.attrs.iter().position(|attr| attr.path().is_ident("duck"))
        else {
          result.push(Item::Struct(struct_item));
          continue;
        };
        let attr = struct_item.attrs.remove(pos);
        let entries = parse_duck_entries_attribute(&attr)?;
        let resolved = resolve_ducky_entries(&struct_item, entries, &registry)?;
        let impls = build_duck_impls(&struct_item, &resolved)?;
        result.push(Item::Struct(struct_item));
        result.extend(impls);
      }
      Item::Mod(mut module) => {
        if let Some((_, inner)) = &mut module.content {
          // this pass already covered the module's contents; strip a
          // redundant `#[ducky]` marker so it is not processed twice
          module.attrs.retain(|attr| !attr.path().is_ident("ducky"));
          process_ducky_scope(inner, Some(&registry))?;
        }
        result.push(Item::Mod(module));
      }
      other => result.push(other),
    }
  }
  *items = result;
  Ok(())
}

/// Resolves the entries of a struct-level `#[duck(..)]` attribute inside a
/// `#[ducky]` scope. Entries with an explicit props list keep their trait
/// path verbatim; brace-less entries resolve their props from the registered
/// `#[props]` trait and infer the shadow-trait arguments.
fn resolve_ducky_entries(
  struct_item: &ItemStruct,
  entries: Vec<DuckEntry>,
  registry: &PropsRegistry<'_>,
) -> syn::Result<Vec<ResolvedDuckEntry>> {
  let mut resolved = Vec::new();
  for entry in entries {
    let DuckEntry { path, props } = entry;
    let Some(props) = props else {
      let (info, level) = lookup_props_trait(&path, registry)?;
      let trait_path = resolve_trait_path(info, level, &path, struct_item)?;
      let props = info.props.iter().map(|prop| prop.name.clone()).collect();
      resolved.push(ResolvedDuckEntry { trait_path, props });
      continue;
    };
    let trait_path = realign_path(&path, registry);
    validate_impl_path(&trait_path)?;
    resolved.push(ResolvedDuckEntry { trait_path, props });
  }
  Ok(resolved)
}

/// Builds the trait path for an impl referencing a registered trait:
/// `level` `super::` hops up to the declaring scope, then the shadow trait's
/// own ident (the exact token the declaration used) and the given arguments.
fn registered_trait_path(info: &PropsTraitInfo, level: usize, args: Option<TokenStream2>) -> Path {
  let shadow_ident = &info.shadow_ident;
  let mut tokens = quote!();
  for _ in 0..level {
    tokens.extend(quote!(super::));
  }
  match args {
    Some(args) => tokens.extend(quote!(#shadow_ident<#args>)),
    None => tokens.extend(quote!(#shadow_ident)),
  }
  parse2(tokens).expect("registered trait path is valid")
}

/// Rebuilds a braced entry's path on the registered trait when it refers to a
/// trait of this scope, so the reference and the macro-generated declaration
/// resolve together even from nested modules. Paths that do not match a
/// registered trait are kept verbatim.
fn realign_path(path: &Path, registry: &PropsRegistry<'_>) -> Path {
  if path.leading_colon.is_some() || path.segments.len() != 1 {
    return path.clone();
  }
  let segment = &path.segments[0];
  let Some((info, level)) = registry.find(&segment.ident.to_string()) else {
    return path.clone();
  };
  match &segment.arguments {
    PathArguments::None => registered_trait_path(info, level, None),
    PathArguments::AngleBracketed(args) => {
      let args = &args.args;
      registered_trait_path(info, level, Some(quote!(#args)))
    }
    PathArguments::Parenthesized(_) => path.clone(),
  }
}

/// Looks up the registered `#[props]` trait behind a brace-less entry path.
fn lookup_props_trait<'a>(
  path: &Path,
  registry: &'a PropsRegistry<'a>,
) -> syn::Result<(&'a PropsTraitInfo, usize)> {
  let ident = match path.segments.last() {
    Some(segment) if path.segments.len() == 1 && path.leading_colon.is_none() => &segment.ident,
    _ => {
      return Err(syn::Error::new_spanned(
        path,
        "the brace-less form `#[duck(_Show)]` requires the shadow trait of a \
         `#[props]` trait declared in a `#[ducky]` scope; write the props \
         explicitly: `#[duck(_Show{field, ..})]`",
      ));
    }
  };
  registry.find(&ident.to_string()).ok_or_else(|| {
    syn::Error::new(
      ident.span(),
      format!(
        "no `#[props]` trait generating `{ident}` was found in this `#[ducky]` scope; \
         declare the trait here or write the props explicitly: \
         `#[duck({ident}{{field, ..}})]`"
      ),
    )
  })
}

/// Builds the trait path of the generated impl for a brace-less entry:
/// explicit arguments are used verbatim with their count checked against the
/// registered trait, and missing arguments are inferred while every generic
/// parameter is the bare type of some prop.
fn resolve_trait_path(
  info: &PropsTraitInfo,
  level: usize,
  path: &Path,
  struct_item: &ItemStruct,
) -> syn::Result<Path> {
  let segment = &path.segments[0];
  match &segment.arguments {
    PathArguments::AngleBracketed(args) => {
      let expected = info.param_idents.len();
      let found = args.args.len();
      if found != expected {
        return Err(syn::Error::new_spanned(
          args,
          format!(
            "`{shadow}` takes {expected} generic argument(s) (trait `{trait}` \
             declares {expected} generic parameter(s)), but {found} were written",
            shadow = info.shadow_ident,
            trait = info.trait_ident,
          ),
        ));
      }
      let args = &args.args;
      Ok(registered_trait_path(info, level, Some(quote!(#args))))
    }
    PathArguments::None if info.param_idents.is_empty() => {
      Ok(registered_trait_path(info, level, None))
    }
    PathArguments::None => {
      // one argument per generic parameter, filled with the field type of the
      // prop declared as that bare parameter
      let fields = named_fields(struct_item)?;
      let mut args: Vec<&Type> = Vec::new();
      for param in &info.param_idents {
        let mut candidates =
          info.props.iter().filter(|prop| is_bare_param(&prop.ty, param)).peekable();
        let Some(first) = candidates.next() else {
          return Err(syn::Error::new(
            param.span(),
            format!(
              "cannot infer the shadow-trait arguments for `{shadow}`: generic \
               parameter `{param}` is not used as the bare type of any prop; \
               write them explicitly: `#[duck({shadow}<..>)]`",
              shadow = info.shadow_ident,
            ),
          ));
        };
        let first_field = find_field(fields, &first.name, &struct_item.ident)?;
        let first_ty = &first_field.ty;
        for other in candidates {
          let other_field = find_field(fields, &other.name, &struct_item.ident)?;
          let other_ty = &other_field.ty;
          if quote!(#first_ty).to_string() != quote!(#other_ty).to_string() {
            return Err(syn::Error::new(
              other.name.span(),
              format!(
                "props `{first}` and `{other}` both fill the generic parameter \
                 `{param}` of `{shadow}`, but their field types differ; write \
                 the arguments explicitly: `#[duck({shadow}<..>)]`",
                first = first.name,
                other = other.name,
                shadow = info.shadow_ident,
              ),
            ));
          }
        }
        args.push(first_ty);
      }
      Ok(registered_trait_path(info, level, Some(quote!(#(#args),*))))
    }
    PathArguments::Parenthesized(_) => unreachable!("rejected by `validate_impl_path`"),
  }
}

/// Whether `ty` is exactly the bare generic parameter `param` (no path
/// arguments, no qualification).
fn is_bare_param(ty: &Type, param: &Ident) -> bool {
  matches!(ty, Type::Path(type_path) if
    type_path.qself.is_none()
      && type_path.path.leading_colon.is_none()
      && type_path.path.segments.len() == 1
      && type_path.path.segments[0].ident == *param
      && type_path.path.segments[0].arguments.is_none())
}
