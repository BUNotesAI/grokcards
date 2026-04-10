use rusqlite::{params, Connection};

use super::errors::TodoError;
use super::models::{Todo, TodoFilter};

/// 将一行 SQLite 结果映射为 Todo 结构体。
fn row_to_todo(row: &rusqlite::Row) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: row.get(0)?,
        title: row.get(1)?,
        completed: row.get::<_, bool>(2)?,
        created_at: row.get(3)?,
    })
}

/// 根据 filter 条件查询 todos 列表，按创建时间倒序。
pub(super) fn list_todos(conn: &Connection, filter: &TodoFilter) -> Result<Vec<Todo>, TodoError> {
    let (sql, filter_params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match filter {
        TodoFilter::All => (
            "SELECT id, title, completed, created_at FROM todos ORDER BY created_at DESC",
            vec![],
        ),
        TodoFilter::Active => (
            "SELECT id, title, completed, created_at FROM todos WHERE completed = 0 ORDER BY created_at DESC",
            vec![],
        ),
        TodoFilter::Completed => (
            "SELECT id, title, completed, created_at FROM todos WHERE completed = 1 ORDER BY created_at DESC",
            vec![],
        ),
    };
    let mut stmt = conn.prepare(sql)?;
    let todos = stmt
        .query_map(rusqlite::params_from_iter(filter_params), row_to_todo)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(todos)
}

/// # 创建 Todo
///
/// ## 前置条件
/// - title 去除首尾空白后不得为空
///
/// ## 执行效果
/// 1. 插入一条 completed = false 的新记录
///
/// ## 幂等性
/// 非幂等 — 每次调用创建新 todo
pub(super) fn create_todo(conn: &Connection, title: &str) -> Result<Todo, TodoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TodoError::EmptyTitle);
    }
    conn.execute(
        "INSERT INTO todos (title, completed) VALUES (?1, 0)",
        params![title],
    )?;
    let id = conn.last_insert_rowid();
    get_todo(conn, id)
}

/// # 更新 Todo 标题
///
/// ## 前置条件
/// - 指定 id 的 todo 必须存在
/// - 新标题去除首尾空白后不得为空
///
/// ## 执行效果
/// 1. 更新指定 id 的 title 字段
///
/// ## 不做的事
/// - 不改变 completed 状态
pub(super) fn update_todo(conn: &Connection, id: i64, title: &str) -> Result<Todo, TodoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TodoError::EmptyTitle);
    }
    let rows = conn.execute("UPDATE todos SET title = ?1 WHERE id = ?2", params![title, id])?;
    if rows == 0 {
        return Err(TodoError::NotFound(id));
    }
    get_todo(conn, id)
}

/// # 切换 Todo 完成状态
///
/// ## 执行效果
/// 1. 翻转指定 id 的 completed 布尔值
pub(super) fn toggle_todo(conn: &Connection, id: i64) -> Result<Todo, TodoError> {
    let rows = conn.execute(
        "UPDATE todos SET completed = NOT completed WHERE id = ?1",
        params![id],
    )?;
    if rows == 0 {
        return Err(TodoError::NotFound(id));
    }
    get_todo(conn, id)
}

/// # 删除 Todo
///
/// ## 执行效果
/// 1. 删除指定 id 的记录
pub(super) fn delete_todo(conn: &Connection, id: i64) -> Result<(), TodoError> {
    let rows = conn.execute("DELETE FROM todos WHERE id = ?1", params![id])?;
    if rows == 0 {
        return Err(TodoError::NotFound(id));
    }
    Ok(())
}

/// # 全部切换
///
/// ## 执行效果
/// 1. 将所有 todo 的 completed 设为指定值
/// 2. 返回更新后的完整列表
pub(super) fn toggle_all(conn: &Connection, completed: bool) -> Result<Vec<Todo>, TodoError> {
    conn.execute(
        "UPDATE todos SET completed = ?1",
        params![completed],
    )?;
    list_todos(conn, &TodoFilter::All)
}

/// # 清除已完成
///
/// ## 执行效果
/// 1. 删除所有 completed = true 的 todo
/// 2. 返回剩余（活跃）的 todo 列表
pub(super) fn clear_completed(conn: &Connection) -> Result<Vec<Todo>, TodoError> {
    conn.execute("DELETE FROM todos WHERE completed = 1", [])?;
    list_todos(conn, &TodoFilter::All)
}

