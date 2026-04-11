---
area: keysight
last_updated: 2026-04-11T19:30:00+08:00
session_id: 69f411ba
status: ready-to-resume
stale_check: cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd ..
---

# Handoff: keysight

## 正在做的 Task

Phase 1 补完 — 5 项遗漏 domain 功能的实现。Spec 和 Plan 已完成并提交，实现尚未开始。

对应 `docs/progress/keysight.md` > Done > "Phase 1" 的补完工作（Phase 1 基础已 Done，补完是追加）

## 已完成步骤

- [x] Phase 1 基础实现 — 10 个 domain 模块, 102 tests pass (`425cfdd`)
- [x] 质量体系升级 — TDD + Trait-First + TS 组件测试 写入 CLAUDE.md (`3dcdb8f`)
- [x] Phase 1 补完 spec — 5 项遗漏功能设计 (`17f1915`)
- [x] Phase 1 补完 plan — 9 tasks 实施计划 (`40cfd34`)
- [ ] **下一步**：用 subagent-driven 模式执行 plan 的 9 个 task

## 下一步具体动作

1. **调用 `superpowers:subagent-driven-development` skill** — 执行 `docs/superpowers/plans/2026-04-11-keysight-phase1-补完.md`
2. **Task 1: models.rs 新增类型** — 添加 `CardLinksResponse`, `WhiteboardOverview`, `CardSummary`, `GraphOverviewResponse` 到 `src-tauri/src/modules/keysight/models.rs`
3. **Task 2: db.rs FTS5 (TDD)** — `SCHEMA_V7_SQL` 追加 `CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts`，先写测试确认 🔴 再实现确认 🟢
4. **Task 3-8: 逐 task 执行** — 每个 TDD task 都有 🔴🟢 人工确认关卡，用户必须跑测试确认
5. **Task 9: 全量验证** — `cargo test --workspace`, `cargo clippy`, `pnpm build`

## 关键上下文（/new 之后会丢的东西）

### 本次会话的假设与决策

- **可见性方案 A 确定**：domain 模块 trait/struct 从 `pub(super)` 改为 `pub(in crate::modules::keysight)`，让 commands.rs 能直接访问。Phase 2 实现 commands.rs 时需要执行这个变更（plan 补完不涉及 commands.rs）
- **CardStore 读写统一**：不拆读写 trait。`SqliteCardStore` 改为持有 `conn + Option<vault_fs>`，`new()` 只读构造，`with_vault_fs()` 读写构造。现有只读测试用 `new()` 不受影响
- **FTS5 策略**：schema 加虚拟表，sync_file 里 DELETE+INSERT 同步（和 entity_tags 同模式），search 先 FTS MATCH 再 LIKE fallback
- **section_move_to_whiteboard 简化**：新架构下全是关系表操作，源项目的 8 个 meta blob 操作全部消失。跨白板 section_link 自动清理
- **query_card_links 只返回 id 列表**：源项目做了 id→title 解析，新架构让前端按需查

### 试过但不行的方案

- **Phase 2 直接写代码不设计** — 被用户制止，正确。任何 Phase 都必须走 brainstorm → spec → plan 流程
- **可见性方案 B (re-export)** — `pub(super)` 的 item 不能被 `pub(super) use` re-export，Rust 编译报错 E0365
- **可见性方案 C (facade 函数)** — 30 个 pass-through wrapper 违反 DRY，被否决

### 开放问题

无

## Resume 检查清单

- [ ] 读 `docs/progress/keysight.md` 确认 Phase 1 在 Done 区
- [ ] 读 `docs/superpowers/plans/2026-04-11-keysight-phase1-补完.md` 确认 plan 完整
- [ ] 跑 `stale_check`：`cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd ..` — 应显示 `84 passed` (keysight) + `test result: ok`
- [ ] `git status` 干净
- [ ] 确认 plan 中的 Task 1-9 均未开始（无对应 commit）
- [ ] 如果状态不匹配 — 不要盲目继续，先 ping 用户确认
