pub mod commands;
pub mod models;

mod db;
mod domain;
mod errors;

/// 初始化 todo 模块的数据库表。
pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    db::init_db(conn)
}
