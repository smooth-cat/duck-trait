# 补充 `#[ducky]` 宏（定稿）

在 `#[ducky]` 修饰的内联模块中，`#[props]` trait 会被展开并注册，struct 级 `#[duck(_Show)]`
条目可以省略花括号里的 props 列表——由注册的 trait 与 struct 自身字段自动推导。

```rust
#[ducky]
mod duckied {
  #[props(name: String, score: i32)]
  pub trait Show {
    fn show(&self) {
      println!("{}: {}", self.name(), self.score());
    }
  }

  // props 从 Show 推导；多出的字段（nickname）不受影响
  #[duck(_Show)]
  struct Player {
    name: String,
    score: i32,
    nickname: String,
  }

  impl Show for Player {}
}
```

## 语法矩阵（`#[duck(...)]` 条目在 `#[ducky]` 内）

| 写法 | props 来源 | 泛型参数来源 |
| --- | --- | --- |
| `#[duck(_Show)]` | 注册的 `#[props]` trait 推导 | 自动填充；trait 无泛型则无参数 |
| `#[duck(_Has<String>)]` | 注册的 trait 推导 | 显式写出，参数个数与 trait 泛型数校验 |
| `#[duck(_Show{name, score})]` | 显式列出 | 显式写出（与独立使用语义一致，不校验） |

## 规则

- **仅接受 shadow 名**（`_Show`），不接受主 trait 名。
- **泛型自动填充（规则一）**：仅当 prop 的声明类型恰好是裸泛型参数（`inner: T`）时推导，填充值
  为该 prop 对应字段的实际类型。无法推导的情况（报错并提示写显式形式）：
  - 复合类型：`#[props(items: Vec<T>)]`（需要做类型合一，不支持）；
  - 泛型参数未被任何 prop 裸引用（如 `trait Two<T, U>` 只有 `a: T`）；
  - 多个 prop 裸引用同一泛型但字段类型不一致。
- **作用域链**：嵌套模块可以看到外层 `#[ducky]` 作用域注册的 trait；生成的 impl 使用带 `super::`
  前缀的显式路径——宏生成的标识符跨模块解析必须走显式路径（实现中发现裸标识符在嵌套模块内
  无法解析到外层宏展开的 trait，即使 context 相同）。
- **无需导入**：`#[props]` / `#[duck]` 标记被 `#[ducky]` 消费，模块内不需要
  `use duck_trait::...`（导入了反而会得到 unused import 警告）。
- **旧字段标记不处理**：`#[duck]` / `#[_duck]` 字段标记仍属于 `#[duck_mod]` / `ducks!` 流程，
  `#[ducky]` 内不扫描（写成字段标记会得到独立 stub 的报错提示）。
- **与独立 API 的关系**：`#[props]`、`#[duck(_Show{..})]` 独立使用的行为完全不变；
  `#[duck_mod]` 保持不动；`#[ducky]` 是新的模块级入口，仅服务 props 流程。
- 仅接受内联模块（`mod name { .. }`）；非 `#[duck(...)]` / `#[props(...)]` 的内容原样保留；
  同一 struct 上的其他属性（derive 等）不受影响。

## 实现要点

- 复用 props 流程的解析与生成：`PropsAttr`（props 解析与三件套方法名去重）、
  `build_shadow_items`（shadow trait + supertrait 改写）、`build_duck_impls`（按字段名匹配
  生成访问器 impl）。
- `process_ducky_scope` 两趟处理：pass 1 展开+注册 `#[props]` trait（位置无关，先于 pass 2）；
  pass 2 处理 struct 级 `#[duck(...)]` 并递归进入嵌套内联模块。嵌套模块自带的 `#[ducky]`
  标记会被剥除（已由外层处理）。

## 验证

`crates/verify/src/lib.rs` 的 `ducky_flow` 覆盖：基本免花括号写法、显式泛型参数（个数校验）、
泛型 struct 自动推导、混合条目（`#[duck(_HasA{a}, _Has)]`）、嵌套模块引用外层 trait；
错误场景（trait 不在作用域、无法推导、泛型个数不符、独立使用免花括号）由 lib.rs 的
compile_fail doc-test 覆盖。
