#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::id;
use crate::modules::keysight::models::CardAlias;

/// 别名存储契约。
pub(super) trait AliasStore {
    fn create(&self, whiteboard_id: &str, card_id: &str) -> Result<CardAlias, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn get(&self, id: &str) -> Result<CardAlias, KeysightError>;
    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<CardAlias>, KeysightError>;
}

pub(super) struct SqliteAliasStore<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteAliasStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }
}

impl AliasStore for SqliteAliasStore<'_> {
    fn create(&self, whiteboard_id: &str, card_id: &str) -> Result<CardAlias, KeysightError> {
        let alias_id = id::gen_alias_id();
        // entities 表的 title 用源卡片 id 作占位（alias 自身无标题）
        self.conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES (?1, 'alias', ?2, ?3)",
            params![alias_id, card_id, whiteboard_id],
        )?;
        self.conn.execute(
            "INSERT INTO alias_fields (entity_id, card_id) VALUES (?1, ?2)",
            params![alias_id, card_id],
        )?;
        Ok(CardAlias {
            alias_id,
            card_id: card_id.to_string(),
            linked_card_ids: None,
            linked_section_ids: None,
            linked_note_ids: None,
            incoming_card_ids: None,
        })
    }

    fn delete(&self, id: &str) -> Result<(), KeysightError> {
        self.conn.execute("DELETE FROM alias_fields WHERE entity_id = ?1", [id])?;
        self.conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id])?;
        self.conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", [id])?;
        self.conn.execute("DELETE FROM entities WHERE id = ?1", [id])?;
        Ok(())
    }

    fn get(&self, id: &str) -> Result<CardAlias, KeysightError> {
        let card_id: String = self.conn.query_row(
            "SELECT card_id FROM alias_fields WHERE entity_id = ?1",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        // 加载 alias_link edges
        let mut stmt = self.conn.prepare(
            "SELECT to_id FROM edges WHERE from_id = ?1 AND edge_type = 'alias_link'"
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

        // 加载 incoming (card_to_alias edges pointing to this alias)
        let mut stmt2 = self.conn.prepare(
            "SELECT from_id FROM edges WHERE to_id = ?1 AND edge_type = 'card_to_alias'"
        )?;
        let incoming: Vec<String> = stmt2.query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(CardAlias {
            alias_id: id.to_string(),
            card_id,
            linked_card_ids: if linked_card_ids.is_empty() { None } else { Some(linked_card_ids) },
            linked_section_ids: if linked_section_ids.is_empty() { None } else { Some(linked_section_ids) },
            linked_note_ids: if linked_note_ids.is_empty() { None } else { Some(linked_note_ids) },
            incoming_card_ids: if incoming.is_empty() { None } else { Some(incoming) },
        })
    }

    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<CardAlias>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id FROM entities e WHERE e.kind = 'alias' AND e.whiteboard_id = ?1"
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
    fn test_create_alias() {
        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        let alias = store.create("wb_root", "card_aaa").unwrap();
        assert!(alias.alias_id.starts_with("alias_"));
        assert_eq!(alias.card_id, "card_aaa");
    }

    #[test]
    fn test_delete_alias() {
        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        let alias = store.create("wb_root", "card_aaa").unwrap();
        store.delete(&alias.alias_id).unwrap();
        assert!(matches!(store.get(&alias.alias_id), Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_query_all_aliases() {
        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        store.create("wb_root", "card_aaa").unwrap();
        store.create("wb_root", "card_bbb").unwrap();
        store.create("other", "card_ccc").unwrap();
        let aliases = store.query_all("wb_root").unwrap();
        assert_eq!(aliases.len(), 2);
    }
}
