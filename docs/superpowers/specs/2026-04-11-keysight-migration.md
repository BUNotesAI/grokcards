# KeySight → Tauri 迁移 Spec

## 动机

KeySight 当前是 Obsidian 插件 + Rust sidecar (Unix socket JSON-RPC) 架构。核心痛点：

1. **sidecar 生命周期管理**：EPIPE、孤儿进程、老 binary 被 reuse、`pnpm build:fresh` + pkill、PID 文件竞争。占总 bug 量 ~40%
2. **TS/Rust 纪律需要 3 层强制**：ESLint restricted-imports + Rust runtime guard + 回归测试。在 Tauri 中架构天然强制
3. **IPC 类型安全靠手工 TypedRpc 接口**：Tauri 用 tauri-specta 编译期保障，无法绕过
4. **Graph 白板受限于 Obsidian 窗口**：Tauri 可独立全屏

KeySight 的核心价值（Graph 白板 + Rust domain 逻辑 + Entity Registry）对 Obsidian **零依赖**，可直接移植。

## 决策记录

| 决策 | 选项 | 选择 | 理由 |
|------|------|------|------|
| DB 位置 | vault 内 / Tauri app data | **Tauri app data** | Tauri 是独立应用，DB 在 `~/.local/share/super-tauri/` 或 `~/Library/Application Support/super-tauri/` |
| Schema | 从 v1 迁移 / 纯 v7 新建 | **纯 v7 新建** | 全新 DB，无旧表，无需 migration 工具 |
| 旧 DB 数据 | 丢弃 / 一次性导入 | **一次性导入脚本** | Section 分组、Position 布局、Alias 积累了大量人工整理，不可丢失 |
| IPC | JSON-RPC socket / Tauri command | **Tauri command** | 消灭 sidecar 生命周期管理 |
| 类型桥接 | ts-rs / tauri-specta | **tauri-specta** | 编译期保障，已在 super-tauri 验证 |
| 文件监听 | Obsidian Vault API / notify crate | **notify crate** | 独立应用无法依赖 Obsidian API |
| Markdown 渲染 | Obsidian MarkdownRenderer / react-markdown | **react-markdown + remark** | 需自定义 wikilink 渲染 |
| Follow mode | Obsidian active-leaf / 放弃 | **Phase 3 后评估** | 迁移核心功能优先，Follow 可通过 Obsidian 迷你插件桥接 |

## 源项目概览

**源仓库**：`~/codes/vibe-coding/obsidian-plugin-keysight/`

### 可直接移植的 Rust domain 代码

源目录：`keysight-core/src/`

```
domain/
├── types.rs              # EntityKind, EdgeType, GraphEntity trait, AtomicCard, GraphSection, GraphNote, CardAlias, Position, WhiteboardId 等
├── entity/               # Entity Registry: entity_connect / entity_disconnect
├── card/
│   ├── queries.rs        # query_all, query_search, query_file, query_by_ids, query_links 等（当前读 insights 表）
│   ├── repository.rs     # row_to_card, query_cards, write_and_resync, vault_file_path
│   └── commands.rs       # edit_title, edit_body, understanding_update, link_to_add/remove, related_add/remove, see_also_add/remove
├── sync/
│   ├── commands.rs       # sync_file (旧), sync_file_v7 (新) — 文件 → DB 同步
│   └── queries.rs        # all_file_mtimes
├── graph/
│   ├── layout.rs         # set_position（当前 dual-write meta+positions 表）
│   ├── meta.rs           # read_meta_json_or, set_meta（meta blob 读写原语）
│   └── overview.rs       # stats, graph_overview
├── section/
│   ├── commands.rs       # create, delete, update, add_card, remove_card, connect, disconnect, move_to_whiteboard
│   ├── queries.rs        # bounds
│   └── layout.rs         # estimate_card_height, compute_bounds, auto_move_into
├── note/
│   ├── commands.rs       # create, update, delete, connect, disconnect
│   ├── queries.rs        # all
│   └── export.rs         # export_notes_to_files（DB-only Note → .md 文件）
├── alias/
│   └── commands.rs       # create, delete, connect, disconnect, card_to_alias_connect/disconnect
├── task/
│   ├── commands.rs       # update_status, update_area, update_project
│   └── queries.rs        # by_status
├── question/
│   ├── commands.rs       # update_status
│   └── queries.rs        # by_status
└── whiteboard/           # WhiteboardId scope resolver
```

