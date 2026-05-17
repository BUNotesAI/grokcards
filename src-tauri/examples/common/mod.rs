//! Examples 间共享的 maintenance 脚本工具:db 路径解析、备份生成、vault 路径读取。
//!
//! 通过子目录 `examples/common/mod.rs` 形式存放,Cargo 的 example 自动发现只扫
//! `examples/*.rs` 顶级文件,不会把本模块误识为独立 example。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 取当前 Unix 时间(秒)
fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 根据原 db 路径生成 `{stem}.backup-{ts}.{ext}` 形式的备份路径
pub fn build_backup_path(db_path: &Path) -> PathBuf {
    let stem = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("keysight");
    let ext = db_path.extension().and_then(|s| s.to_str()).unwrap_or("db");
    db_path.with_file_name(format!("{stem}.backup-{}.{}", timestamp_secs(), ext))
}

/// 从命令行第一个参数取 db 路径,缺省时回退到 macOS 默认位置
pub fn default_db_path() -> PathBuf {
    std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "{}/Library/Application Support/co.bunotes.super-tauri/keysight.db",
                std::env::var("HOME").unwrap_or_default()
            ))
        })
}

/// 从环境变量 `KEYSIGHT_VAULT_PATH` 读 vault 路径;未设置时直接 panic
/// (maintenance 脚本场景,没 vault 路径无法继续)
pub fn read_vault_path() -> PathBuf {
    std::env::var("KEYSIGHT_VAULT_PATH")
        .map(PathBuf::from)
        .expect("环境变量 KEYSIGHT_VAULT_PATH 未设置")
}

/// 备份 db_path 到 `build_backup_path` 生成的位置,返回备份文件路径
pub fn backup_db(db_path: &Path) -> std::io::Result<PathBuf> {
    let backup_path = build_backup_path(db_path);
    std::fs::copy(db_path, &backup_path)?;
    Ok(backup_path)
}
