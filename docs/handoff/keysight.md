---
area: keysight
last_updated: 2026-04-12T00:30:00+08:00
session_id: 7621a80d
status: ready-to-resume
stale_check: cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd ..
---

# Handoff: keysight

## 正在做的 Task

准备进入 Phase 3: 一次性旧 DB 导入 — 对应 `docs/progress/keysight.md` > Next > "Phase 3"

## 已完成步骤

- [x] Phase 1 基础实现 — 10 domain modules, 102 tests (`425cfdd`)
- [x] Phase 1 补完 — 9 tasks via subagent (`67d2f0d`→`314e567`)
- [x] Phase 2 spec — brainstorm → design doc (`ff79875`)
- [x] Phase 2 plan — 5 tasks, full code (`d260b74`)
- [x] Phase 2 实现 — 40 Tauri commands + specta bindings (`412762f`)
  - `src-tauri/src/modules/keysight/state.rs`: KeysightState (Mutex<Connection> + vault_path)
  - `src-tauri/src/modules/keysight/commands.rs`: 40 thin shell commands (20 read + 20 write)
  - 可见性 `pub(super)` → `pub(in crate::modules::keysight)` across 10 domain files
  - `lib.rs`: KeysightState init (独立 keysight.db + KEYSIGHT_VAULT_PATH env var)
  - 20 write commands 有完整 Operation Contract doc comments
- [x] CLAUDE.md 副作用矩阵填充 (`0463b55`)
- [x] Phase 2 标记 Done (`7151ad7`)
- [x] 全量验证 — 122 workspace tests (104 keysight + 18 other), clippy clean, bindings.ts 603 lines, pnpm build pass

## 下一步具体动作

1. **brainstorm Phase 3 需求** — 调用 `superpowers:brainstorming` skill，明确 Phase 3 scope：旧 DB 格式分析、LegacyReader trait 设计、LegacyImporter 的导入策略（增量 vs 全量）、错误处理（跳过 vs 中断）
2. **分析旧 DB schema** — 找到旧 Obsidian 插件的 SQLite DB，理解表结构和数据格式，确定与新 schema 的映射关系
3. **写 Phase 3 spec** — 调用 `superpowers:writing-plans` skill，输出到 `docs/superpowers/specs/`。覆盖：LegacyReader/LegacyImporter trait 定义、数据映射规则、迁移步骤、回滚策略、测试策略
4. **写 Phase 3 plan** — spec 确认后写实施计划到 `docs/superpowers/plans/`
5. **实现 LegacyReader** — 读旧 DB 的 trait + SQLite 实现
6. **实现 LegacyImporter** — 转换 + 写入新 DB 的 trait + 实现
7. **暴露为 Tauri command** — import_legacy_db command（一次性操作）

## 关键上下文（/new 之后会丢的东西）

### 本次会话的假设与决策

- **KeysightState 独立 DB** — keysight 用 `keysight.db`，和 todo demo 的 `super_tauri.db` 分开。启动时读 `KEYSIGHT_VAULT_PATH` 环境变量，缺失则 panic
- **specta 兼容性修复** — subagent 移除了 models.rs 中所有 `skip_serializing_if = "Option::is_none"`，因为 specta unified mode 不支持条件序列化。改为 `#[serde(default)]` only
- **todo 是 demo 不是产品** — 用户明确指出 todo 不应纳入 keysight 的设计考虑。keysight 的架构决策独立于 todo
- **Phase 2 不新增测试** — commands.rs 是薄壳（≤4 行），验证靠 clippy + bindings 生成 + TS 编译。domain 已有 104 tests 覆盖业务逻辑
- **task/question 模块推迟到 Phase 6** — `by_status` 返回 `serde_json::Value`，specta 无法生成类型。等 UI 需求明确后再补 typed struct
- **验证报告要精确** — 用户指出"122 passed"在测试数量未变时没有信息量。正确表述：0 回归 + bindings 生成 + TS 编译通过

### 试过但不行的方案

- **可见性方案 B (re-export)** — `pub(super)` item 不能被 re-export，E0365
- **可见性方案 C (facade)** — 30 个 pass-through wrapper 违反 DRY
- **`skip_serializing_if` + specta** — specta unified mode 不支持，必须移除

### 开放问题

无

## Resume 检查清单

- [ ] 读 `docs/progress/keysight.md` 确认 Phase 3 在 Next 区
- [ ] 跑 `stale_check`：`cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3 && cd ..` — 应显示 `104 passed` + `test result: ok`
- [ ] `git status` 干净
- [ ] 确认 Phase 2 在 Done 区
- [ ] 确认 `src/bindings.ts` 存在且包含 keysight commands
