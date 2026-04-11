---
area: keysight
last_updated: 2026-04-11T22:14:00+08:00
session_id: 36640e1a
status: ready-to-resume
stale_check: cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd ..
---

# Handoff: keysight

## 正在做的 Task

准备进入 Phase 2: Tauri commands + specta 绑定 — 对应 `docs/progress/keysight.md` > Next > "Phase 2"

## 已完成步骤

- [x] Phase 1 基础实现 — 10 domain modules, 102 tests (`425cfdd`)
- [x] Phase 1 补完 spec + plan — 5 项遗漏功能设计 + 9 tasks plan (`17f1915`, `40cfd34`)
- [x] Phase 1 补完执行 — subagent-driven 9 tasks 全部完成 (`67d2f0d`→`314e567`)
  - models.rs: CardLinksResponse + GraphOverviewResponse 等 4 类型
  - db.rs: FTS5 虚拟表
  - sync.rs: sync_file/remove_file FTS 同步
  - card.rs: SqliteCardStore 持 vault_fs + edit_title/edit_body/update_understanding + search(FTS5+LIKE) + query_links
  - section.rs: move_to_whiteboard（位置清理 + 跨白板 link 清理）
  - overview.rs: graph_overview（按白板聚合 + 卡片摘要 + incoming link count）
- [x] 全量验证 — 122 workspace tests (104 keysight), clippy clean, pnpm build pass (`314e567`)

## 下一步具体动作

1. **brainstorm Phase 2 需求** — 调用 `superpowers:brainstorming` skill，明确 Phase 2 scope：哪些 domain 函数需要暴露为 Tauri commands，commands.rs 薄壳设计，State 注入方案（`Mutex<Connection>` + `RealVaultFs`）
2. **写 Phase 2 spec** — 调用 `superpowers:writing-plans` skill，输出到 `docs/superpowers/specs/` 目录。需要覆盖：command 签名列表、Operation Contract、State 结构、specta 注册、capabilities/permissions
3. **写 Phase 2 plan** — spec 确认后写实施计划到 `docs/superpowers/plans/`，按 CLAUDE.md 新模块 Bootstrap 清单的步骤 8-14
4. **实现 commands.rs** — `src-tauri/src/modules/keysight/commands.rs`，每个 command ≤3 行，调用 domain 纯函数
5. **注册到 lib.rs** — `modules/mod.rs` + `lib.rs` 的 `collect_commands![]` 添加 keysight commands
6. **重新生成 bindings.ts** — `cargo test export_bindings`，验证 typed commands 出现在 `src/bindings.ts`
7. **可见性变更** — domain 模块 trait/struct 从 `pub(super)` 改为 `pub(in crate::modules::keysight)`，让 commands.rs 能直接访问（方案 A，已在上一个 session 确定）

## 关键上下文（/new 之后会丢的东西）

### Phase 2 质量关卡（首次跨越 IPC 边界）

Phase 2 是 domain 层首次通过 commands.rs 暴露给 TS 前端，以下 CLAUDE.md 质量机制从此刻正式生效：

**L0 Operation Contract** — commands.rs 每个有副作用的 pub fn 需要完整契约（前置条件 / 执行效果 / 不做的事 / 幂等性 / 关联操作）。虽然 commands.rs 是薄壳（≤3 行），但它是 IPC 入口，契约写在这里让 TS 侧调用者知道行为边界。

**副作用矩阵** — CLAUDE.md 中的副作用矩阵目前是空的（"待 TodoMVC 实现后填充"）。Phase 2 暴露写操作（edit_title, edit_body, update_understanding, section move_to_whiteboard 等）时，**必须同步填充矩阵**，每个写 command 登记影响的表/资源、副作用、测试覆盖。

**IPC 类型安全检查** — 每个 command 必须 `#[tauri::command]` + `#[specta::specta]`，修改签名后立即 `cargo test export_bindings` 重新生成 bindings.ts，TS 侧只从 bindings.ts import（禁止 raw invoke）。

**Tauri 安全** — capabilities/default.json 需要为 keysight commands 添加最小权限。不得使用 `core:default`。

**功能域完成 Code Review** — Phase 2 Active tasks 全部完成时触发 5 项检查（测试覆盖 / 逻辑正确性 / 回归风险 / I/O 正确性 / IPC 类型安全），通过后才 commit + 标记 Done。

**提交前 5 步** — 每次 commit 前：`cargo clippy`, `cargo test --workspace`, `pnpm test`, `pnpm build`, 确认 bindings.ts 最新。

### Phase 2 TDD 原则

**Red-Green 循环不可省略** — 必须先写测试、确认红色（测试因正确原因失败），再写实现、确认绿色。这个过程本身是质量底线。

**人工确认关卡可灵活处理** — subagent 模式下，Red/Green 由 subagent 内部自验证即可（跑测试 → 确认失败/通过原因正确 → 继续）。不需要每次都停下来等用户确认。用户需要介入的场景：测试失败原因不明确、实现方案有多种选择时。

### 本次会话的假设与决策

- **subagent-driven 模式执行了 Phase 1 补完** — 9 tasks 全部通过 opus subagent 实现 + 自动 TDD Red→Green。subagent 内部完成了完整的 Red-Green 循环验证，人工确认关卡由 subagent 自验证替代，这是可接受的做法
- **subagent 有时不 commit** — Task 4 和 Task 5 的 subagent 都报告已 commit 但实际未 commit，需要手动补提交。这是已知行为，下次 dispatch 时强调 "verify commit with git log"
- **rust-analyzer 诊断滞后** — 每个 task 完成后 rust-analyzer 会报 stale 的 unused import / missing trait impl 错误，实际 clippy 和 cargo test 是 clean 的。可以忽略这些 IDE 诊断

### 试过但不行的方案

- **Phase 2 直接写代码不设计** — 上一个 session 被制止。Phase 2 也必须走 brainstorm → spec → plan 流程
- **可见性方案 B (re-export)** — `pub(super)` item 不能被 re-export，E0365
- **可见性方案 C (facade)** — 30 个 pass-through wrapper 违反 DRY

### 开放问题

无

## Resume 检查清单

- [ ] 读 `docs/progress/keysight.md` 确认 Phase 2 在 Next 区
- [ ] 跑 `stale_check`：`cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd ..` — 应显示 `104 passed` + `test result: ok`
- [ ] `git status` 干净（或只有 handoff 文件变更）
- [ ] 确认 Phase 1 补完已在 Done 区
