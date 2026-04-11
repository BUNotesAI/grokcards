# super-tauri

## 项目管理体系

四个正交关注点，覆盖从规划到记录的完整生命周期。

### 术语表

| 术语 | 定义 | 载体 |
|------|------|------|
| **设计 (Design)** | 架构决策 + 模块规划 + 动机约束。回答「建什么、为什么」 | `docs/design.md` |
| **进度 (Progress)** | Task 的执行状态，按功能域拆文件。回答「做到哪、下一步」 | `docs/progress/{功能域}.md` |
| **质量 (Quality)** | 确保正确性的机制和关卡。回答「怎么保证不出错」 | CLAUDE.md 内嵌 |
| **日志 (Records)** | 时间线记录 + 经验积累。回答「发生了什么、学到了什么」 | devlog / changelog / LESSONS.md |
| **功能域** | 项目中长期存在的功能边界，对应独立可演进的子系统。Task 在功能域内产生和完成 | `docs/progress/{name}.md` |
| **Task** | 功能域内一个可完成的工作项 | progress 文件中的一行 checkbox |
| **Handoff** | Task 内部的精准执行快照，让下一个会话能精准接续 | `docs/handoff/{area}.md` |

### 进度 (Progress)

- 位置: `docs/progress/{功能域}.md`，一个功能域一个文件
- 当前功能域: `backend`, `frontend`, `ipc`, `infra`
- 每个文件格式: `## Active` / `## Next` / `## Done`（Done 按日期分组）
- 跨功能域 Active 总数不超过 3 个
- Task 归属于**驱动它的功能域**，不是它触及的每个代码模块
- 每个 session 只写自己关注的功能域文件（多 session 并发安全）

### 日志 (Records)

| 载体 | 位置 | 写入时机 |
|------|------|---------|
| Devlog | `docs/devlog/{YYYY-MM-DD}.md` | Session 结束前（追加） |
| Changelog | slipbox `logs/changelog/{date}.md` | Task 完成时（追加） |
| LESSONS.md | 各模块目录 | Bug 修复后 |

### Handoff 协议

会话结束时 task 未完成 → `/harness-save-next-context` → `docs/handoff/{area}.md`
新会话开始 → `/harness-resume-context` → 读取 + 验证 + 汇报，等用户确认后继续
Devlog「下次从这里开始」简化为 pointer → `docs/handoff/{area}.md`

---

## 质量 (Quality)

### 质量机制总览

| 层 | 机制 | 触发时机 |
|---|------|---------|
| L0 | Operation Contract — pub fn 契约 | 新增/修改 pub fn |
| L0 | Deep Module 规则 — 窄接口 + 深实现 | 新增模块/重构 |
| L0 | 数据写回规则 — SQLite 是 source of truth | 修改数据层 |
| L0 | 功能域完成 Code Review — 强制关卡 | 功能域 Active tasks 全部完成 |
| L0 | TS/Rust 职责边界硬约束 — TS 只负责 UI，Rust 独占业务 | 任何 commit |
| L0 | IPC 类型安全 — tauri-specta 编译期保障 | 新增/修改 command |
| L0 | 测试真实代码路径 — 禁止 test theater | 新增测试 / Code Review |
| L0 | TDD — Red-Green-Refactor 开发循环 | 新功能 / Bug 修复 / 迁移 / 重构 |
| L0 | Trait-First — 面向接口编程 | 新增模块 / 跨模块依赖 / 可测试性设计 |
| L1 | 框架参考资料 — 不猜 API，查 Tauri skills + 官方文档 | 使用 Tauri / React API 时 |
| L1 | Rust Skills — 10 个领域 | 遇到编译错误/设计问题 |
| L2 | LESSONS.md — 模块级踩坑经验 | 修改模块代码前**必须先读** |
| L3 | 方法级 doc comment | 阅读/修改方法时 |
| L4 | MEMORY + Skills — 个人偏好 + 跨项目知识 | 关键词触发 / 按需召回 |
| 测试 | 集成测试 tests/ | cargo test |
| 测试 | 副作用矩阵 | 新增写操作时同步更新 |
| 测试 | TS 组件测试 — Vitest + RTL | 新增/修改 TS 组件 / UI Bug 修复 |

---

### L0: Operation Contract — pub fn 契约

新增或修改有副作用的 pub fn 时，写 doc comment 描述契约：

```rust
/// # 操作名称
///
/// ## 前置条件
/// - 调用前必须满足的条件
///
/// ## 执行效果
/// 1. 按顺序列出所有副作用
///
/// ## 不做的事
/// - 明确列出不会做的事
///
/// ## 幂等性
/// 重复调用的行为（幂等 / 追加 / 报错）
///
/// ## 关联操作
/// - [`other_method`] — 常见配合 / 逆操作
pub fn method_name(&mut self, ...) -> Result<...>
```

