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
use syn::{
  Attribute, Block, Fields, File, GenericArgument, Generics, Ident, Item, ItemEnum, ItemMod,
  ItemStruct, ItemUnion, Meta, Path, PathArguments, Stmt, Token, Type, Visibility,
  parse::ParseStream, parse2, visit_mut::VisitMut,
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

/// Marker stub. `#[duck]` is consumed (stripped) by `#[duck_mod]`/`ducks!`
/// before the compiler ever resolves it, so this macro only runs when the
/// marker is used outside a duck_mod scope, or on something other than a
/// struct field.
#[proc_macro_attribute]
pub fn duck(_attr: TokenStream, item: TokenStream) -> TokenStream {
  syn::Error::new_spanned(
    TokenStream2::from(item),
    "`#[duck]` must be applied to a named struct field inside a scope \
         annotated with `#[duck_mod]` or wrapped in `ducks! { .. }`",
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
