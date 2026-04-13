#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::id;
use crate::modules::keysight::models::GraphNote;
use crate::modules::keysight::parser;

/// 笔记存储契约。
pub(in crate::modules::keysight) trait NoteStore {
    fn create(&self, whiteboard_id: &str, title: &str, content: Option<&str>, color: Option<&str>) -> Result<GraphNote, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn update(&self, id: &str, title: Option<&str>, content: Option<&str>, color: Option<&str>) -> Result<(), KeysightError>;
    fn get(&self, id: &str) -> Result<GraphNote, KeysightError>;
    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphNote>, KeysightError>;
}

pub(in crate::modules::keysight) struct SqliteNoteStore<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteNoteStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }
}

impl NoteStore for SqliteNoteStore<'_> {
    fn create(&self, whiteboard_id: &str, title: &str, content: Option<&str>, color: Option<&str>) -> Result<GraphNote, KeysightError> {
        let note_id = id::gen_note_id();
        let normalized_content = content
            .map(parser::normalize_legacy_toggle_syntax)
            .unwrap_or_default();
        self.conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, content, color) VALUES (?1, 'note', ?2, ?3, ?4, ?5)",
            params![note_id, title, whiteboard_id, normalized_content, color],
        )?;
        Ok(GraphNote {
            id: note_id,
            title: title.to_string(),
            content: normalized_content,
            color: color.map(|s| s.to_string()),
            linked_section_ids: None,
            linked_card_ids: None,
            linked_note_ids: None,
        })
    }

    fn delete(&self, id: &str) -> Result<(), KeysightError> {
        self.conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id])?;
        self.conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", [id])?;
        self.conn.execute("DELETE FROM entities WHERE id = ?1", [id])?;
        Ok(())
    }

    fn update(&self, id: &str, title: Option<&str>, content: Option<&str>, color: Option<&str>) -> Result<(), KeysightError> {
        if let Some(t) = title {
            self.conn.execute("UPDATE entities SET title = ?1 WHERE id = ?2", params![t, id])?;
        }
        if let Some(c) = content {
            let normalized = parser::normalize_legacy_toggle_syntax(c);
            self.conn.execute("UPDATE entities SET content = ?1 WHERE id = ?2", params![normalized, id])?;
        }
        if let Some(c) = color {
            let c_val: Option<&str> = if c == "default" { None } else { Some(c) };
            self.conn.execute("UPDATE entities SET color = ?1 WHERE id = ?2", params![c_val, id])?;
        }
        Ok(())
    }

    fn get(&self, id: &str) -> Result<GraphNote, KeysightError> {
        let (title, raw_content, color): (String, String, Option<String>) = self.conn.query_row(
            "SELECT title, COALESCE(content, ''), color FROM entities WHERE id = ?1 AND kind = 'note'",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;
        let content = parser::normalize_legacy_toggle_syntax(&raw_content);

        // 加载 note_link edges，按目标 id 前缀分组
        let mut stmt = self.conn.prepare(
            "SELECT to_id FROM edges WHERE from_id = ?1 AND edge_type = 'note_link'"
        )?;
        let targets: Vec<String> = stmt.query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut linked_card_ids = Vec::new();
        let mut linked_section_ids = Vec::new();
        let mut linked_note_ids = Vec::new();
        for target in targets {
            if target.starts_with("card_") || target.starts_with("alias_") {
                linked_card_ids.push(target);
            } else if target.starts_with("sec_") {
                linked_section_ids.push(target);
            } else if target.starts_with("note_") {
                linked_note_ids.push(target);
            }
        }

        Ok(GraphNote {
            id: id.to_string(),
            title,
            content,
            color,
            linked_section_ids: if linked_section_ids.is_empty() { None } else { Some(linked_section_ids) },
            linked_card_ids: if linked_card_ids.is_empty() { None } else { Some(linked_card_ids) },
            linked_note_ids: if linked_note_ids.is_empty() { None } else { Some(linked_note_ids) },
        })
    }

    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphNote>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM entities WHERE kind = 'note' AND whiteboard_id = ?1 ORDER BY title"
        )?;
        let ids: Vec<String> = stmt.query_map([whiteboard_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter().map(|id| self.get(id)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keysight::db::init_db;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_note() {
        let conn = test_conn();
        let store = SqliteNoteStore::new(&conn);
        let note = store.create("wb_root", "My Note", Some("Content"), Some("yellow")).unwrap();
        assert!(note.id.starts_with("note_"));
        assert_eq!(note.title, "My Note");
        assert_eq!(note.content, "Content");
        assert_eq!(note.color, Some("yellow".to_string()));
    }

    #[test]
    fn test_delete_note() {
        let conn = test_conn();
        let store = SqliteNoteStore::new(&conn);
        let note = store.create("wb_root", "Del", None, None).unwrap();
        store.delete(&note.id).unwrap();
        assert!(matches!(store.get(&note.id), Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_update_note() {
        let conn = test_conn();
        let store = SqliteNoteStore::new(&conn);
        let note = store.create("wb_root", "Old", Some("old"), None).unwrap();
        store.update(&note.id, Some("New"), Some("new content"), None).unwrap();
        let loaded = store.get(&note.id).unwrap();
        assert_eq!(loaded.title, "New");
        assert_eq!(loaded.content, "new content");
    }

    #[test]
    fn test_update_note_normalizes_legacy_details_summary_to_toggle_syntax() {
        let conn = test_conn();
        let store = SqliteNoteStore::new(&conn);
        let note = store.create("wb_root", "Old", Some("old"), None).unwrap();
        store
            .update(
                &note.id,
                None,
                Some("<details>\n<summary>折叠标题</summary>\n\n这里是详细内容\n</details>"),
                None,
            )
            .unwrap();

        let loaded = store.get(&note.id).unwrap();
        assert_eq!(loaded.content, "?>> 折叠标题\n这里是详细内容\n?<<");
    }

    #[test]
    fn test_get_note_normalizes_legacy_details_summary_to_toggle_syntax() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, content) VALUES (?1, 'note', ?2, ?3, ?4)",
            params![
                "note_toggle001",
                "Legacy Toggle",
                "wb_root",
                "<details>\n<summary>折叠标题</summary>\n\n这里是详细内容\n</details>"
            ],
        ).unwrap();

        let store = SqliteNoteStore::new(&conn);
        let loaded = store.get("note_toggle001").unwrap();
        assert_eq!(loaded.content, "?>> 折叠标题\n这里是详细内容\n?<<");
    }

    #[test]
    fn test_query_all_notes() {
        let conn = test_conn();
        let store = SqliteNoteStore::new(&conn);
        store.create("wb_root", "A", None, None).unwrap();
        store.create("wb_root", "B", None, None).unwrap();
        store.create("other", "C", None, None).unwrap();
        let notes = store.query_all("wb_root").unwrap();
        assert_eq!(notes.len(), 2);
    }
}
