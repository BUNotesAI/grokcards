# KeySight Phase 2: Tauri Commands + Specta 绑定 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose keysight domain functions as 40 type-safe Tauri commands with specta bindings.

**Architecture:** Thin command shell pattern — each command ≤4 lines (lock state, construct store, delegate to domain, convert error). `KeysightState` bundles `Mutex<Connection>` + `vault_path`. Visibility widened from `pub(super)` to `pub(in crate::modules::keysight)` so commands.rs can access domain internals.

**Tech Stack:** Rust, Tauri v2, tauri-specta, rusqlite

**Spec:** `docs/superpowers/specs/2026-04-11-keysight-phase2-design.md`

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src-tauri/src/modules/keysight/state.rs` | `KeysightState` struct |
| Create | `src-tauri/src/modules/keysight/commands.rs` | 40 Tauri command thin shells + Operation Contracts |
| Modify | `src-tauri/src/modules/keysight/models.rs` | Add `VaultInfoResponse` |
| Modify | `src-tauri/src/modules/keysight/mod.rs` | Register `commands` + `state` modules |
| Modify | `src-tauri/src/modules/keysight/errors.rs` | Visibility `pub(super)` → `pub(in crate::modules::keysight)` |
| Modify | `src-tauri/src/modules/keysight/vault_fs.rs` | Visibility change (trait + RealVaultFs) |
| Modify | `src-tauri/src/modules/keysight/domain/card.rs` | Visibility change (trait + SqliteCardStore) |
| Modify | `src-tauri/src/modules/keysight/domain/note.rs` | Visibility change |
| Modify | `src-tauri/src/modules/keysight/domain/section.rs` | Visibility change |
| Modify | `src-tauri/src/modules/keysight/domain/alias.rs` | Visibility change |
| Modify | `src-tauri/src/modules/keysight/domain/layout.rs` | Visibility change |
| Modify | `src-tauri/src/modules/keysight/domain/entity.rs` | Visibility change |
| Modify | `src-tauri/src/modules/keysight/domain/sync.rs` | Visibility change |
| Modify | `src-tauri/src/modules/keysight/domain/overview.rs` | Visibility change |
| Modify | `src-tauri/src/modules/mod.rs` | Remove keysight from `init_all()` |
| Modify | `src-tauri/src/lib.rs` | `KeysightState` init + 40 commands in `collect_commands![]` |
| Modify | `CLAUDE.md` | Fill side effect matrix |

---

### Task 1: Infrastructure — state.rs + visibility changes + VaultInfoResponse

**Files:**
- Create: `src-tauri/src/modules/keysight/state.rs`
- Modify: `src-tauri/src/modules/keysight/models.rs`
- Modify: `src-tauri/src/modules/keysight/errors.rs`
- Modify: `src-tauri/src/modules/keysight/vault_fs.rs`
- Modify: `src-tauri/src/modules/keysight/domain/card.rs`
- Modify: `src-tauri/src/modules/keysight/domain/note.rs`
- Modify: `src-tauri/src/modules/keysight/domain/section.rs`
- Modify: `src-tauri/src/modules/keysight/domain/alias.rs`
- Modify: `src-tauri/src/modules/keysight/domain/layout.rs`
- Modify: `src-tauri/src/modules/keysight/domain/entity.rs`
- Modify: `src-tauri/src/modules/keysight/domain/sync.rs`
- Modify: `src-tauri/src/modules/keysight/domain/overview.rs`

- [ ] **Step 1: Create `state.rs`**

```rust
// src-tauri/src/modules/keysight/state.rs
use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::Connection;

