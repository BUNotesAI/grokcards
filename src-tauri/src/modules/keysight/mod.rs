pub mod commands;

// 从 keysight-core re-export,保持 src-tauri 内部 use 路径不变:
// - `crate::modules::keysight::domain::X` → 解析到 `keysight_core::domain::X`
// - `crate::modules::keysight::{models, state, ...}` 同理
// 原 src-tauri 里 `models` / `state` 是 `pub mod`,`db / domain / errors / id / parser / vault_fs`
// 是 private mod;搬到 keysight-core 后全部变成 `pub mod`(跨 crate 可见),
// 这里的 re-export 只是向后兼容 src-tauri 内部的已有 use 路径,不额外放宽 src-tauri 对外的 surface。
// 只 re-export src-tauri 内部实际引用的 5 个 mod(mod.rs hook fn + commands.rs 用到)。
// `errors` / `id` / `parser` 是 keysight-core 内部细节,src-tauri 不需要感知。
pub use keysight_core::{db, domain, models, state, vault_fs};

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
    if let Err(err) = domain::note::migrate_db_notes_to_files(&conn, &state.db_path, &state.vault_path) {
        return Err(format!("note migration failed: {err}"));
    }
    let fs = vault_fs::RealVaultFs::new(state.vault_path.to_string_lossy().to_string());
    domain::sync::sync_vault(&conn, &fs).map_err(|e| e.to_string())
}

/// # 清理卡片标题历史转义
///
/// ## 前置条件
/// - `conn` 指向目标 keysight SQLite
/// - `vault_path` 指向与该 SQLite 对应的 vault 根目录
///
/// ## 执行效果
/// 1. 遍历所有 `kind='card'` 的实体
/// 2. 只清理 card title 中历史遗留的安全标点转义
/// 3. 同步写回 markdown 文件、`entities.title` 和 `entities_fts.title`
///
/// ## 不做的事
/// - 不修改正文和其他 frontmatter 字段
/// - 不修改非 card 实体
///
/// ## 幂等性
/// 干净数据重复调用不会产生额外写入。
///
/// ## 关联操作
/// - [`startup_sync`] — 启动时从 vault 同步到 DB
pub fn cleanup_card_title_escapes(
    conn: &rusqlite::Connection,
    vault_path: &std::path::Path,
) -> Result<usize, String> {
    let fs = vault_fs::RealVaultFs::new(vault_path.to_string_lossy().to_string());
    domain::card::cleanup_dirty_card_title_escapes(conn, &fs).map_err(|e| e.to_string())
}

/// # 迁移 legacy toggle 语法
///
/// ## 前置条件
/// - `conn` 指向目标 keysight SQLite
/// - `vault_path` 指向与该 SQLite 对应的 vault 根目录
///
/// ## 执行效果
/// 1. 扫描 `whiteboard/` 下全部 markdown 文件
/// 2. 把 legacy `<details>/<summary>` 改写成 `?>> / ?<<`
/// 3. 同步更新 DB 和 FTS
/// 4. 同时清理 DB 内部的非文件型 note
///
/// ## 不做的事
/// - 不修改未使用 legacy 语法的内容
///
/// ## 幂等性
/// 幂等:重复执行不会产生额外修改。
pub fn migrate_toggle_syntax(
    conn: &rusqlite::Connection,
    vault_path: &std::path::Path,
) -> Result<models::ToggleSyntaxMigrationReport, String> {
    let fs = vault_fs::RealVaultFs::new(vault_path.to_string_lossy().to_string());
    domain::migration::migrate_legacy_toggle_syntax(conn, &fs).map_err(|e| e.to_string())
}
