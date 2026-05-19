use std::collections::HashMap;

use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::edge::EntityId;
use crate::modules::keysight::domain::id::WhiteboardId;
use crate::modules::keysight::domain::layout::{LayoutStore, SqliteLayoutStore};
use crate::modules::keysight::models::Position;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

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
