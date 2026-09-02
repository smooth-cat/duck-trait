//! Field-based api declarations — every accessor trait of this crate, declared
//! once here and implemented by `#[props]` structs from any other module.

use duck_trait::fields;

fields! {
  value,      // pub(crate) trait _Value<T>
  pub name,   // pub trait _Name<T>
  inner,      // for the generic-struct fixture below
  r#type,     // raw identifiers: getter `r#type`, setter `type_set`
}
