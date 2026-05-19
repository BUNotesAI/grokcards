//! Keysight Tauri commands —— 按 entity 拆分子模块,每个 `#[tauri::command]` fn 放
//! 对应文件;`pub use {entity}::*;` 让外部路径 `modules::keysight::commands::{cmd}`
//! 保持不变(`lib.rs collect_commands![]` 一行不改、bindings.ts 一字节不变)。
//!
//! 历史:原 `commands.rs` 单文件 1538 行 / 58 cmd,W1-W3 分波拆为 11 个 entity
//! 子文件 + 本 mod.rs(Config 段 1 cmd `get_vault_info` 体量过小,W3 末并入此处)。
//! 详细决策见 task_a60ceca2 design-v1。
//!
//! W1 状态(本 commit):已拆 4 段(card / section / task / entity_graph),
//! Question / Note / Alias / Layout / Sync / Overview / Config / Legacy Import 7 段
//! 仍内联在本 mod.rs,待 W2 / W3 继续抽出。

mod card;
mod entity_graph;
mod section;
mod task;

pub use card::*;
pub use entity_graph::*;
pub use section::*;
pub use task::*;

use rusqlite::Connection;
use tauri::State;

use super::domain::alias::{AliasStore, SqliteAliasStore};
use super::domain::edge::EntityId;
use super::domain::id::{AliasId, CardId, NoteId, QuestionId, WhiteboardId};
use super::domain::layout::{LayoutStore, SqliteLayoutStore};
use super::domain::legacy_import::{LegacyImporter, SqliteLegacyImporter, SqliteLegacyReader};
use super::domain::note::{NoteStore, SqliteNoteStore};
use super::domain::{overview, question, sync, whiteboard};
use super::models::{
    CardAlias, GraphNote, GraphOverviewResponse, ImportSummary, NoteFileMigrationReport,
    Position, QuestionEntity, StatsResponse, SyncFileResponse, SyncVaultReport,
    VaultInfoResponse, WhiteboardSummary,
};
use super::recording_vault_fs::RecordingVaultFs;
use super::runtime_state::KeysightRuntimeState;
use crate::app_error::AppError;
use crate::perf::{lock_db, ScopedTimer};

use std::collections::HashMap;

// ============================================================
// Question
// ============================================================

/// 查询指定白板的所有 question。
#[tauri::command]
#[specta::specta]
pub fn question_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
) -> Result<Vec<QuestionEntity>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:question_query_all");
    let conn = lock_db(&state.core.db, "question_query_all");
    question::query_all(&conn, &whiteboard_id).map_err(Into::into)
}

/// 创建新的 question markdown 文件并同步入库。
#[tauri::command]
#[specta::specta]
pub fn question_create(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
    title: String,
    content: Option<String>,
    status: Option<String>,
    color: Option<String>,
) -> Result<QuestionEntity, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:question_create");
    let conn = lock_db(&state.core.db, "question_create");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    question::create(
        &conn,
        &vault_fs,
        &whiteboard_id,
        &title,
        content.as_deref(),
        status.as_deref(),
        color.as_deref(),
    )
    .map_err(Into::into)
}

/// 更新 question markdown 文件并重新同步。
///
/// color 语义(和 note_update 一致):
/// - `None` → 保留 current.color
/// - `Some("default")` → 清空 color
/// - `Some(other)` → 覆盖为 `other`
#[tauri::command]
#[specta::specta]
pub fn question_update(
    state: State<'_, KeysightRuntimeState>,
    id: QuestionId,
    title: Option<String>,
    content: Option<String>,
    status: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:question_update");
    let conn = lock_db(&state.core.db, "question_update");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    question::update(
        &conn,
        &vault_fs,
        &id,
        title.as_deref(),
        content.as_deref(),
        status.as_deref(),
        color.as_deref(),
    )
    .map_err(Into::into)
}

/// 删除 question markdown 文件并清理数据库。
#[tauri::command]
#[specta::specta]
pub fn question_delete(
    state: State<'_, KeysightRuntimeState>,
    id: QuestionId,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:question_delete");
    let conn = lock_db(&state.core.db, "question_delete");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    question::delete(&conn, &vault_fs, &id).map_err(Into::into)
}