不是每个函数都需要完整 Operation Contract。区分：
- **有副作用的 pub fn**（写 DB、修改状态、发事件）→ 完整 Operation Contract
- **纯查询 / 纯计算 pub fn** → L3 方法级 doc comment 即可

---

### L0: Deep Module 规则 — 窄接口 + 深实现

每个业务功能对应一个 Deep Module，放在 `src-tauri/src/modules/{name}/` 下。模块对外暴露**窄接口**（少量 pub 函数），内部实现可以很复杂但对外不可见。

#### 模块目录结构

```
src-tauri/src/
├── lib.rs                    # Tauri Builder + specta 注册（薄壳）
├── app_error.rs              # 顶层统一错误类型
├── modules/
│   ├── mod.rs                # 只 re-export 各模块的 commands
│   ├── todo/                 # 一个 Deep Module
│   │   ├── mod.rs            # 窄接口：pub use commands + models
│   │   ├── commands.rs       # #[tauri::command] 薄壳 (pub — proc macro 要求)
│   │   ├── domain.rs         # 业务纯函数 (pub(super))
│   │   ├── models.rs         # 数据结构 (pub — 跨 IPC 需要)
│   │   ├── errors.rs         # 模块错误 (pub(super)，impl Into<AppError>)
│   │   └── db.rs             # 建表 + migration (pub(super))
│   ├── notes/                # 未来新模块，同结构
│   └── settings/             # 未来新模块，同结构
```

#### 可见性规则

| 文件 | 可见性 | 原因 |
|---|---|---|
| `mod.rs` | `pub mod commands; pub mod models;` | 窄接口 — 暴露 command 模块和跨 IPC 类型。lib.rs 用完整路径 `modules::todo::commands::*` 引用 |
| `commands.rs` | `pub` | `#[tauri::command]` + `#[specta::specta]` proc macro 生成的隐藏符号需要和函数同等可见性，`pub(super)` 会导致 `collect_commands![]` 找不到符号。lib.rs 用完整路径引用 |
| `domain.rs` | `pub(super)` | 内部实现，外部不可见 |
| `models.rs` | `pub` | 需要跨 IPC 传递，derive `specta::Type` |
| `errors.rs` | `pub(super)` | 模块内使用，转换为 AppError 后对外 |
| `db.rs` | `pub(super)` | 建表和 migration，外部不需要知道 |

#### 判断模块边界是否正确

- 能否不读 domain.rs 就理解这个模块提供什么能力？（看 mod.rs 就够）
- 能否修改 domain.rs 内部实现而不破坏其他模块？
- 模块间是否通过 lib.rs 层面协调，而不是互相 import 内部类型？

如果三个问题的答案都是"是"，边界正确。

#### 新模块添加流程

```
1. mkdir src-tauri/src/modules/new_feature/
2. 写 models.rs — derive Serialize + Deserialize + specta::Type
3. 写 errors.rs — thiserror + impl Into<AppError>
4. 写 db.rs — 建表 migration
5. 写 domain.rs — 业务纯函数，接收 &Connection
6. 写 commands.rs — 薄壳 + #[tauri::command] + #[specta::specta]
7. 写 mod.rs — pub use commands + models
8. 在 modules/mod.rs 注册
9. 在 lib.rs 的 collect_commands![] 添加
10. cargo test export_bindings → bindings.ts 自动更新
11. 在 副作用矩阵 中登记写操作
```

---

### L0: 数据写回规则 — SQLite 是 Source of Truth

- **SQLite 是唯一数据真源**，TS 侧不持有业务状态的持久副本
- 所有写入必须通过 Rust domain 函数 → SQLite，不得绕过
- TS 侧的 `useState` 只是 SQLite 数据的**临时展示缓存**，不是真源
- 读操作：TS 调用 typed command → Rust 查 SQLite → 返回
- 写操作：TS 调用 typed command → Rust 写 SQLite → 返回更新后的数据 → TS 刷新展示

**违规标志**：TS 侧用 localStorage / IndexedDB / 全局 store 持久化业务数据。这会导致 SQLite 和 TS 两份数据不一致。

---

### L0: 功能域完成 Code Review — 强制关卡

**触发条件**：功能域的 Active tasks 全部完成时触发，不绑定线性 Phase。

**完整流程**：
```
功能域 Active tasks 全部完成
  │
  ├─ 1. /harness-check-tests    ← agent 语义自查，补测试缺口
  ├─ 2. Code Review              ← 结构化审查（5 项检查）
  ├─ 3. git commit               ← 提交
  │
  ▼
标记 Done
```

**Review 5 项检查**：

