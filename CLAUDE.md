# super-tauri

## Workflow v4 — Task-Centric

本项目使用 **v4 task-centric 对话式工作流**。Task 是一等 entity,`task.md` 是总入口,frontmatter 是 index,per-task 子目录放 design / spec / plan / reviews。**所有 doc 物理位置都在 Obsidian vault 里**,repo **不再有 `docs/` 目录**,通过 `vault/` symlink 访问 vault。

### Vault Symlink Setup(每台机器本地配置)

本 repo **没有 `docs/` 目录** —— 所有 doc(v4 设计文档 / progress / devlog / handoff / tasks / lessons / 历史记录 / project spec)都在 Obsidian vault:

```
~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/
```

新机器 clone 后必须先建立 `vault/` symlink:

```bash
ln -s ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri vault
```

`vault/` 已在 `.gitignore`,不会被 commit。通过 symlink 访问:

- `vault/docs/progress/*.md` — 功能域进度(legacy 兼容)
- `vault/docs/devlog/*.md` — 日志
- `vault/docs/handoff/*.md` — 区级 handoff(legacy)
- `vault/docs/tasks/{task_id}/*` — **per-task artifacts(v4 核心)**
- `vault/docs/lessons/modules/*.md` — 模块级 LESSONS(跨 task 真源)
- `vault/docs/legacy/*` — 历史记录(迁移前的 collab / superpowers)
- `vault/docs/project.spec.md` — 项目级全局 spec(agent-spec 自动 walk-up 发现)
- `vault/{task_id} 【TASK】{title}.md` — **task 总入口(和 docs/ 同级,不在 docs/ 下)**

**重要**:Task.md 物理位置**不在 `docs/` 下**,而是 vault 根目录,与 `docs/` 同级。这让 Obsidian 能把 task 文件当成一等 note,享受 graph / backlinks / search 原生能力。

### Agent-Spec CLI(从本地 fork 安装)

本项目使用 `agent-spec` CLI 做 task spec 的 BDD 契约验证。**不从 crates.io 安装**,用本地 fork(以便根据项目特定环境改源码):

```bash
cargo install --path /Users/alexwang/codes/vibe-coding/agent-spec-fork
/Users/alexwang/codes/vibe-coding/agent-spec-fork/install-skills.sh  # 幂等,只 copy skills
```

Fork 路径:`/Users/alexwang/codes/vibe-coding/agent-spec-fork/`

验证:

```bash
agent-spec --version  # 应显示 0.2.7 或更新版本
```

`project.spec.md` 放在 `vault/docs/project.spec.md`,task spec 里用 `inherits: project` 声明继承。agent-spec resolver(`spec_parser/resolver.rs::default_search_dirs`)从 task spec 所在目录向上 walk,自动发现 `vault/docs/project.spec.md`,**不需要改 fork 源码**。

### 术语表(v4)

| 术语 | 定义 | 载体 |
|------|------|------|
| **Task** | 项目里一个完整的工作单元(含 design / spec / plan / 实施 / 收口)。Task 是一等 entity | `vault/{task_id} 【TASK】{title}.md` |
| **Task Frontmatter** | Task 的 index,用 YAML 声明 design / spec / plan / review / commits / lessons / mempal / rules / skills | task.md frontmatter |
| **Design** | 对话式推敲的架构方案 | `vault/docs/tasks/{id}/design-v{N}-{author}.md` |
| **Spec** | 从 design 提炼的 Task Contract(agent-spec DSL)+ BDD Scenario + Test: 绑定 | `vault/docs/tasks/{id}/spec.md` |
| **Plan** | (可选)把 spec 拆为 phase 的实施计划 | `vault/docs/tasks/{id}/plan.md` |
| **Review** | 独立 reviewer 的 critique(codex / oracle) | `vault/docs/tasks/{id}/reviews/r{N}-{reviewer}.md` |
| **Handoff(task 级)** | Session 结束时 task 未完成的精准执行快照 | `vault/docs/tasks/{id}/handoff.md` |
| **Handoff(区级 legacy)** | 无 active task 时的功能域级上下文快照 | `vault/docs/handoff/{area}.md` |
| **Progress(legacy)** | 功能域级进度总表,v4 下作 legacy 兼容辅助视图 | `vault/docs/progress/{area}.md` |

### Task Frontmatter Schema

