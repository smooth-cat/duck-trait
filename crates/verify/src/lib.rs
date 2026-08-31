//! Verification project for duck-trait.
//!
//! Every fixture scope carries its own `#[cfg(test)]` tests next to the
//! structs they exercise; visibility-specific fixtures live at the end.

#![allow(dead_code)]

use duck_trait::{duck_mod, ducks};

// ---------------------------------------------------------------------------
// README usage examples — `ducks!` form (recommended)
//
// One `ducks!` scope produces one shared set of accessor traits: A, S, I and B
// all share the `_Value` trait below, exactly as in the README examples.
// ---------------------------------------------------------------------------

ducks! {
  // basic usage
  pub struct A {
    #[duck]
    value: String,
  }

  trait Opr: _Value<String> {
    fn print_val(&self) {
      println!("{}", self.value());
    }

    fn set_good(&mut self) {
      self.value_set(String::from("good"));
    }

    fn get_mut_val(&mut self) -> &str {
      self.value_mut()
    }
  }

  impl Opr for A {}

  // same field name in different structs shares one trait
  pub struct S {
    #[duck]
    value: String,
  }

  pub struct I {
    #[duck]
    value: i32,
  }

  fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }

  // custom trait: `#[duck(MyValue<_>)]` also auto-implements the user's trait
  pub struct B {
    #[duck(MyValue<_>)]
    value: String,
  }

  trait MyValue<T>: _Value<T> {
    fn my_get(&self) -> &T {
      self.value()
    }
  }
}

#[cfg(test)]
#[test]
fn basic_accessors() {
  let mut a = A { value: String::from("hello") };
  assert_eq!(a.value(), "hello");
  a.set_good();
  assert_eq!(a.value(), "good");
  let s: &str = a.get_mut_val();
  assert_eq!(s, "good");
  a.print_val();
}

#[cfg(test)]
#[test]
fn one_generic_fn_accepts_both_structs() {
  let mut s = S { value: String::from("abc") };
  let mut i = I { value: 7 };

  assert_eq!(debug_value(&s), "\"abc\"");
  assert_eq!(debug_value(&i), "7");

  *s.value_mut() = String::from("xyz");
  i.value_set(42);
  assert_eq!(s.value(), "xyz");
  assert_eq!(i.value(), &42);
}

#[cfg(test)]
#[test]
fn custom_trait_auto_impl() {
  let mut b = B { value: String::from("hello") };
  assert_eq!(b.my_get(), "hello");
  b.value_set(String::from("world"));
  assert_eq!(b.my_get(), "world");
}

// ---------------------------------------------------------------------------
// multiple marked fields + unmarked fields — `ducks!` form
// ---------------------------------------------------------------------------

ducks! {
  pub struct Player {
    #[duck]
    name: String,
    #[duck]
    score: u32,
    nickname: String,
  }
}

#[cfg(test)]
#[test]
fn multiple_marked_fields() {
  let mut p = Player { name: String::from("neo"), score: 1, nickname: String::from("the one") };
  assert_eq!(p.name(), "neo");
  assert_eq!(p.score(), &1);
  p.name_set(String::from("trinity"));
  *p.score_mut() += 41;
  assert_eq!(p.name(), "trinity");
  assert_eq!(p.score(), &42);
  assert_eq!(p.nickname, "the one");
}

// ---------------------------------------------------------------------------
// attribute form `#[duck_mod]` — kept at the end; `ducks!` above is the
// recommended usage. README example 1 — attribute form
// ---------------------------------------------------------------------------

#[duck_mod]
mod readme_example {
  #![allow(private_bounds)]
  pub struct A {
    #[duck]
    value: String,
  }

  pub trait Opr: _Value<String> {
    fn print_val(&self) {
      println!("{}", self.value());
    }

    fn set_good(&mut self) {
      self.value_set(String::from("good"));
    }

    fn get_mut_val(&mut self) -> &str {
      self.value_mut()
    }
  }

  impl Opr for A {}

  #[cfg(test)]
  #[test]
  fn accessors() {
    let mut a = A { value: String::from("hello") };
    assert_eq!(a.value(), "hello");
    a.set_good();
    assert_eq!(a.value(), "good");
    let s: &str = a.get_mut_val();
    assert_eq!(s, "good");
    a.print_val();
  }
}

// ---------------------------------------------------------------------------
// README example 2 — same field name in different structs shares one trait
// ---------------------------------------------------------------------------

#[duck_mod]
mod shared_trait {
  #![allow(private_bounds)]
  pub struct A {
    #[duck]
    value: String,
  }

  pub struct B {
    #[duck]
    value: i32,
  }

  pub fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }

  #[cfg(test)]
  #[test]
  fn one_generic_fn_accepts_both_structs() {
    let mut a = A { value: String::from("abc") };
    let mut b = B { value: 7 };

    assert_eq!(debug_value(&a), "\"abc\"");
    assert_eq!(debug_value(&b), "7");

    *a.value_mut() = String::from("xyz");
    b.value_set(42);
    assert_eq!(a.value(), "xyz");
    assert_eq!(b.value(), &42);
  }
}