1. **测试覆盖** — 每个 domain 纯函数至少 happy path + error path 各一个测试。边界条件、跨实体交互是否覆盖
2. **逻辑正确性** — 特别是从旧代码移植的逻辑，逐行核对
3. **回归风险** — 未来变更可能静默破坏的场景，是否有测试锁住
4. **I/O 正确性** — SQLite 读写完整性、文件操作正确性
5. **IPC 类型安全** — 所有 command 是否有 `#[specta::specta]`，bindings.ts 是否最新，TS 是否从 bindings import

---

### L0: TS/Rust 职责边界硬约束

本项目是 TS + Rust 双栈架构（React 前端 + Tauri Rust 后端），通过 Tauri IPC（`invoke` + Events）通信。遵循 **TS 只负责 UI，Rust 独占业务** 的硬约束。

#### 职责边界

| 层 | 可以做 | 不得做 |
|---|---|---|
| **TS（薄 UI 层）** | 调用 bindings.ts 的 typed commands、渲染返回数据、监听 Tauri events 更新 UI、管理纯 UI 状态（selection / modal 开关 / 拖拽中间态） | 生成业务实体 id / 构造业务数据 blob / 业务规则判定 / 解析 Rust error 做分支 / 构造持久化格式 |
| **Rust（Deep Module）** | id 生成、schema 校验、事务、业务规则、持久化格式、数据完整性、所有写入路径 | — |

#### TS 禁止项（硬红线）

1. ❌ **生成业务实体 id** — 禁止 `crypto.randomUUID()` / `Math.random()` / `Date.now().toString(36)` 等。id 全部由 Rust typed command 返回。唯一例外：纯 UI 状态 key（React list key、modal dom id），必须以 `ui_` 前缀标记，且绝不写入持久存储
2. ❌ **绕过 typed command 直接写数据** — 所有业务写入必须通过 Rust command，禁止 TS 直接操作任何持久化存储
3. ❌ **业务决策** — 校验、去重、冲突判定全部在 Rust。TS 不解析 Rust error message 做控制流分支（error 是约定，不是字符串协议）
4. ❌ **构造持久化格式** — JSON blob、文件路径计算全部在 Rust。TS 拿到的是已经结构化的 typed value

#### Enforcement 机制

tauri-specta 替代了传统三层 enforcement 中的大部分工作：

| 传统方案 | tauri-specta 替代 | 仍需补充 |
|---|---|---|
| ESLint 禁 raw `invoke()` | bindings.ts 提供 typed wrapper，用 raw invoke 反而更麻烦 | Code Review 时检查是否有 raw invoke 绕过 |
| Rust runtime guard 校验参数 | 编译期已校验参数类型，不需要 runtime guard | — |
| 回归测试覆盖类型不匹配 | TS 编译失败就是最好的回归测试 | domain 纯函数仍需 unit test |

tauri-specta 将类型安全从运行时后移到编译期，但**不能替代业务逻辑测试**。domain.rs 中的业务规则仍需 unit test 覆盖。

#### 典型违规模式（识别标志）

| 模式 | 示例 | 问题 |
|---|---|---|
| **TS 本地造 id** | `const id = crypto.randomUUID().slice(0, 8)` | 生成的 id 不满足 Rust 侧 schema，产生脏数据 |
| **TS 持有业务状态** | `localStorage.setItem('todos', JSON.stringify(todos))` | TS 成为数据真源的一部分，与 SQLite 不一致 |
| **TS 解析 Rust error 做决策** | `if (err.message.includes("not found")) { createIt(); }` | 业务决策在 TS，Rust 改 error 文案会静默破坏 TS 逻辑 |
| **TS 构造持久化格式** | `const json = JSON.stringify({ id, title, completed })` 然后传给通用写入接口 | 持久化格式在 TS 生成，Rust 对数据形态失去控制 |
| **绕过 bindings 用 raw invoke** | `invoke('create_todo', { title })` 而非 `commands.createTodo(title)` | 放弃编译期类型保障，参数变更不会被 TS 编译器捕获 |

---

### L0: IPC 类型安全 — tauri-specta 编译期保障

项目使用 **tauri-specta** 保证 TS/Rust IPC 边界的编译期类型安全。Rust 是类型的唯一真源。

**规则：**

1. 每个 `#[tauri::command]` 必须同时加 `#[specta::specta]`
2. 所有跨 IPC 传递的类型必须 derive `specta::Type`
   - struct/enum 加 `#[derive(Serialize, Deserialize, specta::Type)]`
   - 错误类型用 thiserror + `#[derive(Serialize, specta::Type)]`
3. TS 侧只从 `src/bindings.ts` import，禁止 raw `invoke()`
   - `import { commands } from './bindings'` ✅
   - `import { invoke } from '@tauri-apps/api/core'` ❌
4. `bindings.ts` 是生成文件，禁止手动编辑，已加入 `.gitignore`
5. 修改 command 签名后，运行 `cargo test export_bindings` 重新生成