```yaml
---
# 基础字段
type: project-task
id: task_{hex8}
status: inbox | next | active | blocked | done
project: super-tauri
color: "#RRGGBB" | null        # 可选
area: backend | frontend | ipc | infra | {其他}

# 上下文(path 相对 task.md 所在目录 = vault 根)
design: docs/tasks/{id}/design-v{N}-{author}.md   # 永远指向最新版
spec: docs/tasks/{id}/spec.md | null              # null = 机械型 task 无 spec
plan: docs/tasks/{id}/plan.md | null              # null = 简单 task 无 plan
review:                                            # append-only,按时间顺序
  - docs/tasks/{id}/reviews/r1-codex.md
  - docs/tasks/{id}/reviews/r1-cc.md
  # ...

# 完成沉淀(05-close 回填)
commits:
  - hash: {short_hash}
    subject: "{commit subject 原文}"
  # ...
lessons:
  - "{非显而易见的经验,一行}"
  # ...
mempal:                                            # KG triples
  - subject: "{entity}"
    predicate: "{relation}"
    object: "{target}"
  # ...
rules:                                             # task 范围的硬约束(in-context reminder)
  - "{规则原文}"
skills:                                            # 适用的 Claude Code skills
  - {skill_name}
---
```

### Task 生命周期

```
inbox ──/harness-01-design──▶ next ──/harness-02-spec──▶ next
                                           │
                                           ▼
                            /harness-03-plan(可选)
                                           │
                                           ▼
                            /harness-04-execute
                                           │
                                           ▼
                             active ──▶ blocked
                                     │          │
                                     │  [resume]
                                     ▼
                            /harness-05-close
                                     │
                                     ▼
                                   done
```

### Slash Commands 速查表(v4 核心 7 个 + 4 个辅助)

| 类别 | 命令 | 作用 |
|---|---|---|
| **核心 1 — Design** | `/harness-01-design` | 对话式推敲架构,产出 `design-v{N}-cc.md` |
| **核心 2 — Spec** | `/harness-02-spec` | 从 design 提炼为 agent-spec DSL contract |
| **核心 3 — Plan** | `/harness-03-plan` | (可选)拆 spec 为实施 phases |
| **核心 4 — Execute** | `/harness-04-execute` | 严格 TDD Red-Green-Refactor,`status: active` |
| **核心 5 — Close** | `/harness-05-close` | 质量关卡 + 回填 commits/lessons/mempal,`status: done` |
| **横向** | `/harness-review` | 请 codex/oracle 对 design/spec/plan/code 做独立 review |
| **横向** | `/harness-view` | 纯读,渲染 task 全景 summary |
| 辅助 | `/harness-resume-context` | 新 session 开始时读 handoff 恢复上下文 |
| 辅助 | `/harness-save-next-context` | Session 结束时写 handoff(task 级优先,fallback 区级) |
| 辅助 | `/harness-check-tests` | spec Completion Criteria 完整性自查 |
| 辅助 | `/harness-type-safety-check` | L0 防火墙自查(类型安全 / 建模强度) |
| 辅助 | `/harness-init-or-migrate` | 新项目 init v4 骨架 / 现有项目 migrate 到 v4 |

**用户认知负担 = 7 个核心命令**(01-05 + review + view)。辅助命令大多自动触发或由其他命令调用。

**Bug fix 无专用命令** —— 手动调 codex CLI 或通过 `/codex:rescue` skill。

### Progress & Devlog(v4 下的定位)

| 载体 | 位置 | 写入时机 | v4 下的角色 |
|------|------|---------|------------|
| Task Frontmatter | task.md | 每个阶段自动更新 | **主状态源** |
| Devlog | `vault/docs/devlog/{YYYY-MM-DD}.md` | Session 结束前追加 | 时间线记录 |
| Changelog | slipbox `logs/changelog/{date}.md` | Task 完成时追加 | 个人日报 |
| Progress(legacy) | `vault/docs/progress/{area}.md` | 功能域级粗粒度追踪 | **辅助**,不是主源 |
| LESSONS(模块级) | `vault/docs/lessons/modules/{name}.md` | Task 收口时或 bug 修复后 | 跨 task 真源 |
| LESSONS(task 级) | task.md frontmatter `lessons:` | 05-close 回填 | task 内快照 |

### Handoff 协议(v4)

**有 active task**:

```
Session 结束 → /harness-save-next-context → vault/docs/tasks/{task_id}/handoff.md(task 级)
新 Session 开始 → /harness-resume-context → 读取 + 跑 stale_check + 验证 git 状态 → 等用户确认
```

**无 active task(legacy)**:

```
Session 结束 → /harness-save-next-context → vault/docs/handoff/{area}.md(区级)
新 Session → /harness-resume-context → 同上
```

