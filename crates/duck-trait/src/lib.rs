//! `duck-trait` — stop repeating `get`/`set`/`get_mut` declarations in traits.
//!
//! Mark fields with `#[duck]` inside a scope annotated with `#[ducky]` (or
//! wrapped in `ducks! { .. }`), and the macros generate one accessor trait per
//! field name together with the impls for every marked struct:
//!
//! ```rust
//! use duck_trait::ducky;
//!
//! #[ducky]
//! mod model {
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

use std::collections::{BTreeMap, BTreeSet, btree_map};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
  Attribute, Fields, File, GenericArgument, Generics, Ident, Item, ItemMod, ItemStruct, Meta, Path,
  PathArguments, Token, Type, parse2, punctuated::Punctuated,
};

/// Attribute form: `#[ducky] mod name { .. }`.
///
/// Scans the module (recursively into nested inline modules) for struct fields
/// marked with `#[duck]`, strips the markers and generates the accessor traits
/// plus their impls into each scope.
#[proc_macro_attribute]
pub fn ducky(attr: TokenStream, item: TokenStream) -> TokenStream {
  expand_attr(attr.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Function-like form: `ducks! { .. }`.
///
/// Same transformation as `#[ducky]`, but the wrapped items are placed directly
/// into the enclosing scope without introducing a module.
#[proc_macro]
pub fn ducks(item: TokenStream) -> TokenStream {
  expand_bang(item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Marker stub. `#[duck]` is consumed (stripped) by `#[ducky]`/`ducks!` before
/// the compiler ever resolves it, so this macro only runs when the marker is
/// used outside a ducky scope, or on something other than a struct field.
#[proc_macro_attribute]
pub fn duck(_attr: TokenStream, item: TokenStream) -> TokenStream {
  syn::Error::new_spanned(
    TokenStream2::from(item),
    "`#[duck]` must be applied to a named struct field inside a scope \
         annotated with `#[ducky]` or wrapped in `ducks! { .. }`",
  )
  .into_compile_error()
  .into()
}

// ---------------------------------------------------------------------------
// expansion entry points
// ---------------------------------------------------------------------------

fn expand_attr(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
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
      "`#[ducky]` cannot scan a file-based module (`mod name;`); \
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

/// One `#[duck]`-marked field, waiting to be grouped into a trait.
struct DuckField {
  struct_ident: Ident,
  generics: Generics,
  field_ident: Ident,
  field_ty: Type,
  /// Trait paths from `#[duck(MyTrait, ..)]` to additionally implement for the
  /// struct.
  custom_impls: Vec<Path>,
}

fn process_scope(items: &mut Vec<Item>) -> syn::Result<()> {
  let mut collected: Vec<DuckField> = Vec::new();

  for item in items.iter_mut() {
    match item {
      Item::Struct(item_struct) => collect_from_struct(item_struct, &mut collected)?,
      Item::Mod(item_mod) => {
        if let Some((_, inner)) = &mut item_mod.content {
          process_scope(inner)?;
        }
      }
      Item::Enum(item_enum) => {
        for variant in &item_enum.variants {
          reject_duck_on_foreign_fields(variant.fields.iter())?;
        }
      }
      Item::Union(item_union) => {
        reject_duck_on_foreign_fields(item_union.fields.named.iter())?;
      }
      _ => {}
    }
  }

  let generated = generate(&collected)?;
  reject_conflicts(items, &generated)?;
  items.extend(generated);
  Ok(())
}

fn collect_from_struct(item_struct: &mut ItemStruct, out: &mut Vec<DuckField>) -> syn::Result<()> {
  match &mut item_struct.fields {
    Fields::Named(fields_named) => {
      for field in fields_named.named.iter_mut() {
        let Some(pos) = field.attrs.iter().position(is_duck) else {
          continue;
        };
        let attr = field.attrs.remove(pos);
        let custom_impls = parse_custom_impls(&attr)?;
        out.push(DuckField {
          struct_ident: item_struct.ident.clone(),
          generics: item_struct.generics.clone(),
          field_ident: field.ident.clone().expect("named field has an ident"),
          field_ty: field.ty.clone(),
          custom_impls,
        });
      }
    }
    Fields::Unnamed(_) | Fields::Unit => {
      for field in item_struct.fields.iter() {
        if let Some(attr) = field.attrs.iter().find(|attr| is_duck(attr)) {
          return Err(syn::Error::new_spanned(attr, "`#[duck]` only supports named struct fields"));
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
    if let Some(attr) = field.attrs.iter().find(|attr| is_duck(attr)) {
      return Err(syn::Error::new_spanned(attr, "`#[duck]` only supports named struct fields"));
    }
  }
  Ok(())
}

fn is_duck(attr: &Attribute) -> bool {
  attr.path().is_ident("duck")
}

/// Extracts the trait paths from a `#[duck]` marker: plain `#[duck]` (no
/// arguments), or `#[duck(MyTrait, ..)]` whose paths the macro additionally
/// implements for the marked struct.
fn parse_custom_impls(attr: &Attribute) -> syn::Result<Vec<Path>> {
  match &attr.meta {
    Meta::Path(_) => Ok(Vec::new()),
    Meta::NameValue(_) => {
      Err(syn::Error::new_spanned(attr, "`#[duck]` does not support name-value arguments"))
    }
    Meta::List(_) => {
      let paths: Punctuated<Path, Token![,]> =
        attr.parse_args_with(Punctuated::parse_terminated).map_err(|_| {
          syn::Error::new_spanned(
            attr,
            "`#[duck(...)]` expects trait paths, e.g. `#[duck(MyValue<_>)]`",
          )
        })?;
      if paths.is_empty() {
        return Err(syn::Error::new_spanned(
          attr,
          "`#[duck(...)]` requires at least one trait path",
        ));
      }
      Ok(paths.into_iter().collect())
    }
  }
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

fn generate(fields: &[DuckField]) -> syn::Result<Vec<Item>> {
  // trait name (without leading `_`) -> (getter method base, fields sharing it)
  let mut groups: BTreeMap<String, (String, String, Vec<&DuckField>)> = BTreeMap::new();
  // (struct, rendered trait path) of custom impls already emitted
  let mut emitted_custom: BTreeSet<(String, String)> = BTreeSet::new();

  for field in fields {
    let trait_name = trait_name_for(&field.field_ident)?;
    let base = method_base(&field.field_ident);
    match groups.entry(trait_name) {
      btree_map::Entry::Occupied(mut occupied) => {
        let trait_name = occupied.key().clone();
        let (existing_base, existing_field, list) = occupied.get_mut();
        if *existing_base != base {
          return Err(syn::Error::new(
            field.field_ident.span(),
            format!(
              "field `{}` and field `{}` produce the same trait `_{}` \
                             but require different method names",
              field.field_ident, existing_field, trait_name,
            ),
          ));
        }
        list.push(field);
      }
      btree_map::Entry::Vacant(vacant) => {
        vacant.insert((base, field.field_ident.to_string(), vec![field]));
      }
    }
  }

  let mut tokens = TokenStream2::new();
  for (trait_name, (base, _, group)) in groups {
    let trait_ident = format_ident!("_{}", trait_name);
    // getter keeps the original spelling (raw idents such as `r#type` stay raw)
    let get_ident = &group[0].field_ident;
    let set_ident = format_ident!("{}_set", base);
    let mut_ident = format_ident!("{}_mut", base);

    tokens.extend(quote! {
        trait #trait_ident<T> {
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

fn reject_conflicts(existing: &[Item], generated: &[Item]) -> syn::Result<()> {
  let generated_names: Vec<&Ident> = generated.iter().filter_map(item_ident).collect();
  for item in existing {
    if let Some(ident) = item_ident(item)
      && generated_names.iter().any(|name| **name == *ident)
    {
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
