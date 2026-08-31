# duck-trait

## 用于处理如下重复的 get / set 代码

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

## 基础用法

```rust
use duck_trait::ducks;

/*----------------- 负责生成 trait 与 impl，内部代码依然属于文件顶级作用域 -----------------*/
ducks! { 
  pub struct A {
    #[duck] // 标记要生成访问器的字段。duck 仅作为标记，不需要被引入
    value: String,
  }

  // 约定：自动为字段 value 生成 “_Value<T>” trait
  trait Opr: _Value<String> {
    fn print_val(&self) {
      println!("{}", self.value());         // 通过 value() 获取 &value
    }
    
    fn set_good(&mut self) {
      self.value_set(String::from("good")); // 通过 value_set(xxx) 设置值
    }
    
    fn get_mut_val(&mut self) -> &str {
      self.value_mut()                      // 通过 value_mut() 获取 &mut value
    }
  }

  impl Opr for A {}
}
```

## `ducks! { .. }` 展开前后对比

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

`#[duck]` 标记由 `ducks!` / `#[duck_mod]` 在展开时消费，因此**无需单独导入**

## 自定义 访问器

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

## 出现重复字段名时会复用同一个 trait

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

## 命名约定

| 字段       | 生成的 trait  | 方法                                        |
| ---------- | ------------- | ------------------------------------------- |
| `value`    | `_Value<T>`   | `value()` / `value_set(v)` / `value_mut()`  |
| `my_field` | `_MyField<T>` | `my_field()` / `my_field_set(v)` / …        |
| `r#type`   | `_Type<T>`    | `r#type()` / `type_set(v)` / `type_mut()`   |

- trait 名：字段名转大驼峰并加 `_` 前缀（`_` 前缀使其默认保持模块私有）。
- 同一作用域内，相同字段名的所有 struct 共享同一个 trait；不同作用域各自独立生成。
- setter 接收值并返回 `()`。

## 支持与限制

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

## 项目结构

```
duck-trait
├── crates
│   ├── duck-trait    # 可发布的 proc-macro crate（duck_mod / ducks / duck）
│   └── verify        # 验证项目（fixture + 测试）
└── readme.md
```

## 运行验证

```sh
cargo test                        # 单元测试 + doctest
cargo clippy --all-targets        # 静态检查
```

需要 Rust 1.85+（edition 2024）。

## 属性形式（不推荐⭕️） `#[duck_mod]`

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

