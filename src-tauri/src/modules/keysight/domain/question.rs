#![allow(dead_code)]
use rusqlite::{params, Connection, OptionalExtension};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::id;
use crate::modules::keysight::models::QuestionEntity;
use crate::modules::keysight::vault_fs::VaultFs;

use super::sync;

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

pub(in crate::modules::keysight) fn get(conn: &Connection, id: &str) -> Result<QuestionEntity, KeysightError> {
    conn.query_row(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, q.status \
         FROM entities e JOIN question_fields q ON e.id = q.entity_id \
         WHERE e.id = ?1 AND e.kind = 'question'",
        [id],
        |r| {
            Ok(QuestionEntity {
                id: r.get(0)?,
                title: r.get(1)?,
                content: r.get::<_, String>(2)?.trim_end_matches('\n').to_string(),
                whiteboard_id: r.get(3)?,
                status: r.get(4)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
        other => KeysightError::Database(other),
    })
}

pub(in crate::modules::keysight) fn create(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    whiteboard_id: &str,
    title: &str,
    content: Option<&str>,
    status: Option<&str>,
) -> Result<QuestionEntity, KeysightError> {
    let question_id = id::gen_question_id();
    let question = QuestionEntity {
        id: question_id.clone(),
        title: title.to_string(),
        content: content.unwrap_or_default().to_string(),
        whiteboard_id: whiteboard_id.to_string(),
        status: status.unwrap_or("pending").to_string(),
    };
    let file_path = question_relative_path(whiteboard_id, &question_id, title);
    let markdown = render_question_markdown(&question);
    vault_fs.write_file(&file_path, &markdown)?;
    sync::sync_file(conn, &file_path, &markdown, current_mtime_ms())?;
    get(conn, &question_id)
}

pub(in crate::modules::keysight) fn update(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    id: &str,
    title: Option<&str>,
    content: Option<&str>,
    status: Option<&str>,
) -> Result<(), KeysightError> {
    let current = get(conn, id)?;
    let file_path: Option<String> = conn
        .query_row("SELECT file_path FROM entities WHERE id = ?1 AND kind = 'question'", [id], |r| r.get(0))
        .optional()?;

    let next = QuestionEntity {
        id: current.id.clone(),
        title: title.unwrap_or(&current.title).to_string(),
        content: content.unwrap_or(&current.content).to_string(),
        whiteboard_id: current.whiteboard_id.clone(),
        status: status.unwrap_or(&current.status).to_string(),
    };
    let relative_path = file_path
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| question_relative_path(&next.whiteboard_id, id, &next.title));
    let markdown = render_question_markdown(&next);
    vault_fs.write_file(&relative_path, &markdown)?;
    sync::sync_file(conn, &relative_path, &markdown, current_mtime_ms())?;
    Ok(())
}

pub(in crate::modules::keysight) fn delete(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    id: &str,
) -> Result<(), KeysightError> {
    let file_path: Option<String> = conn
        .query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'question'",
            [id],
            |r| r.get(0),
        )
        .optional()?;

    if let Some(file_path) = file_path.filter(|path| !path.is_empty()) {
        vault_fs.delete_file(&file_path)?;
        sync::remove_file(conn, &file_path)?;
        return Ok(());
    }

    conn.execute("DELETE FROM question_fields WHERE entity_id = ?1", [id])?;
    conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id])?;
    conn.execute("DELETE FROM entities WHERE id = ?1", [id])?;
    Ok(())
}

/// 查询指定白板的所有问题。
pub(in crate::modules::keysight) fn query_all(conn: &Connection, whiteboard_id: &str) -> Result<Vec<QuestionEntity>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, \
         q.status \
         FROM entities e JOIN question_fields q ON e.id = q.entity_id \
         WHERE e.whiteboard_id = ?1 ORDER BY e.title"
    )?;
    let rows = stmt.query_map([whiteboard_id], |r| {
        Ok(QuestionEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            content: r.get::<_, String>(2)?.trim_end_matches('\n').to_string(),
            whiteboard_id: r.get(3)?,
            status: r.get(4)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(KeysightError::from)
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

fn current_mtime_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn whiteboard_relative_dir(whiteboard_id: &str) -> String {
    if whiteboard_id == "wb_root" {
        "whiteboard".to_string()
    } else {
        format!("whiteboard/{whiteboard_id}")
    }
}

fn sanitize_file_component(text: &str) -> String {
    let cleaned = text
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        "Untitled".to_string()
    } else {
        cleaned
    }
}

fn question_relative_path(whiteboard_id: &str, question_id: &str, title: &str) -> String {
    format!(
        "{}/{} 【QUE】{}.md",
        whiteboard_relative_dir(whiteboard_id),
        question_id,
        sanitize_file_component(title),
    )
}

fn render_question_markdown(question: &QuestionEntity) -> String {
    let body = question.content.trim_end();
    let mut lines = vec![
        "---".to_string(),
        "type: question".to_string(),
        format!("id: {}", question.id),
        format!("status: {}", question.status),
        "---".to_string(),
        String::new(),
        format!("# 【QUE】{}", question.title),
        String::new(),
    ];
    if !body.is_empty() {
        lines.push(body.to_string());
    }
    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keysight::db::init_db;
    use crate::modules::keysight::domain::sync;
    use crate::modules::keysight::vault_fs::MockVaultFs;

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
    fn test_create_question_writes_markdown_file_and_syncs_db() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();

        let question = create(&conn, &vfs, "wb_root", "Test Question", Some("Body."), Some("doing")).unwrap();

        assert_eq!(question.status, "doing");
        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&question.id], |r| r.get(0))
            .unwrap();
        let file = vfs.get_file(&file_path).unwrap();
        assert!(file.contains("type: question"));
        assert!(file.contains("status: doing"));
    }

    #[test]
    fn test_update_question_rewrites_markdown_file() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let question = create(&conn, &vfs, "wb_root", "Old", Some("Body"), Some("pending")).unwrap();

        update(&conn, &vfs, &question.id, Some("New"), Some("Updated"), Some("done")).unwrap();

        let loaded = get(&conn, &question.id).unwrap();
        assert_eq!(loaded.title, "New");
        assert_eq!(loaded.content, "Updated");
        assert_eq!(loaded.status, "done");
    }

    #[test]
    fn test_delete_question_removes_markdown_file_and_db_rows() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let question = create(&conn, &vfs, "wb_root", "Del", None, None).unwrap();
        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&question.id], |r| r.get(0))
            .unwrap();

        delete(&conn, &vfs, &question.id).unwrap();

        assert!(vfs.get_file(&file_path).is_none());
        assert!(matches!(get(&conn, &question.id), Err(KeysightError::NotFound(_))));
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

    #[test]
    fn test_query_all_returns_typed_questions() {
        let conn = test_conn();
        seed_question(&conn);
        let questions = query_all(&conn, "wb_root").unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].id, "q_test00001");
        assert_eq!(questions[0].title, "Test Question");
        assert_eq!(questions[0].status, "pending");
    }

    #[test]
    fn test_query_all_empty_whiteboard() {
        let conn = test_conn();
        seed_question(&conn);
        let questions = query_all(&conn, "other").unwrap();
        assert!(questions.is_empty());
    }
}
