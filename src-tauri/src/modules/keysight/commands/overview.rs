use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::{overview, whiteboard};
use crate::modules::keysight::models::{
    GraphOverviewResponse, StatsResponse, WhiteboardSummary,
};
use crate::modules::keysight::recording_vault_fs::RecordingVaultFs;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

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