---

## 质量 (Quality)

### 质量机制总览

| 层 | 机制 | 触发时机 |
|---|------|---------|
| L0 | Operation Contract — pub fn 契约 | 新增/修改 pub fn |
| L0 | Deep Module 规则 — 窄接口 + 深实现 | 新增模块/重构 |
| L0 | 数据写回规则 — SQLite 是 source of truth | 修改数据层 |
| L0 | Task Contract — spec.md 作为可验证契约 | 非机械型 task 在 `/harness-02-spec` 阶段 |
| L0 | agent-spec lifecycle — BDD 契约机械验证 | Task 有 spec.md 时,在 `/harness-05-close` 阶段 |
| L0 | Task 完成 Code Review — 强制关卡 | Task 进入 `/harness-05-close` 阶段 |
| L0 | Task-Scoped Commits — Phase 边界 + Task-Id trailer | `/harness-04-execute` 每个 Phase 完成时 |
| L0 | TS/Rust 职责边界硬约束 — TS 只负责 UI，Rust 独占业务 | 任何 commit |
| L0 | IPC 类型安全 — tauri-specta 编译期保障 | 新增/修改 command |
| L0 | 测试真实代码路径 — 禁止 test theater | 新增测试 / Code Review |
| L0 | TDD — Red-Green-Refactor 开发循环 | 新功能 / Bug 修复 / 迁移 / 重构 |
| L0 | Trait-First — 面向接口编程 | 新增模块 / 跨模块依赖 / 可测试性设计 |
| L0 | 建模优先 + 强类型 — 防火墙模型（"想出错都难"） | 任何 pub fn / pub struct / trait |
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

### L0: Task 完成 Code Review — 强制关卡

**触发条件**:Task 进入 `/harness-05-close` 阶段时触发。Close 命令内部按顺序跑完下面的流程。

**完整流程**:

```
Task 所有 execute 完成
  │
  ├─ 1. /harness-check-tests        ← spec Completion Criteria 自查
  ├─ 2. agent-spec lifecycle         ← BDD 契约机械验证(如有 spec.md)
  ├─ 3. /harness-type-safety-check   ← L0 防火墙 / 类型安全自查
  ├─ 4. Task Code Review             ← 结构化审查(6 项检查,见下)
  ├─ 5. 手动 walkthrough             ← 如有 UI 改动,真机/dev server 跑一遍
  ├─ 6. 自动 grep commits            ← git log --grep="^Task-Id: {id}$" → 填 task.md
  ├─ 7. 提炼 lessons + mempal triples ← 写入 task.md + 模块级 LESSONS
  ├─ 8. status → done                ← 收口
  └─ 9. git commit                   ← 提交(用户决定 message)
```

**Review 6 项检查**:

