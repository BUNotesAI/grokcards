# agent-spec Contract-First 融入质量体系设计 v3

> 日期：2026-04-15  
> 状态：设计草案 v3（基于 v2-codex review 修订）  
> 修订摘要：① 补 `project.spec` 层；② 修正 `lifecycle`/`lint` 能力边界，`/harness-check-tests` 降级而非废弃  
> 参考：[v1-cc](2026-04-15-agent-spec-quality-integration-v1-cc.md) · [v2-codex review](2026-04-15-agent-spec-quality-integration-v2-codex.md) · [agent-spec](https://github.com/ZhangHanDong/agent-spec)

---

## 一、背景与目标

### 现有质量体系的断层

四关注点模型（Design / Progress / Quality / Records）覆盖了项目管理的完整生命周期，但 **Progress → Quality** 之间存在一条隐式断层：

```
Progress: - [ ] task X      ← 知道要做什么（checkbox）
               ↕ 隐式跳转（靠 TDD 衔接，无结构文档）
Quality:  tests pass        ← 知道怎么验证（测试）
```

**缺失的**：一份正式文档，回答「这个 task 在什么条件下算完成？」

目前 TDD 流程给了测试，但没有把「task 的完成定义」显式绑定到「哪些测试通过」。`/harness-check-tests` 做的是 agent 语义自查（发现并补写缺失测试），不是机械验证。

### agent-spec 填什么空

agent-spec 的 **Task Contract**（`.spec.md` 文件）是这座桥：

| Contract 要素 | 对应现有机制 | 新增了什么 |
|---|---|---|
| `Intent` | progress file 的 task 描述 | 加了 **why**（动机约束） |
| `Decisions` | design.md 的架构决策（全局） | 把本 task 相关决策**局部化** |
| `Boundaries` | Code Review 第 4 项（手动检查） | **机械强制**：staged 变更集 vs 允许路径 |
| `Completion Criteria` | TDD 的测试（存在于代码里） | **BDD 场景显式绑定到测试函数名**，`lifecycle` 机械验证存在且通过 |

### agent-spec 的三层继承模型

agent-spec 的规格按层级组织，下层继承上层约束：

```
specs/project.spec          ← 项目级：全局约束（agent-spec 能机械执行的）
    └── specs/{area}/       ← task 级：每个 task 的局部规格
            task-name.spec.md
```

> **重要边界**：project.spec 只承接 agent-spec **能机械执行**的全局规则。  
> CLAUDE.md 中的原则性约束（TDD 人工关卡、建模优先、TS/Rust 职责边界）  
> **无法**被 agent-spec 机械验证，即使写进 project.spec 也只是文档。  
> 这类约束继续留在 CLAUDE.md，不迁移。

### 设计原则

- **零额外工作量**：写 Completion Criteria 就是写 TDD 的 Red 测试，同一件事换结构化格式
- **能力边界诚实**：`lifecycle` 验证你**声明过的** Contract 有没有被满足；判断 Contract 是否完整是人/agent 的语义工作
- **最小替换**：只替换可被机械验证取代的部分，`/harness-check-tests` 降级而非废弃
- **本地独立工具**：`agent-spec` CLI 全局安装（`~/.cargo/bin`），不进入项目 `Cargo.toml`

---

## 二、Task Contract 结构

每个 task 对应一个 `specs/{area}/{task-name}.spec.md` 文件。

### 完整格式

```spec
spec: task
name: "任务的简短英文描述"
tags: [area, module]
---

## Intent

（1-3 句）这个 task 做什么、为什么做、解决了什么问题。

## Decisions

- 本 task 已固定的技术决策（从 design.md 局部化抄录）
- 不在此列的技术选择属于实现自由度

## Boundaries

### Allowed Changes
- src-tauri/src/modules/{module}/**
- src-tauri/tests/{test_file}.rs

### Forbidden
- 明确禁止触碰的路径（project.spec 的 Forbidden 自动继承，此处写 task 特有的）

## Completion Criteria

Scenario: 场景名（中文）
  Test: test_fn_name_exactly_as_in_code
  Given 前置条件
  When 触发操作
  Then 期望结果
  And 附加断言（可选）

Scenario: 另一个场景
  Test: another_test_fn_name
  ...
```

中文 step 关键词（`场景 / 测试 / 假设 / 当 / 那么 / 并且`）原生支持。

### 关于 `Test:` 字段——最核心的机械检查

`Test: test_fn_name` 是 agent-spec 和现有测试之间的**唯一桥梁**。  
`agent-spec lifecycle` 扫描 codebase，确认该函数名**真实存在且对应测试通过**。

这关闭了一个静默漏洞：agent 认为写了测试，但函数名拼错、函数存在但被 `#[ignore]` 标注——这些在 `lifecycle` 之前不会被发现。

---

## 三、project.spec——项目级全局约束

### 为什么需要 project.spec

没有 project.spec，每个 task spec 要么重复声明相同的全局约束，要么省略它们。前者产生漂移（各 task 的声明不一致），后者让全局规则游离在机械验证之外。

### 范围划定——只写 agent-spec 能机械执行的

| 约束类型 | 放在哪里 | 原因 |
|---|---|---|
| Scenario 必须有 Test: 绑定 | **project.spec** | `lifecycle lint` 能机械检查 |
| src/bindings.ts 永远在 Forbidden | **project.spec** | `guard` 能机械检查变更集 |
| 最低 quality score 阈值 | **project.spec** | `lifecycle --min-score` 能机械执行 |
| TDD 人工 Red/Green 关卡 | **CLAUDE.md**（保留） | 流程性约束，agent-spec 无法验证 |
| 建模优先 / 防火墙规则 | **CLAUDE.md**（保留） | 设计原则，agent-spec 无法验证 |
| TS/Rust 职责边界 | **CLAUDE.md**（保留） | 架构约束，agent-spec 无法验证 |
| Operation Contract doc comment | **CLAUDE.md**（保留） | 函数级约束，agent-spec 不涉及 |

### super-tauri 的 project.spec

```spec
spec: project
name: "super-tauri"
tags: [tauri, rust, typescript]
---

## Intent

super-tauri 是一个 Tauri 桌面应用，Rust 独占业务逻辑，TypeScript 只负责 UI 层。
所有业务写入经过 Rust domain 纯函数，通过 tauri-specta 在编译期保证 IPC 类型安全。

## Global Decisions

- SQLite 是唯一数据真源，TS 侧不持有业务状态的持久副本
- 所有 #[tauri::command] 必须同时加 #[specta::specta]
- IPC 类型绑定由 cargo test export_bindings 自动生成，禁止手动编辑 bindings.ts

## Global Boundaries

### Always Forbidden
- src/bindings.ts（auto-generated，由 cargo test export_bindings 更新，禁止手动编辑）
- .claude/（本地 Claude Code 配置，不进 spec 变更集）

## Global Completion Rules

- 每个 Scenario 必须声明 Test: 绑定（无 Test: 的 Scenario 视为 skip，lifecycle 拒绝通过）
- Completion Criteria 至少包含一个 happy path Scenario 和一个 error path Scenario
- lifecycle 最低质量分：0.7
```

### project.spec 与 task spec 的关系

- task spec 的 Boundaries Forbidden 自动继承 project.spec 的 `Always Forbidden`，无需重复声明
- task spec 的 Completion Rules 自动继承 project.spec 的 `Global Completion Rules`（`lifecycle` lint 阶段执行）
- task spec 只写本 task 特有的 Intent / Decisions / Boundaries / Completion Criteria

---

## 四、`lifecycle`/`lint` 能力边界——修正说明

### 能力边界的准确定义

`agent-spec lifecycle` 的机械验证范围：

```
lifecycle = lint + verify + report

lint:    检查 spec 格式质量
         ✅ 发现"声明了的 Scenario 没有 Test: 绑定"（coverage gap = 声明了但未绑定）
         ❌ 不能发现"应该声明但没声明的业务场景"（无法从代码反推遗漏的 Scenario）

verify:  扫描 codebase，确认 Test: fn_name 存在且 cargo test 通过
         ✅ 机械验证"声明过的 Contract 有没有被满足"
         ❌ 不能判断"Contract 是否足够完整"

report:  输出 pass/fail/skip/uncertain 四种结果
         skip = 有 Scenario 但没有 verifier 覆盖（即没有 Test: 绑定）
```

### 后果：spec 不完整时 lifecycle 依然可能全绿

如果 Completion Criteria 只声明了 happy path，漏掉了所有 error path，`lifecycle` 依然会通过——因为声明的场景都有 Test: 绑定且测试通过。`lifecycle` 不知道"还应该有一个 error path Scenario"。

这是 v1 的核心错误：把「验证已声明 Contract」写成了「自动发现漏写场景」。

### /harness-check-tests 的新职责

`/harness-check-tests` **不废弃，降级为 spec completeness 自查**：

| 版本 | 职责 | 执行时机 |
|---|---|---|
| **旧** | 在代码里发现缺失测试 → 补写测试 | 功能域完成时，替代 lifecycle |
| **新** | 审查 Completion Criteria 是否足够完整 → 发现漏掉的场景 → 补写到 spec | lifecycle **之前**运行，确保 Contract 不是"完整通过但实际不完整" |

新职责的 prompt 调整（需同步更新 slash command）：

```markdown
# /harness-check-tests（新版：spec completeness 自查）

检查当前功能域所有 active task spec 的 Completion Criteria 是否完整。

1. 读取 specs/{area}/*.spec.md（当前功能域的所有 active task spec）
2. 读取对应的实现代码（domain.rs 的 pub fn + doc comment）
3. 对每个 spec，检查 Completion Criteria 是否覆盖了以下场景：
   - 正常路径（happy path）
   - 所有有意义的错误路径（error path）
   - 边界条件（空值、极值、空集合等）
   - 跨实体交互（如有）
4. 发现 Completion Criteria 遗漏的场景 → 补写 Scenario + Test: 绑定到对应 spec
5. 重新运行 agent-spec lint 确认无 coverage gap（声明层面）
```

---

## 五、Contract-First 工作流（完整版）

### 5.1 新建 Task 时

```
1. 在 progress/{area}.md 添加 checkbox，附 spec 指针：
   - [ ] task 描述 → `specs/{area}/task-name.spec.md`

2. 创建 specs/{area}/task-name.spec.md
   - Intent：为什么做这个 task
   - Decisions：本 task 固定的技术决策
   - Boundaries：Allowed/Forbidden 路径（project.spec 的 Always Forbidden 自动继承）
   - Completion Criteria：写出所有 BDD 场景 + Test: 字段（函数名先声明，代码未写）
```

写 Completion Criteria **就是** TDD Red 阶段的前置工作——声明的函数名就是接下来要写的失败测试桩。

### 5.2 TDD Red 阶段

不变的：Red → 用户确认 → Green → 用户确认 → Refactor。

变化的：Red 阶段的起点从「凭感觉想测什么」变成「按 Completion Criteria 声明的函数名写测试桩」。

```
spec 中声明：
  Test: test_create_task_empty_title_returns_error  ← 函数名已声明

  ↓ 按声明的名字写测试桩
  #[test]
  fn test_create_task_empty_title_returns_error() {
      let conn = test_conn();
      let result = create_task(&conn, "   ", ...);
      assert!(result.is_err());   // Red：实现未完成，断言失败
  }

  ↓ cargo test → 确认 Red
  ↓ 用户确认 Red 关卡
```

### 5.3 实现阶段

不变：写代码 → `cargo clippy`（build hook 拦截）→ `cargo test` → 用户确认 Green → Refactor。

### 5.4 功能域完成时的质量关卡

**旧流程**：
```
/harness-check-tests → /harness-type-safety-check → Code Review（6 项）→ git commit
```

**新流程**：
```
/harness-check-tests（降级版：spec completeness 自查）
  ↓ 发现遗漏场景 → 补写 Scenario + Test: → 重复直到 Completion Criteria 完整
agent-spec lifecycle（对每个 active task spec 运行）
  ↓ lint: 无 Test: 缺口，quality score ≥ 0.7
  ↓ verify: 所有 Test: fn_name 存在且 cargo test 通过
  ↓ 任何 fail/skip → 修 spec 或补测试 → 重跑
/harness-type-safety-check（保留）
Code Review（Contract Acceptance + 5 项）
git commit（pre-commit hook：agent-spec guard + cargo test）
```

两步语义不同，不可合并：

| 步骤 | 问题 | 执行者 |
|---|---|---|
| `/harness-check-tests`（新） | Completion Criteria 有没有漏掉重要场景？ | agent 语义判断 |
| `agent-spec lifecycle` | 声明的场景有没有对应测试且通过？ | 机械验证 |

#### Code Review 第 1 项调整

原第 1 项「测试覆盖 — 每个 domain 纯函数至少 happy path + error path 各一个测试」拆分为：

- **Contract Acceptance**（人工）：Completion Criteria 的场景是否足够完整？有没有 `/harness-check-tests` 自查时漏掉的场景？
- **lifecycle 机械验证**（已完成）：声明的场景对应测试存在且通过。

更新后的 Code Review 6 项检查（第 1 项调整，其余不变）：

1. **Contract Acceptance** — Completion Criteria 是否覆盖了所有重要场景？`/harness-check-tests` 自查 + lifecycle 之后，还有没有漏网之鱼？
2. **逻辑正确性** — 特别是从旧代码移植的逻辑，逐行核对
3. **回归风险** — 未来变更可能静默破坏的场景，是否有测试锁住
4. **I/O 正确性** — SQLite 读写完整性、文件操作正确性
5. **IPC 类型安全** — `#[specta::specta]`、bindings.ts 是否最新
6. **建模强度（防火墙）** — 签名中的 `String/bool/&str`、invariant 保证方式、穷尽 match

---

## 六、冗余分析——完整版

### 降级（职责转变，不废弃）

| 机制 | 旧职责 | 新职责 | 执行时机变化 |
|---|---|---|---|
| `/harness-check-tests` | 在代码里找缺失测试 → 补写 | 审查 Completion Criteria 完整性 → 补写场景到 spec | lifecycle **之前**（而非之后或替代） |
| Code Review 第 1 项 | 测试覆盖量检查（人工数测试数量） | Contract Acceptance（场景完整性判断） | 时机不变，内容升级 |

### 新增（机械能力增量）

| 新机制 | 作用 | 触发时机 |
|---|---|---|
| `agent-spec lifecycle` | 机械验证 Test: 绑定存在且通过 | 功能域完成时，位于 `/harness-check-tests` 之后 |
| `agent-spec guard`（pre-commit） | Boundaries 变更集检查 + spec lint | git commit 时自动 |
| `project.spec` | 全局 agent-spec 可执行约束的单一来源 | 所有 task spec 继承 |

### 完全保留（不受影响）

以下机制与 agent-spec 操作的层级不同（函数级 vs task 级），不存在替换关系：

- Operation Contract（pub fn doc comment）
- Deep Module 规则 / 数据写回规则 / TS/Rust 职责边界 / IPC 类型安全
- Anti-Test-Theater / TDD Red/Green 人工确认关卡 / Trait-First / 建模优先 + 强类型
- `/harness-type-safety-check`
- L1-L4 全部层级（框架参考 / LESSONS.md / doc comment / MEMORY）
- 副作用矩阵 / TS 组件测试 / Handoff 协议

---

## 七、CLAUDE.md 需要更新的段落

### 7.1 质量机制总览表——新增两行，无删除

在「L0 | TDD」行之前插入：

```markdown
| L0 | Task Contract — task 级规格（`specs/{area}/*.spec.md`） | 新建 task 时 |
| L0 | project.spec — 项目级全局约束（agent-spec 可执行） | 持续继承，无需手动触发 |
```

`/harness-check-tests` **保留**在表中，更新触发时机描述：

```markdown
| 测试 | /harness-check-tests — spec completeness 自查 | 功能域完成时，lifecycle 之前 |
```

### 7.2 新增「L0: Task Contract」章节

在「L0: TDD」之前新增，内容包含：
- Task Contract 四要素
- `.spec.md` 文件格式（中英文示例）
- `Test:` 字段的机械验证说明
- 与 TDD Red 阶段的关系
- 与 Operation Contract 的区别（task 级 vs 函数级）

### 7.3 新增「L0: project.spec」章节

在「L0: Task Contract」之后新增，内容包含：
- project.spec 的范围划定（只含 agent-spec 可机械执行的规则）
- super-tauri 的 `specs/project.spec` 完整内容
- 与 CLAUDE.md 的分工（原则性约束留在 CLAUDE.md，可执行约束进 project.spec）

### 7.4 功能域完成 Code Review 流程更新

**旧**：
```
1. /harness-check-tests
2. /harness-type-safety-check
3. Code Review（6 项检查）
4. git commit
```

**新**：
```
1. /harness-check-tests（新职责：spec completeness 自查，补写遗漏场景）
2. agent-spec lifecycle（机械验证声明的 Contract 全部满足）
3. /harness-type-safety-check
4. Code Review（Contract Acceptance + 5 项）
5. git commit（pre-commit hook 内含 agent-spec guard）
```

### 7.5 Code Review 6 项——第 1 项措辞更新

**旧**：
> 1. **测试覆盖** — 每个 domain 纯函数至少 happy path + error path 各一个测试。边界条件、跨实体交互是否覆盖

**新**：
> 1. **Contract Acceptance** — Completion Criteria 的场景是否足够完整？`/harness-check-tests` 自查和 `lifecycle lint` 之后，还有没有遗漏的 error path / 边界条件 / 跨实体交互？（Test: 函数是否存在且通过已由 `lifecycle` 机械确认，人工只审场景的**完整性**）

### 7.6 /harness-check-tests slash command——更新 prompt

slash command 文件内容需更新为「spec completeness 自查」版本（见第四章）。

### 7.7 提交前检查列表——新增 guard

**新**：
```
1. cargo clippy --workspace -- -D warnings
2. agent-spec guard --spec-dir specs --code . --change-scope worktree
3. cargo test --workspace
4. pnpm test
5. pnpm build
6. 确认 bindings.ts 是最新的
```

### 7.8 新 Session 开始流程——新增 spec 读取

在「读 docs/progress/*.md → 全局视图」之后：

```
4b. 如果当前 task 有 spec 文件（progress 条目末尾有 → specs/...），先读 spec 了解 Completion Criteria
```

---

## 八、目录结构

```
super-tauri/
├── specs/                         ← 新增
│   ├── project.spec               ← 项目级全局约束（agent-spec 可执行）
│   ├── backend/
│   │   ├── kanban-subtask-create.spec.md
│   │   └── task-status-transition.spec.md
│   ├── frontend/
│   │   └── kanban-card-drag.spec.md
│   ├── ipc/
│   │   └── specta-export-bindings.spec.md
│   └── infra/
│       └── db-migration-v2.spec.md
├── docs/
│   ├── progress/                  ← 不变，checkbox 添加 spec 指针
│   └── handoff/                   ← 不变
└── src-tauri/                     ← 不变
```

---

## 九、安装与配置

### 9.1 安装 agent-spec CLI

```bash
cargo install agent-spec
agent-spec --version
```

### 9.2 pre-commit hook 更新

**旧**：
```json
{
  "type": "command",
  "if": "Bash(git commit *)",
  "command": "cargo test --workspace 2>&1 || { echo 'pre-commit: tests failed' >&2; exit 2; }",
  "timeout": 120000
}
```

**新**：
```json
{
  "type": "command",
  "if": "Bash(git commit *)",
  "command": "agent-spec guard --spec-dir specs --code . --change-scope staged 2>&1 || { echo 'pre-commit: agent-spec guard failed' >&2; exit 2; }; cargo test --workspace 2>&1 || { echo 'pre-commit: tests failed' >&2; exit 2; }",
  "timeout": 180000
}
```

顺序：`guard` 先（Boundaries 检查 + spec lint），`cargo test` 后（全量测试保底）。

### 9.3 初始化目录

```bash
mkdir -p specs/{backend,frontend,ipc,infra}
# 创建 project.spec（内容见第三章）
```

---

## 十、进度文件格式变化

### 新格式

```markdown
## Active
- [ ] task 描述 → [`specs/backend/task-name.spec.md`](../../specs/backend/task-name.spec.md)

## Active（显式免 spec 示例）
- [x] 修复 clippy 警告 [no-spec]
```

规则：
- spec 指针用相对路径，确保可点击
- task 完成后 spec 文件**保留**（历史记录 + 回归 Contract）
- 显式免 spec 必须加 `[no-spec]` 标记，默认不放行

---

## 十一、过渡策略

### 现有 Active Task

不回溯补写 spec，继续用旧流程（`/harness-check-tests` 旧职责）完成。

`/harness-check-tests` 旧版 prompt 保留到所有旧 Active task 完成，届时切换为新版 prompt。

### 新 Task（从今天起）

所有新 task 默认须写 `.spec.md`，用户显式标 `[no-spec]` 才可免。

### `/harness-check-tests` prompt 切换时机

当所有旧 Active task 都已进入 Done，将 slash command 的 prompt 切换为「spec completeness 自查」版本。

---

## 十二、完整示例

### specs/backend/task-create-area-field.spec.md

```spec
spec: task
name: "task-create-area-field"
tags: [backend, task]
---

## Intent

task_create 当前不支持 area 字段（工作域分类）。
为 TaskFields 增加 area: Option<String>，让前端能在创建时指定归属工作域，
支持 Kanban 视图按工作域筛选。

## Decisions

- area 存入 task_fields.area 列（VARCHAR, nullable）
- area 值不做枚举约束，由前端自由传入
- 空字符串视为 None（Rust 侧 trim 后判空）
- task_create 命令新增 area: Option<String> 参数
- 不修改现有的 task_update 签名

## Boundaries

### Allowed Changes
- src-tauri/src/modules/task/**
- src-tauri/tests/task_stories.rs

### Forbidden
- src-tauri/src/modules/kanban/**
- Do not change task_update signature

## Completion Criteria

场景: 创建任务时指定 area 成功持久化
  测试: test_task_create_with_area_persists_to_db
  假设 数据库已初始化
  当 调用 create_task(&conn, "Buy milk", Some("work"))
  那么 返回 Task { title: "Buy milk", area: Some("work"), ... }
  并且 SELECT area FROM task_fields WHERE entity_id = ? 返回 "work"

场景: area 为空字符串时视为 None
  测试: test_task_create_empty_area_stored_as_null
  假设 数据库已初始化
  当 调用 create_task(&conn, "Buy milk", Some(""))
  那么 返回 Task { area: None, ... }
  并且 DB 中 area 列为 NULL

场景: area 为 None 时向后兼容
  测试: test_task_create_without_area_backward_compatible
  假设 数据库已初始化
  当 调用 create_task(&conn, "Buy milk", None)
  那么 返回 Task { area: None, ... }
  并且 不报错，行为与旧版一致
```

### 对应 progress/backend.md

```markdown
## Active
- [ ] task_create 支持 area 字段 → [`specs/backend/task-create-area-field.spec.md`](../../specs/backend/task-create-area-field.spec.md)
```

### 功能域完成时的操作序列

```bash
# 1. spec completeness 自查（/harness-check-tests 新版 prompt）
#    → 发现"还缺 area 值含空格时的边界测试"→ 补写到 spec

# 2. 机械验证
agent-spec lifecycle specs/backend/task-create-area-field.spec.md --code . --format json
# 期望：3 个 Scenario 全部 pass，quality score ≥ 0.7

# 3. 类型安全自查（/harness-type-safety-check）

# 4. Code Review（Contract Acceptance：场景完整性 + 5 项）

# 5. 提交（guard 自动运行）
git commit -m "feat(task): add area field to task_create"
```

---

## 十三、更新后的质量机制总览表

| 层 | 机制 | 触发时机 |
|---|------|---------|
| L0 | **Task Contract** — task 级规格（`specs/{area}/*.spec.md`） | **新建 task 时** |
| L0 | **project.spec** — 项目级全局约束（agent-spec 可执行） | 持续继承 |
| L0 | Operation Contract — pub fn 契约 | 新增/修改 pub fn |
| L0 | Deep Module 规则 — 窄接口 + 深实现 | 新增模块/重构 |
| L0 | 数据写回规则 — SQLite 是 source of truth | 修改数据层 |
| L0 | 功能域完成 Code Review — 强制关卡 | 功能域 Active tasks 全部完成 |
| L0 | TS/Rust 职责边界硬约束 | 任何 commit |
| L0 | IPC 类型安全 — tauri-specta 编译期保障 | 新增/修改 command |
| L0 | 测试真实代码路径 — 禁止 test theater | 新增测试 / Code Review |
| L0 | TDD — Red-Green-Refactor 开发循环 | 新功能 / Bug 修复 / 迁移 / 重构 |
| L0 | Trait-First — 面向接口编程 | 新增模块 / 跨模块依赖 |
| L0 | 建模优先 + 强类型 — 防火墙模型 | 任何 pub fn / pub struct / trait |
| L1 | 框架参考资料 — 不猜 API | 使用 Tauri / React API 时 |
| L1 | Rust Skills — 10 个领域 | 遇到编译错误/设计问题 |
| L2 | LESSONS.md — 模块级踩坑经验 | 修改模块代码前**必须先读** |
| L3 | 方法级 doc comment | 阅读/修改方法时 |
| L4 | MEMORY + Skills — 个人偏好 + 跨项目知识 | 关键词触发 / 按需召回 |
| 测试 | **/harness-check-tests — spec completeness 自查（新职责）** | **功能域完成时，lifecycle 之前** |
| 测试 | 集成测试 tests/ | cargo test |
| 测试 | 副作用矩阵 | 新增写操作时同步更新 |
| 测试 | TS 组件测试 — Vitest + RTL | 新增/修改 TS 组件 / UI Bug 修复 |

---

## 十四、功能域完成关卡——新旧对照

**旧**：
```
功能域 Active tasks 全部完成
  │
  ├─ 1. /harness-check-tests        ← agent 在代码里找缺失测试 → 补写
  ├─ 2. /harness-type-safety-check
  ├─ 3. Code Review（6 项）
  ├─ 4. git commit（cargo test）
  ▼
标记 Done
```

**新**：
```
功能域 Active tasks 全部完成
  │
  ├─ 1. /harness-check-tests（新）  ← agent 审查 Completion Criteria 完整性
  │       ↳ 遗漏场景 → 补写 Scenario + Test: → 重复
  ├─ 2. agent-spec lifecycle         ← 机械验证"声明的 Contract 有没有被满足"
  │       ↳ lint: 所有 Scenario 有 Test: 绑定，quality ≥ 0.7
  │       ↳ verify: Test: fn_name 存在且 cargo test 通过
  │       ↳ fail/skip → 修 spec 或补测试 → 重跑
  ├─ 3. /harness-type-safety-check  ← 保留
  ├─ 4. Code Review（Contract Acceptance + 5 项）
  ├─ 5. git commit（pre-commit: agent-spec guard + cargo test）
  ▼
标记 Done
```

---

## 附录：agent-spec 命令速查

| 命令 | 用途 | 何时运行 |
|---|---|---|
| `agent-spec init --level task --lang zh --name "X"` | 生成 spec 模板 | 新建 task 时 |
| `agent-spec lint specs/{area}/x.spec.md` | 检查 spec 质量（vague verbs / 缺 Test: / coverage gap） | 写完 spec 后 |
| `agent-spec contract specs/{area}/x.spec.md` | 渲染 Contract 视图（给 agent 看） | 开始实现前 |
| `agent-spec lifecycle specs/{area}/x.spec.md --code .` | **主质量关卡**：lint + verify + report | `/harness-check-tests` 之后 |
| `agent-spec guard --spec-dir specs --code . --change-scope staged` | repo 级验证（pre-commit，自动运行） | git commit 时 |
| `agent-spec explain specs/{area}/x.spec.md --code . --format markdown` | 生成 Contract Acceptance 摘要 | Code Review 时 |
| `agent-spec guard --change-scope worktree` | 手动全量检查 | 提交前手动验证 |
