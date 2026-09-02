# 基于 shadow-trait 的生成方案（定稿）

## 背景与痛点

旧方案以 struct 为中心：必须先有标记了 `#[duck]` 字段的 struct，才能得到 `_Xxx` trait，然后才能
在其上定义业务 trait。对于"先抽象、后落地"的场景，trait 定义被迫依赖 struct 先存在，不合理。

## 定案的设计决策

1. **所见即所得**：`#[props(a: String, b: i32)]` 直接生成固定类型的访问器。需要泛型时，prop 类型
   引用原 trait 的同名泛型（`#[props(a: T)] trait Show<T>` → `_Show<T>`）；生命周期同理。
2. **struct 侧关键字沿用 `#[duck(...)]`**：字段级标记（`#[duck]` / `#[_duck]`，位于字段上）与
   struct 级标记（`#[duck(_Show{..})]`，位于 struct 上）按放置位置区分，不新增关键字。
3. **shadow trait 可见性跟随原 trait**：`pub trait Show` → `pub trait _Show`。
4. **匹配机制：struct 侧显式列出 props**（`#[duck(_Show{name, score})]`）。

   原因：proc-macro 展开期没有名字解析能力，独立使用时 struct 侧宏拿不到 `_Show` 的 props
   （名字、类型、泛型映射），而生成 `impl` 又必须要这些信息。方案 A（要求 `ducks!` 作用域包裹，
   作用域宏同时看到两边）、方案 C（blanket impl 自动实现）均被否决；显式列表让两个宏完全独立、
   不要求作用域包裹，且与旧 API 互不干扰。

   曾考虑过"宏生成宏桥接"（`#[props]` 生成 `macro_rules!`，struct 侧转发调用），但 macro_rules
   无法在展开期比较标识符（prop 名 ↔ 字段名匹配做不到），且 `#[macro_export]` / 文本顺序作用域
   带来跨模块可用性问题，放弃。
5. **`impl Show for A` 保持手写**。

## 最终 API

### trait 侧 `#[props]`

```rust
#[props(name: String, score: i32)]
pub trait Show {
    fn show(&self) {
        println!("{}: {}", self.name(), self.score());
    }
}

// 展开后
pub trait _Show {
    fn name(&self) -> &String;
    fn name_set(&mut self, v: String);
    fn name_mut(&mut self) -> &mut String;
    fn score(&self) -> &i32;
    fn score_set(&mut self, v: i32);
    fn score_mut(&mut self) -> &mut i32;
}
pub trait Show: _Show {
    fn show(&self) { /* 原样保留 */ }
}
```

- shadow trait 的可见性、generics、where 子句原样复制自原 trait；原 trait 自动追加 supertrait
  `: _Show<..>`（已有 supertrait 则合并），否则默认方法里的 `self.name()` 无法编译。
- 泛型：prop 类型引用 trait 同名泛型时，shadow trait 携带该泛型；生命周期同理
  （`#[props(text: &'a str)]` + `trait Show<'a>` → `_Show<'a>`）。
- 方法名冲突检测：每个 prop 生成 `xxx()` / `xxx_set()` / `xxx_mut()` 三件套，全部名字全局去重，
  冲突报错（如 prop `a` 与 `a_set`）。
- raw ident 支持：prop `r#type` → `r#type()` / `type_set` / `type_mut`；但 raw ident 的 **trait 名**
  无法派生 shadow trait 名（`_r#type` 不是合法标识符），报错。
- 不支持 auto trait（自动追加 supertrait 与 auto trait 语义冲突）。
- `#[props]` 独立可用，不要求 `ducks!` 作用域包裹；作用域内也可用（作用域宏不处理它，属性随后
  独立展开）。

### struct 侧 `#[duck(_Show{props})]`

```rust
#[duck(_Show{name, score})]
struct A {
    name: String,
    score: i32,
}

// 展开后：原 struct 原样保留 +
impl _Show for A {
    fn name(&self) -> &String { &self.name }
    fn name_set(&mut self, v: String) { self.name = v; }
    fn name_mut(&mut self) -> &mut String { &mut self.name }
    // score 同理
}
```

- 语法：`#[duck(_A{a}, _B{b, c})]` 逗号分隔多项；花括号形式贴近 struct 字面量的视觉语义。
- 泛型 shadow trait 必须显式写全参数：`#[duck(_Show<String>{name})]`；可引用 struct 自身泛型
  `#[duck(_Show<T>{a})]`（泛型 struct 自动生成 `impl<T> _Show<T> for W<T>`）。`_` 占位符不支持
  （无法推断），由宏报错。
- prop 按名字匹配字段：
  - 找不到同名字段 → 宏报清晰错误（指向该 prop 名）；
  - 方法签名用**字段实际类型**生成 → 字段类型与 prop 类型不一致时，由 rustc 在生成的 impl 处
    报错（E0053 / E0308），宏无法校验（不知道真实 prop 类型）；
  - 完全未列出的 props → trait 未实现（rustc E0046），宏无法检测（不知道 props 总数）。
- 空花括号 `_Show{}`、同一 prop 列两次 → 宏报错。
- 在 `ducks!{}` / `#[duck_mod]` 作用域内同样可用（作用域扫描器只处理字段级标记，struct 级属性
  原样留在输出中随后独立展开），与旧字段标记互不干扰。

## dyn 兼容性

泛型均为 trait 级参数（而非方法级），实例化后（如 `dyn Show<String>`）方法签名具体化，因此
dyn-compatible；无对象安全问题。

## 已验证的边界

`crates/verify/src/lib.rs` 覆盖：基本用法、泛型 prop（`_Has<String>` / `_Has<T>`）、泛型 struct、
生命周期（`&'a str`）、raw ident、一次实现多个 shadow trait、可见性跟随、新旧方案在同一
`ducks!` 作用域内共存。错误场景（缺失字段、方法名冲突等）由 lib.rs 的 compile_fail doc-test 覆盖。

注意：新旧方案生成的访问器 trait 命名规则不同（旧方案按字段名 `_Name`，新方案按 trait 名
`_Show`），同一作用域内若 prop trait 恰好与字段名撞名（如字段 `show` 生成 `_Show`，同时
`#[props]` 的 trait 也叫 `Show`），会产生 E0428 重复定义，属于预期行为。
