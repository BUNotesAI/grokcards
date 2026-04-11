pub mod keysight;
pub mod todo;

/// 初始化所有模块的数据库表。
pub fn init_all(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    todo::init(conn)?;
    keysight::init(conn)?;
    Ok(())
}
