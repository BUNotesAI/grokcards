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
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, q.status, e.color \
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
                color: r.get(5)?,
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
    color: Option<&str>,
) -> Result<QuestionEntity, KeysightError> {
    let title = validate_title(title)?;
    let question_id = id::gen_question_id();
    let question = QuestionEntity {
        id: question_id.clone(),
        title: title.to_string(),
        content: content.unwrap_or_default().to_string(),
        whiteboard_id: whiteboard_id.to_string(),
        status: status.unwrap_or("pending").to_string(),
        color: color.filter(|v| !v.is_empty()).map(|v| v.to_string()),
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
    color: Option<&str>,
) -> Result<(), KeysightError> {
    let current = get(conn, id)?;
    let file_path: Option<String> = conn
        .query_row("SELECT file_path FROM entities WHERE id = ?1 AND kind = 'question'", [id], |r| r.get(0))
        .optional()?;

    // color "default" sentinel 清空(和 note.rs 一致):
    //   Some("default") → None / Some(other) → 覆盖 / None → 保留 current
    let next_color = match color {
        Some("default") => None,
        Some(other) => Some(other.to_string()),
        None => current.color.clone(),
    };
    let next = QuestionEntity {
        id: current.id.clone(),
        title: match title {
            Some(value) => validate_title(value)?.to_string(),
            None => current.title.clone(),
        },
        content: content.unwrap_or(&current.content).to_string(),
        whiteboard_id: current.whiteboard_id.clone(),
        status: status.unwrap_or(&current.status).to_string(),
        color: next_color,
    };
    let previous_path = file_path.filter(|path| !path.is_empty());
    let relative_path = desired_question_relative_path(&next.whiteboard_id, id, &next.title, previous_path.as_deref());
    let markdown = render_question_markdown(&next);
    vault_fs.write_file(&relative_path, &markdown)?;
    sync::sync_file(conn, &relative_path, &markdown, current_mtime_ms())?;
    if let Some(previous_path) = previous_path
        && previous_path != relative_path
    {
        vault_fs.delete_file(&previous_path)?;
        conn.execute("DELETE FROM file_mtimes WHERE filePath = ?1", [&previous_path])?;
    }
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
         q.status, e.color \
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
            color: r.get(5)?,
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

fn validate_title(title: &str) -> Result<&str, KeysightError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        Err(KeysightError::EmptyTitle)
    } else {
        Ok(trimmed)
    }
}

fn filename_title_component(text: &str) -> String {
    text.replace("**", "")
}

fn sanitize_file_component(text: &str) -> String {
    let cleaned = filename_title_component(text)
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
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

fn desired_question_relative_path(
    whiteboard_id: &str,
    question_id: &str,
    title: &str,
    current_file_path: Option<&str>,
) -> String {
    let desired = question_relative_path(whiteboard_id, question_id, title);
    match current_file_path {
        Some(path) if !path.is_empty() => {
            if path == desired {
                path.to_string()
            } else {
                desired
            }
        }
        _ => desired,
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
    ];
    if let Some(color) = question.color.as_deref().filter(|v| !v.is_empty()) {
        // YAML 里裸的 `#` 会被当成行内注释,hex 色值必须加引号。
        lines.push(format!("color: \"{color}\""));
    }
    lines.extend([
        "---".to_string(),
        String::new(),
        format!("# 【QUE】{}", question.title),
        String::new(),
    ]);
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

        let question = create(&conn, &vfs, "wb_root", "Test Question", Some("Body."), Some("doing"), None).unwrap();

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
        let question = create(&conn, &vfs, "wb_root", "Old", Some("Body"), Some("pending"), None).unwrap();

        update(&conn, &vfs, &question.id, Some("New"), Some("Updated"), Some("done"), None).unwrap();

        let loaded = get(&conn, &question.id).unwrap();
        assert_eq!(loaded.title, "New");
        assert_eq!(loaded.content, "Updated");
        assert_eq!(loaded.status, "done");
    }

    #[test]
    fn test_create_question_sanitizes_filename_without_markdown_bold_or_special_chars() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();

        let question = create(&conn, &vfs, "wb_root", "**Why** / Question", Some("Body"), None, None).unwrap();

        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&question.id], |r| r.get(0))
            .unwrap();
        assert!(file_path.ends_with("【QUE】Why _ Question.md"));
    }

    #[test]
    fn test_update_question_renames_file_when_title_changes() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let question = create(&conn, &vfs, "wb_root", "Old", Some("Body"), Some("pending"), None).unwrap();
        let old_file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&question.id], |r| r.get(0))
            .unwrap();

        update(&conn, &vfs, &question.id, Some("**New** / Question"), None, None, None).unwrap();

        let new_file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&question.id], |r| r.get(0))
            .unwrap();
        assert_ne!(new_file_path, old_file_path);
        assert!(new_file_path.ends_with("【QUE】New _ Question.md"));
        assert!(vfs.get_file(&old_file_path).is_none());
        assert!(vfs.get_file(&new_file_path).is_some());
    }

    #[test]
    fn test_delete_question_removes_markdown_file_and_db_rows() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let question = create(&conn, &vfs, "wb_root", "Del", None, None, None).unwrap();
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

    #[test]
    fn test_create_question_with_color() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();

        let question = create(
            &conn,
            &vfs,
            "wb_root",
            "Colored Q",
            None,
            None,
            Some("#ffadad"),
        )
        .unwrap();

        assert_eq!(question.color, Some("#ffadad".to_string()));
        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1",
                [&question.id],
                |r| r.get(0),
            )
            .unwrap();
        let content = vfs.get_file(&file_path).unwrap();
        // hex 色值在 frontmatter 中必须加引号,避免被 YAML 当作行内注释
        assert!(content.contains("color: \"#ffadad\""));
    }

    #[test]
    fn test_update_question_sets_and_clears_color() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let question = create(&conn, &vfs, "wb_root", "Q", None, None, None).unwrap();

        // 第一步:设置 color
        update(
            &conn,
            &vfs,
            &question.id,
            None,
            None,
            None,
            Some("#a0c4ff"),
        )
        .unwrap();
        let loaded = get(&conn, &question.id).unwrap();
        assert_eq!(loaded.color, Some("#a0c4ff".to_string()));

        // 第二步:"default" sentinel 清空 color
        update(&conn, &vfs, &question.id, None, None, None, Some("default")).unwrap();
        let loaded = get(&conn, &question.id).unwrap();
        assert_eq!(loaded.color, None);
    }

    #[test]
    fn test_update_question_preserves_color_when_color_is_none() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let question = create(
            &conn,
            &vfs,
            "wb_root",
            "Q",
            None,
            None,
            Some("#ffadad"),
        )
        .unwrap();

        // 不传 color 参数 → 保留当前 color
        update(&conn, &vfs, &question.id, Some("New title"), None, None, None).unwrap();
        let loaded = get(&conn, &question.id).unwrap();
        assert_eq!(loaded.color, Some("#ffadad".to_string()));
        assert_eq!(loaded.title, "New title");
    }
}
