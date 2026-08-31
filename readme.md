# duck-trait

[中文](#中文) | [English](#english)

## English

### Eliminates the following repetitive get / set code

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

/*-------- Generates the traits and impls; the inner code still lives at file top-level scope --------*/
ducks! { 
  pub struct A {
    // Marks the field to generate accessors for. `duck` is only a marker attribute; no import needed
    #[duck] 
    value: String,
  }

  // Convention: automatically generates the "_Xxx<T>" trait for the field `xxx`
  trait Opr: _Value<String> {
    fn print_val(&self) {
      println!("{}", self.value());         // Read &xxx via xxx(), this case is &value
    }
    
    fn set_good(&mut self) {
      self.value_set(String::from("good")); // Set xxx via xxx_set(_)
    }
    
    fn get_mut_val(&mut self) -> &str {
      self.value_mut()                      // Get &mut xxx via xxx_mut()
    }
  }

  impl Opr for A {}
}
```

### Before / after expansion of `ducks! { .. }`

```rust
/*----------------- Before expansion -----------------*/
ducks! { 
  pub struct A {
    #[duck] // Marks the field to generate accessors for
    value: String,
  }
}
/*----------------- After expansion -----------------*/
trait _Value<T> {
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

### Custom accessors

Inside a `#[duck_mod]` / `ducks!` scope, define `trait MyGetSet: _Xxx<some_type>` to create a custom accessor.

Then use `#[duck(MyGetSet)]` or `#[duck(MyGetSet<_>)]` (for custom traits with generics) to generate the code.

(PS: in `#[duck(MyGetSet<_>)]`, `_` is the placeholder for the type of the decorated `value` field.)

```rust
use duck_trait::ducks;
/*----------------- Before expansion -----------------*/
ducks! {
  pub struct B {
    #[duck(MyValue<_>)] // Additionally generates impl MyValue<String> for B
    value: String,
  }

  // The custom trait is bound to the field type via _Value<some_type>
  trait MyValue<V>: _Value<V> {
    // In Rust you cannot declare a function with the same name as the supertrait (_Value),
    // so just pick a nice name yourself
    fn my_get(&self) -> &V {
      // Put your extra logic here
      self.value()
    }
  }
  
  let b = B { value: String::from("good") };
  b.my_get();
  b.value(); // Still works
}
/*----------------- After expansion -----------------*/
pub struct B {
  value: String,
}

trait MyValue<V>: _Value<V> {
  fn my_get(&self) -> &V {
    self.value()
  }
}

trait _Value<T> {
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

// All MyValue methods have default implementations, so the generated impl is empty
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

  // A single generic function accepts both structs — this is exactly the point of "duck typing"
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

- Trait name: the field name converted to UpperCamelCase with a `_` prefix (the `_` prefix keeps it
  module-private by default).
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
- Recursively processes nested inline modules; each scope generates its own set of traits.
- Detects `_Xxx` naming conflicts before generation and emits a clear compile error on conflict.

**Limitations**
- The auto impl of `#[duck(MyTrait(..))]` requires every method of the custom trait to have a default
  implementation.
- Cannot scan the contents of `mod foo;` file modules (rustc does not pass file contents to macros).
  Use `ducks! { .. }` inside the file or switch to an inline module.
- The generated traits are private items of the containing module; adjust visibility yourself if you
  need to use them across modules.
- If a field name clashes with an existing method (e.g. `clone`), call sites may hit method resolution
  ambiguity — an inherent behavior of Rust trait methods.

### Project structure

```
duck-trait
├── crates
│   ├── duck-trait    # Publishable proc-macro crate (duck_mod / ducks / duck)
│   └── verify        # Verification project (fixtures + tests)
└── readme.md
```

### Running verification

```sh
cargo test                        # Unit tests + doctests
cargo clippy --all-targets        # Static checks
```

Requires Rust 1.85+ (edition 2024).

### Attribute form (not recommended❌) `#[duck_mod]`

`#[duck_mod]` is the equivalent attribute-macro form of `ducks!`, applied to an inline module; the
generated private traits stay inside the module namespace.
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

  // Convention: the field `value` generates the "_Value" accessor trait (generic over the field type T)
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

  // A single generic function accepts both structs — this is exactly the point of "duck typing"
  pub fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }
}
```

- `#[duck_mod]` can only be applied to inline modules (`mod name { .. }`); it cannot scan file modules
  (`mod name;`, see "Supported features and limitations").

