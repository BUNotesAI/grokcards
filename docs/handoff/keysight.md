---
area: keysight
last_updated: 2026-04-11T15:30:00+08:00
session_id: current
status: ready-to-resume
stale_check: ls src-tauri/src/modules/keysight/mod.rs 2>/dev/null && echo "module exists" || echo "module not yet created"
---

# Handoff: keysight

## 正在做的 Task

KeySight 功能从 Obsidian 插件迁移到 Tauri 独立应用。迁移 spec 已完成，实现尚未开始。

对应 `docs/progress/keysight.md` > Next > "Phase 1: Rust domain 移植 + 新表读写"

## 已完成步骤

- [x] 架构评估：Obsidian plugin vs Tauri app 全面对比
- [x] 决策：迁移到 Tauri，理由记录在 spec
- [x] 分析哪些代码可复用、哪些需改写、哪些删除
- [x] 确认 DB 策略：Tauri app data 全新 DB，纯 v7 schema，无旧表
- [x] 确认旧数据策略：一次性导入脚本（Section/Position/Alias 不可丢）
- [x] 迁移 spec 完成 — `docs/superpowers/specs/2026-04-11-keysight-migration.md`
- [x] 进度文件创建 — `docs/progress/keysight.md`
- [ ] **下一步**：Phase 1 implementation plan → 执行

## 下一步具体动作

1. **读 spec** — `docs/superpowers/specs/2026-04-11-keysight-migration.md` 的 Phase 1 section（已更新为 TDD + Trait-First 流程）
2. **写 implementation plan** — 基于 Phase 1 的 5 个 Step：
   - Step 1: 模块骨架 + models/errors/db/id（无行为代码）
   - Step 2: Parser TDD — 先写测试再移植
   - Step 3: 定义 domain traits（CardStore / SectionStore / EntityGraph 等）
   - Step 4: 逐模块 TDD 循环 — 写测试（Red）→ 实现（Green）→ Refactor
   - Step 5: cargo test 全量验证
3. **方法论**：TDD + Trait-First 已写入 CLAUDE.md L0 质量体系，适用于本项目所有开发
4. **注意**：domain 函数从旧项目复制后**必须改写 SQL**——旧代码读 `insights` 表和 `meta` blob，新代码必须读 `entities` / `edges` / `positions` / `section_members` 等。不改写直接搬 = 编译可能过但运行时 crash
5. **依赖**：parser.rs 依赖 `yaml-rust` 或 `serde_yaml` — 检查旧项目的 `Cargo.toml` dependencies 确认用了哪个，新项目也要加

## 关键上下文（/new 之后会丢的东西）

### 源项目信息

- **源仓库路径**：`~/codes/vibe-coding/obsidian-plugin-keysight/keysight-core/src/`
- **源 Cargo.toml**：`~/codes/vibe-coding/obsidian-plugin-keysight/Cargo.toml`（注意 Cargo.toml 在仓库根目录，不在 keysight-core/ 下）
- **源 DB**（可用于旧数据导入）：`~/Documents/obsidian_workspace/agent-slipbox-v3/.obsidian/plugins/keysight/keysight.db`
- **源 Vault 路径**（.md 文件所在）：`~/Documents/obsidian_workspace/agent-slipbox-v3/`
- **Scan 目录**：只扫描 `atomic cards/` 子目录

### Entity Registry 半成品状态（源项目）

Entity Registry 在源项目做了 18 个 task（全是写入侧 dual-write），但**读路径零切换**——所有读仍走旧表。源项目 DB schema=v1，新表（entities 等）不存在。

**对 Tauri 迁移的影响**：**无障碍**。Tauri 全新 DB 直接用 v7 schema，不存在旧表兼容问题。但移植 domain 代码时不能直接复制旧的读路径（它读 `insights` 旧表），必须改写为新表查询。

**`sync_file_v7` 可以原样复用** — 这个函数已经写新表，不碰旧表。

### 旧项目的 domain 函数如何读新表（参考设计）

旧项目的 Entity Registry spec 里有完整的 SQL 设计（见 `~/codes/vibe-coding/obsidian-plugin-keysight/docs/superpowers/specs/2026-04-10-entity-registry-design.md`），核心 pattern：