**原理**：如果 Rust 改了参数/返回值，bindings.ts 重新生成后 TS 编译失败。这比 ESLint 规则和运行时 guard 更可靠——绕过这个机制（用 raw invoke）意味着放弃编译期保障，任何类型不匹配都会变成运行时才发现的 bug。

---

### L0: 测试真实代码路径（Anti-Test-Theater）

**原则**：测试必须调用真实的业务代码，不得在测试中复制业务逻辑。

**Test Theater 的定义**：测试文件自己写一遍和生产代码"等价"的逻辑（查询、校验、转换），看起来覆盖了功能，但测的是副本而非真实实现。生产代码改了副本没跟着改时，测试仍通过——覆盖率是假象。

**根因**：几乎所有 Web/App 框架都把业务逻辑锁在框架 wrapper 的闭包里（route handler、server function、command handler）。在 Tauri 中，`#[tauri::command]` handler 就是这种 wrapper——业务逻辑写在 handler 内部无法直接 import 测试，开发者/AI 为了写测试"复制一份"逻辑，这是 test theater 的系统性来源。

**解法**：

```
#[tauri::command] 薄壳   ←  参数解构 + State 注入
  └── domain 纯函数       ←  核心逻辑，接收 &Connection 等 plain 依赖
        └── 测试直接 import ←  零框架开销，用 :memory: SQLite
```

**Tauri 中的正确模式**：

```rust
// modules/todo/commands.rs — 薄壳，每个函数 ≤3 行
#[tauri::command]
#[specta::specta]
fn create_todo(state: State<AppState>, title: String) -> Result<Todo, AppError> {
    let conn = state.db.lock().unwrap();
    domain::create_todo(&conn, &title).map_err(Into::into)
}

// modules/todo/domain.rs — 业务纯函数，可直接 import 测试
pub(super) fn create_todo(conn: &Connection, title: &str) -> Result<Todo, TodoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TodoError::EmptyTitle);
    }
    conn.execute("INSERT INTO todos (title, completed) VALUES (?1, 0)", params![title])?;
    let id = conn.last_insert_rowid();
    get_todo(conn, id)
}

// modules/todo/domain.rs 内的 tests — 测试真实代码路径
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::todo::db::init_db;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_todo() {
        let conn = test_conn();
        let todo = create_todo(&conn, "Buy milk").unwrap();
        assert_eq!(todo.title, "Buy milk");
        assert!(!todo.completed);
    }

    #[test]
    fn test_create_todo_empty_title() {
        let conn = test_conn();
        let err = create_todo(&conn, "  ").unwrap_err();
        assert!(matches!(err, TodoError::EmptyTitle));
    }
}
```

**判断标准**：如果测试文件里出现了和生产代码结构相似的 DB 查询、条件分支、数据转换，大概率是 test theater。

**和 Deep Module 的关系**：抽取不是拆散模块——纯函数仍在同一模块目录内（`domain.rs`），不导出到 `mod.rs` 的公开接口，对外接口不变。模块的"深度"不减反增（框架胶水被剥离后，纯业务逻辑更集中）。

**触发时机**：
- 写新测试时：如果发现需要复制逻辑才能测，先抽取到 domain.rs 再测
- Code Review 时：如果发现测试文件中有和生产代码结构相似的逻辑，标记为 test theater

---

### L0: TDD — Red-Green-Refactor 开发循环

**原则**：所有代码变更（新功能、Bug 修复、迁移、重构）遵循 Red-Green-Refactor 循环。先写失败的测试定义期望行为，再写最少的代码让测试通过，最后重构保持测试绿色。

**为什么测试先行**：

测试先行的核心价值不是"测试覆盖率"，而是**让失败的测试成为进度的客观度量**。测试红色 = 确切地知道缺什么。测试绿色 = 确切地知道做完了。"写完代码再补测试"会让测试去适应实现的 bug——你以为测通了，其实是测试和 bug 一起错。

**各场景的 TDD 流程**：

| 场景 | Red（写什么测试） | Green（写什么代码） | Refactor |
|------|------------------|-------------------|----------|
| **新功能** | 期望行为的 happy path + error path | 最少的实现让测试通过 | 提取重复、命名优化 |
| **Bug 修复** | 精确复现 bug 的测试用例 | 修复 bug | 测试永久保留为回归防护 |
| **迁移** | 旧系统已知行为写成测试 | 新实现让测试逐个通过 | 清理迁移临时代码 |
| **重构** | 现有测试必须全绿（不新增/不修改测试） | 改结构 | 确认仍全绿 |

**Bug 修复的 TDD 特殊价值**：

