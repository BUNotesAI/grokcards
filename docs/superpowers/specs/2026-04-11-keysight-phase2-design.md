# KeySight Phase 2: Tauri Commands + Specta 绑定

## 目标

将 Phase 1 完成的 domain 层通过 Tauri commands 暴露给 TS 前端，建立端到端类型安全的 IPC 管道。

## 范围

- 暴露 40 个 command（跳过 task/question 模块，推迟到 Phase 6）
- 新增 `KeysightState` 管理 DB 连接 + vault 路径
- 可见性从 `pub(super)` 放宽到 `pub(in crate::modules::keysight)`
- 填充 CLAUDE.md 副作用矩阵

## 不做的事

- 不改 domain 层业务逻辑（已有 104 tests 锁住）
- 不暴露 task/question 模块（`by_status` 返回 `serde_json::Value`，需先补 typed struct，Phase 6 再做）
- 不暴露 `sync::derive_whiteboard_id`（纯计算，内部实现细节）
- 不收紧 capabilities 安全（发布前再做）
- 不新增 command 层测试（薄壳 ≤3 行，domain 已有充分测试）

---

## 1. KeysightState + 初始化

### State 结构

```rust
// src-tauri/src/modules/keysight/state.rs
use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::Connection;

pub struct KeysightState {
    pub db: Mutex<Connection>,
    pub vault_path: PathBuf,
}
```

### 初始化流程（lib.rs setup 闭包）

1. 读 `KEYSIGHT_VAULT_PATH` 环境变量，缺失则 panic with clear message
2. 开独立 `keysight.db`（app data 目录下，和 todo demo 的 DB 分开）
3. `keysight::init(&conn)` 建表
4. `.manage(KeysightState { db: Mutex::new(conn), vault_path })`

### Command 内部模式

读操作：
```rust
#[tauri::command]
#[specta::specta]
pub fn card_get(state: State<'_, KeysightState>, id: String) -> Result<AtomicCard, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn, None);
    store.get(&id).map_err(Into::into)
}
```

写操作（需要 VaultFs）：
```rust
#[tauri::command]
#[specta::specta]
pub fn card_edit_title(state: State<'_, KeysightState>, id: String, new_title: String) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let vault_fs = RealVaultFs::new(&state.vault_path);
    let store = SqliteCardStore::new(&conn, Some(&vault_fs));
    store.edit_title(&id, &new_title).map_err(Into::into)
}
```

---

## 2. Command 清单

命名规则：`{domain}_{method}`，specta 自动转 camelCase（`card_get` → `commands.cardGet()`）。

### 读操作（20 个）

| Command | 参数 | 返回值 |
|---------|------|--------|
| `card_get` | `id: String` | `AtomicCard` |
| `card_query_all` | `limit: Option<i64>, offset: Option<i64>` | `Vec<AtomicCard>` |
| `card_query_by_file` | `file_path: String` | `Vec<AtomicCard>` |
| `card_query_by_ids` | `ids: Vec<String>` | `Vec<AtomicCard>` |
| `card_count` | — | `i64` |
| `card_search` | `text: String` | `Vec<AtomicCard>` |
| `card_query_links` | `id: String` | `CardLinksResponse` |
| `section_get` | `id: String` | `GraphSection` |
| `section_query_all` | `whiteboard_id: String` | `Vec<GraphSection>` |
| `note_get` | `id: String` | `GraphNote` |
| `note_query_all` | `whiteboard_id: String` | `Vec<GraphNote>` |
| `alias_get` | `id: String` | `CardAlias` |
| `alias_query_all` | `whiteboard_id: String` | `Vec<CardAlias>` |
| `layout_query_positions` | `whiteboard_id: String` | `HashMap<String, Position>` |
| `entity_edges_from` | `entity_id: String` | `Vec<Edge>` |
| `entity_edges_to` | `entity_id: String` | `Vec<Edge>` |
| `sync_all_file_mtimes` | — | `Vec<(String, f64)>` |
| `overview_stats` | — | `StatsResponse` |
| `overview_graph` | — | `GraphOverviewResponse` |
| `get_vault_info` | — | `VaultInfoResponse` |

### 写操作（20 个）

| Command | 参数 | 返回值 |
|---------|------|--------|
| `card_edit_title` | `id: String, new_title: String` | `()` |
| `card_edit_body` | `id: String, new_body: String` | `()` |
| `card_update_understanding` | `id: String, text: String` | `()` |
| `section_create` | `whiteboard_id: String, title: String, color: Option<String>` | `GraphSection` |
| `section_delete` | `id: String` | `()` |
| `section_update` | `id: String, title: Option<String>, color: Option<String>` | `()` |
| `section_add_member` | `section_id: String, entity_id: String` | `()` |
| `section_remove_member` | `section_id: String, entity_id: String` | `()` |
| `section_move_to_whiteboard` | `section_id: String, target_whiteboard_id: String` | `()` |
| `note_create` | `whiteboard_id: String, title: String, content: Option<String>, color: Option<String>` | `GraphNote` |
| `note_delete` | `id: String` | `()` |
| `note_update` | `id: String, title: Option<String>, content: Option<String>, color: Option<String>` | `()` |
| `alias_create` | `whiteboard_id: String, card_id: String` | `CardAlias` |
| `alias_delete` | `id: String` | `()` |
| `layout_set_position` | `whiteboard_id: String, entity_id: String, x: f64, y: f64` | `()` |
| `layout_remove_position` | `whiteboard_id: String, entity_id: String` | `()` |
| `entity_connect` | `from_id: String, to_id: String, edge_type: EdgeType, style: Option<EdgeStyle>, label: Option<String>` | `()` |
| `entity_disconnect` | `from_id: String, to_id: String, edge_type: EdgeType` | `()` |
| `sync_file` | `file_path: String, content: String, mtime: f64` | `SyncFileResponse` |
| `sync_remove_file` | `file_path: String` | `()` |