**关键：这些 domain 函数当前有两个问题：**
1. Card queries 读 `insights` 旧表（不存在于新 DB）
2. Section/Note/Alias commands 读写 `meta` blob（不存在于新 DB）

**所以搬过来时必须同时改写为读写新表（entities/edges/positions/section_members/alias_fields）。不改写直接搬 = 启动即 crash。**

### 可直接移植的 infra 代码

| 文件 | 内容 | 迁移方式 |
|------|------|---------|
| `infra/db.rs` | `SCHEMA_V7_SQL` DDL + `open_db` + WAL 配置 | 只取 `SCHEMA_V7_SQL`，丢弃旧 `SCHEMA_SQL`。`open_db` 适配 Tauri app data path |
| `infra/parser.rs` | frontmatter YAML 解析 + H1 title 提取 + 多类型 parse (Card/Note/Task/Question) | 原样移植 |
| `store.rs` | `gen_card_id/gen_sec_id/gen_note_id/gen_alias_id/gen_task_id/gen_question_id` | id 工厂函数原样移植 |

### 可直接移植的 TS 组件

源目录：`~/codes/vibe-coding/obsidian-plugin-keysight/src/`

| 组件 | 行数 | 迁移方式 |
|------|------|---------|
| `components/GraphView.tsx` | ~3200 | 核心画布。需要：(1) 删除 Obsidian API 依赖，(2) 将 `plugin.store.*` 调用改为 `commands.*` |
| `components/TaskCard.tsx` | ~80 | 原样移植，改 import |
| `components/QuestionCard.tsx` | ~60 | 原样移植，改 import |
| `components/ToggleList.tsx` | ~40 | 原样移植 |
| `components/InsightCard.tsx` | ~200 | 卡片渲染组件 |
| `components/RenderedMarkdown.tsx` | ~100 | 依赖 Obsidian MarkdownRenderer → 改为 react-markdown |

### 不需要搬的代码（删除）

| 文件/模块 | 原因 |
|----------|------|
| `src/rpc/RpcClient.ts` | Unix socket spawn/kill/PID 管理，Tauri 不需要 |
| `src/rpc/TypedRpc.ts` | JSON-RPC 类型包装，tauri-specta 替代 |
| `src/store/InsightStore.ts` | RPC 缓存层 + meta blob 解析，Tauri command 直接返回数据 |
| `src/main.ts` | Obsidian plugin onload/onunload 生命周期 |
| `keysight-core/src/main.rs` | Unix socket server + JSON-RPC dispatch，Tauri 用 command |
| `keysight-core/src/rpc/dispatch.rs` | JSON-RPC dispatch 路由 |
| `keysight-core/src/rpc/types.rs` | MutateRequest/QueryRequest enum（Tauri 每个 command 独立） |
| `keysight-core/src/rpc/wire.rs` | JSON-RPC 协议层 |
| `keysight-core/src/bin/keysight-cli.rs` | CLI binary（后续可独立重建，不阻塞迁移） |
| ESLint restricted-imports 规则 | Tauri 架构天然强制 |

## 数据库设计

### Schema：纯 v7（无旧表）

直接使用 `SCHEMA_V7_SQL`，加上 `file_mtimes` 和 `meta` 表：

