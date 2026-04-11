---
area: keysight
last_updated: 2026-04-12T02:30:00+08:00
session_id: ca953e61
status: ready-to-resume
stale_check: cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd ..
---

# Handoff: keysight

## 正在做的 Task

准备进入 Phase 4: 文件监听 — 对应 `docs/progress/keysight.md` > Next > "Phase 4"

## 已完成步骤

- [x] Phase 3 spec — brainstorm + 数据审计 + 设计 (`3eb6bde`)
- [x] Phase 3 plan — 7 tasks, 完整代码 (`70a8e82`)
- [x] Phase 3 实现 — subagent-driven, 7 tasks 全部完成 (`82916a9`→`952c325`)
- [x] test fixtures 重命名 `"atomic cards/"` → `"whiteboard/"` (`bd91789`)
- [x] 真实旧 DB 导入验证 — 145 cards, 0 skipped, 1.3 MB 写入 app_data_dir (`c44895e`)
- [x] 补 5 个覆盖缺口测试 (`7c23918`)
- [x] Code Review 5/5 通过, Phase 3 标记 Done (`f52fc63`)

## 下一步具体动作

1. **brainstorm Phase 4 需求** — 调用 `superpowers:brainstorming` skill，明确 scope：文件监听机制（哪个 crate？notify / tauri-plugin-fs watch？）、FileEventSource trait 设计、与 sync_file 的集成、增量 vs 全量扫描策略
2. **理解现有 sync 管道** — 读 `src-tauri/src/modules/keysight/domain/sync.rs` 的 `sync_file` 和 `all_file_mtimes` 函数，它们是文件监听后要调用的下游逻辑
3. **确认 vault 目录结构** — 读 `KEYSIGHT_VAULT_PATH` 指向的 vault，确认 `whiteboard/` 子目录布局，理解哪些文件需要监听
4. **设计 FileEventSource trait** — 定义文件变更事件的抽象（create/modify/delete），与 sync_file/remove_file 对接
5. **写 Phase 4 spec** — 调用 `superpowers:writing-plans` skill
6. **实施 Phase 4** — TDD + Trait-First，按质量机制执行

## 关键上下文（/new 之后会丢的东西）

### 本次会话的假设与决策

- **app_data_dir 已迁移** — keysight.db 不再用相对路径，位于 `~/Library/Application Support/com.super-tauri.app/keysight.db`。`KeysightState.db_path` 存储绝对路径
- **旧 DB 数据已导入到 app_data_dir** — 145 cards + 33 sections + 80 notes + 125 aliases + 518 edges + 378 positions，零 skipped
- **import_legacy_db command 保留** — 虽然一次性导入已通过测试跑完，command 仍可用于从 UI 触发重跑（幂等）
- **test fixtures 全部改为 `whiteboard/`** — `"atomic cards/"` 已不存在于代码中
- **specta 不支持 `skip_serializing_if`** — 仍然有效，Phase 2 的决策
- **todo 是 demo 不是产品** — keysight 架构决策独立于 todo

### Phase 4 质量约束（CLAUDE.md 五层机制）

**L0 TDD — Red-Green-Refactor 不可省略：**
- 先写失败测试 → 🔴 用户确认 Red → 写实现 → 🟢 用户确认 Green → Refactor
- subagent 模式下 Red/Green 由 subagent 内部自验证即可

**L0 Trait-First — 先定义行为契约再写实现：**
- FileEventSource trait — 文件变更事件源的行为契约
- 先定义 trait → 写测试 → 写 impl

**L0 Operation Contract — 有副作用的 pub fn 需要完整契约**

**L0 Anti-Test-Theater — 测试调用真实代码路径**

**L0 功能域完成 Code Review — Phase 4 Active tasks 全部完成时触发 5 项检查**

### 试过但不行的方案

无（Phase 4 还未开始）

### 开放问题

无

## Resume 检查清单

- [ ] 读 `docs/progress/keysight.md` 确认 Phase 4 在 Next 区
- [ ] 跑 `stale_check`：应显示 `145 passed` + `1 ignored` + `test result: ok`
- [ ] `git status` 干净
- [ ] 确认 Phase 3 在 Done 区
- [ ] 确认 `~/Library/Application Support/com.super-tauri.app/keysight.db` 存在（1.3 MB）
