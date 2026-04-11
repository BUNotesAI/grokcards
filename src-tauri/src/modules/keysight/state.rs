use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::Connection;

/// KeySight 模块的共享状态，由 Tauri managed state 注入。
pub struct KeysightState {
    /// SQLite 连接（keysight 专用 DB）。
    pub db: Mutex<Connection>,
    /// Obsidian vault 根目录路径。
    pub vault_path: PathBuf,
}
