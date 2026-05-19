use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::sync;
use crate::modules::keysight::models::{SyncFileResponse, SyncVaultReport};
use crate::modules::keysight::recording_vault_fs::RecordingVaultFs;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

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
