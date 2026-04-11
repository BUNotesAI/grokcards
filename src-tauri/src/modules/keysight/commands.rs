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
