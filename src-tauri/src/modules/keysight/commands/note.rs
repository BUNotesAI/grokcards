use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::id::{NoteId, WhiteboardId};
use crate::modules::keysight::domain::note::{self, NoteStore, SqliteNoteStore};
use crate::modules::keysight::models::{GraphNote, NoteFileMigrationReport};
use crate::modules::keysight::recording_vault_fs::RecordingVaultFs;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

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
    note::migrate_db_notes_to_files(&conn, &state.core.db_path, &state.core.vault_path)
        .map_err(Into::into)
}
