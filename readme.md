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

⭐️ By convention the accessors are declared once in `src/_fields.rs`:

```rust
use duck_trait::{fields, props};

// 1. src/_fields.rs declares the accessor traits
use duck_trait::fields;
fields! {
  value,    // pub(crate) trait _Value<T> ...
  pub name, // pub trait _Name<T> ...
}

// 2. inherit the accessors via name: type
// no import needed — the macro adds it for you
// value: i32 maps to _Value<i32>
#[props(value: i32)]
trait Opr {
  fn double(&self) -> i32 {
    // you get the following 3 methods

    // read &mut i32
    self.value_mut();

    // set value
    self.value_set(99);

    // read &i32
    *self.value() * 2
  }
}

// 3. every field implements the declared traits by default;
// no marker needed — #[_prop] opts a field out
#[props]
struct A { 
  value: i32,
  // mark this field not impl accessor
  #[_prop] 
  ignored_prop: bool,
}
#[props]
struct B { value: i32 }

impl Opr for A {}
impl Opr for B {}

// use fields as generic bound
use crate::_fields::_Value;
fn double<T: _Value<i32>>(v: T) -> i32 {
	*v.value() * 2
}
```

**Limitations**: a field removed entirely by `#[cfg]` breaks the generated impl; avoid
mixing the marker api ([readme-duck.md](https://github.com/smooth-cat/duck-trait/blob/main/readme-duck.md)) and the field-based api for the same
field name in one crate (method resolution would be ambiguous).

### LLM

Add the following prompt to your project's `AGENTS.md`:

````markdown
### Simplify accessor code with duck-trait
1. only declare the accessor traits in src/_fields.rs
```rust
use duck_trait::fields;
fields! {
  value,    // pub(crate) trait _Value<T> ...
  pub name, // pub trait _Name<T> ...
}
```
2. inherit the accessors via name: type
```rust
#[props(value: i32)]
trait Foo {
  fn access(&self) {
    // you get the following 3 methods
    // read &i32
    *self.value();
    // read &mut i32
    self.value_mut();
    // set value
    self.value_set(99);
  }
}
```
3. every field implements the trait by default, no marker needed
```rust
#[props]
struct Player { value: i32 }
impl Foo for Player {}
```
4. use fields as generic bound
```rust
use crate::_fields::_Value;
fn double<T: _Value<i32>>(v: T) -> i32 {
  *v.value() * 2
}
```
````

### VS Code extension

Install the `duck-trait` extension to keep `_fields.rs` in sync:

quick fixes on the unresolved-trait errors,

and `Cmd+Shift+P` commands — declare the missing fields of the current file or the whole crate,

scaffold the declaration file, and wire up `mod` declarations, in monorepos and non-`src` layouts
too.

### Naming conventions

| Field      | Generated trait | Method                                     |
| ---------- | --------------- | ------------------------------------------ |
| `value`    | `_Value<T>`     | `value()` / `value_set(v)` / `value_mut()` |
| `my_field` | `_MyField<T>`   | `my_field()` / `my_field_set(v)` / …       |
| `r#type`   | `_Type<T>`      | `r#type()` / `type_set(v)` / `type_mut()`  |

- Trait name: the field name converted to UpperCamelCase with a `_` prefix.
- The same field name maps to the same trait everywhere, regardless of the field type — that is
  the whole point of the duck typing.
- The setter takes the value by value and returns `()`.

### Notes

- `fields!` derives trait and method names from the field name (see "Naming conventions");
  visibility defaults to `pub(crate)`, and any qualifier written before the name (`pub`,
  `pub(crate)`, `pub(super)`, …) is applied to its trait.
- `#[props]` on a struct generates `impl crate::_fields::_Value<i32> for Player` for **every**
  field; mark the exceptions with `#[_prop]`. Override the module with
  `#[props(path = crate::my_fields)]`.
- `#[props]` on a trait appends one supertrait bound per `name: Type` pair, so default methods
  can use the accessors without any imports.
- A field that is not ignored with `#[_prop]` must have its name declared in `fields!`, otherwise
  its trait fails to resolve at the impl site — the declaration list is the single source of truth.

### Project structure

```
duck-trait
├── crates
│   ├── duck-trait    # Publishable proc-macro crate
│   └── verify        # Verification project (fixtures + tests)
├── editors/code      # VS Code extension (quick fixes + palette commands)
├── readme.md         # This file — the field-based api
└── readme-duck.md    # The marker api (`#[duck]` / `ducks!` / `#[duck_mod]`)
```

### Running verification

```sh
cargo test                        # Unit tests + doctests
cargo clippy --all-targets        # Static checks
```

Requires Rust 1.85+ (edition 2024).

### License

Licensed under either of [MIT](https://github.com/smooth-cat/duck-trait/blob/main/LICENSE) or
[Apache-2.0](https://github.com/smooth-cat/duck-trait/blob/main/LICENSE-APACHE) at your option.

### The marker api — `#[duck]` / `ducks!`

The marker api generates the traits in the scope where the struct is declared: mark fields with
`#[duck]` / `#[_duck]` (with visibility and custom-trait options) inside `ducks! { .. }` /
`#[duck_mod]` scopes. The full documentation lives in
[readme-duck.md](https://github.com/smooth-cat/duck-trait/blob/main/readme-duck.md).

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

⭐️ 约定在 `src/_fields.rs` 中统一声明访问器：

```rust
use duck_trait::{fields, props};

// 1. src/_fields.rs 声明访问器 trait
use duck_trait::fields;
fields! {
  value,    // pub(crate) trait _Value<T> ...
  pub name, // pub trait _Name<T> ...
}

// 2. 通过 name: type 继承访问器
// 不需要引入，宏会自动帮忙引入
// value: i32 对应 _Value<i32>
#[props(value: i32)]
trait Opr {
  fn double(&self) -> i32 {
    // 你获得了以下 3 个方法
    
    // 获取 &mut i32
    self.value_mut();
    
    // 设置 value
    self.value_set(99);
    
    // 获取 &i32
    *self.value() * 2
  }
}

// 3. 所有字段默认实现声明的 trait，无需标记；#[_prop] 可忽略字段
#[props]
struct A { 
  value: i32,
  // 标记这个字段不实现 访问器
  #[_prop] 
  ignored_prop: bool,
}
#[props]
struct B { value: i32 }

impl Opr for A {}
impl Opr for B {}

// 将 fields 用于泛型约束
use crate::_fields::_Value;
fn double<T: _Value<i32>>(v: T) -> i32 {
	*v.value() * 2
}
```

**限制**：被 `#[cfg]` 整体移除的字段会导致生成的 impl 编译失败；避免在同一 crate
中对同名字段混用标记 api（[readme-duck.md](https://github.com/smooth-cat/duck-trait/blob/main/readme-duck.md)）与字段声明 api（方法解析会歧义）。

### LLM

在项目的 `AGENTS.md` 加以下提示词

````markdown
### 使用 duck-trait 简化访问器代码
1. src/_fields.rs 只能声明访问器 trait
```rust
use duck_trait::fields;
fields! {
  value,    // pub(crate) trait _Value<T> ...
  pub name, // pub trait _Name<T> ...
}
```
2. 通过 name: type 继承访问器
```rust
#[props(value: i32)]
trait Foo {
  fn access(&self) {
    // 你获得了以下 3 个方法
    // 获取 &i32
    *self.value();
    // 获取 &mut i32
    self.value_mut();
    // 设置 value
    self.value_set(99);
  }
}
```
3. 所有字段默认实现 trait，无需任何标记（#[_prop] 可忽略字段）
```rust
#[props]
struct Player { value: i32 }
impl Foo for Player {}
```
4. 将 fields 用于泛型约束
```rust
use crate::_fields::_Value;
fn double<T: _Value<i32>>(v: T) -> i32 {
  *v.value() * 2
}
```
````

### VS Code 插件

安装 `duck-trait` 插件帮你保持 `_fields.rs` 同步：

针对 unresolved trait 错误的 quick fix，

以及 `Cmd+Shift+P` 命令——补充当前文件/整个 crate 缺失的字段、创建声明文件、

写入 `mod` 声明，monorepo 与非 `src` 布局同样支持。

### 命名约定

| 字段       | 生成的 trait  | 方法                                        |
| ---------- | ------------- | ------------------------------------------- |
| `value`    | `_Value<T>`   | `value()` / `value_set(v)` / `value_mut()`  |
| `my_field` | `_MyField<T>` | `my_field()` / `my_field_set(v)` / …        |
| `r#type`   | `_Type<T>`    | `r#type()` / `type_set(v)` / `type_mut()`   |

- trait 名：字段名转大驼峰并加 `_` 前缀。
- 相同字段名映射到同一个 trait，与字段类型无关——这正是鸭子类型的意义。
- setter 接收值并返回 `()`。

### 注意事项

- `fields!` 按字段名推导 trait 与方法名（见「命名约定」）；可见性默认 `pub(crate)`，在名字前
  写任意限定符（`pub`、`pub(crate)`、`pub(super)`、…）会应用到对应的 trait。
- `#[props]` 用于 struct 时为**每个**字段生成 `impl crate::_fields::_Value<i32> for Player`；
  不需要生成访问器的字段用 `#[_prop]` 忽略。用 `#[props(path = crate::my_fields)]`
  可覆盖声明所在的模块。
- `#[props]` 用于 trait 时为每个 `name: Type` 对追加一个 supertrait bound，默认方法无需任何
  import 即可使用访问器。
- 未被 `#[_prop]` 忽略的字段必须在 `fields!` 中声明，否则 impl 处报 unresolved trait——声明列表是唯一的事实来源。

### 项目结构

```
duck-trait
├── crates
│   ├── duck-trait    # 可发布的 proc-macro crate
│   └── verify        # 验证项目（fixture + 测试）
├── editors/code      # VS Code 插件（quick fix + 命令面板指令）
├── readme.md         # 本文件 —— 基于字段声明的 api
└── readme-duck.md    # 标记 api（`#[duck]` / `ducks!` / `#[duck_mod]`）
```

### 运行验证

```sh
cargo test                        # 单元测试 + doctest
cargo clippy --all-targets        # 静态检查
```

需要 Rust 1.85+（edition 2024）。

### 许可证

采用 [MIT](https://github.com/smooth-cat/duck-trait/blob/main/LICENSE) 或
[Apache-2.0](https://github.com/smooth-cat/duck-trait/blob/main/LICENSE-APACHE) 双许可，任选其一。

### 标记 api —— `#[duck]` / `ducks!`

标记 api 会在 struct 所在作用域生成 trait：在 `ducks! { .. }` / `#[duck_mod]` 作用域内用
`#[duck]` / `#[_duck]` 标记字段（支持可见性与自定义 trait 选项）。完整文档见
[readme-duck.md](https://github.com/smooth-cat/duck-trait/blob/main/readme-duck.md)。
