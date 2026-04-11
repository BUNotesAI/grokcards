pub mod commands;
pub mod models;
pub mod state;

mod db;
mod domain;
mod errors;
mod id;
mod parser;
mod vault_fs;

/// 初始化 keysight 模块的数据库表。
pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    db::init_db(conn)
}