[中文](#中文) | [English](#english)

# duck-trait

## 中文

### 用于处理如下重复的 get / set 代码

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

/*----------------- 负责生成 trait 与 impl，内部代码依然属于文件顶级作用域 -----------------*/
ducks! { 
  pub struct A {
    #[duck] // 标记要生成访问器的字段。duck 仅作为标记，不需要被引入
    value: String,
  }

  // 约定：自动为字段 xxx 生成 “_Xxx<T>” trait
  trait Opr: _Value<String> {
    fn print_val(&self) {
      println!("{}", self.value());         // 通过 xxx() 获取 &xxx 此处为 &value
    }
    
    fn set_good(&mut self) {
      self.value_set(String::from("good")); // 通过 xxx_set(_) 设置值
    }
    
    fn get_mut_val(&mut self) -> &str {
      self.value_mut()                      // 通过 xxx_mut() 获取 &mut xxx
    }
  }

  impl Opr for A {}
}
```

### `ducks! { .. }` 展开前后对比

```rust
/*----------------- 展开前 -----------------*/
ducks! { 
  pub struct A {
    #[duck] // 标记要生成访问器的字段
    value: String,
  }
}
/*----------------- 展开后 -----------------*/
trait _Value<T> {
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
    #[duck(MyValue<_>)] // 额外生成 impl MyValue<String> for B
    value: String,
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
}
/*----------------- 展开后 -----------------*/
pub struct B {
  value: String,
}

trait MyValue<V>: _Value<V> {
  fn my_get(&self) -> &V {
    self.value()
  }
}

trait _Value<T> {
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

// MyValue 的所有方法均有默认实现，因此自动生成的 impl 为空
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

  // 一个泛型函数同时接受两个 struct —— 这正是“鸭子类型”的意义
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

- trait 名：字段名转大驼峰并加 `_` 前缀（`_` 前缀使其默认保持模块私有）。
- 同一作用域内，相同字段名的所有 struct 共享同一个 trait；不同作用域各自独立生成。
- setter 接收值并返回 `()`。

### 支持与限制

**支持**

- 泛型 struct（含 where 子句）：`struct Wrapper<T: Clone> { #[duck] inner: T }` 会生成
  `impl<T: Clone> _Inner<T> for Wrapper<T>`。
- `#[duck(MyTrait(..))]`：在访问器之外额外为该 struct 自动实现自定义 trait，`_` 占位符等于
  字段类型（`#[duck_mod]` 与 `ducks!` 均支持）。
- 递归处理嵌套的内联模块，每个作用域生成自己的一组 trait。
- 生成前检测 `_Xxx` 命名冲突，冲突时给出明确的编译错误。

**限制**
- `#[duck(MyTrait(..))]` 的自动 impl 要求自定义 trait 所有方法均有默认实现。
- 无法扫描 `mod foo;` 文件模块的内容（rustc 不会把文件内容传给宏）。请在文件内使用
  `ducks! { .. }` 或改用内联模块。
- 生成的 trait 是所在模块的私有项；如需跨模块使用，需要自行调整可见性。
- 字段名若与既有方法重名（如 `clone`），调用处可能出现方法解析歧义，这是 Rust trait 方法的固有行为。

### 项目结构

```
duck-trait
├── crates
│   ├── duck-trait    # 可发布的 proc-macro crate（duck_mod / ducks / duck）
│   └── verify        # 验证项目（fixture + 测试）
└── readme.md
```

### 运行验证

```sh
cargo test                        # 单元测试 + doctest
cargo clippy --all-targets        # 静态检查
```

需要 Rust 1.85+（edition 2024）。

### 属性形式（不推荐⭕️） `#[duck_mod]`

`#[duck_mod]` 是 `ducks!` 的等价属性宏形式，作用于内联模块，生成的私有 trait 保持在模块命名空间内。
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

  // 一个泛型函数同时接受两个 struct —— 这正是“鸭子类型”的意义
  pub fn debug_value<T: std::fmt::Debug>(x: &impl _Value<T>) -> String {
    format!("{:?}", x.value())
  }
}
```

- `#[duck_mod]` 只能作用于内联模块（`mod name { .. }`），无法扫描文件模块
  （`mod name;`，见「支持与限制」）。