```sql
-- 基础表
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS file_mtimes (filePath TEXT PRIMARY KEY, mtime REAL NOT NULL);

-- Entity Registry (v7)
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL,
    whiteboard_id TEXT NOT NULL, file_path TEXT, content TEXT, color TEXT
);
CREATE TABLE IF NOT EXISTS card_fields (entity_id TEXT PRIMARY KEY, understanding TEXT DEFAULT '', source TEXT DEFAULT '');
CREATE TABLE IF NOT EXISTS task_fields (entity_id TEXT PRIMARY KEY, status TEXT DEFAULT 'next', area TEXT, project TEXT);
CREATE TABLE IF NOT EXISTS question_fields (entity_id TEXT PRIMARY KEY, status TEXT DEFAULT 'pending');
CREATE TABLE IF NOT EXISTS alias_fields (entity_id TEXT PRIMARY KEY, card_id TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS entity_tags (entity_id TEXT NOT NULL, tag TEXT NOT NULL, PRIMARY KEY(entity_id, tag));
CREATE TABLE IF NOT EXISTS edges (from_id TEXT NOT NULL, to_id TEXT NOT NULL, edge_type TEXT NOT NULL, style TEXT, label TEXT, PRIMARY KEY(from_id, to_id, edge_type));
CREATE TABLE IF NOT EXISTS positions (entity_id TEXT NOT NULL, whiteboard_id TEXT NOT NULL, x REAL NOT NULL, y REAL NOT NULL, PRIMARY KEY(entity_id, whiteboard_id));
CREATE TABLE IF NOT EXISTS section_members (section_id TEXT NOT NULL, entity_id TEXT NOT NULL, PRIMARY KEY(section_id, entity_id));

-- 索引
CREATE INDEX IF NOT EXISTS idx_entities_kind ON entities(kind);
CREATE INDEX IF NOT EXISTS idx_entities_wb ON entities(whiteboard_id);
CREATE INDEX IF NOT EXISTS idx_entities_file ON entities(file_path);
CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
CREATE INDEX IF NOT EXISTS idx_task_status ON task_fields(status);
CREATE INDEX IF NOT EXISTS idx_question_status ON question_fields(status);
CREATE INDEX IF NOT EXISTS idx_positions_wb ON positions(whiteboard_id);
CREATE INDEX IF NOT EXISTS idx_section_members_entity ON section_members(entity_id);
```

无 `insights` 表。无 `insight_links`/`insight_related`/`insight_tags`。无 `insights_fts`。无 `whiteboard_cards`。

### Vault 路径配置

用户首次启动时选择 vault 路径（如 `~/Documents/obsidian_workspace/agent-slipbox-v3`），存入 `meta` 表 `vault_path` key。后续启动自动读取。

### 一次性旧 DB 导入

旧 DB：`~/Documents/obsidian_workspace/agent-slipbox-v3/.obsidian/plugins/keysight/keysight.db`

导入内容（旧表 → 新表）：

| 旧来源 | 新目标 |
|--------|--------|
| `insights` 表全部行 | `entities` (kind='card') + `card_fields` + `entity_tags` + `edges` (link_to/related/see_also) |
| `meta` graph_positions:* blob | `positions` 表 |
| `meta` graph_sections:* blob | `entities` (kind='section') + `section_members` + `edges` (section_link) |
| `meta` graph_aliases:* blob | `entities` (kind='alias') + `alias_fields` + `edges` (note_link/card_to_alias) |
| `meta` graph_notes:* blob | `entities` (kind='note') + `edges` (note_link) |

导入实现：Tauri command `import_legacy_db(old_db_path: String)` + domain 纯函数 `legacy_import::import(conn, old_conn)`。单事务 + 全量验证。

## 实体类型

6 种实体，frontmatter `type:` 字段区分：

| EntityKind | type 值 | 文件名前缀 | ID 前缀 | 数据源 |
|---|---|---|---|---|
| Card | `atomic-card` | `card_` | `card_` | FileBacked |
| Note | `note` | `note_` | `note_` | FileBacked |
| Task | `project-task` | `task_` | `task_` | FileBacked |
| Question | `question` | `q_` | `q_` | FileBacked |
| Section | _(DB only)_ | `sec_` | `sec_` | DbOnly |
| Alias | _(DB only)_ | `alias_` | `alias_` | DbOnly |

## Tauri 模块结构（目标）