/// KeySight 模块的共享状态，由 Tauri managed state 注入。
pub struct KeysightState {
    /// SQLite 连接（keysight 专用 DB）。
    pub db: Mutex<Connection>,
    /// Obsidian vault 根目录路径。
    pub vault_path: PathBuf,
}
```

- [ ] **Step 2: Add `VaultInfoResponse` to `models.rs`**

在 `models.rs` 文件末尾（`GraphOverviewResponse` 之后）添加：

```rust
/// 当前 vault 配置信息。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VaultInfoResponse {
    pub vault_path: String,
}
```

- [ ] **Step 3: Visibility changes — errors.rs**

`src-tauri/src/modules/keysight/errors.rs` — 将 `pub(super)` 替换为 `pub(in crate::modules::keysight)`：

```rust
// 从
pub(super) enum KeysightError {
// 改为
pub(in crate::modules::keysight) enum KeysightError {
```

- [ ] **Step 4: Visibility changes — vault_fs.rs**

`src-tauri/src/modules/keysight/vault_fs.rs` — 两处替换：

```rust
// trait VaultFs
// 从
pub(super) trait VaultFs {
// 改为
pub(in crate::modules::keysight) trait VaultFs {

// struct RealVaultFs
// 从
pub(super) struct RealVaultFs {
// 改为
pub(in crate::modules::keysight) struct RealVaultFs {
```

- [ ] **Step 5: Visibility changes — domain/card.rs**

两处替换：

```rust
// 从
pub(super) trait CardStore {
// 改为
pub(in crate::modules::keysight) trait CardStore {

// 从
pub(super) struct SqliteCardStore<'a> {
// 改为
pub(in crate::modules::keysight) struct SqliteCardStore<'a> {
```

- [ ] **Step 6: Visibility changes — domain/note.rs**

两处替换：

```rust
// 从
pub(super) trait NoteStore {
// 改为
pub(in crate::modules::keysight) trait NoteStore {

// 从
pub(super) struct SqliteNoteStore<'a> {
// 改为
pub(in crate::modules::keysight) struct SqliteNoteStore<'a> {
```

- [ ] **Step 7: Visibility changes — domain/section.rs**

两处替换：

```rust
// 从
pub(super) trait SectionStore {
// 改为
pub(in crate::modules::keysight) trait SectionStore {

// 从
pub(super) struct SqliteSectionStore<'a> {
// 改为
pub(in crate::modules::keysight) struct SqliteSectionStore<'a> {
```

- [ ] **Step 8: Visibility changes — domain/alias.rs**

两处替换：

```rust
// 从
pub(super) trait AliasStore {
// 改为
pub(in crate::modules::keysight) trait AliasStore {

// 从
pub(super) struct SqliteAliasStore<'a> {
// 改为
pub(in crate::modules::keysight) struct SqliteAliasStore<'a> {
```

- [ ] **Step 9: Visibility changes — domain/layout.rs**

两处替换：

```rust
// 从
pub(super) trait LayoutStore {
// 改为
pub(in crate::modules::keysight) trait LayoutStore {

// 从
pub(super) struct SqliteLayoutStore<'a> {
// 改为
pub(in crate::modules::keysight) struct SqliteLayoutStore<'a> {
```

- [ ] **Step 10: Visibility changes — domain/entity.rs**

两处替换：

```rust
// 从
pub(super) trait EntityGraph {
// 改为
pub(in crate::modules::keysight) trait EntityGraph {

// 从
pub(super) struct SqliteEntityGraph<'a> {
// 改为
pub(in crate::modules::keysight) struct SqliteEntityGraph<'a> {
```

- [ ] **Step 11: Visibility changes — domain/sync.rs**

三处替换（只改暴露给 commands 的函数，`derive_whiteboard_id` 保持 `pub(super)`）：

```rust
// 从
pub(super) fn sync_file(
// 改为
pub(in crate::modules::keysight) fn sync_file(

// 从
pub(super) fn remove_file(conn: &Connection, file_path: &str) -> Result<(), KeysightError> {
// 改为
pub(in crate::modules::keysight) fn remove_file(conn: &Connection, file_path: &str) -> Result<(), KeysightError> {

// 从
pub(super) fn all_file_mtimes(
// 改为
pub(in crate::modules::keysight) fn all_file_mtimes(
```

- [ ] **Step 12: Visibility changes — domain/overview.rs**

两处替换：

```rust
// 从
pub(super) fn stats(conn: &Connection) -> Result<StatsResponse, KeysightError> {
// 改为
pub(in crate::modules::keysight) fn stats(conn: &Connection) -> Result<StatsResponse, KeysightError> {

// 从
pub(super) fn graph_overview(conn: &Connection) -> Result<GraphOverviewResponse, KeysightError> {
// 改为
pub(in crate::modules::keysight) fn graph_overview(conn: &Connection) -> Result<GraphOverviewResponse, KeysightError> {
```

- [ ] **Step 13: Verify compilation**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5`

Expected: compiles with no errors. May have dead_code warnings for the new `state.rs` and widened visibility items — that's OK, they'll be used after Task 2.

- [ ] **Step 14: Commit**

```bash
git add src-tauri/src/modules/keysight/state.rs src-tauri/src/modules/keysight/models.rs src-tauri/src/modules/keysight/errors.rs src-tauri/src/modules/keysight/vault_fs.rs src-tauri/src/modules/keysight/domain/
git commit -m "refactor(keysight): widen visibility for Phase 2 commands layer

Add KeysightState, VaultInfoResponse. Change pub(super) to
pub(in crate::modules::keysight) on traits, structs, and functions
that commands.rs will access."
```

---

### Task 2: commands.rs — 40 Tauri commands with Operation Contracts

**Files:**
- Create: `src-tauri/src/modules/keysight/commands.rs`

**Context:** All domain traits, structs, and functions are now visible to `crate::modules::keysight::commands` thanks to Task 1. Constructor patterns:
- `SqliteCardStore::new(&conn)` — 读操作
- `SqliteCardStore::with_vault_fs(&conn, &vault_fs)` — 写操作（需要文件回写）
- `SqliteNoteStore::new(&conn)`, `SqliteSectionStore::new(&conn)`, etc. — 统一 `new(&conn)`
- `SqliteEntityGraph::new(&conn)` — entity graph
- `sync::sync_file(&conn, ...)`, `overview::stats(&conn)` — 独立函数

- [ ] **Step 1: Create `commands.rs` with full content**

创建 `src-tauri/src/modules/keysight/commands.rs`，完整内容如下：

```rust
use std::collections::HashMap;

use tauri::State;

use super::domain::alias::{AliasStore, SqliteAliasStore};
use super::domain::card::{CardStore, SqliteCardStore};
use super::domain::entity::{EntityGraph, SqliteEntityGraph};
use super::domain::layout::{LayoutStore, SqliteLayoutStore};
use super::domain::note::{NoteStore, SqliteNoteStore};
use super::domain::section::{SectionStore, SqliteSectionStore};
use super::domain::{overview, sync};
use super::models::{
    AtomicCard, CardAlias, CardLinksResponse, Edge, EdgeStyle, EdgeType, GraphNote,
    GraphOverviewResponse, GraphSection, Position, StatsResponse, SyncFileResponse,
    VaultInfoResponse,
};
use super::state::KeysightState;
use super::vault_fs::RealVaultFs;
use crate::app_error::AppError;

// ============================================================
// Card 读操作
// ============================================================

/// 按 ID 查询单张卡片。
#[tauri::command]
#[specta::specta]
pub fn card_get(state: State<'_, KeysightState>, id: String) -> Result<AtomicCard, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询所有卡片，按 mtime 降序，支持分页。
#[tauri::command]
#[specta::specta]
pub fn card_query_all(
    state: State<'_, KeysightState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<AtomicCard>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_all(limit, offset).map_err(Into::into)
}

/// 按文件路径查询卡片。
#[tauri::command]
#[specta::specta]
pub fn card_query_by_file(
    state: State<'_, KeysightState>,
    file_path: String,
) -> Result<Vec<AtomicCard>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_by_file(&file_path).map_err(Into::into)
}

/// 按 ID 列表批量查询卡片。
#[tauri::command]
#[specta::specta]
pub fn card_query_by_ids(
    state: State<'_, KeysightState>,
    ids: Vec<String>,
) -> Result<Vec<AtomicCard>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_by_ids(&ids).map_err(Into::into)
}

/// 卡片总数。
#[tauri::command]
#[specta::specta]
pub fn card_count(state: State<'_, KeysightState>) -> Result<i64, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.count().map_err(Into::into)
}

/// 全文搜索卡片（FTS5 + LIKE fallback）。
#[tauri::command]
#[specta::specta]
pub fn card_search(
    state: State<'_, KeysightState>,
    text: String,
) -> Result<Vec<AtomicCard>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.search(&text).map_err(Into::into)
}

/// 查询单卡片的完整链接图谱（出边 + 入边）。
#[tauri::command]
#[specta::specta]
pub fn card_query_links(
    state: State<'_, KeysightState>,
    id: String,
) -> Result<CardLinksResponse, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_links(&id).map_err(Into::into)
}

// ============================================================
// Card 写操作
// ============================================================

/// # card_edit_title
///
/// ## 前置条件
/// - id 对应的 card 必须存在（否则 NotFound）
/// - new_title trim 后非空（否则 EmptyTitle）
///
/// ## 执行效果
/// 1. 通过 VaultFs 读取 markdown 文件，替换 H1 标题行
/// 2. 通过 VaultFs 写回更新后的文件
/// 3. 更新 entities 表的 title 字段
///
/// ## 不做的事
/// - 不更新 FTS 索引（需 sync_file 触发）
/// - 不触发前端事件通知
///
/// ## 幂等性
/// 幂等 — 相同 title 重复调用无额外效果
///
/// ## 关联操作
/// - [`card_edit_body`] — 编辑正文
/// - [`sync_file`] — 含 FTS 更新的全量同步
#[tauri::command]
#[specta::specta]
pub fn card_edit_title(
    state: State<'_, KeysightState>,
    id: String,
    new_title: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let vault_fs = RealVaultFs::new(state.vault_path.to_string_lossy().into_owned());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.edit_title(&id, &new_title).map_err(Into::into)
}

/// # card_edit_body
///
/// ## 前置条件
/// - id 对应的 card 必须存在（否则 NotFound）
///
/// ## 执行效果
/// 1. 通过 VaultFs 读取 markdown 文件
/// 2. 保留 frontmatter + H1，替换 H1 之后的正文
/// 3. 通过 VaultFs 写回更新后的文件
/// 4. 更新 entities 表的 content 字段
///
/// ## 不做的事
/// - 不更新 FTS 索引（需 sync_file 触发）
/// - 不修改 frontmatter 和标题
///
/// ## 幂等性
/// 幂等 — 相同 body 重复调用无额外效果
///
/// ## 关联操作
/// - [`card_edit_title`] — 编辑标题
/// - [`sync_file`] — 含 FTS 更新的全量同步
#[tauri::command]
#[specta::specta]
pub fn card_edit_body(
    state: State<'_, KeysightState>,
    id: String,
    new_body: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let vault_fs = RealVaultFs::new(state.vault_path.to_string_lossy().into_owned());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.edit_body(&id, &new_body).map_err(Into::into)
}

/// # card_update_understanding
///
/// ## 前置条件
/// - id 对应的 card 必须存在（否则 NotFound）
///
/// ## 执行效果
/// 1. 通过 VaultFs 读取 markdown 文件
/// 2. 更新 frontmatter 中的 understanding 字段
/// 3. 通过 VaultFs 写回更新后的文件
/// 4. 更新 card_fields 表的 understanding 字段
///
/// ## 不做的事
/// - 不更新 FTS 索引
/// - 不修改正文和标题
///
/// ## 幂等性
/// 幂等 — 相同 text 重复调用无额外效果
///
/// ## 关联操作
/// - [`card_edit_title`] / [`card_edit_body`] — 其他卡片编辑操作
#[tauri::command]
#[specta::specta]
pub fn card_update_understanding(
    state: State<'_, KeysightState>,
    id: String,
    text: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let vault_fs = RealVaultFs::new(state.vault_path.to_string_lossy().into_owned());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.update_understanding(&id, &text).map_err(Into::into)
}

// ============================================================
// Section
// ============================================================

/// 按 ID 查询单个 section。
#[tauri::command]
#[specta::specta]
pub fn section_get(
    state: State<'_, KeysightState>,
    id: String,
) -> Result<GraphSection, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 section。
#[tauri::command]
#[specta::specta]
pub fn section_query_all(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
) -> Result<Vec<GraphSection>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store.query_all(&whiteboard_id).map_err(Into::into)
}

/// # section_create
///
/// ## 前置条件
/// - title 非空
///
/// ## 执行效果
/// 1. 生成 sec_ 前缀的唯一 ID
/// 2. 插入 entities 表（kind=section）
///
/// ## 不做的事
/// - 不自动添加成员（需调用 section_add_member）
/// - 不设置位置（需调用 layout_set_position）
///
/// ## 幂等性
/// 非幂等 — 每次调用创建新 section
///
/// ## 关联操作
/// - [`section_add_member`] — 添加成员
/// - [`section_delete`] — 删除
#[tauri::command]
#[specta::specta]
pub fn section_create(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
    title: String,
    color: Option<String>,
) -> Result<GraphSection, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .create(&whiteboard_id, &title, color.as_deref())
        .map_err(Into::into)
}

/// # section_delete
///
/// ## 前置条件
/// - id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 删除 entities 表中的 section 行
/// 2. 删除 section_members 中该 section 的所有成员关系
/// 3. 删除 edges 中该 section 的所有边
/// 4. 删除 positions 中该 section 的位置
///
/// ## 不做的事
/// - 不删除成员实体本身（只删除成员关系）
///
/// ## 幂等性
/// 幂等 — 已删除的 section 再次删除不报错（NotFound）
///
/// ## 关联操作
/// - [`section_create`] — 创建（逆操作）
#[tauri::command]
#[specta::specta]
pub fn section_delete(state: State<'_, KeysightState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store.delete(&id).map_err(Into::into)
}

/// # section_update
///
/// ## 前置条件
/// - id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 更新 entities 表中 title 和/或 color（仅更新提供的字段）
///
/// ## 不做的事
/// - 不修改成员列表
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`section_create`] — 创建
#[tauri::command]
#[specta::specta]
pub fn section_update(
    state: State<'_, KeysightState>,
    id: String,
    title: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .update(&id, title.as_deref(), color.as_deref())
        .map_err(Into::into)
}

/// # section_add_member
///
/// ## 前置条件
/// - section_id 和 entity_id 都必须存在
///
/// ## 执行效果
/// 1. 插入 section_members 表
///
/// ## 不做的事
/// - 不设置成员位置
///
/// ## 幂等性
/// 幂等 — 重复添加同一成员无额外效果（INSERT OR IGNORE）
///
/// ## 关联操作
/// - [`section_remove_member`] — 移除成员（逆操作）
#[tauri::command]
#[specta::specta]
pub fn section_add_member(
    state: State<'_, KeysightState>,
    section_id: String,
    entity_id: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store.add_member(&section_id, &entity_id).map_err(Into::into)
}

/// # section_remove_member
///
/// ## 前置条件
/// - section_id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 删除 section_members 表中该行
///
/// ## 幂等性
/// 幂等 — 不存在的成员关系删除无效果
///
/// ## 关联操作
/// - [`section_add_member`] — 添加成员（逆操作）
#[tauri::command]
#[specta::specta]
pub fn section_remove_member(
    state: State<'_, KeysightState>,
    section_id: String,
    entity_id: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .remove_member(&section_id, &entity_id)
        .map_err(Into::into)
}

/// # section_move_to_whiteboard
///
/// ## 前置条件
/// - section_id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 更新 entities 表中 section 的 whiteboard_id
/// 2. 清理 positions 表中旧白板的位置
/// 3. 清理跨白板的 section_link edges
///
/// ## 不做的事
/// - 不移动 section 的成员实体
/// - 不在目标白板设置位置
///
/// ## 幂等性
/// 幂等 — 移动到同一白板无额外效果
///
/// ## 关联操作
/// - [`section_create`] — 创建新 section
/// - [`layout_set_position`] — 在目标白板设置位置
#[tauri::command]
#[specta::specta]
pub fn section_move_to_whiteboard(
    state: State<'_, KeysightState>,
    section_id: String,
    target_whiteboard_id: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .move_to_whiteboard(&section_id, &target_whiteboard_id)
        .map_err(Into::into)
}

// ============================================================
// Note
// ============================================================

/// 按 ID 查询单个 note。
#[tauri::command]
#[specta::specta]
pub fn note_get(state: State<'_, KeysightState>, id: String) -> Result<GraphNote, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteNoteStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 note。
#[tauri::command]
#[specta::specta]
pub fn note_query_all(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
) -> Result<Vec<GraphNote>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteNoteStore::new(&conn);
    store.query_all(&whiteboard_id).map_err(Into::into)
}

/// # note_create
///
/// ## 前置条件
/// - title 非空
///
/// ## 执行效果
/// 1. 生成 note_ 前缀的唯一 ID
/// 2. 插入 entities 表（kind=note）
///
/// ## 幂等性
/// 非幂等 — 每次调用创建新 note
///
/// ## 关联操作
/// - [`note_delete`] — 删除（逆操作）
/// - [`note_update`] — 更新
#[tauri::command]
#[specta::specta]
pub fn note_create(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
    title: String,
    content: Option<String>,
    color: Option<String>,
) -> Result<GraphNote, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteNoteStore::new(&conn);
    store
        .create(&whiteboard_id, &title, content.as_deref(), color.as_deref())
        .map_err(Into::into)
}

/// # note_delete
///
/// ## 前置条件
/// - id 对应的 note 必须存在
///
/// ## 执行效果
/// 1. 删除 entities 表中的 note 行
/// 2. 删除 edges 中该 note 的所有边
/// 3. 删除 positions 中该 note 的位置
///
/// ## 幂等性
/// 幂等 — 已删除的 note 再次删除报 NotFound
///
/// ## 关联操作
/// - [`note_create`] — 创建（逆操作）
#[tauri::command]
#[specta::specta]
pub fn note_delete(state: State<'_, KeysightState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteNoteStore::new(&conn);
    store.delete(&id).map_err(Into::into)
}

/// # note_update
///
/// ## 前置条件
/// - id 对应的 note 必须存在
///
/// ## 执行效果
/// 1. 更新 entities 表中 title / content / color（仅更新提供的字段）
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`note_create`] — 创建
#[tauri::command]
#[specta::specta]
pub fn note_update(
    state: State<'_, KeysightState>,
    id: String,
    title: Option<String>,
    content: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteNoteStore::new(&conn);
    store
        .update(&id, title.as_deref(), content.as_deref(), color.as_deref())
        .map_err(Into::into)
}

// ============================================================
// Alias
// ============================================================

/// 按 ID 查询单个 alias。
#[tauri::command]
#[specta::specta]
pub fn alias_get(state: State<'_, KeysightState>, id: String) -> Result<CardAlias, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 alias。
#[tauri::command]
#[specta::specta]
pub fn alias_query_all(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
) -> Result<Vec<CardAlias>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.query_all(&whiteboard_id).map_err(Into::into)
}

/// # alias_create
///
/// ## 前置条件
/// - card_id 对应的 card 必须存在
///
/// ## 执行效果
/// 1. 生成 alias_ 前缀的唯一 ID
/// 2. 插入 entities 表（kind=alias）+ alias_fields 表
///
/// ## 幂等性
/// 非幂等 — 每次调用创建新 alias
///
/// ## 关联操作
/// - [`alias_delete`] — 删除（逆操作）
#[tauri::command]
#[specta::specta]
pub fn alias_create(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
    card_id: String,
) -> Result<CardAlias, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.create(&whiteboard_id, &card_id).map_err(Into::into)
}

/// # alias_delete
///
/// ## 前置条件
/// - id 对应的 alias 必须存在
///
/// ## 执行效果
/// 1. 删除 entities 表中的 alias 行
/// 2. 删除 alias_fields 中该 alias 的行
/// 3. 删除 edges 中该 alias 的所有边
///
/// ## 幂等性
/// 幂等 — 已删除的 alias 再次删除报 NotFound
///
/// ## 关联操作
/// - [`alias_create`] — 创建（逆操作）
#[tauri::command]
#[specta::specta]
pub fn alias_delete(state: State<'_, KeysightState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.delete(&id).map_err(Into::into)
}

// ============================================================
// Layout
// ============================================================

/// 查询指定白板所有实体的位置。
#[tauri::command]
#[specta::specta]
pub fn layout_query_positions(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
) -> Result<HashMap<String, Position>, AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteLayoutStore::new(&conn);
    store.query_positions(&whiteboard_id).map_err(Into::into)
}

/// # layout_set_position
///
/// ## 前置条件
/// - 无（upsert 语义）
///
/// ## 执行效果
/// 1. INSERT OR REPLACE into positions 表
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`layout_remove_position`] — 移除位置（逆操作）
/// - [`layout_query_positions`] — 查询
#[tauri::command]
#[specta::specta]
pub fn layout_set_position(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
    entity_id: String,
    x: f64,
    y: f64,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteLayoutStore::new(&conn);
    store
        .set_position(&whiteboard_id, &entity_id, x, y)
        .map_err(Into::into)
}

/// # layout_remove_position
///
/// ## 前置条件
/// - 无
///
/// ## 执行效果
/// 1. 删除 positions 表中该行
///
/// ## 幂等性
/// 幂等 — 不存在则无效果
///
/// ## 关联操作
/// - [`layout_set_position`] — 设置位置（逆操作）
#[tauri::command]
#[specta::specta]
pub fn layout_remove_position(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
    entity_id: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let store = SqliteLayoutStore::new(&conn);
    store
        .remove_position(&whiteboard_id, &entity_id)
        .map_err(Into::into)
}

// ============================================================
// Entity Graph
// ============================================================

/// 查询指定实体的出边。
#[tauri::command]
#[specta::specta]
pub fn entity_edges_from(
    state: State<'_, KeysightState>,
    entity_id: String,
) -> Result<Vec<Edge>, AppError> {
    let conn = state.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph.edges_from(&entity_id).map_err(Into::into)
}

/// 查询指定实体的入边。
#[tauri::command]
#[specta::specta]
pub fn entity_edges_to(
    state: State<'_, KeysightState>,
    entity_id: String,
) -> Result<Vec<Edge>, AppError> {
    let conn = state.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph.edges_to(&entity_id).map_err(Into::into)
}

/// # entity_connect
///
/// ## 前置条件
/// - from_id 和 to_id 对应的实体应存在（不强制校验）
///
/// ## 执行效果
/// 1. 插入 edges 表
///
/// ## 幂等性
/// 幂等 — 相同边重复插入无额外效果（主键约束）
///
/// ## 关联操作
/// - [`entity_disconnect`] — 删除边（逆操作）
#[tauri::command]
#[specta::specta]
pub fn entity_connect(
    state: State<'_, KeysightState>,
    from_id: String,
    to_id: String,
    edge_type: EdgeType,
    style: Option<EdgeStyle>,
    label: Option<String>,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph
        .connect(&from_id, &to_id, edge_type, style, label.as_deref())
        .map_err(Into::into)
}

/// # entity_disconnect
///
/// ## 前置条件
/// - 无
///
/// ## 执行效果
/// 1. 删除 edges 表中匹配的行
///
/// ## 幂等性
/// 幂等 — 不存在则无效果
///
/// ## 关联操作
/// - [`entity_connect`] — 创建边（逆操作）
#[tauri::command]
#[specta::specta]
pub fn entity_disconnect(
    state: State<'_, KeysightState>,
    from_id: String,
    to_id: String,
    edge_type: EdgeType,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph
        .disconnect(&from_id, &to_id, edge_type)
        .map_err(Into::into)
}

// ============================================================
// Sync
// ============================================================

/// # sync_file
///
/// ## 前置条件
/// - file_path 非空
///
/// ## 执行效果
/// 1. 解析 markdown（frontmatter + 正文）
/// 2. 插入或更新 entities / card_fields / entity_tags / edges
/// 3. 更新 file_mtimes 表
/// 4. 同步 FTS5 索引（entities_fts）
///
/// ## 不做的事
/// - 不读取真实文件（content 由调用方提供）
///
/// ## 幂等性
/// 幂等 — 相同内容重复调用 → updated=1, inserted=0
///
/// ## 关联操作
/// - [`sync_remove_file`] — 按文件删除（逆操作）
/// - [`sync_all_file_mtimes`] — 查询所有文件时间戳
#[tauri::command]
#[specta::specta]
pub fn sync_file(
    state: State<'_, KeysightState>,
    file_path: String,
    content: String,
    mtime: f64,
) -> Result<SyncFileResponse, AppError> {
    let conn = state.db.lock().unwrap();
    sync::sync_file(&conn, &file_path, &content, mtime).map_err(Into::into)
}

/// # sync_remove_file
///
/// ## 前置条件
/// - 无（不存在的文件删除无效果）
///
/// ## 执行效果
/// 1. 删除 entities 中 file_path 匹配的所有实体
/// 2. 删除 file_mtimes 中的记录
/// 3. 清理 FTS5 索引
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`sync_file`] — 同步文件（逆操作）
#[tauri::command]
#[specta::specta]
pub fn sync_remove_file(
    state: State<'_, KeysightState>,
    file_path: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    sync::remove_file(&conn, &file_path).map_err(Into::into)
}

/// 查询所有已同步文件的 mtime。
#[tauri::command]
#[specta::specta]
pub fn sync_all_file_mtimes(
    state: State<'_, KeysightState>,
) -> Result<Vec<(String, f64)>, AppError> {
    let conn = state.db.lock().unwrap();
    sync::all_file_mtimes(&conn).map_err(Into::into)
}

// ============================================================
// Overview
// ============================================================

/// 查询全局统计信息。
#[tauri::command]
#[specta::specta]
pub fn overview_stats(state: State<'_, KeysightState>) -> Result<StatsResponse, AppError> {
    let conn = state.db.lock().unwrap();
    overview::stats(&conn).map_err(Into::into)
}

/// 查询按白板聚合的图谱总览。
#[tauri::command]
#[specta::specta]
pub fn overview_graph(
    state: State<'_, KeysightState>,
) -> Result<GraphOverviewResponse, AppError> {
    let conn = state.db.lock().unwrap();
    overview::graph_overview(&conn).map_err(Into::into)
}

// ============================================================
// Config
// ============================================================

/// 返回当前 vault 配置信息。
#[tauri::command]
#[specta::specta]
pub fn get_vault_info(state: State<'_, KeysightState>) -> Result<VaultInfoResponse, AppError> {
    Ok(VaultInfoResponse {
        vault_path: state.vault_path.to_string_lossy().into_owned(),
    })
}
```

- [ ] **Step 2: Verify syntax**

Run: `cd src-tauri && cargo check 2>&1 | head -20`

Expected: 可能因为 mod.rs 还没注册 commands 模块而看不到此文件。如果 cargo check 报的只是 dead_code / unused，说明代码本身语法正确。实际编译验证在 Task 3 注册后进行。

---

### Task 3: Module registration + KeysightState initialization

**Files:**
- Modify: `src-tauri/src/modules/keysight/mod.rs`
- Modify: `src-tauri/src/modules/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update `keysight/mod.rs`**

```rust
pub mod commands;
pub mod models;
pub mod state;

mod db;
mod domain;
mod errors;
mod id;
mod parser;
mod vault_fs;

/// 初始化 keysight 模块的数据库表。
pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    db::init_db(conn)
}
```

- [ ] **Step 2: Update `modules/mod.rs` — 移除 keysight 的 init_all 调用**

keysight 使用独立 DB，不再和 todo 共享初始化。

```rust
pub mod keysight;
pub mod todo;

/// 初始化 todo 模块的数据库表（keysight 有独立 DB，在 lib.rs 单独初始化）。
pub fn init_all(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    todo::init(conn)?;
    Ok(())
}
```

- [ ] **Step 3: Update `lib.rs` — KeysightState init + collect_commands**

完整替换 `src-tauri/src/lib.rs`：

```rust
use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

mod app_error;
mod modules;

fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        // todo commands
        modules::todo::commands::list_todos,
        modules::todo::commands::create_todo,
        modules::todo::commands::update_todo,
        modules::todo::commands::toggle_todo,
        modules::todo::commands::delete_todo,
        modules::todo::commands::toggle_all,
        modules::todo::commands::clear_completed,
        // keysight: card
        modules::keysight::commands::card_get,
        modules::keysight::commands::card_query_all,
        modules::keysight::commands::card_query_by_file,
        modules::keysight::commands::card_query_by_ids,
        modules::keysight::commands::card_count,
        modules::keysight::commands::card_search,
        modules::keysight::commands::card_query_links,
        modules::keysight::commands::card_edit_title,
        modules::keysight::commands::card_edit_body,
        modules::keysight::commands::card_update_understanding,
        // keysight: section
        modules::keysight::commands::section_get,
        modules::keysight::commands::section_query_all,
        modules::keysight::commands::section_create,
        modules::keysight::commands::section_delete,
        modules::keysight::commands::section_update,
        modules::keysight::commands::section_add_member,
        modules::keysight::commands::section_remove_member,
        modules::keysight::commands::section_move_to_whiteboard,
        // keysight: note
        modules::keysight::commands::note_get,
        modules::keysight::commands::note_query_all,
        modules::keysight::commands::note_create,
        modules::keysight::commands::note_delete,
        modules::keysight::commands::note_update,
        // keysight: alias
        modules::keysight::commands::alias_get,
        modules::keysight::commands::alias_query_all,
        modules::keysight::commands::alias_create,
        modules::keysight::commands::alias_delete,
        // keysight: layout
        modules::keysight::commands::layout_query_positions,
        modules::keysight::commands::layout_set_position,
        modules::keysight::commands::layout_remove_position,
        // keysight: entity graph
        modules::keysight::commands::entity_edges_from,
        modules::keysight::commands::entity_edges_to,
        modules::keysight::commands::entity_connect,
        modules::keysight::commands::entity_disconnect,
        // keysight: sync
        modules::keysight::commands::sync_file,
        modules::keysight::commands::sync_remove_file,
        modules::keysight::commands::sync_all_file_mtimes,
        // keysight: overview
        modules::keysight::commands::overview_stats,
        modules::keysight::commands::overview_graph,
        // keysight: config
        modules::keysight::commands::get_vault_info,
    ])
}

/// 初始化 todo 的 SQLite 连接并建表。
fn init_todo_database() -> Connection {
    let conn = Connection::open("super_tauri.db").expect("无法打开数据库");
    modules::init_all(&conn).expect("建表失败");
    conn
}

/// 初始化 KeySight 的独立 SQLite 连接 + vault 路径。
fn init_keysight_state() -> modules::keysight::state::KeysightState {
    let vault_path = std::env::var("KEYSIGHT_VAULT_PATH")
        .expect("环境变量 KEYSIGHT_VAULT_PATH 未设置，请设置为 Obsidian vault 根目录路径");
    let conn = Connection::open("keysight.db").expect("无法打开 keysight 数据库");
    modules::keysight::init(&conn).expect("keysight 建表失败");
    modules::keysight::state::KeysightState {
        db: Mutex::new(conn),
        vault_path: PathBuf::from(vault_path),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = make_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    let todo_conn = init_todo_database();
    let keysight_state = init_keysight_state();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(todo_conn))
        .manage(keysight_state)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_bindings() {
        make_builder()
            .export(Typescript::default(), "../src/bindings.ts")
            .expect("Failed to export typescript bindings");
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -10`

Expected: 编译通过，无 error。可能有 `dead_code` warning（`#![allow(dead_code)]` 在 domain 文件顶部已声明）。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/commands.rs src-tauri/src/modules/keysight/mod.rs src-tauri/src/modules/mod.rs src-tauri/src/lib.rs
git commit -m "feat(keysight): Phase 2 — 40 Tauri commands with specta bindings

Commands layer: thin shells delegating to domain functions.
KeysightState: independent DB + vault_path from env var.
All 20 write commands have Operation Contract doc comments."
```

---

### Task 4: Verification pipeline

**Files:** none (read-only verification)

- [ ] **Step 1: Rust lint**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings`

Expected: 无 error，无 warning（或仅 `dead_code` 在 `#![allow]` 范围内）。

- [ ] **Step 2: Rust tests**

Run: `cd src-tauri && cargo test --workspace -- -q 2>&1 | tail -5`

Expected: `104 passed`（keysight）+ todo tests，`test result: ok`。

- [ ] **Step 3: Export bindings**

Run: `cd src-tauri && cargo test export_bindings -- --nocapture 2>&1 | tail -3`

Expected: `test tests::export_bindings ... ok`。

- [ ] **Step 4: Verify bindings content**

Run: `grep -c 'function ' ../src/bindings.ts` 或 `grep 'cardGet\|sectionCreate\|syncFile\|getVaultInfo' ../src/bindings.ts`

Expected: bindings.ts 中出现 40 个 keysight command 的 camelCase 函数名。

- [ ] **Step 5: TS build**

Run: `cd .. && pnpm build 2>&1 | tail -5`

Expected: `vite build` 成功，tsc 类型检查通过。

- [ ] **Step 6: 如果任何步骤失败**

诊断并修复。常见问题：
- specta 不支持 `HashMap` → 改用 typed struct
- specta 不支持 `(String, f64)` tuple → 改用 typed struct `FileMtime { path: String, mtime: f64 }`
- `pub(in ...)` 可见性不够 → 检查 domain/mod.rs 的 module 可见性
- `init_keysight_state()` 在 test 中 panic（env var 未设置）→ export_bindings test 不调用 `run()`，只调用 `make_builder()`，不受影响

---

### Task 5: CLAUDE.md side effect matrix update

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: 定位副作用矩阵**

在 `CLAUDE.md` 中找到 `### 测试：副作用矩阵` 段落，当前内容为：

```markdown
| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|---|---|---|---|
| （项目初始，待 TodoMVC 实现后填充） | | | |
```

- [ ] **Step 2: 替换为完整矩阵**

替换为：

```markdown
| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|---|---|---|---|
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
```

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: fill side effect matrix with keysight write operations"
```
