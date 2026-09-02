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

If you are sure the trait and struct are in the same file, see [ducky module](#user-content-ducky-module)

```rust
use duck_trait::{duck, props};
// Declare the data the trait needs; this generates
// trait _Show {
//   name() name_set() name_mut()
//   score() score_set() score_mut()
// }
#[props(name: String, score: i32)]
pub trait Show {
  fn show(&self) {
    println!("{}: {}", self.name(), self.score());
  }
}

// Implement _Show via the duck macro.
// Since the trait and struct may live in different files,
// all prop names of _Show must be written out manually
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

- The shadow trait copies visibility, generics and where clauses verbatim from the annotated trait
  a prop type may reference a same-named trait generic — `#[props(inner: T)] trait Has<T>` generates `trait _Has<T>`
  lifetimes work the same way (`#[props(text: &'a str)] trait Text<'a>`)
- The struct side writes out the shadow trait's generic arguments in full (`#[duck(_Has<String>{inner})]`)
  or references the struct's own generics:
  `#[duck(_Has<T>{inner})] struct W<T> { inner: T }` generates `impl<T> _Has<T> for W<T>`
- Multiple shadow traits can be implemented at once: `#[duck(_A{a}, _B{b, c})]`.
- Props are matched to fields by name: a missing field is reported by the macro; a field whose type
  differs from the prop type is reported by the compiler on the generated impl (the impl's method
  signatures are built from the field types). Props not listed at all leave the trait unimplemented
  (E0046).
- Generated method names must not collide: a prop named `a_set` clashes with the setter of a prop
  named `a`.
- The legacy field-marker flow is untouched; both flows can be mixed freely, even inside one scope
  (see [`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md)).

### ducky module

If the trait and struct live in the same file, you can use it

Inside a `#[ducky]` module `#[duck(_Show{name, score})]` can be shortened to `#[duck(_Show)]`

```rust
use duck_trait::ducky;

#[ducky]
mod duckied {
  #[props(name: String, score: i32)]
  pub trait Show {
    fn show(&self) {
      println!("{}: {}", self.name(), self.score());
    }
  }

  // props derived from `Show`; extra fields are fine
  #[duck(_Show)]
  pub struct Player {
    pub name: String,
    pub score: i32,
  }

  impl Show for Player {}
}

use duckied::{Player, Show};

Player { name: "duck".to_owned(), score: 7 }.show();
```

Generic arguments may be written explicitly (their count is checked against the registered trait)
or are inferred for generic structs:

```rust
use duck_trait::ducky;

#[ducky]
mod duckied {
  #[props(inner: T)]
  pub trait Has<T> {
    fn get(&self) -> &T {
      self.inner()
    }
  }

  // generic arguments written explicitly; props still derived
  #[duck(_Has<String>)]
  pub struct Wrapper {
    inner: String,
  }

  impl Has<String> for Wrapper {}

  // generic structs infer `impl<T> _Has<T> for W<T>`
  #[duck(_Has)]
  pub struct W<T> {
    inner: T,
  }

  impl<T> Has<T> for W<T> {}
}
```

- Traits registered in enclosing `#[ducky]` scopes are visible to nested modules.
- Entries with an explicit props list (`#[duck(_Show{name, ..})]`) keep the standalone semantics.
- Entries referencing a trait outside every enclosing `#[ducky]` scope need the explicit props list.
- Field-level `#[duck]` / `#[_duck]` markers are not touched inside `#[ducky]`: the old flow belongs
  to `#[duck_mod]` / `ducks!` (see [`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md)).
- Inference covers props declared as a bare generic parameter; compound types (`items: Vec<T>`) and
  parameters not used as a bare prop type require the explicit form.

### Naming conventions

| Prop                            | Shadow trait | Method                                     |
| ------------------------------- | ------------ | ------------------------------------------ |
| `value` (prop of `trait Show`)  | `_Show`      | `value()` / `value_set(v)` / `value_mut()` |
| `my_field`                      | `_Show`      | `my_field()` / `my_field_set(v)` / …       |
| `r#type`                        | `_Show`      | `r#type()` / `type_set(v)` / `type_mut()`  |

- Shadow trait name: `_` + the trait's name (`Show` → `_Show`); visibility, generics and where
  clauses are copied from the trait.
- Method names come from the prop: `xxx()` / `xxx_set(v)` / `xxx_mut()`; the setter takes the value
  by value and returns `()`.

### Supported features and limitations

**Supported**

- Generic traits and structs: `#[props(a: T)] trait Has<T>` with `#[duck(_Has<String>{a})]`, or
  `#[duck(_Has<T>{a})] struct W<T> { a: T }` generating `impl<T> _Has<T> for W<T>`.
- Lifetimes: `#[props(text: &'a str)] trait Text<'a>` with `#[duck(_Text<'a>{text})]`.
- Raw-identifier props: `r#type` generates `r#type()` / `type_set(v)` / `type_mut()`.
- Shadow traits are dyn-compatible (`dyn Show<String>`), since all generics are trait-level.
- `#[ducky]` module scopes: brace-less `#[duck(_Show)]` entries, argument inference, nested modules
  seeing the enclosing scopes' traits.
- Multiple shadow traits per struct: `#[duck(_A{a}, _B{b, c})]`.
- Coexists with the legacy field-marker flow (see [`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md)), even
  inside one scope.

**Limitations**
- The brace-less form only works inside `#[ducky]`; everywhere else the props list is required.
- Inference only covers props declared as a bare generic parameter; compound types (`items: Vec<T>`)
  and parameters not used as a bare prop type require the explicit form.
- Props not listed at all leave the trait unimplemented (E0046) — outside `#[ducky]` the macro
  cannot know the full props list.
- Raw-identifier trait names (`trait r#type`) cannot derive a shadow trait name; auto traits are
  rejected.

### Legacy field-marker flow

The original struct-first flow — field markers `#[duck]` / `#[_duck]` inside `ducks! { .. }` /
`#[duck_mod]` scopes, the `#[duck(MyValue<_>)]` custom-impl markers and their visibility rules — is
unchanged and documented in [`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md).

### Project structure

```
duck-trait
├── crates
│   ├── duck-trait    # Publishable proc-macro crate (ducks / duck_mod / duck / props / ducky)
│   └── verify        # Verification project (fixtures + tests)
├── docs              # Design notes (Chinese)
├── duck-trait-old.md # Legacy field-marker flow
└── readme.md         # Trait-first API
```

### Running verification

```sh
cargo test                        # Unit tests + doctests
cargo clippy --all-targets        # Static checks
```

Requires Rust 1.85+ (edition 2024).

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

### 基础使用

如果你确定 trait 和 struct 在同一文件可以查看 [ducky模块](#user-content-ducky模块)

```rust
use duck_trait::{duck, props};
// 声明需要的数据, 这里会生成
// trait _Show {
//   name() name_set() name_mut()
//   score() score_set() score_mut()
// }          
#[props(name: String, score: i32)]
pub trait Show {
  fn show(&self) {
    println!("{}: {}", self.name(), self.score());
  }
}

// 通过 duck 宏实现 _Show,
// 考虑 trait 和 struct 可能在不同文件中
// 所以必须手动写出 _Show 包含的所有属性名
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

- shadow trait 的可见性、generics、where 子句原样复制自原 trait
  prop 类型可以引用 trait 的同名泛型 —— `#[props(inner: T)] trait Has<T>` 生成 `trait _Has<T>`
  生命周期同理（`#[props(text: &'a str)] trait Text<'a>`）
- struct 侧需要完整写出 shadow trait 的泛型参数（`#[duck(_Has<String>{inner})]`）
  也可以引用 struct 自身的泛型：
  `#[duck(_Has<T>{inner})] struct W<T> { inner: T }` 生成 `impl<T> _Has<T> for W<T>`
- 一次可以实现多个 shadow trait：`#[duck(_A{a}, _B{b, c})]`。
- prop 按名字匹配字段：缺少同名字段由宏报错；字段类型与 prop 类型不一致时，由编译器在生成的 impl
  处报错（impl 的方法签名由字段类型生成）。完全未列出的 props 会导致 trait 未实现（E0046）。
- 生成的方法名不允许冲突：prop `a_set` 会与 prop `a` 的 setter 冲突。
- 旧字段标记方案完全不变；两种方案可以自由混用，甚至可以在同一个作用域内混用
  （见 [`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md)）。

### ducky模块

如果 trait 和 struct 在同一文件，可以使用它

在 `#[ducky]` 模块内 `#[duck(_Show{name, score})]` 可以直接简写成 `#[duck(_Show)]`

```rust
use duck_trait::ducky;

#[ducky]
mod duckied {
  #[props(name: String, score: i32)]
  pub trait Show {
    fn show(&self) {
      println!("{}: {}", self.name(), self.score());
    }
  }

  // props 从 Show 推导；多出的字段不受影响
  #[duck(_Show)]
  pub struct Player {
    pub name: String,
    pub score: i32,
  }

  impl Show for Player {}
}

use duckied::{Player, Show};

Player { name: "duck".to_owned(), score: 7 }.show();
```

泛型参数可以显式写出（个数会与注册的 trait 校验），泛型 struct 则自动推导：

```rust
use duck_trait::ducky;

#[ducky]
mod duckied {
  #[props(inner: T)]
  pub trait Has<T> {
    fn get(&self) -> &T {
      self.inner()
    }
  }

  // 泛型参数显式写出；props 仍然自动推导
  #[duck(_Has<String>)]
  pub struct Wrapper {
    inner: String,
  }

  impl Has<String> for Wrapper {}

  // 泛型 struct 自动推导出 `impl<T> _Has<T> for W<T>`
  #[duck(_Has)]
  pub struct W<T> {
    inner: T,
  }

  impl<T> Has<T> for W<T> {}
}
```

- 嵌套模块可以看到外层 `#[ducky]` 作用域注册的 trait。
- 显式列出 props 的条目（`#[duck(_Show{name, ..})]`）保持独立使用时的语义。
- 引用的 trait 不在任何外层 `#[ducky]` 作用域内时，需要写显式 props 列表。
- 字段级 `#[duck]` / `#[_duck]` 标记在 `#[ducky]` 内不处理：旧流程仍属于
  `#[duck_mod]` / `ducks!`（见 [`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md)）。
- 自动推导仅覆盖"声明为裸泛型参数"的 props；复合类型（`items: Vec<T>`）与未被裸引用的泛型
  参数需要写显式形式。

### 命名约定

| prop                            | shadow trait | 方法                                        |
| ------------------------------- | ------------ | ------------------------------------------- |
| `value`（`trait Show` 的 prop） | `_Show`      | `value()` / `value_set(v)` / `value_mut()`  |
| `my_field`                      | `_Show`      | `my_field()` / `my_field_set(v)` / …        |
| `r#type`                        | `_Show`      | `r#type()` / `type_set(v)` / `type_mut()`   |

- shadow trait 名：`_` + trait 名（`Show` → `_Show`）；可见性、generics、where 子句复制自原
  trait。
- 方法名来自 prop：`xxx()` / `xxx_set(v)` / `xxx_mut()`；setter 接收值并返回 `()`。

### 支持与限制

**支持**

- 泛型 trait 与泛型 struct：`#[props(a: T)] trait Has<T>` 配 `#[duck(_Has<String>{a})]`，或
  `#[duck(_Has<T>{a})] struct W<T> { a: T }` 生成 `impl<T> _Has<T> for W<T>`。
- 生命周期：`#[props(text: &'a str)] trait Text<'a>` 配 `#[duck(_Text<'a>{text})]`。
- raw ident prop：`r#type` 生成 `r#type()` / `type_set(v)` / `type_mut()`。
- shadow trait 满足 dyn 兼容（`dyn Show<String>`），泛型均为 trait 级参数。
- `#[ducky]` 模块作用域：免花括号的 `#[duck(_Show)]` 条目、泛型参数自动推导、嵌套模块可见外层
  注册的 trait。
- 一个 struct 一次实现多个 shadow trait：`#[duck(_A{a}, _B{b, c})]`。
- 与旧字段标记流程（见 [`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md)）自由混用，甚至同一作用域内混用。

**限制**
- 免花括号写法只在 `#[ducky]` 内可用；其余场景必须写 props 列表。
- 自动推导仅覆盖"声明为裸泛型参数"的 props；复合类型（`items: Vec<T>`）与未被裸引用的泛型参数
  需要写显式形式。
- 完全未列出的 props 会导致 trait 未实现（E0046）——`#[ducky]` 之外宏无法知道完整的 props 列表。
- raw ident 的 trait 名（`trait r#type`）无法派生 shadow trait 名；不支持 auto trait。

### 旧版字段标记流程

最初的 struct 优先流程——`ducks! { .. }` / `#[duck_mod]` 作用域内的字段标记 `#[duck]` /
`#[_duck]`、`#[duck(MyValue<_>)]` 自定义 impl 标记及其可见性规则——完全不变，文档移至
[`duck-trait-old.md`](https://github.com/smooth-cat/duck-trait/blob/main/duck-trait-old.md)。

### 项目结构

```
duck-trait
├── crates
│   ├── duck-trait    # 可发布的 proc-macro crate（ducks / duck_mod / duck / props / ducky）
│   └── verify        # 验证项目（fixture + 测试）
├── docs              # 设计笔记（中文）
├── duck-trait-old.md # 旧版字段标记流程
└── readme.md         # trait 优先的新 API
```

### 运行验证

```sh
cargo test                        # 单元测试 + doctest
cargo clippy --all-targets        # 静态检查
```

需要 Rust 1.85+（edition 2024）。
