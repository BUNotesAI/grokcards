#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;

/// 更新任务状态。
pub(super) fn transition_status(conn: &Connection, id: &str, status: &str) -> Result<(), KeysightError> {
    let rows = conn.execute(
        "UPDATE task_fields SET status = ?1 WHERE entity_id = ?2",
        params![status, id],
    )?;
    if rows == 0 {
        return Err(KeysightError::NotFound(id.to_string()));
    }
    Ok(())
}

/// 更新任务领域。
pub(super) fn update_area(conn: &Connection, id: &str, area: &str) -> Result<(), KeysightError> {
    let rows = conn.execute(
        "UPDATE task_fields SET area = ?1 WHERE entity_id = ?2",
        params![area, id],
    )?;
    if rows == 0 {
        return Err(KeysightError::NotFound(id.to_string()));
    }
    Ok(())
}

/// 更新任务所属项目。
pub(super) fn update_project(conn: &Connection, id: &str, project: &str) -> Result<(), KeysightError> {
    let rows = conn.execute(
        "UPDATE task_fields SET project = ?1 WHERE entity_id = ?2",
        params![project, id],
    )?;
    if rows == 0 {
        return Err(KeysightError::NotFound(id.to_string()));
    }
    Ok(())
}

/// 按状态查询任务。
pub(super) fn by_status(conn: &Connection, status: &str) -> Result<Vec<serde_json::Value>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.title, e.whiteboard_id, e.content, t.status, t.area, t.project \
         FROM entities e JOIN task_fields t ON e.id = t.entity_id \
         WHERE t.status = ?1 ORDER BY e.title"
    )?;
    let rows = stmt.query_map([status], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "title": r.get::<_, String>(1)?,
            "whiteboardId": r.get::<_, String>(2)?,
            "content": r.get::<_, Option<String>>(3)?,
            "status": r.get::<_, String>(4)?,
            "area": r.get::<_, Option<String>>(5)?,
            "project": r.get::<_, Option<String>>(6)?,
        }))
    })?;
    let results: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keysight::db::init_db;
    use crate::modules::keysight::domain::sync;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    fn seed_task(conn: &Connection) {
        let md = "---\ntype: project-task\nid: task_test0001\nstatus: next\narea: backend\nproject: keysight\n---\n\n# 【TASK】Test Task\n\nTask body.\n";
        sync::sync_file(conn, "tasks/test.md", md, 1000.0).unwrap();
    }

    #[test]
    fn test_transition_status() {
        let conn = test_conn();
        seed_task(&conn);
        transition_status(&conn, "task_test0001", "active").unwrap();

        let status: String = conn.query_row(
            "SELECT status FROM task_fields WHERE entity_id = 'task_test0001'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "active");
    }

    #[test]
    fn test_transition_status_not_found() {
        let conn = test_conn();
        let result = transition_status(&conn, "task_nonexist", "active");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_update_area() {
        let conn = test_conn();
        seed_task(&conn);
        update_area(&conn, "task_test0001", "frontend").unwrap();

        let area: String = conn.query_row(
            "SELECT area FROM task_fields WHERE entity_id = 'task_test0001'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(area, "frontend");
    }

    #[test]
    fn test_update_project() {
        let conn = test_conn();
        seed_task(&conn);
        update_project(&conn, "task_test0001", "super-tauri").unwrap();

        let project: String = conn.query_row(
            "SELECT project FROM task_fields WHERE entity_id = 'task_test0001'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(project, "super-tauri");
    }

    #[test]
    fn test_by_status() {
        let conn = test_conn();
        seed_task(&conn);
        let tasks = by_status(&conn, "next").unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["id"], "task_test0001");
    }

    #[test]
    fn test_by_status_empty() {
        let conn = test_conn();
        let tasks = by_status(&conn, "done").unwrap();
        assert!(tasks.is_empty());
    }
}