// ---------------------------------------------------------------------------
// generic structs
// ---------------------------------------------------------------------------

#[duck_mod]
mod generics {
  pub struct Wrapper<T: Clone> {
    #[duck]
    inner: T,
  }

  pub struct Borrowed<'a> {
    #[duck]
    text: &'a str,
  }

  #[cfg(test)]
  #[test]
  fn generic_structs() {
    let mut w = Wrapper { inner: vec![1, 2] };
    w.inner_mut().push(3);
    assert_eq!(w.inner(), &vec![1, 2, 3]);
    w.inner_set(Vec::new());
    assert!(w.inner().is_empty());

    let mut b = Borrowed { text: "quack" };
    assert_eq!(b.text(), &"quack");
    b.text_set("moo");
    assert_eq!(b.text(), &"moo");
  }
}

// ---------------------------------------------------------------------------
// nested inline modules are processed recursively, one trait set per scope
// ---------------------------------------------------------------------------

#[duck_mod]
mod outer {
  pub mod inner {
    pub struct Deep {
      #[duck]
      payload: u8,
    }

    #[cfg(test)]
    #[test]
    fn deep_scope() {
      let mut d = Deep { payload: 1 };
      *d.payload_mut() = 9;
      assert_eq!(d.payload(), &9);
    }
  }

  pub struct Shallow {
    #[duck]
    v: u8,
  }

  #[cfg(test)]
  #[test]
  fn shallow_scope() {
    let mut s = Shallow { v: 2 };
    s.v_set(3);
    assert_eq!(s.v(), &3);
  }
}

// ---------------------------------------------------------------------------
// raw identifiers
// ---------------------------------------------------------------------------

#[duck_mod]
mod raw_idents {
  pub struct Keyword {
    #[duck]
    r#type: String,
  }

