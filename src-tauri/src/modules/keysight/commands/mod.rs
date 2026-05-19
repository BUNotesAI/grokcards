//! Keysight Tauri commands —— 按 entity 拆分子模块,每个 `#[tauri::command]` fn 放
//! 对应文件;`pub use {entity}::*;` 让外部路径 `modules::keysight::commands::{cmd}`
//! 保持不变(`lib.rs collect_commands![]` 一行不改、bindings.ts 一字节不变)。
//!
//! 历史:原 `commands.rs` 单文件 1538 行 / 58 cmd,W1-W3 分波拆为 11 个 entity
//! 子文件 + 本 mod.rs(Config 段 1 cmd `get_vault_info` 体量过小,W3 末并入此处)。
//! 详细决策见 task_a60ceca2 design-v1。
//!
//! W2 状态(本 commit):已拆 8 段(card / section / task / entity_graph / note /
//! sync / question / alias),Layout / Overview / Config / Legacy Import 4 段
//! 仍内联在本 mod.rs,待 W3 继续抽出。

mod alias;
mod card;
mod entity_graph;
mod note;
mod question;
mod section;
mod sync;
mod task;

pub use alias::*;
pub use card::*;
pub use entity_graph::*;
pub use note::*;
pub use question::*;
pub use section::*;
pub use sync::*;
pub use task::*;

use std::collections::HashMap;

use rusqlite::Connection;
use tauri::State;

use super::domain::edge::EntityId;
use super::domain::id::WhiteboardId;
use super::domain::layout::{LayoutStore, SqliteLayoutStore};
use super::domain::legacy_import::{LegacyImporter, SqliteLegacyImporter, SqliteLegacyReader};
use super::domain::{overview, whiteboard};
use super::models::{
    GraphOverviewResponse, ImportSummary, Position, StatsResponse, VaultInfoResponse,
    WhiteboardSummary,
};
use super::recording_vault_fs::RecordingVaultFs;
use super::runtime_state::KeysightRuntimeState;
use crate::app_error::AppError;
use crate::perf::{lock_db, ScopedTimer};

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
