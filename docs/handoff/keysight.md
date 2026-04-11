---
area: keysight
last_updated: 2026-04-12T04:50:00+08:00
session_id: 4c993118
status: ready-to-resume
stale_check: cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd .. && pnpm test 2>&1 | tail -3
---

# Handoff: keysight

## 正在做的 Task

准备执行 Phase 5b 实体渲染 — plan 已写好，待 subagent-driven 执行 8 个 tasks

## 已完成步骤

### Phase 4: Vault 同步管道（本 session 完成）

- [x] VaultFs trait 扩展 `list_md_files` — `src-tauri/src/modules/keysight/vault_fs.rs` (`add8539`)
- [x] SyncVaultReport 类型 — `src-tauri/src/modules/keysight/models.rs` (`7d973a2`)
- [x] insert_id_into_frontmatter 纯函数 TDD — `src-tauri/src/modules/keysight/domain/sync.rs` (`7f338e9`)
- [x] sync_vault domain 函数 TDD (8 tests) — `domain/sync.rs` (`5311723`)
- [x] sync_vault command + startup_sync + bindings — `commands.rs`, `mod.rs`, `lib.rs` (`6c878b7`)
- [x] 孤儿清理范围修复（whiteboard/ prefix） — `domain/sync.rs:321` (`6e05603`)
- [x] 副作用矩阵更新 — `CLAUDE.md` (`7125c0f`)
- [x] Phase 4 Code Review passed, 标记 Done — (`49908fa`)

### Phase 5a: 画布基础设施（本 session 完成）

- [x] TS 测试基础设施（vitest + RTL + jsdom） — `vitest.config.ts`, `src/test/setup.ts` (`91c708c`)
- [x] useViewport hook TDD (6 tests) — `src/components/keysight/useViewport.ts` (`01596de`)
- [x] GraphCanvas 组件 TDD (3 tests) — `src/components/keysight/GraphCanvas.tsx` (`e04f045`)
- [x] GraphView + /keysight 路由 + 侧边栏 — `src/App.tsx`, `src/components/AppShell.tsx` (`ed029de`)
- [x] 手动验证 pan/zoom 全部通过 — (`d592f47`)
- [x] Phase 5a 标记 Done — (`2b30724`)

### Phase 5b: 准备工作（本 session 完成）

- [x] 旧项目完整功能调研 — `~/codes/vibe-coding/obsidian-plugin-keysight` 全量扫描
- [x] progress 功能全景更新 — `docs/progress/keysight.md` 旧项目映射表
- [x] Phase 5b brainstorm — 视觉风格选定 Clean Elevated (B)，TanStack Query，全部 6 种实体
- [x] Phase 5b spec — `docs/superpowers/specs/2026-04-12-keysight-phase5b-entity-rendering.md` (`e0f9846`)
- [x] Phase 5b plan — `docs/superpowers/plans/2026-04-12-keysight-phase5b-entity-rendering.md` (2670 行, `679b10f`)

## 下一步具体动作

1. **执行 Phase 5b plan** — 用 `superpowers:subagent-driven-development` skill，从 Task 1 开始（Rust: Task/Question 查询 commands TDD）
2. **Task 1 具体**: 在 `src-tauri/src/modules/keysight/models.rs` 新增 `TaskEntity` + `QuestionEntity` structs，在 `domain/task.rs` 和 `domain/question.rs` 新增 `query_all(conn, whiteboard_id)` 函数，在 `commands.rs` 新增 `task_query_all` + `question_query_all` commands
3. **Task 2 具体**: `pnpm add @tanstack/react-query`，在 `src/main.tsx` 包裹 `QueryClientProvider`，创建 `src/lib/unwrap-command.ts` 工具函数处理 `typedError` wrapper
4. **Task 3 具体**: 重构 `src/components/keysight/GraphCanvas.tsx` — viewport 从 props 注入而非内部创建，导出 `UseViewportReturn` type，创建 `useContainerSize` hook
5. **后续 Tasks 4-8**: 见 plan 文件完整步骤

## 关键上下文（/new 之后会丢的东西）

### 本次会话的假设与决策

- **旧项目路径**: `~/codes/vibe-coding/obsidian-plugin-keysight`（Obsidian 插件 + keysight-core Rust sidecar）
- **视觉风格选定 Clean Elevated (B)**: 白底卡片 + 精致投影 + 渐变图标 + pill 标签，Notion/Linear 风格。用户明确表示对旧项目视觉不满意
- **全部 6 种实体类型**: 用户选了 C（全部），不要让用户取舍子集——最终需求就是全部
- **TanStack Query**: 用户主动要求用 TQ 管理数据获取，和视口裁剪不冲突（TQ 管数据获取层，viewport culling 管渲染层）
- **视口裁剪默认包含**: 用户要求即使当前量级不需要也要默认加入。简单 bounds 检查 O(N)，设计面向 1000+ cards per whiteboard
- **Toolbar 完整复刻**: 用户说"最终需求肯定是 C"，不要让用户做子集选择产生焦虑
- **cardQueryAll 只返回 kind='card'**: Task/Question 在 DB 中但没有查询 commands，Phase 5b plan Task 1 先补
- **typedError wrapper**: bindings.ts 所有 commands 返回 `{ status: "ok", data } | { status: "error", error }`，TQ queryFn 需要 unwrap
- **Phase 5 拆分为 6 个 sub-phases**: 5a(画布) → 5b(实体渲染) → 5c(交互) → 5d(Edge) → 5e(多白板) → 5f(性能)
- **KEYSIGHT_VAULT_PATH 环境变量**: 启动 `pnpm tauri dev` 需要设置，如 `KEYSIGHT_VAULT_PATH=~/Documents/obsidian_workspace/agent-slipbox-v3 pnpm tauri dev`
- **TanStack Query skills**: 用户要求实现时调用本地 TanStack Query 相关 skills 确保用法正确
- **app_data_dir 已迁移**: keysight.db 在 `~/Library/Application Support/com.super-tauri.app/keysight.db`

### 试过但不行的方案

无

### 开放问题

无

## Resume 检查清单

- [ ] 读 `docs/progress/keysight.md` 确认 Phase 5a 在 Done，Phase 5b 在 Next
- [ ] 跑 `stale_check`：Rust 应显示 `138 passed, 1 ignored`；TS 应显示 `9 passed`
- [ ] `git status` 干净
- [ ] 确认 plan 文件存在：`docs/superpowers/plans/2026-04-12-keysight-phase5b-entity-rendering.md`
- [ ] 确认 spec 文件存在：`docs/superpowers/specs/2026-04-12-keysight-phase5b-entity-rendering.md`
- [ ] `pnpm tauri dev` 可启动（需设 KEYSIGHT_VAULT_PATH），GraphView 画布 + pan/zoom 正常