Bug 修复是 TDD 收益最高的场景。先写复现测试有三个好处：
1. 确认你真正理解了 bug（测试能复现 = 理解正确）
2. 修复后测试变绿 = 修复确实有效（不是你以为有效）
3. 测试永久留下 = 这个 bug 不会再回来（回归防护）

```rust
#[test]
fn test_empty_title_should_error_not_insert() {
    // 复现：旧代码对空白 title 没校验，直接 INSERT 产生脏数据
    let conn = test_conn();
    let result = create_todo(&conn, "   ");
    assert!(result.is_err()); // Red → 旧代码这里会 Ok（bug！）

    // 确认没有脏数据写入
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM todos", [], |r| r.get(0)
    ).unwrap();
    assert_eq!(count, 0);
}
```

**执行流程（人工确认关卡）**：

Red 和 Green 的转换必须经过用户确认。Agent 不得跳过确认自行继续。

```
1. 写测试
2. 跑测试 → 展示结果给用户
3. 🔴 Red 关卡 — 等待用户确认
   - 用户自己跑测试，确认确实是 Red 且失败原因正确
   - 用户确认 → 继续
   - 用户否定 → 停下来讨论（测试写错了？断言不对？）
4. 写实现
5. 跑测试 → 展示结果给用户
6. 🟢 Green 关卡 — 等待用户确认
   - 用户自己跑测试，确认确实是 Green
   - 用户确认 → 进入 Refactor
   - 用户否定 → 停下来讨论（哪个测试还红？为什么？）
7. Refactor（保持绿色）
```

**为什么需要人工确认**：
- Red 确认：确保测试是因为正确的原因失败（不是语法错误、import 错误、断言写反）
- Green 确认：确保实现真正让测试通过（不是测试被意外跳过、mock 吞了错误）
- 用户保持对代码状态的掌控，不会出现 agent 自以为通过但实际没通过的情况

**和 Anti-Test-Theater 的关系**：

TDD 回答"什么时候写测试"（代码之前），Anti-Test-Theater 回答"怎么写测试"（调用真实代码，不复制逻辑）。两者互补。

**触发时机**：
- 新功能开发：先写测试再写实现，没有例外
- Bug 修复：先写复现测试，确认红色，再修复
- 迁移：从旧系统已知行为推导测试，先全红，逐个变绿
- 重构：不改测试，只改实现，全程保持绿色
- **所有场景都遵循 Red/Green 人工确认关卡**

---

### L0: Trait-First — 面向接口编程

**原则**：先定义行为契约（trait），再写实现（impl）。Trait 是可执行的契约——编译器强制实现者遵守签名，比 doc comment 更强。

**Trait-First 不是"每个 struct 抽一个 trait"的教条**。判断标准：**这个行为边界是否需要在测试或开发中被替换？** 需要 → trait；不需要 → 具体类型。

**使用 trait 的场景**：

| 场景 | 为什么需要 trait | 示例 |
|------|----------------|------|
| **模块对外行为** | 调用者依赖契约，不依赖实现细节 | `CardStore` trait — 卡片读写能力 |
| **外部依赖隔离** | 测试时替换为 mock，不碰真实副作用 | `FileSystem` trait — 测试不碰真实文件 |
| **增量开发** | A 依赖 B 但 B 还没实现 | B 的 trait 先定，A 用 mock 开发测试 |
| **多种实现** | 同一行为的不同策略 | `Parser` trait — 不同 frontmatter 格式 |

**不需要 trait 的场景**：

- 纯数据类型（struct 只有字段，没行为方法）
- 模块内部的 helper 函数
- 已有天然替身的依赖（如 `&Connection` — in-memory SQLite 就是测试替身，不需要额外 trait）

**和 Deep Module 的关系**：Deep Module 说"窄接口 + 深实现"。Trait-First 把它具体化——trait 的方法签名 = 窄接口，impl block = 深实现。定义 trait 时就是在画模块边界。

**和 TDD 的联合工作流**：

```
1. 定义 trait（行为契约 — 这一步强制你想清楚模块"做什么"）
2. 写测试（针对 trait 的期望行为）→ 跑测试
3. 🔴 Red 关卡 — 等用户确认失败原因正确
4. 写 impl → 跑测试
5. 🟢 Green 关卡 — 等用户确认全部通过
6. Refactor（保持绿色）
```

**Rust 实践模式**：

