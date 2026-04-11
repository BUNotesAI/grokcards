#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::id;
use crate::modules::keysight::models::GraphSection;

/// 分组存储契约。
pub(super) trait SectionStore {
    fn create(&self, whiteboard_id: &str, title: &str, color: Option<&str>) -> Result<GraphSection, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn update(&self, id: &str, title: Option<&str>, color: Option<&str>) -> Result<(), KeysightError>;
    fn add_member(&self, section_id: &str, entity_id: &str) -> Result<(), KeysightError>;
    fn remove_member(&self, section_id: &str, entity_id: &str) -> Result<(), KeysightError>;
    fn get(&self, id: &str) -> Result<GraphSection, KeysightError>;
    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphSection>, KeysightError>;
}

pub(super) struct SqliteSectionStore<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteSectionStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }
}

impl SectionStore for SqliteSectionStore<'_> {
    fn create(&self, whiteboard_id: &str, title: &str, color: Option<&str>) -> Result<GraphSection, KeysightError> {
        let sec_id = id::gen_sec_id();
        self.conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, color) VALUES (?1, 'section', ?2, ?3, ?4)",
            params![sec_id, title, whiteboard_id, color],
        )?;
        Ok(GraphSection {
            id: sec_id,
            title: title.to_string(),
            card_ids: Vec::new(),
            color: color.map(|s| s.to_string()),
            linked_section_ids: None,
        })
    }

    fn delete(&self, id: &str) -> Result<(), KeysightError> {
        self.conn.execute("DELETE FROM section_members WHERE section_id = ?1", [id])?;
        self.conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id])?;
        self.conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", [id])?;
        self.conn.execute("DELETE FROM entities WHERE id = ?1", [id])?;
        Ok(())
    }

    fn update(&self, id: &str, title: Option<&str>, color: Option<&str>) -> Result<(), KeysightError> {
        if let Some(t) = title {
            self.conn.execute("UPDATE entities SET title = ?1 WHERE id = ?2", params![t, id])?;
        }
        if let Some(c) = color {
            let c_val: Option<&str> = if c == "default" { None } else { Some(c) };
            self.conn.execute("UPDATE entities SET color = ?1 WHERE id = ?2", params![c_val, id])?;
        }
        Ok(())
    }

    fn add_member(&self, section_id: &str, entity_id: &str) -> Result<(), KeysightError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO section_members (section_id, entity_id) VALUES (?1, ?2)",
            params![section_id, entity_id],
        )?;
        Ok(())
    }

    fn remove_member(&self, section_id: &str, entity_id: &str) -> Result<(), KeysightError> {
        self.conn.execute(
            "DELETE FROM section_members WHERE section_id = ?1 AND entity_id = ?2",
            params![section_id, entity_id],
        )?;
        Ok(())
    }

    fn get(&self, id: &str) -> Result<GraphSection, KeysightError> {
        let (title, color): (String, Option<String>) = self.conn.query_row(
            "SELECT title, color FROM entities WHERE id = ?1 AND kind = 'section'",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        let mut stmt = self.conn.prepare("SELECT entity_id FROM section_members WHERE section_id = ?1")?;
        let card_ids: Vec<String> = stmt.query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut stmt2 = self.conn.prepare(
            "SELECT to_id FROM edges WHERE from_id = ?1 AND edge_type = 'section_link'"
        )?;
        let linked: Vec<String> = stmt2.query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let linked_section_ids = if linked.is_empty() { None } else { Some(linked) };

        Ok(GraphSection { id: id.to_string(), title, card_ids, color, linked_section_ids })
    }

    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphSection>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM entities WHERE kind = 'section' AND whiteboard_id = ?1 ORDER BY title"
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
    fn test_create_section() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create("wb_root", "My Section", Some("blue")).unwrap();
        assert!(sec.id.starts_with("sec_"));
        assert_eq!(sec.title, "My Section");
        assert_eq!(sec.color, Some("blue".to_string()));
        assert!(sec.card_ids.is_empty());
    }

    #[test]
    fn test_delete_section() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create("wb_root", "To Delete", None).unwrap();
        store.delete(&sec.id).unwrap();
        assert!(matches!(store.get(&sec.id), Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_update_section_title() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create("wb_root", "Old", None).unwrap();
        store.update(&sec.id, Some("New"), None).unwrap();
        let updated = store.get(&sec.id).unwrap();
        assert_eq!(updated.title, "New");
    }

    #[test]
    fn test_add_remove_member() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create("wb_root", "Sec", None).unwrap();
        store.add_member(&sec.id, "card_aaa").unwrap();
        store.add_member(&sec.id, "card_bbb").unwrap();

        let loaded = store.get(&sec.id).unwrap();
        assert_eq!(loaded.card_ids.len(), 2);

        store.remove_member(&sec.id, "card_aaa").unwrap();
        let loaded2 = store.get(&sec.id).unwrap();
        assert_eq!(loaded2.card_ids.len(), 1);
    }

    #[test]
    fn test_add_member_idempotent() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create("wb_root", "Sec", None).unwrap();
        store.add_member(&sec.id, "card_aaa").unwrap();
        store.add_member(&sec.id, "card_aaa").unwrap(); // 重复
        let loaded = store.get(&sec.id).unwrap();
        assert_eq!(loaded.card_ids.len(), 1);
    }

    #[test]
    fn test_query_all_sections() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        store.create("wb_root", "Alpha", None).unwrap();
        store.create("wb_root", "Beta", None).unwrap();
        store.create("other_wb", "Gamma", None).unwrap();

        let sections = store.query_all("wb_root").unwrap();
        assert_eq!(sections.len(), 2);
    }
}
