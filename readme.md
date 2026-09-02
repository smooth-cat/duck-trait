# duck-trait

[中文](#user-content-中文) | [English](#user-content-english)

## English

### Eliminates repetitive get / set code in trait as below :

```rust
struct A { value: i32 }
struct B { value: i32 }

trait Get {
  fn value(&self) -> i32;
}

trait Opr: Get {
  fn double(&self) -> i32 {
    self.value() * 2
  }
}
/*----------------- Repetitive code you must implement -----------------*/
impl Get for A {
  fn value(&self) -> i32 {
    self.value
  }
}

impl Get for B {
  fn value(&self) -> i32 {
    self.value
  }
}
/*----------------- Only then can you use -----------------*/
impl Opr for A;
impl Opr for B; 
```

### Basic usage

```rust
use duck_trait::ducks;

/*-------- Generates traits and impls; code stays in top scope --------*/
// ducks! can contain multiple structs or other code
ducks! { 
  pub struct A {
    // Marks the field to generate accessors for.
    // `duck` is a marker only; no import needed
    #[duck] 
    value: String,
  }
}

// Convention: the field `xxx` generates the "_Xxx<T>" trait
trait Opr: _Value<String> {
  fn print_val(&self) {
    // Read &xxx via xxx(), this case xxx is value
    println!("{}", self.value());         
  }

  fn set_good(&mut self) {
    // Set xxx via xxx_set(_)
    self.value_set(String::from("good")); 
  }

  fn get_mut_val(&mut self) -> &str {
    // Get &mut xxx via xxx_mut()
    self.value_mut()                      
  }
}

impl Opr for A {}
```

### Before / after expansion of `ducks! { .. }`

```rust
/*----------------- Before expansion -----------------*/
ducks! { 
  pub struct A {
    // Marks the field to generate accessors for
    #[duck] 
    value: String,
  }
}
/*----------------- After expansion -----------------*/
pub(crate) trait _Value<T> {
  fn value(&self) -> &T;
  fn value_set(&mut self, v: T);
  fn value_mut(&mut self) -> &mut T;
}

struct A {
  value: String,
}

impl _Value<String> for A {
  fn value(&self) -> &String {
    &self.value
  }
  fn value_set(&mut self, v: String) {
    self.value = v;
  }
  fn value_mut(&mut self) -> &mut String {
    &mut self.value
  }
}
```

### Generated trait visibility

```rust
use duck_trait::ducks;

ducks! {
  pub struct Player {
    // generates: pub(crate) trait _Name<T>
    #[duck] 
    name: String,
    // generates: pub trait _Score<T>
    #[duck(pub)] 
    score: u32,
    // generates: private trait _Title<T>
    #[_duck] 
    title: String,
  }
}

// pub(crate): usable anywhere in the crate
fn shout_name(player: &impl _Name<String>) {
  println!("{}", player.name());
}

// pub: external crates can use it too,
// e.g. as a supertrait of a public trait
fn shout_score(player: &impl _Score<u32>) {
  println!("{}", player.score());
}

// private: usable in this scope only
fn shout_title(player: &impl _Title<String>) {
  println!("{}", player.title());
}
```

The marker chooses the visibility of the generated trait:

| Marker                      | Generated trait visibility     |
| --------------------------- | ------------------------------ |
| `#[duck]`                   | `pub(crate)` (default)         |
| `#[duck(pub)]`              | `pub`                          |
| `#[duck(pub = crate)]`      | `pub(crate)`                   |
| `#[duck(pub = super)]`      | `pub(super)`                   |
| `#[duck(pub = self)]`       | `pub(self)`                    |
| `#[duck(pub = crate::foo)]` | `pub(in crate::foo)`           |
| `#[_duck]`                  | private to the declaring scope |

- `#[duck]` defaults to `pub(crate)` so accessors work across the whole crate without exporting
  them; `#[duck(pub)]` exports the trait for external users (e.g. as a supertrait of a public
  trait).
- The visibility item may sit anywhere in `#[duck(..)]`, next to custom trait paths
  `#[duck(MyValue<_>, pub = super)]` — at most one visibility item per marker.
- Markers can be mixed within one struct — each field's trait gets its own visibility

- All structs sharing one trait must declare the same visibility; mismatched markers fail to
  compile.
- `pub = <path>` is rendered as `pub(in path)`; the path must start with `crate`, `super`, or
  `self` and must be an ancestor module of the declaring struct.
- Block scopes (function bodies, closures, ...) cannot carry visibility qualifiers, so they only
  accept `#[_duck]`.
- `#[_duck]` always generates a private trait and rejects `pub` items.

### Custom accessors

Inside a `#[duck_mod]` / `ducks!` scope, define `trait MyGetSet: _Xxx<some_type>` to create a custom accessor.

Then use `#[duck(MyGetSet)]` or `#[duck(MyGetSet<_>)]` (for custom traits with generics) to generate the code.

(PS: in `#[duck(MyGetSet<_>)]`, `_` is the placeholder for the type of the decorated `value` field.)

```rust
use duck_trait::ducks;
/*----------------- Before expansion -----------------*/
ducks! {
  pub struct B {
    // Additionally generates impl MyValue<String> for B
    #[duck(MyValue<_>)] 
    value: String,
  }
}
// The custom trait binds to the field type
// via `_Value<some_type>`
trait MyValue<V>: _Value<V> {
  // In Rust, a method cannot share its name
  // with the supertrait (_Value);
  // so just pick a nice name yourself
  fn my_get(&self) -> &V {
    // Put your extra logic here
    self.value()
  }
}

let b = B { value: String::from("good") };
b.my_get();
b.value(); // Still works

/*----------------- After expansion -----------------*/
pub struct B {
  value: String,
}

pub(crate) trait MyValue<V>: _Value<V> {
  fn my_get(&self) -> &V {
    self.value()
  }
}

pub(crate) trait _Value<T> {
  fn value(&self) -> &T;
  fn value_set(&mut self, v: T);
  fn value_mut(&mut self) -> &mut T;
}

impl _Value<String> for B {
  fn value(&self) -> &String {
    &self.value
  }
  fn value_set(&mut self, v: String) {
    self.value = v;
  }
  fn value_mut(&mut self) -> &mut String {
    &mut self.value
  }
}

// All MyValue methods have default impls,
// so the generated impl is empty
impl MyValue<String> for B {}

let b = B { value: String::from("good") };
b.my_get();
b.value();
```

A bare `_` placeholder inside `(..)` is replaced with the type of the decorated field; all other
arguments are kept as-is (you may reference the struct's own generic parameters):

| Marker (field `value: String`) | Generated                        |
| ------------------------------ | -------------------------------- |
| `#[duck(MyValue)]`             | `impl MyValue for B`             |
| `#[duck(MyValue<_>)]`          | `impl MyValue<String> for B`     |
| `#[duck(MyValue<u8, _>)]`      | `impl MyValue<u8, String> for B` |

- `_` always equals the field type: the supertrait bound guarantees only the field type can satisfy
  `_Value<V>`; any other type fails to compile.
- Generic structs work the same way: `struct Wrapper<T: Clone> { #[duck(MyValue<_>)] inner: T }`
  generates `impl<T: Clone> MyValue<T> for Wrapper<T>`.
- The auto-generated impl requires every method of the custom trait to have a default implementation;
  otherwise use a plain `#[duck]` marker and hand-write `impl MyValue<String> for B {}` in the same
  scope.
- `_` only supports a bare placeholder (e.g. the `_` in `MyValue<Vec<_>>` is not replaced).

### Props: write the trait first

With field markers, a trait has to wait for a struct to exist. `#[props(..)]` declares the data a
trait needs on the trait itself: the macro generates a shadow trait `_Show` with the accessor
methods and binds `Show` to it as a supertrait. A struct opts in with `#[duck(_Show{name, score})]`,
listing the props it provides; the generated impl reads the same-named fields.

```rust
use duck_trait::{duck, props};

#[props(name: String, score: i32)]
pub trait Show {
  fn show(&self) {
    println!("{}: {}", self.name(), self.score());
  }
}

#[duck(_Show{name, score})]
struct Player {
  name: String,
  score: i32,
}

impl Show for Player {}
```

Before / after expansion:

```rust
/*----------------- Before -----------------*/
#[props(name: String, score: i32)]
pub trait Show { /* ... */ }

#[duck(_Show{name, score})]
struct Player { name: String, score: i32 }
/*----------------- After -----------------*/
pub trait _Show {
  fn name(&self) -> &String;
  fn name_set(&mut self, v: String);
  fn name_mut(&mut self) -> &mut String;
  fn score(&self) -> &i32;
  fn score_set(&mut self, v: i32);
  fn score_mut(&mut self) -> &mut i32;
}

pub trait Show: _Show { /* ... */ }

struct Player { name: String, score: i32 }

impl _Show for Player {
  fn name(&self) -> &String { &self.name }
  fn name_set(&mut self, v: String) { self.name = v; }
  fn name_mut(&mut self) -> &mut String { &mut self.name }
  // score likewise
}
```

- The shadow trait copies visibility, generics and where clauses verbatim from the annotated trait;
  a prop type may reference a same-named trait generic — `#[props(inner: T)] trait Has<T>` generates
  `trait _Has<T>` — lifetimes work the same way (`#[props(text: &'a str)] trait Text<'a>`).
- The struct side writes the shadow-trait arguments in full (`#[duck(_Has<String>{inner})]`) and may
  reference the struct's own generics: `#[duck(_Has<T>{inner})] struct W<T> { inner: T }` generates
  `impl<T> _Has<T> for W<T>`.
- Multiple shadow traits can be implemented at once: `#[duck(_A{a}, _B{b, c})]`.
- Props are matched to fields by name: a missing field is reported by the macro; a field whose type
  differs from the prop type is reported by the compiler on the generated impl (the impl's method
  signatures are built from the field types). Props not listed at all leave the trait unimplemented
  (E0046).
- Generated method names must not collide: a prop named `a_set` clashes with the setter of a prop
  named `a`.
- The old field-marker flow is untouched; both flows can be mixed freely, even inside one
  `ducks!` / `#[duck_mod]` scope.

### Duplicate field names reuse the same trait

```rust
use duck_trait::ducks;

ducks! {
  pub struct A {
    #[duck]
    value: String,
  }

  pub struct B {
    #[duck]
    value: i32,
  }

  // A single generic function accepts both structs —
  // this is exactly the point of "duck typing"
  fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }
}
```

### Naming conventions

| Field      | Generated trait | Method                                     |
| ---------- | --------------- | ------------------------------------------ |
| `value`    | `_Value<T>`     | `value()` / `value_set(v)` / `value_mut()` |
| `my_field` | `_MyField<T>`   | `my_field()` / `my_field_set(v)` / …       |
| `r#type`   | `_Type<T>`      | `r#type()` / `type_set(v)` / `type_mut()`  |

- Trait name: the field name converted to UpperCamelCase with a `_` prefix.
- Within the same scope, all structs with the same field name share one trait; different scopes each
  generate their own.
- The setter takes the value by value and returns `()`.

### Supported features and limitations

**Supported**

- Generic structs (including where clauses): `struct Wrapper<T: Clone> { #[duck] inner: T }` generates
  `impl<T: Clone> _Inner<T> for Wrapper<T>`.
- `#[duck(MyTrait(..))]`: in addition to the accessors, automatically implements the custom trait for
  the struct, with the `_` placeholder equal to the field type (supported by both `#[duck_mod]` and
  `ducks!`).
- Trait visibility: `#[duck]` defaults to `pub(crate)`; `#[duck(pub)]` / `#[duck(pub = ..)]` set
  `pub` or restricted visibility; `#[_duck]` keeps the trait private (see "Trait visibility").
- Recursively processes nested inline modules and block scopes: function bodies, closures,
  `unsafe`/`async`/`const` blocks, loop/`if`/`match` branch blocks and method bodies each get their
  own set of traits, generated inside the scope where the struct is declared.
- Detects `_Xxx` naming conflicts before generation and emits a clear compile error on conflict.
- `#[props(..)]` / `#[duck(_Show{..})]`: trait-first shadow traits, see "Props: write the trait
  first".

**Limitations**
- The auto impl of `#[duck(MyTrait(..))]` requires every method of the custom trait to have a default
  implementation.
- Cannot scan the contents of `mod foo;` file modules (rustc does not pass file contents to macros).
  Use `ducks! { .. }` inside the file or switch to an inline module.
- Other macro invocations inside the scope are opaque: structs produced by a sibling macro cannot be
  scanned.
- Block scopes only accept `#[_duck]`: generated items there cannot carry visibility qualifiers
  (rustc E0449).
- All fields sharing one trait must declare the same visibility; mismatched markers fail to compile.
- If a field name clashes with an existing method (e.g. `clone`), call sites may hit method resolution
  ambiguity — an inherent behavior of Rust trait methods.

### Project structure

```
duck-trait
├── crates
│   ├── duck-trait    # Publishable proc-macro crate (duck_mod / ducks / duck / props)
│   └── verify        # Verification project (fixtures + tests)
└── readme.md
```

### Running verification

```sh
cargo test                        # Unit tests + doctests
cargo clippy --all-targets        # Static checks
```

Requires Rust 1.85+ (edition 2024).

### Attribute form #[duck_mod]`

`#[duck_mod]` is the equivalent attribute-macro form of `ducks!`, applied to an inline module; the
generated traits (defaulting to `pub(crate)`) stay inside the module namespace.
`ducks! { .. }` is preferred; the following `#[duck_mod]` examples correspond one-to-one with the
usages shown above.

```rust
use duck_trait::duck_mod;

#[duck_mod] // Generates the traits and impls
mod model {
  pub struct A {
    #[duck] // Marks the field
    value: String,
  }

  // Convention: the field `value` generates the "_Value" trait
  // (generic over the field type T)
  pub trait Opr: _Value<String> {
    fn print_val(&self) {
      // Read &value via value()
      println!("{}", self.value());
    }

    fn set_good(&mut self) {
      // Set the value via value_set(xxx)
      self.value_set(String::from("good"));
    }

    fn get_mut_val(&mut self) -> &str {
      // Get &mut value via value_mut()
      self.value_mut()
    }
  }

  impl Opr for A {}
}
```

The same rule — duplicate field names reusing one trait (traits are generated per scope) — holds here
as well:

```rust
use duck_trait::duck_mod;

#[duck_mod]
mod model {
  pub struct A {
    #[duck]
    value: String,
  }

  pub struct B {
    #[duck]
    value: i32,
  }

  // A single generic function accepts both structs —
  // this is exactly the point of "duck typing"
  pub fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }
}
```

- `#[duck_mod]` can only be applied to inline modules (`mod name { .. }`); it cannot scan file modules
  (`mod name;`, see "Supported features and limitations").

[中文](#user-content-中文) | [English](#user-content-english)

# duck-trait

## 中文

### 消除如下 trait 中重复的 get / set 代码

```rust
struct A { value: i32 }
struct B { value: i32 }

trait Get {
  fn value(&self) -> i32;
}

trait Opr: Get {
  fn double(&self) -> i32 {
    self.value() * 2
  }
}
/*----------------- 必须实现的重复代码 -----------------*/
impl Get for A {
  fn value(&self) -> i32 {
    self.value
  }
}

impl Get for B {
  fn value(&self) -> i32 {
    self.value
  }
}
/*----------------- 此时才能使用 -----------------*/
impl Opr for A;
impl Opr for B;
```

### 基础用法

```rust
use duck_trait::ducks;

/*----------------- 生成 trait 与 impl，代码仍在顶级作用域 -----------------*/
// ducks! 内可以放多个 struct 或 其他代码
ducks! { 
  pub struct A {
    // 标记要生成访问器的字段；
    // duck 仅作为标记，不需要被引入
    #[duck]
    value: String,
  }
}
// 约定：自动为字段 xxx 生成 “_Xxx<T>” trait
trait Opr: _Value<String> {
  fn print_val(&self) {
    // 通过 xxx() 获取 &xxx 此处为 &value
    println!("{}", self.value());         
  }

  fn set_good(&mut self) {
    // 通过 xxx_set(_) 设置值
    self.value_set(String::from("good")); 
  }

  fn get_mut_val(&mut self) -> &str {
    // 通过 xxx_mut() 获取 &mut xxx
    self.value_mut()                      
  }
}

impl Opr for A {}
```

### `ducks! { .. }` 展开前后对比

```rust
/*----------------- 展开前 -----------------*/
ducks! { 
  pub struct A {
    // 标记要生成访问器的字段
    #[duck] 
    value: String,
  }
}
/*----------------- 展开后 -----------------*/
pub(crate) trait _Value<T> {
  fn value(&self) -> &T;
  fn value_set(&mut self, v: T);
  fn value_mut(&mut self) -> &mut T;
}

struct A {
  value: String,
}

impl _Value<String> for A {
  fn value(&self) -> &String {
    &self.value
  }
  fn value_set(&mut self, v: String) {
    self.value = v;
  }
  fn value_mut(&mut self) -> &mut String {
    &mut self.value
  }
}
```

### 生成的 trait 可见性

```rust
use duck_trait::ducks;

ducks! {
  pub struct Player {
    // 生成: pub(crate) trait _Name<T>
    #[duck] 
    name: String,
    // 生成: pub trait _Score<T>
    #[duck(pub)] 
    score: u32,
    // 生成: 私有 trait _Title<T>
    #[_duck] 
    title: String,
  }
}

// pub(crate)：crate 内任何地方都可用
fn shout_name(player: &impl _Name<String>) {
  println!("{}", player.name());
}

// pub：外部 crate 也能使用，
// 比如作为公开 trait 的 supertrait
fn shout_score(player: &impl _Score<u32>) {
  println!("{}", player.score());
}

// 私有：仅当前作用域内可用
fn shout_title(player: &impl _Title<String>) {
  println!("{}", player.title());
}
```

由标记决定生成 trait 的可见性：

| 标记                        | 生成 trait 的可见性      |
| --------------------------- | ------------------------ |
| `#[duck]`                   | `pub(crate)`（默认）     |
| `#[duck(pub)]`              | `pub`                    |
| `#[duck(pub = crate)]`      | `pub(crate)`             |
| `#[duck(pub = super)]`      | `pub(super)`             |
| `#[duck(pub = self)]`       | `pub(self)`              |
| `#[duck(pub = crate::foo)]` | `pub(in crate::foo)`     |
| `#[_duck]`                  | 私有，仅声明作用域内可见 |

- `#[duck]` 默认生成 `pub(crate)`，访问器在整个 crate 内可用且不会导出到外部；需要给外部使用
  （如作为公开 trait 的 supertrait）时用 `#[duck(pub)]`。
- 可见性项在 `#[duck(..)]` 中位置任意，可与自定义 trait path 混排
  `#[duck(MyValue<_>, pub = super)]`，但最多写一个。
- 一个 struct 内可以混用不同可见性的标记，每个字段的 trait 各自独立

- 共享同一个 trait 的所有 struct 必须声明相同可见性，不一致会编译报错。
- `pub = <path>` 渲染为 `pub(in path)`；path 必须以 `crate`、`super` 或 `self` 开头，且必须是
  声明 struct 的祖先模块。
- 块级作用域（函数体、闭包等）内的 item 不能带可见性修饰符，只接受 `#[_duck]`。
- `#[_duck]` 永远生成私有 trait，并拒绝 `pub` 项。

### 自定义 访问器

在 `#[duck_mod]` / `ducks!` 作用域内，

通过 `trait MyGetSet: _Xxx<some_type>` 来自定义访问器。

此时使用  `#[duck(MyGetSet)]`  或  `#[duck(MyGetSet<_>)]` (自定义 trait 有泛型的场景) 来生成代码。

(PS:  `#[duck(MyGetSet<_>)]` 中 `_` 代表被修饰的 value 的类型占位符)

```rust
use duck_trait::ducks;
/*----------------- 展开前 -----------------*/
ducks! {
  pub struct B {
    // 额外生成 impl MyValue<String> for B
    #[duck(MyValue<_>)] 
    value: String,
  }
}
// 自定义 trait 通过 _Value<some_type> 绑定到字段类型
trait MyValue<V>: _Value<V> {
  // rust 中你不能声明与 supertrait 即 _Value 同名的函数，
  // 所以自己取个好听的名字即可
  fn my_get(&self) -> &V {
    // 这里写你的额外逻辑
    self.value()
  }
}

let b = B { value: String::from("good") };
b.my_get();
b.value(); // 依然可以工作

/*----------------- 展开后 -----------------*/
pub struct B {
  value: String,
}

pub(crate) trait MyValue<V>: _Value<V> {
  fn my_get(&self) -> &V {
    self.value()
  }
}

pub(crate) trait _Value<T> {
  fn value(&self) -> &T;
  fn value_set(&mut self, v: T);
  fn value_mut(&mut self) -> &mut T;
}

impl _Value<String> for B {
  fn value(&self) -> &String {
    &self.value
  }
  fn value_set(&mut self, v: String) {
    self.value = v;
  }
  fn value_mut(&mut self) -> &mut String {
    &mut self.value
  }
}

// MyValue 的所有方法均有默认实现，
// 因此自动生成的 impl 为空
impl MyValue<String> for B {}

let b = B { value: String::from("good") };
b.my_get();
b.value();
```

`(..)` 中裸写的 `_` 占位符会被替换为被标记字段的类型，其余实参原样保留（可引用 struct 自身
的泛型参数）：

| 标记（字段 `value: String`） | 生成                             |
| ---------------------------- | -------------------------------- |
| `#[duck(MyValue)]`           | `impl MyValue for B`             |
| `#[duck(MyValue<_>)]`        | `impl MyValue<String> for B`     |
| `#[duck(MyValue<u8, _>)]`    | `impl MyValue<u8, String> for B` |

- `_` 永远等于字段类型：supertrait 约束决定了只有字段类型能满足 `_Value<V>`，填其他类型无法
  编译。
- 泛型 struct 同理：`struct Wrapper<T: Clone> { #[duck(MyValue<_>)] inner: T }` 生成
  `impl<T: Clone> MyValue<T> for Wrapper<T>`。
- 自动生成的 impl 要求自定义 trait 的所有方法都有默认实现；否则请使用纯 `#[duck]` 标记，并在
  同一作用域内手写 `impl MyValue<String> for B {}`。
- `_` 只支持裸占位（如 `MyValue<Vec<_>>` 中的 `_` 不会被替换）。

### Props：先写 trait

字段标记方案下，trait 必须等 struct 先存在。`#[props(..)]` 把 trait 需要的数据直接声明在 trait
上：宏生成 shadow trait `_Show` 及其访问器方法，并把 `Show` 绑定为它的 supertrait。struct 侧用
`#[duck(_Show{name, score})]` 显式列出自己提供的 props，生成的 impl 直接读取同名字段。

```rust
use duck_trait::{duck, props};

#[props(name: String, score: i32)]
pub trait Show {
  fn show(&self) {
    println!("{}: {}", self.name(), self.score());
  }
}

#[duck(_Show{name, score})]
struct Player {
  name: String,
  score: i32,
}

impl Show for Player {}
```

展开前后对比：

```rust
/*----------------- 展开前 -----------------*/
#[props(name: String, score: i32)]
pub trait Show { /* ... */ }

#[duck(_Show{name, score})]
struct Player { name: String, score: i32 }
/*----------------- 展开后 -----------------*/
pub trait _Show {
  fn name(&self) -> &String;
  fn name_set(&mut self, v: String);
  fn name_mut(&mut self) -> &mut String;
  fn score(&self) -> &i32;
  fn score_set(&mut self, v: i32);
  fn score_mut(&mut self) -> &mut i32;
}

pub trait Show: _Show { /* ... */ }

struct Player { name: String, score: i32 }

impl _Show for Player {
  fn name(&self) -> &String { &self.name }
  fn name_set(&mut self, v: String) { self.name = v; }
  fn name_mut(&mut self) -> &mut String { &mut self.name }
  // score 同理
}
```

- shadow trait 的可见性、generics、where 子句原样复制自原 trait；prop 类型可以引用 trait 的同名
  泛型 —— `#[props(inner: T)] trait Has<T>` 生成 `trait _Has<T>`；生命周期同理
  （`#[props(text: &'a str)] trait Text<'a>`）。
- struct 侧需要完整写出 shadow trait 的泛型参数（`#[duck(_Has<String>{inner})]`），也可以引用
  struct 自身的泛型：`#[duck(_Has<T>{inner})] struct W<T> { inner: T }` 生成
  `impl<T> _Has<T> for W<T>`。
- 一次可以实现多个 shadow trait：`#[duck(_A{a}, _B{b, c})]`。
- prop 按名字匹配字段：缺少同名字段由宏报错；字段类型与 prop 类型不一致时，由编译器在生成的 impl
  处报错（impl 的方法签名由字段类型生成）。完全未列出的 props 会导致 trait 未实现（E0046）。
- 生成的方法名不允许冲突：prop `a_set` 会与 prop `a` 的 setter 冲突。
- 旧字段标记方案完全不变；两种方案可以自由混用，甚至可以在同一个 `ducks!` / `#[duck_mod]`
  作用域内混用。

### 出现重复字段名时会复用同一个 trait

```rust
use duck_trait::ducks;

ducks! {
  pub struct A {
    #[duck]
    value: String,
  }

  pub struct B {
    #[duck]
    value: i32,
  }

  // 一个泛型函数同时接受两个 struct ——
  // 这正是“鸭子类型”的意义
  fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }
}
```

### 命名约定

| 字段       | 生成的 trait  | 方法                                        |
| ---------- | ------------- | ------------------------------------------- |
| `value`    | `_Value<T>`   | `value()` / `value_set(v)` / `value_mut()`  |
| `my_field` | `_MyField<T>` | `my_field()` / `my_field_set(v)` / …        |
| `r#type`   | `_Type<T>`    | `r#type()` / `type_set(v)` / `type_mut()`   |

- trait 名：字段名转大驼峰并加 `_` 前缀。
- 同一作用域内，相同字段名的所有 struct 共享同一个 trait；不同作用域各自独立生成。
- setter 接收值并返回 `()`。

### 支持与限制

**支持**

- 泛型 struct（含 where 子句）：`struct Wrapper<T: Clone> { #[duck] inner: T }` 会生成
  `impl<T: Clone> _Inner<T> for Wrapper<T>`。
- `#[duck(MyTrait(..))]`：在访问器之外额外为该 struct 自动实现自定义 trait，`_` 占位符等于
  字段类型（`#[duck_mod]` 与 `ducks!` 均支持）。
- trait 可见性：`#[duck]` 默认 `pub(crate)`；`#[duck(pub)]` / `#[duck(pub = ..)]` 可设为 `pub`
  或受限可见性；`#[_duck]` 保持私有（见「trait 可见性」）。
- 递归处理嵌套的内联模块与块级作用域：函数体、闭包、`unsafe`/`async`/`const` 块、
  loop/`if`/`match` 分支块、方法体各自生成一组 trait，且生成在 struct 所在的作用域内。
- 生成前检测 `_Xxx` 命名冲突，冲突时给出明确的编译错误。
- `#[props(..)]` / `#[duck(_Show{..})]`：trait 优先的 shadow trait，见「Props：先写 trait」。

**限制**
- `#[duck(MyTrait(..))]` 的自动 impl 要求自定义 trait 所有方法均有默认实现。
- 无法扫描 `mod foo;` 文件模块的内容（rustc 不会把文件内容传给宏）。请在文件内使用
  `ducks! { .. }` 或改用内联模块。
- 作用域内的其他宏调用是不透明的：兄弟宏生成的 struct 无法被扫描。
- 块级作用域只接受 `#[_duck]`：其中生成的 item 不能带可见性修饰符（rustc E0449）。
- 共享同一个 trait 的所有字段必须声明相同可见性，不一致会编译报错。
- 字段名若与既有方法重名（如 `clone`），调用处可能出现方法解析歧义，这是 Rust trait 方法的固有行为。

### 项目结构

```
duck-trait
├── crates
│   ├── duck-trait    # 可发布的 proc-macro crate（duck_mod / ducks / duck / props）
│   └── verify        # 验证项目（fixture + 测试）
└── readme.md
```

### 运行验证

```sh
cargo test                        # 单元测试 + doctest
cargo clippy --all-targets        # 静态检查
```

需要 Rust 1.85+（edition 2024）。

### 属性形式 `#[duck_mod]`

`#[duck_mod]` 是 `ducks!` 的等价属性宏形式，作用于内联模块，生成的 trait（默认 `pub(crate)`）
保持在模块命名空间内。
推荐优先使用 `ducks! { .. }`；以下 `#[duck_mod]` 示例与前面的用法一一对应。

```rust
use duck_trait::duck_mod;

#[duck_mod] // 负责生成 trait 与 impl
mod model {
  pub struct A {
    #[duck] // 标记字段
    value: String,
  }

  // 约定：字段 value 生成 “_Value” 访问器 trait（对字段类型 T 泛型）
  pub trait Opr: _Value<String> {
    fn print_val(&self) {
      // 通过 value() 获取 &value
      println!("{}", self.value());
    }

    fn set_good(&mut self) {
      // 通过 value_set(xxx) 设置值
      self.value_set(String::from("good"));
    }

    fn get_mut_val(&mut self) -> &str {
      // 通过 value_mut() 获取 &mut value
      self.value_mut()
    }
  }

  impl Opr for A {}
}
```

重复字段名复用同一个 trait 的规则同样成立（trait 按作用域生成）：

```rust
use duck_trait::duck_mod;

#[duck_mod]
mod model {
  pub struct A {
    #[duck]
    value: String,
  }

  pub struct B {
    #[duck]
    value: i32,
  }

  // 一个泛型函数同时接受两个 struct ——
  // 这正是“鸭子类型”的意义
  pub fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }
}
```

- `#[duck_mod]` 只能作用于内联模块（`mod name { .. }`），无法扫描文件模块
  （`mod name;`，见「支持与限制」）。