```rust
/// 卡片存储的行为契约
pub(super) trait CardStore {
    /// 创建新卡片，返回完整的 AtomicCard（含生成的 id）
    fn create(&self, title: &str, content: &str) -> Result<AtomicCard, KeysightError>;
    /// 按 id 查询单张卡片
    fn get(&self, id: &str) -> Result<AtomicCard, KeysightError>;
    /// 查询所有卡片，支持分页
    fn query_all(&self, limit: Option<i64>) -> Result<Vec<AtomicCard>, KeysightError>;
}

/// 真实实现 — SQLite
pub(super) struct SqliteCardStore<'a> {
    conn: &'a Connection,
}

impl<'a> CardStore for SqliteCardStore<'a> {
    fn create(&self, title: &str, content: &str) -> Result<AtomicCard, KeysightError> {
        // 真实 SQLite 操作
    }
    // ...
}

// 测试 — 用 in-memory SQLite 的真实实现（不需要 mock，因为 SQLite 天然有测试替身）
#[cfg(test)]
mod tests {
    #[test]
    fn test_create_card() {
        let conn = test_conn(); // in-memory SQLite
        let store = SqliteCardStore { conn: &conn };
        let card = store.create("test", "content").unwrap();
        assert_eq!(card.title, "test");
    }
}
```

**何时用 mock vs 真实实现测试**：

| 依赖类型 | 测试策略 | 原因 |
|----------|---------|------|
| SQLite | in-memory 真实实现 | SQLite 本身就是测试友好的，不需要 mock |
| 文件系统 | trait + mock | 真实文件操作慢、有副作用、难清理 |
| 网络 / 外部 API | trait + mock | 不稳定、慢、测试环境不可控 |
| 其他模块（未实现） | trait + stub | 模块还不存在，trait 定义了契约 |

**触发时机**：
- 新增模块：先定义 trait 描述对外行为，再写实现
- 跨模块依赖：通过 trait 边界解耦，允许独立开发和测试
- 需要隔离外部副作用（文件系统、网络）：定义 trait，测试用 mock

---

### L1: 框架参考资料 — 不猜 API，查源码

使用 Tauri / React API 时，不凭记忆猜测，查以下资料：

| 框架 | 参考来源 |
|---|---|
| Tauri commands/events/state | `tauri-commands` / `tauri-events` skills |
| Tauri plugins (fs/store/dialog/http) | `tauri-plugins` skill |
| Tauri security (capabilities/permissions/CSP) | `tauri-security` skill |
| Tauri 构建分发 | `tauri-distribution` skill |
| Tauri 前端集成 (Vite) | `tauri-frontend` skill |
| tauri-specta 用法 | `gh api repos/oscartbeaumont/tauri-specta` README + examples |
| rusqlite API | `rusqlite` crate docs |
| React 19 | `react-best-practices` skill |

遇到不确定的 API 行为时，优先查 skill / 官方文档，不猜。

### L1: Rust Skills

遇到 Rust 编译错误或设计问题时，使用对应的 Rust skill：

| 领域 | Skill |
|---|---|
| 所有权/借用/生命周期 | `rust-ownership` |
| 类型/泛型/trait | `rust-types` |
| 错误处理 | `rust-errors` |
| 并发/async | `rust-concurrency` |
| 领域建模 | `rust-modeling` |
| unsafe/FFI | `rust-unsafe` |
| 代码风格 | `rust-style` |
| Web 服务 | `rust-web` |
| 重构 | `rust-refactor-helper` |

---

### L2: LESSONS.md — 模块级踩坑经验

每个模块目录下可以有 `LESSONS.md`，记录该模块的踩坑经验。

**质量规则：修改某模块代码前，必须先读该模块的 LESSONS.md**（如果存在）。

```markdown
# {模块名} LESSONS

## [YYYY-MM-DD] 简短标题

**症状**: 观察到的现象
**根因**: 为什么会这样
**解决**: 正确做法
**影响范围**: 哪些代码/功能受影响
```

---

### L3: 方法级 doc comment

所有 pub fn 都应该有 doc comment，哪怕只是一句话。和 Operation Contract 的区分：

| 类型 | 适用范围 | 内容 |
|---|---|---|
| **Operation Contract (L0)** | 有副作用的 pub fn（写 DB、状态变更、发事件） | 前置条件 / 执行效果 / 不做的事 / 幂等性 / 关联操作 |
| **方法级 doc comment (L3)** | 所有 pub fn（包括纯查询、纯计算） | 一句话说明做什么。参数非显而易见时加 `# Arguments` |

```rust
/// 根据 filter 条件查询 todos 列表
pub(super) fn list_todos(conn: &Connection, filter: TodoFilter) -> Result<Vec<Todo>, TodoError> {
```

---

### 测试：全栈测试结构

```
src-tauri/
├── src/modules/todo/domain.rs    # 内含 #[cfg(test)] mod tests — Rust unit tests
└── tests/                        # Rust 集成测试（跨模块交互）
    └── todo_stories.rs           # 按用户故事组织

src/
└── __tests__/                    # TS 组件测试
    └── components/               # 按组件组织
        └── TodoList.test.tsx
```