```
src-tauri/src/
├── lib.rs                         # Tauri Builder + specta + State<Mutex<Connection>> 注入
├── app_error.rs                   # 顶层统一错误类型
├── modules/
│   ├── mod.rs                     # re-export + init_all
│   ├── todo/                      # （现有 TodoMVC 验证模块，保留）
│   │
│   ├── keysight/                  # ← 核心迁移目标
│   │   ├── mod.rs                 # 窄接口
│   │   ├── models.rs              # AtomicCard, GraphSection, GraphNote, CardAlias, Position, EntityKind, EdgeType 等 + specta::Type
│   │   ├── errors.rs              # KeysightError + impl Into<AppError>
│   │   ├── db.rs                  # 建表（纯 v7 schema）+ DB 路径管理
│   │   ├── parser.rs              # frontmatter YAML 解析 + parse_entity 多类型
│   │   ├── id.rs                  # gen_card_id / gen_sec_id / gen_note_id 等
│   │   ├── domain/                # 业务纯函数（从旧 domain/ 移植 + 改写为新表读写）
│   │   │   ├── mod.rs
│   │   │   ├── card.rs            # card 查询 + 编辑
│   │   │   ├── section.rs         # section CRUD
│   │   │   ├── note.rs            # note CRUD
│   │   │   ├── alias.rs           # alias CRUD
│   │   │   ├── entity.rs          # entity_connect / entity_disconnect
│   │   │   ├── task.rs            # task status/area/project
│   │   │   ├── question.rs        # question status
│   │   │   ├── sync.rs            # sync_file（只有 v7 路径）
│   │   │   ├── layout.rs          # set_position（只写 positions 表）
│   │   │   └── overview.rs        # stats / graph_overview
│   │   ├── commands.rs            # #[tauri::command] 薄壳，每个 ≤3 行
│   │   └── legacy_import.rs       # 一次性旧 DB 导入
│   │
│   └── watcher/                   # 文件监听模块
│       ├── mod.rs
│       ├── domain.rs              # notify crate 封装 + debounce
│       └── commands.rs            # start_watching / stop_watching / manual_sync
```

## Tauri Command 清单

每个 command 对应旧 JSON-RPC 的一个 op。分组列出：

### Card 查询

| Command | 参数 | 返回 | 对应旧 op |
|---------|------|------|----------|
| `query_all_cards` | `limit?, offset?` | `Vec<AtomicCard>` | `QueryRequest::All` |
| `query_card_by_file` | `path: String` | `Vec<AtomicCard>` | `QueryRequest::File` |
| `search_cards` | `text: String` | `Vec<AtomicCard>` | `QueryRequest::Search` |
| `query_cards_by_ids` | `ids: Vec<String>` | `Vec<AtomicCard>` | `QueryRequest::ByIds` |
| `query_card_links` | `id: String` | `LinksResponse` | `QueryRequest::Links` |
| `query_card_count` | — | `i64` | `QueryRequest::Count` |

### Graph 数据查询

| Command | 参数 | 返回 | 对应旧 op |
|---------|------|------|----------|
| `query_positions` | `whiteboard_id: String` | `HashMap<String, Position>` | 旧 `queryMeta("graph_positions:*")` |
| `query_sections` | `whiteboard_id: String` | `Vec<GraphSection>` | 旧 `queryMeta("graph_sections:*")` |
| `query_notes` | `whiteboard_id: String` | `Vec<GraphNote>` | 旧 `queryMeta("graph_notes:*")` |
| `query_aliases` | `whiteboard_id: String` | `Vec<CardAlias>` | 旧 `queryMeta("graph_aliases:*")` |
| `query_edges` | `whiteboard_id: String, edge_types?: Vec<String>` | `Vec<Edge>` | 新：直接查 edges 表 |
| `query_stats` | — | `StatsResponse` | `QueryRequest::Stats` |
| `query_graph_overview` | — | `GraphOverviewResponse` | `QueryRequest::GraphOverview` |

### 实体写入

| Command | 参数 | 返回 | 对应旧 op |
|---------|------|------|----------|
| `edit_card_title` | `id, title` | `MutateResponse` | `MutateRequest::EditTitle` |
| `edit_card_body` | `id, body` | `MutateResponse` | `MutateRequest::EditBody` |
| `update_understanding` | `id, text` | `MutateResponse` | `MutateRequest::UnderstandingUpdate` |
| `set_position` | `whiteboard_id, id, x, y` | `MutateResponse` | `MutateRequest::SetPosition` |
| `entity_connect` | `from, to, edge_type, style?, label?` | `MutateResponse` | `MutateRequest::EntityConnect` |
| `entity_disconnect` | `from, to, edge_type` | `MutateResponse` | `MutateRequest::EntityDisconnect` |

### Section

| Command | 参数 | 返回 |
|---------|------|------|
| `section_create` | `whiteboard_id, title, color?` | `MutateResponse` (含 id) |
| `section_delete` | `whiteboard_id, id` | `MutateResponse` |
| `section_update` | `whiteboard_id, id, title?, color?` | `MutateResponse` |
| `section_add_member` | `whiteboard_id, section_id, entity_id` | `MutateResponse` |
| `section_remove_member` | `whiteboard_id, section_id, entity_id` | `MutateResponse` |
| `section_move_to_whiteboard` | `section_id, source_wb, target_wb` | `MutateResponse` |

