pub mod keysight;
pub mod todo;

/// 初始化 todo 模块的数据库表（keysight 有独立 DB，在 lib.rs 单独初始化）。
pub fn init_all(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    todo::init(conn)?;
    Ok(())
}
