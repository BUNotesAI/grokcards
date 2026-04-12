pub mod commands;
pub mod models;
pub mod state;

mod db;
mod domain;
mod errors;
mod id;
mod parser;
mod vault_fs;

use crate::perf::lock_db;

/// 初始化 keysight 模块的数据库表。
pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    db::init_db(conn)
}

/// 启动时同步 vault 到 DB。
///
/// 供 lib.rs setup 阶段调用。内部委托 domain::sync::sync_vault。
pub fn startup_sync(state: &state::KeysightState) -> Result<models::SyncVaultReport, String> {
    let conn = lock_db(&state.db, "startup_sync");
    let fs = vault_fs::RealVaultFs::new(state.vault_path.to_string_lossy().to_string());
    domain::sync::sync_vault(&conn, &fs).map_err(|e| e.to_string())
}