### Note

| Command | 参数 | 返回 |
|---------|------|------|
| `note_create` | `whiteboard_id, title, content?, color?` | `MutateResponse` (含 id) |
| `note_delete` | `whiteboard_id, id` | `MutateResponse` |
| `note_update` | `whiteboard_id, id, title?, content?, color?` | `MutateResponse` |

### Alias

| Command | 参数 | 返回 |
|---------|------|------|
| `alias_create` | `whiteboard_id, card_id` | `MutateResponse` (含 id) |
| `alias_delete` | `whiteboard_id, id` | `MutateResponse` |

### Task / Question

| Command | 参数 | 返回 |
|---------|------|------|
| `task_update_status` | `id, status` | `MutateResponse` |
| `task_update_area` | `id, area` | `MutateResponse` |
| `task_update_project` | `id, project` | `MutateResponse` |
| `question_update_status` | `id, status` | `MutateResponse` |

### Sync + Watcher

| Command | 参数 | 返回 |
|---------|------|------|
| `sync_file` | `file_path, content, mtime` | `SyncFileResponse` |
| `remove_file` | `file_path` | `MutateResponse` |
| `start_watching` | `vault_path` | `()` |
| `manual_sync` | — | `SyncSummary` |

### Legacy 导入

| Command | 参数 | 返回 |
|---------|------|------|
| `import_legacy_db` | `old_db_path: String` | `ImportSummary` |

## 实施 Phases

### Phase 1: Rust domain 移植 + 新表读写（TDD + Trait-First）

目标：KeySight 的全部 Rust domain 逻辑在 Tauri 项目中编译通过、测试通过。

**方法论**：TDD + Trait-First。先定义行为契约（trait），再从旧系统已知行为写测试（Red），最后实现让测试通过（Green）。失败的测试数量 = 剩余迁移工作量。

#### Step 1: 模块骨架 + 数据类型（无行为，无需 TDD）

1. 创建 `modules/keysight/` 目录骨架
2. 移植 `models.rs`（从 `domain/types.rs`，添加 `specta::Type`）— 纯数据类型
3. 移植 `errors.rs` — thiserror + impl Into<AppError>
4. 移植 `db.rs`（纯 v7 schema）— 验证：in-memory SQLite 建表成功
5. 移植 `id.rs`（从 `store.rs` 的 id 工厂）— TDD：先写 id 格式测试，再移植实现

#### Step 2: Parser（TDD — 有明确输入输出的纯函数）

1. 写 parser 测试：已知的 frontmatter YAML → 期望的 parse 结果
2. 跑测试 → 🔴 **Red 关卡** — 用户确认测试因正确原因失败
3. 移植 `parser.rs`（从 `infra/parser.rs`）
4. 跑测试 → 🟢 **Green 关卡** — 用户确认全部通过
5. 覆盖：Card / Note / Task / Question / 无效格式 等 case（每组都走 🔴→🟢）

#### Step 3: Domain traits 定义（Trait-First — 先画边界）

为每个 domain 区域定义 trait，明确"做什么"再动手"怎么做"：

```rust
// domain/card.rs
pub(super) trait CardStore {
    fn create(&self, title: &str, content: &str, ...) -> Result<AtomicCard, KeysightError>;
    fn get(&self, id: &str) -> Result<AtomicCard, KeysightError>;
    fn query_all(&self, limit: Option<i64>) -> Result<Vec<AtomicCard>, KeysightError>;
    fn update_title(&self, id: &str, title: &str) -> Result<(), KeysightError>;
    // ... 从旧 commands.rs + queries.rs 提取完整行为列表
}

// domain/section.rs
pub(super) trait SectionStore { ... }

// domain/entity.rs
pub(super) trait EntityGraph {
    fn connect(&self, from: &str, to: &str, edge_type: EdgeType) -> Result<(), KeysightError>;
    fn disconnect(&self, from: &str, to: &str, edge_type: EdgeType) -> Result<(), KeysightError>;
}
```

**trait 提取原则**：从旧项目的 `commands.rs` + `queries.rs` 公开方法签名推导，一个 domain 文件一个 trait。

#### Step 4: 逐模块 TDD 循环（Red → Green）

对每个 domain 模块重复：

