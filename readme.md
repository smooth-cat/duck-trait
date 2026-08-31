# duck-trait

处理 trait 中重复的 get / set：给 struct 字段标记 `#[duck]`，由 `#[ducky]` / `ducks! { .. }`
在作用域内生成访问器 trait 与实现。trait 里再也不用重复声明 getter / setter。

## 基础用法

```rust
use duck_trait::ducky;

#[ducky] // 负责生成 trait 与 impl
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

`#[ducky]` 展开后大致如下（`cargo expand` 可查看）：

```rust
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

## 出现重复字段名时会复用同一个 trait

```rust
use duck_trait::ducky;

#[ducky]
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

## 两种用法

```rust
use duck_trait::{ducky, ducks};

#[ducky]                 // 形式一：属性宏，作用于内联模块
mod model {
  pub struct A {
    #[duck]
    value: String,
  }
}

ducks! {                 // 形式二：函数式宏，包裹一组条目，不引入额外模块
  pub struct B {
    #[duck]
    value: String,
  }
}
```

`#[duck]` 标记由 `#[ducky]` / `ducks!` 在展开时消费，因此**无需单独导入**；
crate 同时导出了占位宏 `duck`（在 `#[duck]` 被误用于字段以外的地方时给出指引信息）。

## 自定义 访问器

在 `#[ducky]` / `ducks!` 作用域内可以直接定义自己的 trait（以生成的 `_Xxx<T>` 为
supertrait），并在 `#[duck]` 标记上写 `#[duck(MyTrait(..))]`，宏会在生成访问器 impl 之后
额外为该 struct 生成 `impl MyTrait(..)`：

```rust
ducks! {
  pub struct B {
    #[duck(MyValue<_>)] // 额外生成 impl MyValue<String> for B
    value: String,
  }

  // 自定义 trait：V 通过 _Value<V> supertrait 绑定到字段类型
  trait MyValue<V>: _Value<V> {
    fn my_get(&self) -> &V {
      self.value()
    }
  }
}
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
  字段类型（`#[ducky]` 与 `ducks!` 均支持）。
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
│   ├── duck-trait    # 可发布的 proc-macro crate（ducky / ducks / duck）
│   └── verify        # 验证项目（fixture + 测试）
└── readme.md
```

## 运行验证

```sh
cargo test                        # 单元测试 + doctest
cargo clippy --all-targets        # 静态检查
```

需要 Rust 1.85+（edition 2024）。
