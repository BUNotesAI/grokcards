#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;

/// 更新问题状态。
pub(super) fn transition_status(conn: &Connection, id: &str, status: &str) -> Result<(), KeysightError> {
    let rows = conn.execute(
        "UPDATE question_fields SET status = ?1 WHERE entity_id = ?2",
        params![status, id],
    )?;
    if rows == 0 {
        return Err(KeysightError::NotFound(id.to_string()));
    }
    Ok(())
}

/// 按状态查询问题。
pub(super) fn by_status(conn: &Connection, status: &str) -> Result<Vec<serde_json::Value>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.title, e.whiteboard_id, e.content, q.status \
         FROM entities e JOIN question_fields q ON e.id = q.entity_id \
         WHERE q.status = ?1 ORDER BY e.title"
    )?;
    let rows = stmt.query_map([status], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "title": r.get::<_, String>(1)?,
            "whiteboardId": r.get::<_, String>(2)?,
            "content": r.get::<_, Option<String>>(3)?,
            "status": r.get::<_, String>(4)?,
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

    fn seed_question(conn: &Connection) {
        let md = "---\ntype: question\nid: q_test00001\nstatus: pending\n---\n\n# 【QUE】Test Question\n\nQuestion body.\n";
        sync::sync_file(conn, "questions/test.md", md, 1000.0).unwrap();
    }

    #[test]
    fn test_transition_status() {
        let conn = test_conn();
        seed_question(&conn);
        transition_status(&conn, "q_test00001", "doing").unwrap();

        let status: String = conn.query_row(
            "SELECT status FROM question_fields WHERE entity_id = 'q_test00001'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "doing");
    }

    #[test]
    fn test_transition_status_not_found() {
        let conn = test_conn();
        let result = transition_status(&conn, "q_nonexist00", "doing");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_by_status() {
        let conn = test_conn();
        seed_question(&conn);
        let questions = by_status(&conn, "pending").unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0]["id"], "q_test00001");
    }

    #[test]
    fn test_by_status_empty() {
        let conn = test_conn();
        let questions = by_status(&conn, "done").unwrap();
        assert!(questions.is_empty());
    }
}
