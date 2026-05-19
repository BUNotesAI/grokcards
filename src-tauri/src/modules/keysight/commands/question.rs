use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::id::{QuestionId, WhiteboardId};
use crate::modules::keysight::domain::question;
use crate::modules::keysight::models::QuestionEntity;
use crate::modules::keysight::recording_vault_fs::RecordingVaultFs;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

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