### 新增类型

```rust
// models.rs 新增
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VaultInfoResponse {
    pub vault_path: String,
}
```

### 不暴露的函数

| 函数 | 原因 |
|------|------|
| `task::transition_status`, `update_area`, `update_project`, `by_status` | Phase 6 再做，`by_status` 返回 `serde_json::Value` 需先补 typed struct |
| `question::transition_status`, `by_status` | 同上 |
| `sync::derive_whiteboard_id` | 纯计算，内部实现细节 |

---

## 3. 可见性变更

从 `pub(super)` → `pub(in crate::modules::keysight)`，让 commands.rs（同模块不同子模块）能访问。

### 需要变更的文件

| 文件 | 变更项 |
|------|--------|
| `vault_fs.rs` | `trait VaultFs`, `struct RealVaultFs` |
| `domain/card.rs` | `trait CardStore`, `struct SqliteCardStore` |
| `domain/note.rs` | `trait NoteStore`, `struct SqliteNoteStore` |
| `domain/section.rs` | `trait SectionStore`, `struct SqliteSectionStore` |
| `domain/alias.rs` | `trait AliasStore`, `struct SqliteAliasStore` |
| `domain/layout.rs` | `trait LayoutStore`, `struct SqliteLayoutStore` |
| `domain/entity.rs` | `trait EntityGraph`, `struct SqliteEntityGraph` |
| `domain/sync.rs` | `sync_file`, `remove_file`, `all_file_mtimes` |
| `domain/overview.rs` | `stats`, `graph_overview` |
| `errors.rs` | `enum KeysightError` |

### 不变的文件

| 文件 | 原因 |
|------|------|
| `models.rs` | 已经是 `pub`（跨 IPC 需要） |
| `db.rs` | 只有 `mod.rs` 调用 `init_db`，保持 `pub(super)` |
| `id.rs` | domain 内部用，保持 `pub(super)` |
| `parser.rs` | domain 内部用，保持 `pub(super)` |

---

## 4. 模块注册

### mod.rs 变更

```rust
pub mod commands;  // 新增
pub mod models;    // 已有
pub mod state;     // 新增

mod db;
mod domain;
mod errors;
mod id;
mod parser;
mod vault_fs;

pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    db::init_db(conn)
}
```

### lib.rs 变更

- `collect_commands![]` 追加 40 个 keysight command
- `setup` 闭包新增 `KeysightState` 初始化

---

## 5. Operation Contract 模板

commands.rs 每个写操作 command 的 doc comment 遵循此模板：

```rust
/// # {command_name}
///
/// ## 前置条件
/// - {调用前必须满足的条件}
///
/// ## 执行效果
/// 1. {按顺序列出副作用}
///
/// ## 不做的事
/// - {明确列出不会做的事}
///
/// ## 幂等性
/// {重复调用的行为}
///
/// ## 关联操作
/// - [`related_command`] — {关系说明}
```

实现时逐个 command 写具体内容，不在 spec 中重复 40 次。

---

## 6. 副作用矩阵

Phase 2 完成后更新到 CLAUDE.md：

| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|--------|-------------|--------|----------|
| `card_edit_title` | `entities` + markdown 文件 | DB 写 + 文件写 | domain unit test |
| `card_edit_body` | `entities` + markdown 文件 | DB 写 + 文件写 | domain unit test |
| `card_update_understanding` | `card_fields` + markdown 文件 | DB 写 + 文件写 | domain unit test |
| `section_create` | `entities` | DB 写 | domain unit test |
| `section_delete` | `entities`, `section_members`, `edges`, `positions` | 级联删除 | domain unit test |
| `section_update` | `entities` | DB 写 | domain unit test |
| `section_add_member` | `section_members` | DB 写 | domain unit test |
| `section_remove_member` | `section_members` | DB 写 | domain unit test |
| `section_move_to_whiteboard` | `entities`, `positions`, `edges` | 跨白板迁移 + 清理 | domain unit test |
| `note_create` | `entities` | DB 写 | domain unit test |
| `note_delete` | `entities`, `edges`, `positions` | 级联删除 | domain unit test |
| `note_update` | `entities` | DB 写 | domain unit test |
| `alias_create` | `entities`, `alias_fields` | DB 写 | domain unit test |
| `alias_delete` | `entities`, `alias_fields`, `edges` | 级联删除 | domain unit test |
| `layout_set_position` | `positions` | DB 写 | domain unit test |
| `layout_remove_position` | `positions` | DB 写 | domain unit test |
| `entity_connect` | `edges` | DB 写 | domain unit test |
| `entity_disconnect` | `edges` | DB 写 | domain unit test |
| `sync_file` | `entities`, `card_fields`, `entity_tags`, `edges`, `entities_fts` | 全量同步 | domain unit test |
| `sync_remove_file` | `entities`, `entities_fts` | 按文件删除 | domain unit test |

---

## 7. 验证清单

Phase 2 完成时必须通过：

1. `cargo clippy --workspace -- -D warnings` — clean
2. `cargo test --workspace` — 104 keysight tests + existing tests 全部通过
3. `cargo test export_bindings` — bindings.ts 包含 40 个 keysight command
4. `pnpm build` — TS 编译通过（类型链路端到端验证）
5. 副作用矩阵已更新到 CLAUDE.md
6. 每个写操作 command 有 Operation Contract doc comment