- **Rust unit test**：每个 domain.rs 内部，用 `:memory:` SQLite，测单个业务函数
- **Rust 集成测试**：`tests/` 目录，测跨模块交互和完整用户场景
- **TS 组件测试**：`src/__tests__/` 目录，测组件渲染和事件接线
- 测试辅助：Rust 共享 `test_conn()` helper；TS 共享 `vi.mock('../bindings')` setup

---

### 测试：副作用矩阵

每个写操作登记在此表中。新增写操作时必须同步更新矩阵，并确保有对应测试。

| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|---|---|---|---|
| `card_edit_title` | `entities` + markdown 文件 | DB 写 + 文件写 | domain unit test |
| `card_edit_body` | `entities` + markdown 文件 | DB 写 + 文件写 | domain unit test |
| `card_update_understanding` | `card_fields` + markdown 文件 | DB 写 + 文件写 | domain unit test |
| `section_create` | `entities` | DB 写 | domain unit test |
| `section_delete` | `entities`, `section_members`, `edges`, `positions` | 级联删除 | domain unit test |
| `section_update` | `entities` | DB 写 | domain unit test |
| `section_add_member` | `section_members` | DB 写 | domain unit test |
| `section_remove_member` | `section_members` | DB 写 | domain unit test |
| `section_move_to_whiteboard` | `entities`, `positions`, `edges` | 跨白板迁移 + 清理 | domain unit test |
| `note_create` | `entities` | DB 写 | domain unit test |
| `note_delete` | `entities`, `edges`, `positions` | 级联删除 | domain unit test |
| `note_update` | `entities` | DB 写 | domain unit test |
| `alias_create` | `entities`, `alias_fields` | DB 写 | domain unit test |
| `alias_delete` | `entities`, `alias_fields`, `edges` | 级联删除 | domain unit test |
| `layout_set_position` | `positions` | DB 写 | domain unit test |
| `layout_remove_position` | `positions` | DB 写 | domain unit test |
| `entity_connect` | `edges` | DB 写 | domain unit test |
| `entity_disconnect` | `edges` | DB 写 | domain unit test |
| `sync_file` | `entities`, `card_fields`, `entity_tags`, `edges`, `entities_fts` | 全量同步 | domain unit test |
| `sync_remove_file` | `entities`, `entities_fts` | 按文件删除 | domain unit test |

**更新时机**：新增任何 domain 写函数时，同步在此登记。Code Review 时核对矩阵是否与代码一致。

---

### 测试：TS 组件测试 — Vitest + React Testing Library

**工具**：Vitest + @testing-library/react + @testing-library/jest-dom

**TS 测试的定位**：TS 没有业务逻辑（L0 硬约束），所以 TS 测试只验证两件事：

| 测试什么 | 为什么需要测 | 不测什么 |
|----------|-------------|---------|
| **渲染正确性** — 给定数据，组件是否正确显示 | CSS/JSX 变更可能静默破坏 UI | 数据本身的正确性（Rust 测试覆盖） |
| **事件接线** — 用户操作是否触发正确的 command | 重构时可能断开 handler 绑定 | command 的业务行为（Rust 测试覆盖） |
| **UI 状态切换** — loading / error / empty 状态是否正确渲染 | 异步状态处理容易出错 | 错误本身的语义（Rust 定义） |

**Mock 策略**：mock `commands` from `bindings.ts`。这不是 test theater，因为：
- 业务逻辑正确性由 Rust unit test 保证
- TS 测试只验证"给定 command 返回值 X，UI 是否渲染为 Y"
- 两层测试互补：Rust 保证数据正确，TS 保证展示正确

**标准模式**：

```tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { vi } from 'vitest';
import { commands } from '../bindings';

// mock Tauri commands — 隔离 IPC，只测 UI 层
vi.mock('../bindings', () => ({
  commands: {
    listTodos: vi.fn(),
    createTodo: vi.fn(),
  },
}));

describe('TodoList', () => {
  // 渲染正确性：给定数据 → 正确显示
  it('renders todos from command response', async () => {
    vi.mocked(commands.listTodos).mockResolvedValue([
      { id: 1, title: 'Buy milk', completed: false },
    ]);

    render(<TodoList />);

    expect(await screen.findByText('Buy milk')).toBeInTheDocument();
  });

  // 事件接线：用户操作 → 正确的 command 被调用
  it('calls createTodo on form submit', async () => {
    render(<TodoList />);

    fireEvent.change(screen.getByRole('textbox'), {
      target: { value: 'New' },
    });
    fireEvent.submit(screen.getByRole('form'));

    expect(commands.createTodo).toHaveBeenCalledWith('New');
  });

  // UI 状态：loading 态正确渲染
  it('shows loading state while fetching', () => {
    vi.mocked(commands.listTodos).mockReturnValue(new Promise(() => {}));

    render(<TodoList />);

    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });
});
```

