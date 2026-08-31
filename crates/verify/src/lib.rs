//! Verification project for duck-trait.
//!
//! Every fixture scope carries its own `#[cfg(test)]` tests, because the
//! generated accessor traits are private to the scope that produced them
//! (matching the README's `trait _Value<T>`).

#![allow(dead_code)]

use duck_trait::{ducks, ducky};

// ---------------------------------------------------------------------------
// README example 1 — attribute form
// ---------------------------------------------------------------------------

#[ducky]
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

#[ducky]
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
// multiple marked fields + unmarked fields + `ducks!` form
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
  use crate::Player;

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
// generic structs
// ---------------------------------------------------------------------------

#[ducky]
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

#[ducky]
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

#[ducky]
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

#[ducky]
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
// custom trait: `#[duck(MyValue<_>)]` also auto-implements the user's trait
// ---------------------------------------------------------------------------

ducks! {
  pub struct B {
    #[duck(MyValue<_>)]
    value: String,
  }

  trait MyValue<T>: _Value<T> {
    fn my_get(&self) -> &T {
      self.value()
    }
  }

  #[cfg(test)]
  #[test]
  fn custom_trait_auto_impl() {
    let mut b = B { value: String::from("hello") };
    assert_eq!(b.my_get(), "hello");
    b.value_set(String::from("world"));
    assert_eq!(b.my_get(), "world");
  }
}