// ============================================================
// Note
// ============================================================

/// 按 ID 查询单个 note。
#[tauri::command]
#[specta::specta]
pub fn note_get(state: State<'_, KeysightRuntimeState>, id: NoteId) -> Result<GraphNote, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteNoteStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 note。
#[tauri::command]
#[specta::specta]
pub fn note_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
) -> Result<Vec<GraphNote>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:note_query_all");
    let conn = lock_db(&state.core.db, "note_query_all");
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
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
    title: String,
    content: Option<String>,
    color: Option<String>,
) -> Result<GraphNote, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
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
pub fn note_delete(state: State<'_, KeysightRuntimeState>, id: NoteId) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
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
    state: State<'_, KeysightRuntimeState>,
    id: NoteId,
    title: Option<String>,
    content: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:note_update");
    let conn = lock_db(&state.core.db, "note_update");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
    store
        .update(&id, title.as_deref(), content.as_deref(), color.as_deref())
        .map_err(Into::into)
}

/// 备份 DB 和 `whiteboard/` 后，把 DB-only notes 导出成 markdown 文件。
#[tauri::command]
#[specta::specta]
pub fn note_migrate_to_files(
    state: State<'_, KeysightRuntimeState>,
) -> Result<NoteFileMigrationReport, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:note_migrate_to_files");
    let conn = lock_db(&state.core.db, "note_migrate_to_files");
    super::domain::note::migrate_db_notes_to_files(&conn, &state.core.db_path, &state.core.vault_path)
        .map_err(Into::into)
}

// ============================================================
// Alias
// ============================================================

/// 按 ID 查询单个 alias。
#[tauri::command]
#[specta::specta]
pub fn alias_get(state: State<'_, KeysightRuntimeState>, id: AliasId) -> Result<CardAlias, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 alias。
#[tauri::command]
#[specta::specta]
pub fn alias_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
) -> Result<Vec<CardAlias>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:alias_query_all");
    let conn = lock_db(&state.core.db, "alias_query_all");
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
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
    card_id: CardId,
) -> Result<CardAlias, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
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
pub fn alias_delete(state: State<'_, KeysightRuntimeState>, id: AliasId) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
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
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
) -> Result<HashMap<String, Position>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:layout_query_positions");
    let conn = lock_db(&state.core.db, "layout_query_positions");
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
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
    entity_id: EntityId,
    x: f64,
    y: f64,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:layout_set_position");
    let conn = lock_db(&state.core.db, "layout_set_position");
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
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
    entity_id: EntityId,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteLayoutStore::new(&conn);
    store
        .remove_position(&whiteboard_id, &entity_id)
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
    state: State<'_, KeysightRuntimeState>,
    file_path: String,
    content: String,
    mtime: f64,
) -> Result<SyncFileResponse, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
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
    state: State<'_, KeysightRuntimeState>,
    file_path: String,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    sync::remove_file(&conn, &file_path).map_err(Into::into)
}

/// 查询所有已同步文件的 mtime。
#[tauri::command]
#[specta::specta]
pub fn sync_all_file_mtimes(
    state: State<'_, KeysightRuntimeState>,
) -> Result<Vec<(String, f64)>, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    sync::all_file_mtimes(&conn).map_err(Into::into)
}

/// # sync_vault
///
/// ## 前置条件
/// - KEYSIGHT_VAULT_PATH 已设置且目录存在
///
/// ## 执行效果
/// 1. 扫描 whiteboard/ 下所有 .md 文件
/// 2. mtime diff → 同步变更文件到 DB
/// 3. 清理孤儿（DB 有但文件不存在的实体）
/// 4. 回写缺少 id 的文件 frontmatter
///
/// ## 幂等性
/// 幂等 — mtime 未变的文件不重复同步
///
/// ## 关联操作
/// - [`sync_file`] — 单文件同步（内部调用）
/// - [`sync_remove_file`] — 删除文件（内部调用）
#[tauri::command]
#[specta::specta]
pub fn sync_vault(state: State<'_, KeysightRuntimeState>) -> Result<SyncVaultReport, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:sync_vault");
    let conn = lock_db(&state.core.db, "sync_vault");
    let fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    sync::sync_vault(&conn, &fs).map_err(Into::into)
}

