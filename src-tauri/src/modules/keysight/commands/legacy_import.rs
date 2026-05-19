use rusqlite::Connection;
use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::legacy_import::{
    LegacyImporter, SqliteLegacyImporter, SqliteLegacyReader,
};
use crate::modules::keysight::models::ImportSummary;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;

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
/// - [`crate::modules::keysight::commands::overview_stats`] — 导入后验证数据量
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