  #[cfg(test)]
  #[test]
  fn raw_ident_field() {
    let mut k = Keyword { r#type: String::from("mallard") };
    assert_eq!(k.r#type(), "mallard");
    k.type_set(String::from("duck"));
    assert_eq!(k.r#type(), "duck");
  }
}

// ---------------------------------------------------------------------------
// inner attributes of the annotated module must survive the rewrite
// ---------------------------------------------------------------------------

#[duck_mod]
mod inner_attrs {
  #![allow(dead_code)]

  pub struct Idle {
    #[duck]
    v: u8,
  }

  fn never_called() {}

  #[cfg(test)]
  #[test]
  fn idle_usable() {
    let mut i = Idle { v: 1 };
    i.v_set(2);
    assert_eq!(i.v(), &2);
  }
}

// ---------------------------------------------------------------------------
// deep parsing: structs declared inside function bodies and other block
// scopes — every block generates (and can only see) its own trait set
// ---------------------------------------------------------------------------

ducks! {
  fn deep_in_ducks_fn() -> u8 {
    struct Local {
      #[_duck]
      v: u8,
    }
    let mut local = Local { v: 1 };
    local.v_set(5);
    *local.v()
  }
}

#[cfg(test)]
#[test]
fn ducks_fn_body_scope() {
  assert_eq!(deep_in_ducks_fn(), 5);
}

#[duck_mod]
mod fn_scopes {
  pub fn use_local() -> String {
    struct Local {
      #[_duck]
      v: String,
    }
    let mut local = Local { v: String::from("deep") };
    local.v_set(String::from("duck"));
    local.v().clone()
  }

  pub struct Outer {
    marker: u8,
  }

  impl Outer {
    pub fn deep(&self) -> u8 {
      struct Inner {
        #[_duck]
        v: u8,
      }
      *Inner { v: self.marker + 1 }.v()
    }
  }

  pub fn nested_mod_user() -> u8 {
    mod inner {
      pub struct Deep {
        #[duck]
        payload: u8,
      }

      pub fn make() -> u8 {
        let mut d = Deep { payload: 1 };
        d.payload_set(9);
        *d.payload()
      }
    }
    inner::make()
  }

  #[cfg(test)]
  #[test]
  fn local_struct_in_fn_body() {
    assert_eq!(use_local(), "duck");
  }

  #[cfg(test)]
  #[test]
  fn struct_inside_test_fn() {
    struct InTest {
      #[_duck]
      v: u8,
    }
    let mut t = InTest { v: 1 };
    t.v_set(2);
    assert_eq!(t.v(), &2);
  }

  #[cfg(test)]
  #[test]
  fn struct_in_method_body() {
    assert_eq!(Outer { marker: 1 }.deep(), 2);
  }

  #[cfg(test)]
  #[test]
  fn struct_in_nested_inline_mod_of_fn_body() {
    assert_eq!(nested_mod_user(), 9);
  }

  #[cfg(test)]
  #[test]
  fn structs_in_sibling_block_scopes() {
    {
      struct InBlock {
        #[_duck]
        v: u8,
      }
      let mut b = InBlock { v: 1 };
      b.v_set(2);
      assert_eq!(b.v(), &2);
    }

    let in_closure = || {
      struct InClosure {
        #[_duck]
        v: u8,
      }
      *InClosure { v: 3 }.v()
    };
    assert_eq!(in_closure(), 3);

    for _ in 0..1 {
      struct InLoop {
        #[_duck]
        v: u8,
      }
      let mut l = InLoop { v: 4 };
      l.v_set(5);
      assert_eq!(l.v(), &5);
    }
  }
}

// ---------------------------------------------------------------------------
// trait visibility — `#[duck]` defaults to pub(crate), levels via `pub = ..`
// ---------------------------------------------------------------------------

#[duck_mod]
mod vis_default {
  pub struct CrateVisible {
    #[duck] // generates: pub(crate) trait _Value<T>
    value: u8,
  }

  pub fn make() -> CrateVisible {
    CrateVisible { value: 1 }
  }
}

// the default pub(crate) trait is usable from a sibling module
fn crate_visible_value(x: &impl vis_default::_Value<u8>) -> &u8 {
  x.value()
}

#[cfg(test)]
#[test]
fn default_vis_reaches_sibling_module() {
  assert_eq!(crate_visible_value(&vis_default::make()), &1);
}

#[duck_mod]
mod vis_pub {
  pub struct FullyPublic {
    #[duck(pub)] // generates: pub trait _Value<T>
    value: String,
  }

  pub fn make() -> FullyPublic {
    FullyPublic { value: String::from("duck") }
  }
}

#[cfg(test)]
#[test]
fn full_pub_vis() {
  use vis_pub::_Value;

  let mut f = vis_pub::make();
  f.value_set(String::from("goose"));
  assert_eq!(f.value(), "goose");
}

#[duck_mod]
mod vis_root {
  // the module the restricted trait is published *to* — it must be an
  // ancestor of the module declaring the struct (E0742)
  pub mod vis_in {
    use self::deep::_V;

    pub fn use_restricted(h: &deep::Restricted) -> &u8 {
      h.v()
    }

    pub mod deep {
      pub struct Restricted {
        // generates: pub(in crate::vis_root::vis_in) trait _V<T>
        #[duck(pub = crate::vis_root::vis_in)]
        v: u8,
      }

      pub fn make() -> Restricted {
        Restricted { v: 1 }
      }
    }
  }
}

#[cfg(test)]
#[test]
fn in_path_vis_reaches_target_module() {
  assert_eq!(vis_root::vis_in::use_restricted(&vis_root::vis_in::deep::make()), &1);
}

#[duck_mod]
mod vis_super_outer {
  pub mod inner {
    pub struct SuperVisible {
      #[duck(pub = super)] // generates: pub(super) trait _V<T>
      v: u8,
    }

    pub fn make() -> SuperVisible {
      SuperVisible { v: 1 }
    }
  }

  // `pub = super` makes the trait visible in `vis_super_outer`
  #[cfg(test)]
  #[test]
  fn super_vis_reaches_parent_module() {
    use self::inner::_V;

    let mut s = inner::make();
    s.v_set(2);
    assert_eq!(s.v(), &2);
  }
}

// visibility item and custom trait path mix freely, in any order
#[duck_mod]
mod vis_mixed_outer {
  pub mod inner {
    pub struct Mixed {
      // pub(super) trait _Value<T> + auto impl MyValue<String>
      #[duck(MyValue<_>, pub = super)]
      value: String,
    }

    pub(super) trait MyValue<T>: _Value<T> {
      fn my_get(&self) -> &T {
        self.value()
      }
    }

    pub fn make() -> Mixed {
      Mixed { value: String::from("duck") }
    }
  }

  #[cfg(test)]
  #[test]
  fn mixed_vis_and_custom_impl() {
    use self::inner::{_Value, MyValue};

    let mut m = inner::make();
    assert_eq!(m.my_get(), "duck");
    m.value_set(String::from("goose"));
    assert_eq!(m.value(), "goose");
  }
}

// `#[_duck]` keeps the trait private even at module level
#[duck_mod]
mod vis_private {
  pub struct Hidden {
    #[_duck]
    v: u8,
  }

  pub fn make() -> Hidden {
    Hidden { v: 1 }
  }

  #[cfg(test)]
  #[test]
  fn private_trait_usable_in_scope() {
    let mut h = make();
    h.v_set(2);
    assert_eq!(h.v(), &2);
  }
}

// keyword levels: `pub = crate` (explicit default) and `pub = self`
#[duck_mod]
mod vis_kw {
  pub struct KwCrate {
    #[duck(pub = crate)] // pub(crate) — same as the default, written out
    v: u8,
  }

  pub struct KwSelf {
    #[duck(pub = self)] // pub(self) — visible in `vis_kw` only
    w: u8,
  }

  pub fn make() -> (KwCrate, KwSelf) {
    (KwCrate { v: 1 }, KwSelf { w: 2 })
  }

  #[cfg(test)]
  #[test]
  fn keyword_levels() {
    let (mut c, mut s) = make();
    c.v_set(2);
    s.w_set(3);
    assert_eq!(c.v(), &2);
    assert_eq!(s.w(), &3);
  }
}