// ============================================================
// Overview
// ============================================================

/// 查询全局统计信息。
#[tauri::command]
#[specta::specta]
pub fn overview_stats(state: State<'_, KeysightRuntimeState>) -> Result<StatsResponse, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    overview::stats(&conn).map_err(Into::into)
}

/// 查询按白板聚合的图谱总览。
#[tauri::command]
#[specta::specta]
pub fn overview_graph(
    state: State<'_, KeysightRuntimeState>,
) -> Result<GraphOverviewResponse, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    overview::graph_overview(&conn).map_err(Into::into)
}

/// 查询所有子白板的轻量统计。
#[tauri::command]
#[specta::specta]
pub fn whiteboard_list(
    state: State<'_, KeysightRuntimeState>,
) -> Result<Vec<WhiteboardSummary>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:whiteboard_list");
    let conn = lock_db(&state.core.db, "whiteboard_list");
    let fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    overview::list_whiteboards(&conn, &fs).map_err(Into::into)
}

/// # whiteboard_create
///
/// ## 前置条件
/// - `name` trim 后非空
/// - `name` 不能包含 `/` 或 `:`
/// - `whiteboard/{name}/` 目录尚不存在
///
/// ## 执行效果
/// 1. 在 vault 下创建空目录 `whiteboard/{name}/`
/// 2. 返回该 whiteboard 的零统计摘要
///
/// ## 不做的事
/// - 不创建任何 card / section / note
/// - 不直接写入 positions；root 画布会在看到新 whiteboard 后自动补位置
///
/// ## 幂等性
/// 非幂等 — 已存在同名文件夹时报错
#[tauri::command]
#[specta::specta]
pub fn whiteboard_create(
    state: State<'_, KeysightRuntimeState>,
    name: String,
) -> Result<WhiteboardSummary, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:whiteboard_create");
    whiteboard::create_folder(&state.core.vault_path, &name).map_err(Into::into)
}

// ============================================================
// Config
// ============================================================

/// 返回当前 vault 配置信息。
#[tauri::command]
#[specta::specta]
pub fn get_vault_info(state: State<'_, KeysightRuntimeState>) -> Result<VaultInfoResponse, AppError> {
    let state = state.resolved()?;
    Ok(VaultInfoResponse {
        vault_path: state.core.vault_path.to_string_lossy().into_owned(),
    })
}

// ============================================================
// Legacy Import
// ============================================================

/// # import_legacy_db
///
/// ## 前置条件
/// - old_db_path 指向旧 Obsidian 插件的 keysight.db（v1 schema）
/// - 文件必须存在且可读
///
/// ## 执行效果
/// 1. 备份旧 DB（cp → {old_db_path}.bak-import-{timestamp}）
/// 2. 只读打开旧 DB
/// 3. 单事务写入新 DB（INSERT OR REPLACE）
/// 4. 同步 FTS 索引
///
/// ## 幂等性
/// 可重复执行，结果一致
///
/// ## 关联操作
/// - [`overview_stats`] — 导入后验证数据量
#[tauri::command]
#[specta::specta]
pub fn import_legacy_db(
    state: State<'_, KeysightRuntimeState>,
    old_db_path: String,
) -> Result<ImportSummary, AppError> {
    let state = state.resolved()?;
    let path = std::path::Path::new(&old_db_path);
    if !path.exists() {
        return Err(AppError::Keysight {
            message: format!("旧 DB 文件不存在: {old_db_path}"),
        });
    }

    // 1. 备份旧 DB
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let backup_path = format!("{old_db_path}.bak-import-{timestamp}");
    std::fs::copy(path, &backup_path).map_err(|e| AppError::Keysight {
        message: format!("备份旧 DB 失败: {e}"),
    })?;

    // 2. 只读打开旧 DB
    let old_conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| AppError::Keysight {
        message: format!("打开旧 DB 失败: {e}"),
    })?;

    // 3. 导入
    // 例外: Mutex poisoning 不可恢复
    let new_conn = state.core.db.lock().unwrap();
    let reader = SqliteLegacyReader::new(&old_conn);
    let importer = SqliteLegacyImporter::new(&new_conn);
    importer.import(&reader).map_err(Into::into)
}