**Card query 3-query batch**（避免 N+1）：
```sql
-- Q1: 实体 + card_fields + mtime
SELECT e.id, e.title, e.content, e.file_path,
       COALESCE(c.understanding, '') AS understanding,
       COALESCE(c.source, '') AS source,
       COALESCE(f.mtime, 0) AS mtime
FROM entities e
LEFT JOIN card_fields c ON e.id = c.entity_id
LEFT JOIN file_mtimes f ON e.file_path = f.filePath
WHERE e.kind = 'card'
ORDER BY f.mtime DESC

-- Q2: 批量 tags
SELECT entity_id, tag FROM entity_tags WHERE entity_id IN (...)

-- Q3: 批量 edges
SELECT from_id, to_id, edge_type FROM edges WHERE from_id IN (...)
```

**Section query**：`entities WHERE kind='section'` + `section_members` + `edges WHERE edge_type='section_link'` → 组装为 `GraphSection { id, title, cardIds, color, linkedSectionIds }`

**Position query**：`SELECT entity_id, x, y FROM positions WHERE whiteboard_id = ?` → `HashMap<String, Position>`

### 旧项目 Rust 类型定义（需移植 + 加 specta::Type）

```rust
pub struct Position { pub x: f64, pub y: f64 }
pub struct AtomicCard { pub id: String, pub file_path: String, pub title: String, pub content: String, pub tags: Vec<String>, pub link_to: Vec<String>, pub related: Vec<String>, pub understanding: String, pub source: String, pub see_also: Vec<String>, pub mtime: Option<f64> }
pub struct GraphSection { pub id: String, pub title: String, pub card_ids: Vec<String>, pub color: Option<String>, pub linked_section_ids: Option<Vec<String>> }
pub struct GraphNote { pub id: String, pub title: String, pub content: String, pub color: Option<String>, pub linked_section_ids: Option<Vec<String>>, pub linked_card_ids: Option<Vec<String>>, pub linked_note_ids: Option<Vec<String>> }
pub struct CardAlias { pub alias_id: String, pub card_id: String, pub linked_card_ids: Option<Vec<String>>, pub linked_section_ids: Option<Vec<String>>, pub linked_note_ids: Option<Vec<String>>, pub incoming_card_ids: Option<Vec<String>> }
pub enum EntityKind { Card, Note, Alias, Section, Task, Question }
pub enum EdgeType { LinkTo, Related, SeeAlso, SectionLink, CardToAlias, NoteLink }
```

所有 struct 原来 derive `Serialize, Deserialize, TS (ts-rs)`。迁移后改为 `Serialize, Deserialize, specta::Type`。

### Tauri 项目当前状态

- **框架已就绪**：Tauri 2 + React 19 + Vite + tauri-specta + rusqlite + shadcn/ui
- **TodoMVC 模块存在但未完成**：`modules/todo/` 有骨架代码
- **Git 已初始化**：有 commits
- **CLAUDE.md 完整**：17 个质量机制已落地
- **bindings.ts 生成可用**：`cargo test export_bindings` 可运行

### 试过但不行的方案

- **在 Obsidian 插件内完成 Entity Registry 读迁移再搬 Tauri** — 会产生大量 TS 层工作（TypedRpc 新方法、InsightStore 改写、GraphView edges 重写）全部浪费，因为 Tauri 用 specta commands 替代

### 开放问题

无

## Resume 检查清单

- [ ] 读 `docs/progress/keysight.md` 确认 Phase 1 在 Next 区
- [ ] 读 `docs/superpowers/specs/2026-04-11-keysight-migration.md` 全文
- [ ] 读 CLAUDE.md 质量体系（特别是 Deep Module、IPC 类型安全、Anti-Test-Theater）
- [ ] 确认源项目可访问：`ls ~/codes/vibe-coding/obsidian-plugin-keysight/keysight-core/src/domain/types.rs`
- [ ] 确认源 DB 可访问：`sqlite3 ~/Documents/obsidian_workspace/agent-slipbox-v3/.obsidian/plugins/keysight/keysight.db "SELECT COUNT(*) FROM insights"` 应返回 145
- [ ] 确认 Tauri 项目可编译：`cd ~/codes/vibe-coding/super-tauri/src-tauri && cargo clippy --workspace -- -D warnings`
- [ ] 确认 `modules/keysight/` 目录是否已创建（首次 resume 应不存在）
- [ ] 如果状态不匹配 — 不要盲目继续，先 ping 用户确认
