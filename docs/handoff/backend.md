---
area: backend
last_updated: 2026-04-11T23:50:00+08:00
session_id: a6043f40
status: ready-to-resume
stale_check: cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -1
---

# Handoff: backend

## 正在做的 Task

构建 TodoMVC 应用验证架构设计 — 对应 `docs/progress/backend.md` > Next > "定义领域模型"

## 已完成步骤（本 task 内部）

- [x] 项目脚手架：Tauri + React + Vite — `package.json`, `src-tauri/Cargo.toml`
- [x] tauri-specta 集成 — `src-tauri/src/lib.rs` 配置 Builder + export test
- [x] bindings.ts 生成验证 — `src/bindings.ts` 自动生成，App.tsx 改用 `commands.greet()`
- [x] 四关注点项目管理体系落地 — `docs/` 结构 + `CLAUDE.md` 完整质量体系（17/17 机制）
- [x] TodoMVC 设计讨论完成 — rusqlite 方案确定，Deep Module 模块结构确定，7 个 command 契约确定
- [ ] **待开始**：TodoMVC spec 文档编写 + 实现

## 下一步具体动作

1. 编写 TodoMVC spec — 写入 `docs/superpowers/specs/2026-04-11-todomvc-design.md`，包含数据模型、模块结构、command 契约、TS 组件、测试策略、验证清单（设计已在对话中确认，需要落地为文件）
2. git init + 初始提交 — 项目目前还没有 git，需要 `git init` 并做第一个 commit 包含全部 bootstrap 内容
3. 创建 Rust 模块结构 — `mkdir -p src-tauri/src/modules/todo/`，按 Deep Module 规则创建 `mod.rs`, `models.rs`, `errors.rs`, `db.rs`, `domain.rs`, `commands.rs`
4. 实现 SQLite 初始化 — `db.rs` 建表 migration，`lib.rs` 注入 `State<Mutex<Connection>>`
5. 实现 7 个 domain 纯函数 + unit tests — `domain.rs` 内 `list_todos`, `create_todo`, `update_todo`, `toggle_todo`, `delete_todo`, `toggle_all`, `clear_completed`
6. 实现 7 个 command 薄壳 — `commands.rs`，每个 ≤3 行
7. 重新生成 bindings.ts — `cargo test export_bindings`，验证 7 个 typed command 出现在 bindings 中
8. 实现 React 组件 — `TodoHeader`, `TodoList`, `TodoItem`, `TodoFooter`，全部从 `bindings.ts` import

## 关键上下文（/new 之后会丢的东西）

### 本次会话的假设与决策

- **SQLite 方案选 rusqlite 而非 tauri-plugin-sql** — 因为 tauri-plugin-sql 把 SQL 暴露给 TS 侧，违背 "TS 不碰持久化" 原则。rusqlite 由 Rust 完全控制
- **tauri-specta v2.0.0-rc.24** — 2026-03-30 发布，confirmed via `gh api` + `cargo search`。配合 `specta v2.0.0-rc.24` + `specta-typescript v0.0.11`。Cargo.toml 用 `=2.0.0-rc.24` 锁定版本
- **bindings.ts 通过 test 生成而非 app 启动** — `cargo test export_bindings` 在 CI/CLI 无 GUI 环境也能跑。lib.rs 中 `make_builder()` 提取为独立函数供 test 和 run 共用
- **bindings.ts 加入 .gitignore** — 生成文件不提交，避免 merge 冲突
- **不用 hooks，用 CLAUDE.md 流程规则** — 用户明确要求取消 hooks，质量检查通过流程规则而非机械拦截
- **Deep Module 按 domain 拆模块** — `src-tauri/src/modules/{name}/` 结构，每个模块 mod.rs 暴露窄接口，内部 `pub(super)`
- **TodoMVC 7 个 command**: `list_todos(filter)`, `create_todo(title)`, `update_todo(id, title)`, `toggle_todo(id)`, `delete_todo(id)`, `toggle_all(completed)`, `clear_completed()`
- **TodoFilter enum**: `All | Active | Completed`
- **Todo struct**: `id: i64 (SQLite rowid), title: String, completed: bool, created_at: String (ISO 8601)`

### 试过但不行的方案

- **WebFetch 查版本号** — WebFetch 内置小模型会幻觉日期（把 2026 年的 release 报成 2025 年）。改用 `gh api` 和 `npm view` 获取精确版本数据
- **Explore agent 默认模型** — Explore agent 默认跑 Haiku（小模型），输出质量不够。需要显式指定 `model: "opus"`

### 开放问题

无

## Resume 检查清单

- [ ] 读 `docs/progress/backend.md` 确认 Active task 没变
- [ ] 读 CLAUDE.md 质量体系段落（17 个机制已完整落地）
- [ ] 跑 `stale_check` 命令，输出应为 `Finished` 且无 warning
- [ ] `ls src/bindings.ts` 确认 bindings 文件存在
- [ ] 项目可能还没有 git — 如果没有，第一步先 git init + 初始提交