**TDD 同样适用于 UI（含人工确认关卡）**：
- UI Bug 修复：写 RTL 测试复现 bug → 🔴 用户确认 Red → 修 UI 代码 → 🟢 用户确认 Green → 测试永久保留
- 新组件：写"给定数据应该渲染什么"的测试 → 🔴 用户确认 Red → 写组件 → 🟢 用户确认 Green

**判断标准**：如果 TS 测试里出现了业务逻辑判定（校验规则、数据转换、条件分支），说明业务逻辑泄漏到了 TS 层——先修正架构（把逻辑移到 Rust），再写测试。

---

### 质量流程（非 Hook，CLAUDE.md 流程规则）

由于不使用 Claude Code hooks，以下质量检查作为 agent 必须遵循的**流程规则**执行。

#### 编码中

- **每次 `cargo build`**：改为 `cargo clippy --workspace -- -D warnings`（lint 融入开发，错误自然暴露）
- **修改 command 签名后**：立即 `cargo test export_bindings` 重新生成 bindings.ts
- **修改 TS 代码后**：`pnpm test`（组件测试）+ `pnpm build`（tsc 类型检查 + vite 构建）

#### 提交前

```
1. cargo clippy --workspace -- -D warnings    ← Rust lint
2. cargo test --workspace                     ← Rust 测试（unit + 集成）
3. pnpm test                                  ← TS 组件测试
4. pnpm build                                 ← TS 类型检查 + 构建
5. 确认 bindings.ts 是最新的
```

五项全部通过后才可 commit。任何一项失败必须先修复。

#### 功能域完成时

```
1. /harness-check-tests    ← agent 语义自查，补测试缺口
2. Code Review（5 项检查）  ← 测试覆盖 / 逻辑正确性 / 回归风险 / I/O 正确性 / IPC 类型安全
3. git commit              ← 提交
```

---

## 开发流程

### 新 Session 开始

```
1. /harness-resume-context → 读 docs/handoff/*.md（精准状态 + 验证）
2. 读 docs/devlog/ 最新一篇 → 背景时间线
3. 读 docs/progress/*.md → 全局视图，确认 Active 总数 ≤ 3
4. 确定本 session 要做的功能域
5. 如果要修改某模块 → 先读该模块 LESSONS.md
```

### 开发中

```
1. 编译用 clippy 不用 build
2. Task 完成 → 更新 docs/progress/{area}.md + changelog
3. 功能域 Active tasks 全部完成 → 触发质量流程（/harness-check-tests → Review → commit）
```

### Session 结束

```
1. Task 未完成 → /harness-save-next-context → docs/handoff/{area}.md
2. 追加 devlog
3. 如有踩坑 → 写 LESSONS.md + 路由到五层记忆体系对应层级
```

---

## 新模块 Bootstrap 清单

新增业务模块时，按以下顺序执行：

```
1. 创建模块目录 src-tauri/src/modules/{name}/
2. 写 models.rs — derive Serialize + Deserialize + specta::Type
3. 写 errors.rs — thiserror + impl Into<AppError>
4. 写 db.rs — 建表 migration
5. 定义 domain traits — 行为契约（Trait-First）
6. 写 domain 测试 — 针对 trait 的期望行为（TDD: Red）
7. 写 domain.rs — 业务纯函数实现 trait（TDD: Green → Refactor）
8. 写 commands.rs — 薄壳 + #[tauri::command] + #[specta::specta]
9. 写 mod.rs — pub use commands + models
10. 在 modules/mod.rs 注册
11. 在 lib.rs 的 collect_commands![] 添加
12. cargo test export_bindings → 验证 bindings.ts 更新
13. 在副作用矩阵中登记写操作
14. 如涉及新 plugin → 更新 capabilities/default.json（最小权限）
15. 写 TS 组件时 → 配套组件测试（渲染 + 事件接线）
```

---

## Tauri 安全质量项

| 项目 | 规则 | 触发时机 |
|---|---|---|
| Capability 最小权限 | 不得使用 `core:default`，逐条列出需要的权限 | 新增 plugin / 新增 command |
| CSP 配置 | 生产环境必须配置 CSP，当前 `"csp": null` 仅限开发 | 发布前 |
| Plugin 权限 Scoping | HTTP 限定 URL、FS 限定路径 | 添加 http/fs plugin |
| IPC 契约变更 | command 签名变更后必须重新生成 bindings.ts 并验证 TS 编译 | 修改 command |

---

## 构建命令

```bash
# 开发
pnpm tauri dev

# 生产构建
pnpm tauri build

# 重新生成 TS 类型绑定
cargo test export_bindings

# Rust lint（编码中用这个代替 cargo build）
cargo clippy --workspace -- -D warnings

# Rust 测试
cargo test --workspace

# TS 组件测试
pnpm test

# TS 类型检查
pnpm build  # tsc && vite build
```