```
a. 写测试（从旧系统已知行为推导，调用 trait 方法）
b. 跑测试 → 🔴 Red 关卡 — 用户确认失败原因正确
c. 写 impl（改写 SQL 为新表读写）
d. 跑测试 → 🟢 Green 关卡 — 用户确认全部通过
e. Refactor（保持绿色）
```

**迁移顺序**（按依赖关系排列）：

| 顺序 | 模块 | 新表读写（不读旧表！） | 测试重点 |
|------|------|----------------------|---------|
| 1 | `domain/entity.rs` | `INSERT/DELETE edges` | connect/disconnect + 幂等性 |
| 2 | `domain/card.rs` | `entities LEFT JOIN card_fields` | 查询组装（tags/edges batch）+ CRUD |
| 3 | `domain/section.rs` | `entities + section_members` | create/delete/add_member/remove_member |
| 4 | `domain/note.rs` | `entities WHERE kind='note'` | CRUD + edges |
| 5 | `domain/alias.rs` | `entities + alias_fields` | create/delete + card_to_alias 关联 |
| 6 | `domain/layout.rs` | `positions` 表 | set_position + query_positions |
| 7 | `domain/sync.rs` | 只保留 `sync_file_v7` 逻辑 | 文件 → DB 同步（parse + upsert） |
| 8 | `domain/task.rs` | 原样移植（已写新表） | status/area/project 更新 |
| 9 | `domain/question.rs` | 原样移植（已写新表） | status 更新 |
| 10 | `domain/overview.rs` | `entities + edges` 统计 | stats 正确性 |

**每个模块的测试覆盖要求**：
- Happy path：正常输入 → 期望输出
- Error path：非法输入 → 期望错误类型
- 边界条件：空列表、不存在的 id、重复操作
- 数据完整性：写入后立即查询验证一致性

#### Step 5: 全量验证

`cargo test --workspace` 全过。所有测试从 Red 变为 Green = Phase 1 完成。

### Phase 2: Tauri commands + specta 绑定（接线验证）

Phase 2 是 Phase 1 domain 逻辑的接线层。commands.rs 是 ≤3 行的薄壳，不含业务逻辑，所以不需要独立 TDD。但接线正确性需要验证。

1. 写 `commands.rs` — 薄壳，每个 command 调用 Phase 1 的 domain trait 实现
   - 每个 command 加 `#[tauri::command]` + `#[specta::specta]`
   - 有副作用的 pub fn 写 Operation Contract（L0）
2. 在 `modules/mod.rs` 注册，在 `lib.rs` 的 `collect_commands![]` 添加
3. `cargo clippy --workspace -- -D warnings` — 编译 + lint 通过
4. `cargo test export_bindings` — 生成 `bindings.ts`
5. `pnpm build` — TS 编译通过，确认所有 typed command 出现在 bindings 中
6. 在副作用矩阵中登记所有写操作

### Phase 3: 一次性旧 DB 导入（TDD + Trait-First）

Legacy import 有真实的转换逻辑（旧格式 → 新格式），必须 TDD。

#### Trait 定义

```rust
/// 旧 DB 数据读取契约 — 隔离旧 DB 格式细节
pub(super) trait LegacyReader {
    fn read_insights(&self) -> Result<Vec<LegacyInsight>, KeysightError>;
    fn read_meta_blob(&self, key: &str) -> Result<Option<String>, KeysightError>;
}

/// 导入执行契约
pub(super) trait LegacyImporter {
    /// # 一次性旧 DB 导入
    ///
    /// ## 前置条件
    /// - old_db_path 指向有效的旧 keysight.db
    /// - 新 DB 表已建好（Phase 1 db.rs）
    ///
    /// ## 执行效果
    /// 1. 读旧 DB 的 insights + meta blob
    /// 2. 转换为 entities/edges/positions/section_members 等新表格式
    /// 3. 单事务写入新 DB
    /// 4. 验证写入数量与读取数量一致
    ///
    /// ## 幂等性
    /// 重复调用会报错（entity id 冲突），需先清空新表
    fn import(&self, old_db: &dyn LegacyReader) -> Result<ImportSummary, KeysightError>;
}
```

#### TDD 流程