1. **Contract Acceptance** — spec.md 的 Intent 是否被实现覆盖?agent-spec lifecycle 是否全绿?所有 Scenario 的 Test: 绑定都存在且通过?
2. **逻辑正确性** — 特别是从旧代码移植的逻辑,逐行核对
3. **回归风险** — 未来变更可能静默破坏的场景,是否有测试锁住
4. **I/O 正确性** — SQLite 读写完整性、文件操作正确性
5. **IPC 类型安全** — 所有 command 是否有 `#[specta::specta]`,bindings.ts 是否最新,TS 是否从 bindings import
6. **建模强度(防火墙)** — pub fn 是否有 `String/bool/&str` 当业务参数?invariant 是被构造器保证还是注释提醒?enum 加 variant 后所有 match 是否被强制穷尽(无 `_` 通配)?跨模块依赖是否走 trait?详见 [L0: 建模优先 + 强类型](#l0-建模优先--强类型--防火墙模型)

---

### L0: Task-Scoped Commits — Phase 边界 + Task-Id Trailer

**触发时机**:`/harness-04-execute` 期间,plan 里每个 Phase 完成(所有 Scenario Green + 5 项 gate pass)后,**进入下一个 Phase 之前必须 commit**。

**动机**:让每个 task 的 commit 可机械过滤(`/harness-05-close` 自动收集 commits),避免手工挑选和漏挑。同时让 git 历史上每个 commit 都能回溯到 task 的 design / spec / review 全景。

**Commit message 格式**(Git trailer 约定):

```
{type}({scope}): {phase 做的事一句话}

{可选 body,2-3 行说明做了什么}

Task-Id: task_{hex8}
Phase: {plan 里的 Phase 编号或名称}
```

- **Subject** —— conventional commit 规范(`feat` / `fix` / `refactor` / `test` / `docs` / `chore`),scope 通常是模块名(`kanban` / `keysight` / `app_error` 等)
- **Task-Id trailer** —— 必须严格格式 `Task-Id: task_{hex8}`,不能用 `TaskId:` / `task-id:` / 其他变体。`/harness-05-close` 用 `git log --grep="^Task-Id: task_xxx$"` 精确过滤
- **Phase trailer** —— 可选但推荐,帮助 code archaeology

**示例**:

```
feat(kanban): TaskEditModal + ChecklistEditor

Implement double-click open flow with controlled ChecklistEditor component.
Integrates with domain::task::update_with_subtasks for server-side body rewrite.

Task-Id: task_275a92d6
Phase: 6.5 — TaskEditModal + ChecklistEditor
```

**硬约束**:

1. **选择性 staging** —— 禁用 `git add -A` / `git add .`,逐文件 add 避免误 commit 敏感文件或不相关改动
2. **5 项 gate 必须在 commit 之前全过** —— clippy / cargo test / pnpm test / pnpm build / bindings 最新
3. **pre-commit hook 失败 → 修根因**,不 `--no-verify` 绕过(除非用户明确授权)
4. **每次 commit 由 agent draft message + 用户确认后执行**,不自动 commit —— 敏感操作的决定权必须在用户

**例外**(不需要 Task-Id trailer):

- 迁移 / 基础设施 / 纯文档改动(不在 `/harness-04-execute` 期间)
- ad-hoc bug fix(不走 v4 task workflow)
- Merge commit / Revert commit(revert 的 message 里会保留原 commit 的 Task-Id,间接可查)

**Task rules 冗余提醒**:`/harness-02-spec` 给每个 task 的 `rules:` 字段**自动插入**一条提醒"每个 Phase 边界必须 commit,message 含 `Task-Id: {task_id}` trailer",让执行阶段有 in-context 强化。这是"in-context 重复 > 单一来源"的故意冗余。

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

### L0: 建模优先 + 强类型 — 防火墙模型

**核心理念**：一切都在模型中。模型未定义的行为默认非法。

> **"想出错都难"原则**：如果一段代码可能出错，则它一定会出错。我们把所有可能出错的地方在编译期 close 掉，让非法状态根本无法被构造，而不是依赖运行时检查或注释提醒。

**防火墙思路**：每个 pub fn 的签名是一道防火墙，只接受显式建模的输入。**Default-deny** —— 没在类型里允许的就是禁止的。要新增允许的组合，只能改类型本身（编译器会逼你回到所有读写端补 case，没有任何静默路径）。

**Rust 建模 = 六件套**：`struct + enum + trait + impl + module + lifetime` 合起来用，**不是只 struct**。Java OO 把"数据 + 行为"塞进 class，Rust 把"数据形状 / 类型集合 / 行为契约 / 实现 / 可见性 / 借用"分成六个正交工具。其中 **enum 判别联合 + trait 行为契约 + type state 状态机** 是 Rust 比 Java OO 显著强的部分。

#### 反模式杜绝清单

**A. Primitive obsession — 裸基础类型承载业务语义**

| ❌ 反模式 | ✅ 替代 |
|---|---|
| `fn delete_card(id: &str)` | `fn delete_card(id: CardId)` — newtype value object |
| `status: String` | `status: enum CardStatus { Draft, Reviewed, ... }` |
| `kind: String` 当判别 | 判别联合 enum |
| `fn foo(force: bool, dry_run: bool)` | enum 参数（消除 boolean blindness） |
| `price_cents: i64` 满天飞 | `Price` newtype，构造时校验非负 |
| `HashMap<String, Value>` 业务存储 | typed struct |

**B. 逃生舱口与通配符**

| ❌ 反模式 | ✅ 替代 |
|---|---|
| `match foo { _ => ... }` 吞未来 variant | 列穷所有 variant（编译器强制） |
| `Option<Option<T>>` | 重新建模为单层 enum / struct |
| `as` 数值转换可能截断 | `try_from` |
| `.unwrap()` / `.expect()` 在业务路径 | `?` 传播 + 边界 match |
| `let _ = result_returning_fn()` 吞 Result | 显式 handle 或 propagate |
| 业务 crate 用 `anyhow::Error` | thiserror enum per module |
| `String` 当 error type | typed error variant |
| 调用方解析 error message 字符串做控制流 | match enum variant |

**C. 贫血模型 + 错位封装**

| ❌ 反模式 | ✅ 替代 |
|---|---|
| pub 字段允许外部直改 | 私有字段 + 校验构造器 (`fn new(...) -> Result<Self, E>`) |
| 业务规则散落在 handler/service | 方法挂在聚合根 / domain entity 上 |
| 派生 `Default` 的有 invariant 类型 | 不派生 Default（Default 通常违反 invariant） |
| `pub` 暴露内部数据结构 | `pub(super)` / `pub(in crate::module)` |
| getter 返回 `Vec<T>` | 返回 `&[T]`（避免外部突变） |

**D. ID / 边界数据未校验**

| ❌ 反模式 | ✅ 替代 |
|---|---|
| DB / IPC 拿 string 直接当 id 用 | 边界一次性 `EntityId::parse(&str)`，之后类型保证 |
| 假设 prefix 永远对 / format 永远对 | parse 时校验，parse 之后免检（"parse don't validate"） |
| 跨模块用 string 互相引用实体 | 跨模块用强类型 newtype id |

**E. Stringly-typed dispatch**

| ❌ 反模式 | ✅ 替代 |
|---|---|
| `connect(from: &str, to: &str, kind: EdgeType)` —— 三个参数全是逃生舱口，编译器无法保证一致性 | `connect(edge: Edge)` 接判别联合，每个变体写死 (from_kind, to_kind) 合法组合 |
| 跨模块依赖直接 import `RealFs`/`RealHttp` | trait + impl，依赖注入 |
| 测试需要复制业务逻辑才能写 | 抽到纯函数 + 接口边界处 mock |

**F. 状态机未编码进类型**

| ❌ 反模式 | ✅ 替代 |
|---|---|
| 字段记 `status: enum` + 方法内 if-check | type state pattern：`Order<Draft>` vs `Order<Submitted>`，编译器禁止 Submitted 调 publish |
| 不区分"已校验" / "未校验"数据 | `RawInput` vs `ValidatedInput` 两个类型 |

#### 务实例外（必须有 `// 例外:` 注释）

下面这些被一般规则禁但本项目接受 —— **凡列入例外的代码旁必须有简短 `// 例外:` 注释说明理由**：

- `state.db.lock().unwrap()` —— Mutex poisoning 不可恢复，unwrap 是 Rust 社区惯例
- `RealVaultFs::new(...)` 在 commands.rs 直接构造 —— 避免 trait 泛型在 IPC 边界扩散
- 一些 `let _ =` —— 仅当返回值完全无业务意义（如 drop guard）
- specta/serde 派生需要的 `Default` —— 仅当被派生类型本身就是"零值合法"

凡没注释的，审视时一律当违规。

#### 触发时机 + 自审 prompt

| 时机 | 自审问题 |
|---|---|
| 新增 pub fn | 签名里有 `&str`/`String`/`bool`/`u32` 当业务参数？拿掉是否仍能完整表达接口？ |
| 新增 pub struct | pub 字段有几个？invariant 是构造器保证还是注释提醒？ |
| 新增 trait | 拿掉所有 String/bool 后还剩什么？是否退化成函数指针集合？ |
| 新模块 bootstrap | **type-first**：先写 type 直到"凭 signature 写不出 illegal 调用"再写 impl |
| Code Review | 6 项检查里的第 6 项（建模强度） |
| Session 结束 | 当天有新增 pub fn → 跑 `/harness-type-safety-check` |

#### Make Illegal States Unrepresentable — 七个具体技术

按使用频率排序，详见 `rust-modeling` / `rust-types` skills：

1. **Newtype value object** — 每个有语义的基础值都包成 struct，构造器校验，之后免检
2. **判别联合作 dispatch 入口** — 所有"按 kind 派发"的逻辑用 enum，编译器强制穷尽
3. **Type state pattern** — 状态写入类型参数（`Order<Draft>` vs `Order<Submitted>`），方法只挂在合法状态的 impl block 上 —— Java OO 没有的能力
4. **聚合根** — 对外只暴露根的方法，内部成员 `pub(super)` / `pub(in crate::xxx)`
5. **Parse don't validate** — 边界一次性 parse 进强类型，之后**永远不再校验**
6. **Errors as types** — thiserror enum + From trait 链路传播；调用方 match variant 不解析 message 字符串
7. **Traits for boundaries** — 所有外部依赖（fs / http / db / 跨模块）都是 trait

#### 与 Trait-First 的关系

Trait-First 是这条原则的**特例应用**：把"用 trait 表达行为契约"具体化到 Rust 实践层面。建模优先是更高层的 umbrella 原则 —— 不仅 trait，所有 type 都该承载语义。

#### 踩坑样例 1 — keysight Note↔Note 静默失败（2026-04-13）

**症状**：从 Note ⋯ 菜单 Draw connection 到另一个 Note，UI 永远画不出线。

**根因链**：
1. `EntityGraph::connect(from: &str, to: &str, edge_type: EdgeType, ...)` —— 三个 stringly typed 逃生舱口，trait 退化成函数指针集合
2. 前端硬编码 `edge_type: "LinkTo"`
3. Rust 写入 `(note_a, note_b, 'link_to')`
4. `note.get` 按 `edge_type = 'note_link'` 读取 → 读不回
5. UI 不渲染 → DB 留 orphan edge

**类型系统本可阻止**：
- 如果 `from_id` 是 `EntityId` 判别联合，传 `(EntityId::Note, EdgeType::LinkTo)` 给 `connect()` 接 `Edge` 判别联合时根本不存在该变体 → 编译期失败
- 如果 `connect()` 接 `Edge::NoteLink { from: NoteId, to: NoteLinkTarget }`，前端只能传 `NoteLinkTarget::{Card,Note,Section,Alias}`，永远没机会传 `LinkTo`

**教训**：trait 接受 stringly typed 参数，本质上把 trait 退化成函数指针集合，类型系统形同虚设。这就是 L0 建模优先存在的原因。

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
| `card_set_color` (B2) | `entities.color` + markdown 文件 | DB 写 + frontmatter `color` 更新/删除 | domain unit test |
| `section_create` | `entities` | DB 写 | domain unit test |
| `section_delete` | `entities`, `section_members`, `edges`, `positions` | 级联删除 | domain unit test |
| `section_update` | `entities` | DB 写 | domain unit test |
| `section_add_member` | `section_members` | DB 写 | domain unit test |
| `section_remove_member` | `section_members` | DB 写 | domain unit test |
| `section_move_to_whiteboard` | `entities`, `positions`, `edges` | 跨白板迁移 + 清理 | domain unit test |
| `note_create` | `entities` | DB 写 | domain unit test |
| `note_delete` | `entities`, `edges`, `positions` | 级联删除 | domain unit test |
| `note_update` | `entities` | DB 写 | domain unit test |
| `question_create` | `entities`, `question_fields` + markdown 文件 | DB 写 + 文件写(含 color 支持) | domain unit test |
| `question_update` | `entities`, `question_fields` + markdown 文件 | DB 写 + 文件重写 + 可能 rename;`color` 参数 `"default"` sentinel 清空 | domain unit test |
| `question_delete` | `entities`, `question_fields`, `edges`, `positions` + markdown 文件 | 级联删除 | domain unit test |
| `task_create` (B2) | `entities`, `task_fields`, `file_mtimes`, `entities_fts`, **`positions`** + `whiteboard/projects/{project}/{id} 【TASK】{title}.md` | DB 写 + 文件写(含 color/area/project) + auto-position 写 positions 行(x=0, y=最底元素下方);路径由 `ProjectName` 决定 | domain unit test |
| `task_update` (B2) | `entities`, `task_fields`, `file_mtimes`, `entities_fts` + markdown 文件 | DB 写 + 文件重写 + title 改时 rename;不允许改 project;color `"default"` sentinel 清空 | domain unit test |
| `task_delete` (B2) | `entities`, `task_fields`, `edges`, `positions`, `file_mtimes`, `entities_fts` + markdown 文件 | 级联删除(domain::task::delete 经 sync::remove_file) | domain unit test |
| `task_set_color` (B2) | 等价于 `task_update` 只改 color | 同上 | 复用 task_update 测试 |
| `task_update_with_subtasks` (V1.1) | 等价 `task_update(content)` + 可能 title/status/area/color | Rust 从 `current.content` 作 merge base,用 `render_subtasks_into_body` 把新 subtasks 按 line_index 替换原 checklist 行(保留非 checklist 文本);多 block 时 fail-closed 返 `AppError::MultiBlockChecklist { task_id, block_count }` | domain unit test(render 9 + 端到端 4) |
| `alias_create` | `entities`, `alias_fields` | DB 写 | domain unit test |
| `alias_delete` | `entities`, `alias_fields`, `edges` | 级联删除 | domain unit test |
| `layout_set_position` | `positions` | DB 写 | domain unit test |
| `layout_remove_position` | `positions` | DB 写 | domain unit test |
| `entity_connect` | `edges` | DB 写 | domain unit test |
| `entity_disconnect` | `edges` | DB 写 | domain unit test |
| `sync_file` | `entities`, `card_fields`, `entity_tags`, `edges`, `entities_fts` | 全量同步 | domain unit test |
| `sync_remove_file` | `entities`, `entities_fts` | 按文件删除 | domain unit test |
| `import_legacy_db` | `entities`, `card_fields`, `alias_fields`, `entity_tags`, `edges`, `positions`, `section_members`, `entities_fts` + 旧 DB 文件备份 | DB 批量写 + 文件 copy | domain unit test |
| `sync_vault` | `entities`, `card_fields`, `task_fields`, `question_fields`, `entity_tags`, `edges`, `entities_fts`, `file_mtimes`, `positions`, `section_members` + vault md 文件 | 全量 DB 同步 + 孤儿清理 + 文件 ID 回写 | domain unit test |

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

### 质量流程(agent 必须遵循的流程规则)

由于不使用 Claude Code hooks,以下质量检查作为 agent 必须遵循的**流程规则**执行。

#### 编码中

- **每次 `cargo build`**:改为 `cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings`(lint 融入开发,错误自然暴露)
- **修改 command 签名后**:立即 `cargo test export_bindings --manifest-path src-tauri/Cargo.toml` 重新生成 bindings.ts
- **修改 TS 代码后**:`pnpm test -- --run`(组件测试) + `pnpm build`(tsc 类型检查 + vite 构建)

#### 提交前(5 项 gate)

```
1. cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings    ← Rust lint
2. cargo test --workspace --manifest-path src-tauri/Cargo.toml                     ← Rust 测试(unit + 集成)
3. pnpm test -- --run                                                              ← TS 组件测试
4. pnpm build                                                                      ← TS 类型检查 + 构建
5. 确认 bindings.ts 最新(cargo test export_bindings 后 git status 看 src/bindings.ts)
```

五项全部通过才可 commit。任何一项失败必须先修复根因,禁止 `--no-verify` 绕过。

#### Task Close 阶段(`/harness-05-close` 内部按此顺序跑)

```
1. /harness-check-tests        ← spec Completion Criteria 完整性自查
2. agent-spec lifecycle        ← BDD 契约机械验证(如 task 有 spec.md)
3. /harness-type-safety-check  ← L0 防火墙 / 类型安全自查
4. Task Code Review(6 项)     ← Contract Acceptance / 逻辑 / 回归 / I/O / IPC / 建模
5. 手动 walkthrough            ← 如有 UI 改动,dev server 跑一遍
6. 自动 grep commits(`git log --grep="^Task-Id: {id}$"`)+ 提炼 lessons + 提取 mempal triples
7. status → done
8. git commit(用户决定 message)
```

---

## 开发流程

### 新 Session 开始

```
1. /harness-resume-context → 读 task 级 handoff (vault/docs/tasks/{id}/handoff.md) 或区级 handoff (vault/docs/handoff/{area}.md)
2. /harness-view {task_id} 或读 task.md 了解当前 task 全景(含 frontmatter 的 rules / skills / lessons in-context 强化)
3. 读 vault/docs/devlog/ 最新一篇 → 背景时间线
4. 如要修改某模块 → 先读该模块 LESSONS(模块级 `src-tauri/src/modules/{name}/LESSONS.md` 或 `vault/docs/lessons/modules/{name}.md`)
5. 默念 L0 防火墙原则 — 写任何 pub fn 前先问"这签名能不能写出 illegal 调用?"
```

### 开发中(无 active task,要建新 task)

```
1. /harness-01-design 对话式推敲架构 → design-v1-cc.md
2. (可选)/harness-review 请 codex 做独立 review → r1-codex.md
3. 根据 review 出 v2 → design-v2-cc.md(status: final)
4. /harness-02-spec 提炼为 agent-spec DSL contract → spec.md(非机械型 task 必须)
5. (可选)/harness-03-plan 拆为实施 phases
6. /harness-04-execute 开始 TDD 实施(status: active)
```

### 开发中(有 active task)

```
1. 读 task.md frontmatter 的 rules: / skills: / lessons: 字段 — in-context 强化
2. 编译用 clippy 不用 build
3. 新增 pub fn / pub struct / trait → 自审签名是否有 String/bool/&str 当业务参数;有就回去包 newtype/enum(务实例外要加 // 例外: 注释)
4. TDD Red → 用户确认 → Green → 用户确认 → Refactor(不跳步,不合并关卡)
5. 所有 Scenario 实施完成 → 跑 5 项提交前 gate → /harness-05-close
```

### Session 结束

```
1. 有 active task 未完成 → /harness-save-next-context → vault/docs/tasks/{id}/handoff.md (task 级)
2. 无 active task 但讨论集中某功能域 → /harness-save-next-context → vault/docs/handoff/{area}.md (legacy 区级)
3. 如果今天加过 pub fn / pub struct / trait → 跑 /harness-type-safety-check
4. 追加 devlog 到 vault/docs/devlog/{today}.md
5. 如有踩坑 → 写 LESSONS.md(模块级 vault/docs/lessons/modules/{name}.md) + 路由到记忆体系对应层级
```

---

## 新模块 Bootstrap 清单

**v4 触发时机**:当 task 在 `/harness-02-spec` 阶段决定"需要新建一个 Rust 业务模块"时,spec.md 应把本清单的 step 3-17 映射成 phase plan,然后在 `/harness-04-execute` 阶段按清单执行。Step 1-2(type sketch)通常在 `/harness-01-design` 阶段完成,属于 design v{N} 的一部分。

新增业务模块时,按以下顺序执行:

```
1. 创建模块目录 src-tauri/src/modules/{name}/

2. Type Sketch — 在写任何 impl 之前列出本模块的类型骨架（防火墙）
   a. 列 newtype id（CardId / NoteId / ...）+ 状态 enum + 度量值
   b. 列 trait 行为契约的方法签名（无 impl）
      - 跨模块依赖（fs / http / db）也用 trait 表达，便于测试 mock
   c. 写一段 //! 文档注释或 fake usage 函数，演示典型 happy path 流程
      - 检验：能否凭这些 type 自然表达 happy path？
      - 检验：能否凭这些 type 写出 illegal path？写得出 → 类型不够紧
   d. 反复 a/b/c 直到「凭 signature 写不出 illegal 调用，也不缺合法表达」
   e. 然后才进入 step 3（写 models.rs）；允许在 impl 期间回 step 2 微调 1-3 次

3. 写 models.rs — 把 step 2 的 type sketch 落地为 derive Serialize + Deserialize + specta::Type 的 struct/enum
4. 写 errors.rs — thiserror + impl Into<AppError>
5. 写 db.rs — 建表 migration
6. 定义 domain traits — 把 step 2.b 的方法签名落地到 trait（Trait-First）
7. 写 domain 测试 — 针对 trait 的期望行为（TDD: Red）
8. 写 domain.rs — 业务纯函数实现 trait（TDD: Green → Refactor）
9. 写 commands.rs — 薄壳 + #[tauri::command] + #[specta::specta]
10. 写 mod.rs — pub use commands + models
11. 在 modules/mod.rs 注册
12. 在 lib.rs 的 collect_commands![] 添加
13. cargo test export_bindings → 验证 bindings.ts 更新
14. 在副作用矩阵中登记写操作
15. 如涉及新 plugin → 更新 capabilities/default.json（最小权限）
16. 写 TS 组件时 → 配套组件测试（渲染 + 事件接线）
17. 跑 `/harness-type-safety-check` → 对照 L0 防火墙清单做最后自查
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

# 重新生成 TS 类型绑定(修改 command 签名后必跑)
cargo test export_bindings --manifest-path src-tauri/Cargo.toml

# Rust lint(编码中用这个代替 cargo build)
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings

# Rust 测试
cargo test --workspace --manifest-path src-tauri/Cargo.toml

# TS 组件测试
pnpm test -- --run

# TS 类型检查 + 构建
pnpm build  # tsc && vite build
```

## Agent-Spec 命令(v4 task spec 验证)

```bash
# 渲染 task contract 给 agent 看
agent-spec contract vault/docs/tasks/{task_id}/spec.md

# 生成 plan(含 Codebase Context + Task Sketch)
agent-spec plan vault/docs/tasks/{task_id}/spec.md --code . --format markdown

# 跑完整 lifecycle(lint + verify + report) — /harness-05-close 内部用
agent-spec lifecycle vault/docs/tasks/{task_id}/spec.md --code . --change-scope worktree --format json

# lint 单个 spec
agent-spec lint vault/docs/tasks/{task_id}/spec.md --min-score 0.7

# 生成 reviewer-friendly 的 Contract Acceptance 总结
agent-spec explain vault/docs/tasks/{task_id}/spec.md --code . --format markdown
```

`project.spec.md` 的发现是**自动的**:只要 task spec 有 `inherits: project`,resolver 就会从 task spec 目录向上 walk 找到 `vault/docs/project.spec.md`。不需要 `--spec-dir` 参数。