/// 根据 id 查询单条 todo。
fn get_todo(conn: &Connection, id: i64) -> Result<Todo, TodoError> {
    conn.query_row(
        "SELECT id, title, completed, created_at FROM todos WHERE id = ?1",
        params![id],
        row_to_todo,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => TodoError::NotFound(id),
        other => TodoError::Database(other),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::todo::db::init_db;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    // -- create_todo --

    #[test]
    fn create_todo_happy_path() {
        let conn = test_conn();
        let todo = create_todo(&conn, "Buy milk").unwrap();
        assert_eq!(todo.title, "Buy milk");
        assert!(!todo.completed);
        assert!(todo.id > 0);
        assert!(!todo.created_at.is_empty());
    }

    #[test]
    fn create_todo_trims_whitespace() {
        let conn = test_conn();
        let todo = create_todo(&conn, "  Buy milk  ").unwrap();
        assert_eq!(todo.title, "Buy milk");
    }

    #[test]
    fn create_todo_empty_title_rejected() {
        let conn = test_conn();
        let err = create_todo(&conn, "  ").unwrap_err();
        assert!(matches!(err, TodoError::EmptyTitle));
    }

    // -- list_todos --

    #[test]
    fn list_todos_all() {
        let conn = test_conn();
        create_todo(&conn, "A").unwrap();
        create_todo(&conn, "B").unwrap();
        let todos = list_todos(&conn, &TodoFilter::All).unwrap();
        assert_eq!(todos.len(), 2);
    }

    #[test]
    fn list_todos_filtered() {
        let conn = test_conn();
        let t = create_todo(&conn, "A").unwrap();
        create_todo(&conn, "B").unwrap();
        toggle_todo(&conn, t.id).unwrap();

        let active = list_todos(&conn, &TodoFilter::Active).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "B");

        let completed = list_todos(&conn, &TodoFilter::Completed).unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].title, "A");
    }

    #[test]
    fn list_todos_empty() {
        let conn = test_conn();
        let todos = list_todos(&conn, &TodoFilter::All).unwrap();
        assert!(todos.is_empty());
    }

    // -- update_todo --

    #[test]
    fn update_todo_happy_path() {
        let conn = test_conn();
        let todo = create_todo(&conn, "Old title").unwrap();
        let updated = update_todo(&conn, todo.id, "New title").unwrap();
        assert_eq!(updated.title, "New title");
        assert_eq!(updated.id, todo.id);
    }

    #[test]
    fn update_todo_not_found() {
        let conn = test_conn();
        let err = update_todo(&conn, 999, "title").unwrap_err();
        assert!(matches!(err, TodoError::NotFound(999)));
    }

    #[test]
    fn update_todo_empty_title_rejected() {
        let conn = test_conn();
        let todo = create_todo(&conn, "Title").unwrap();
        let err = update_todo(&conn, todo.id, "").unwrap_err();
        assert!(matches!(err, TodoError::EmptyTitle));
    }

    // -- toggle_todo --

    #[test]
    fn toggle_todo_flips_completed() {
        let conn = test_conn();
        let todo = create_todo(&conn, "Task").unwrap();
        assert!(!todo.completed);

        let toggled = toggle_todo(&conn, todo.id).unwrap();
        assert!(toggled.completed);

        let toggled_back = toggle_todo(&conn, todo.id).unwrap();
        assert!(!toggled_back.completed);
    }

    #[test]
    fn toggle_todo_not_found() {
        let conn = test_conn();
        let err = toggle_todo(&conn, 999).unwrap_err();
        assert!(matches!(err, TodoError::NotFound(999)));
    }

    // -- delete_todo --

    #[test]
    fn delete_todo_happy_path() {
        let conn = test_conn();
        let todo = create_todo(&conn, "To delete").unwrap();
        delete_todo(&conn, todo.id).unwrap();
        let todos = list_todos(&conn, &TodoFilter::All).unwrap();
        assert!(todos.is_empty());
    }

    #[test]
    fn delete_todo_not_found() {
        let conn = test_conn();
        let err = delete_todo(&conn, 999).unwrap_err();
        assert!(matches!(err, TodoError::NotFound(999)));
    }

    // -- toggle_all --

    #[test]
    fn toggle_all_marks_all_completed() {
        let conn = test_conn();
        create_todo(&conn, "A").unwrap();
        create_todo(&conn, "B").unwrap();
        let todos = toggle_all(&conn, true).unwrap();
        assert!(todos.iter().all(|t| t.completed));
    }

    #[test]
    fn toggle_all_marks_all_active() {
        let conn = test_conn();
        let t = create_todo(&conn, "A").unwrap();
        toggle_todo(&conn, t.id).unwrap();
        create_todo(&conn, "B").unwrap();

        let todos = toggle_all(&conn, false).unwrap();
        assert!(todos.iter().all(|t| !t.completed));
    }

    // -- clear_completed --

    #[test]
    fn clear_completed_removes_done_todos() {
        let conn = test_conn();
        let t1 = create_todo(&conn, "Done").unwrap();
        create_todo(&conn, "Active").unwrap();
        toggle_todo(&conn, t1.id).unwrap();

        let remaining = clear_completed(&conn).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].title, "Active");
    }

    #[test]
    fn clear_completed_noop_when_none_completed() {
        let conn = test_conn();
        create_todo(&conn, "A").unwrap();
        let remaining = clear_completed(&conn).unwrap();
        assert_eq!(remaining.len(), 1);
    }
}