1. 准备 fixture 旧 DB（从真实旧 DB 导出的子集，或手工构造的最小数据集）
2. 写测试：fixture → import → 验证新表数据正确性
   - insights 行 → entities(kind='card') + card_fields + entity_tags + edges
   - meta graph_sections:* → entities(kind='section') + section_members
   - meta graph_positions:* → positions 表
   - meta graph_notes:* → entities(kind='note') + edges
   - meta graph_aliases:* → entities(kind='alias') + alias_fields
3. 🔴 **Red 关卡** — 用户确认
4. 实现 `legacy_import.rs` — 单事务 + 全量验证
5. 🟢 **Green 关卡** — 用户确认
6. Tauri command `import_legacy_db(path)` 薄壳暴露给 UI

### Phase 4: 文件监听（TDD + Trait-First）

文件系统是外部依赖，必须用 trait 隔离以实现可测试性。

#### Trait 定义

```rust
/// 文件系统事件源契约 — 测试时用 mock 替代真实 notify
pub(super) trait FileEventSource {
    fn watch(&self, path: &Path) -> Result<(), WatcherError>;
    fn stop(&self) -> Result<(), WatcherError>;
}

/// 文件事件处理契约 — 纯逻辑，不碰文件系统
pub(super) trait FileEventHandler {
    fn on_create(&self, path: &Path, content: &str, mtime: f64) -> Result<(), KeysightError>;
    fn on_modify(&self, path: &Path, content: &str, mtime: f64) -> Result<(), KeysightError>;
    fn on_delete(&self, path: &Path) -> Result<(), KeysightError>;
}
```

#### TDD 流程

1. 写事件处理测试（mock FileEventSource，真实 domain 函数处理事件）：
   - file create → sync_file 被调用，DB 有对应 entity
   - file modify → entity 内容更新
   - file delete → entity 被移除
   - debounce：200ms 内多次事件 → 只触发一次 sync
   - mtime 检查：mtime 未变 → 跳过
2. 🔴 **Red 关卡** — 用户确认
3. 实现 `modules/watcher/` — notify crate 封装 + debounce 逻辑
4. 🟢 **Green 关卡** — 用户确认
5. Tauri commands: `start_watching` / `stop_watching` / `manual_sync` 薄壳

### Phase 5: GraphView 移植（TDD — TS 组件测试）

**前置**：搭建 TS 测试基础设施（Vitest + @testing-library/react + @testing-library/jest-dom）

1. `pnpm add -D vitest @testing-library/react @testing-library/jest-dom jsdom`
2. 配置 `vitest.config.ts`（environment: jsdom）
3. 配置 `package.json` scripts: `"test": "vitest run"`
4. 编写 bindings.ts mock helper（`src/__tests__/helpers/mockCommands.ts`）

**GraphView 移植（TDD 流程）**：

5. 写组件测试 — mock commands，验证渲染和事件接线
6. 🔴 **Red 关卡** — 用户确认测试因正确原因失败
7. 移植 GraphView：
   - 将 `GraphView.tsx` 复制到 `src/pages/GraphPage.tsx`
   - 删除 Obsidian API 依赖（`plugin.store.*` → `commands.*`）
   - edges 从 `commands.queryEdges()` 加载
   - positions 从 `commands.queryPositions()` 加载
   - Section/Note/Alias 从对应 command 加载
   - RenderedMarkdown 改为 react-markdown
8. 🟢 **Green 关卡** — 用户确认组件测试全绿
9. `pnpm test && pnpm build` 通过

### Phase 6: UI 补全 + 打磨（TDD — 每个组件配套测试）

每个新组件遵循 TDD 流程：写测试 → 🔴 用户确认 → 实现 → 🟢 用户确认。

1. **侧边栏：Card 列表、搜索、过滤**
   - 测试：渲染 card 列表、搜索输入触发 searchCards command、过滤器切换
   - 🔴→🟢 关卡
2. **设置页：vault 路径配置、导入旧 DB**
   - 测试：路径选择触发配置 command、导入按钮触发 importLegacyDb command、进度/完成状态渲染
   - 🔴→🟢 关卡
3. **主题**（shadcn/ui 已集成）— 无逻辑，无需 TDD
4. **Keyboard shortcuts** — 测试：快捷键事件触发正确 command

## 范围外

- Kanban View / Table View（后续迭代）
- Follow mode（需 Obsidian 迷你插件桥接，Phase 6 后评估）
- CLI 重建（旧 `keysight-cli` 后续按需）
- 移动端支持
- 多 vault 支持
