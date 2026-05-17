#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::domain::edge::EntityId;
use crate::domain::id::WhiteboardId;
use crate::errors::KeysightError;
use crate::id;
use crate::models::CardAlias;

/// 别名存储契约。
pub trait AliasStore {
    fn create(&self, whiteboard_id: &WhiteboardId, card_id: &str) -> Result<CardAlias, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn get(&self, id: &str) -> Result<CardAlias, KeysightError>;
    fn query_all(&self, whiteboard_id: &WhiteboardId) -> Result<Vec<CardAlias>, KeysightError>;
}

pub struct SqliteAliasStore<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteAliasStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }
}

impl AliasStore for SqliteAliasStore<'_> {
    fn create(&self, whiteboard_id: &WhiteboardId, card_id: &str) -> Result<CardAlias, KeysightError> {
        let alias_id = id::gen_alias_id();
        // entities 表的 title 用源卡片 id 作占位（alias 自身无标题）
        self.conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES (?1, 'alias', ?2, ?3)",
            params![alias_id, card_id, whiteboard_id.as_str()],
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
            linked_question_ids: None,
            linked_task_ids: None,
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

        // 穷尽 match EntityId 所有 6 个 variant —— 禁止 `_` 通配(同 note.rs::get 的防御)。
        let mut linked_card_ids = Vec::new();
        let mut linked_section_ids = Vec::new();
        let mut linked_note_ids = Vec::new();
        let mut linked_question_ids = Vec::new();
        let mut linked_task_ids = Vec::new();
        for target_str in targets {
            let entity_id = EntityId::parse(&target_str).map_err(|e| {
                KeysightError::ParseError(format!(
                    "alias_link target 无法 parse 为 EntityId: {target_str} ({e})"
                ))
            })?;
            match entity_id {
                EntityId::Card(_) | EntityId::Alias(_) => linked_card_ids.push(target_str),
                EntityId::Section(_) => linked_section_ids.push(target_str),
                EntityId::Note(_) => linked_note_ids.push(target_str),
                EntityId::Question(_) => linked_question_ids.push(target_str),
                EntityId::Task(_) => linked_task_ids.push(target_str),
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
            linked_question_ids: if linked_question_ids.is_empty() { None } else { Some(linked_question_ids) },
            linked_task_ids: if linked_task_ids.is_empty() { None } else { Some(linked_task_ids) },
            incoming_card_ids: if incoming.is_empty() { None } else { Some(incoming) },
        })
    }

    fn query_all(&self, whiteboard_id: &WhiteboardId) -> Result<Vec<CardAlias>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id FROM entities e WHERE e.kind = 'alias' AND e.whiteboard_id = ?1"
        )?;
        let ids: Vec<String> = stmt.query_map([whiteboard_id.as_str()], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter().map(|id| self.get(id)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_alias() {
        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        let wb = WhiteboardId::parse("wb_root").unwrap();
        let alias = store.create(&wb, "card_aaa").unwrap();
        assert!(alias.alias_id.starts_with("alias_"));
        assert_eq!(alias.card_id, "card_aaa");
    }

    #[test]
    fn test_delete_alias() {
        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        let wb = WhiteboardId::parse("wb_root").unwrap();
        let alias = store.create(&wb, "card_aaa").unwrap();
        store.delete(&alias.alias_id).unwrap();
        assert!(matches!(store.get(&alias.alias_id), Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_query_all_aliases() {
        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        let wb_root = WhiteboardId::parse("wb_root").unwrap();
        // `"other"` 字面量不带 wb_ prefix —— 这是一个 DB-level invariant 测试用例
        // (kind='alias' 行的 whiteboard_id 列即便不是合法 wb_ prefix,SQL 查询仍 scope by =),
        // 但我们的 trait 已强制 newtype。用 parse("wb_other") 维持语义。
        let wb_other = WhiteboardId::parse("wb_other").unwrap();
        store.create(&wb_root, "card_aaa").unwrap();
        store.create(&wb_root, "card_bbb").unwrap();
        store.create(&wb_other, "card_ccc").unwrap();
        let aliases = store.query_all(&wb_root).unwrap();
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn test_get_alias_reads_question_and_task_linked_ids() {
        // Phase A 2b 防火墙验证:reader 穷尽 match EntityId 6 个 variant,
        // Alias→Question / Alias→Task 目标不再 silent drop
        use crate::domain::edge::{user_draw_edge, EntityId};
        use crate::domain::entity::{EntityGraph, SqliteEntityGraph};

        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        let wb = WhiteboardId::parse("wb_root").unwrap();
        let alias = store.create(&wb, "card_aaa").unwrap();

        let graph = SqliteEntityGraph::new(&conn);
        let to_question = user_draw_edge(
            EntityId::parse(&alias.alias_id).unwrap(),
            EntityId::parse("q_qqq11111").unwrap(),
        )
        .unwrap();
        let to_task = user_draw_edge(
            EntityId::parse(&alias.alias_id).unwrap(),
            EntityId::parse("task_ttt22222").unwrap(),
        )
        .unwrap();
        graph.connect(&to_question).unwrap();
        graph.connect(&to_task).unwrap();

        let loaded = store.get(&alias.alias_id).unwrap();
        assert_eq!(
            loaded.linked_question_ids.clone().unwrap(),
            vec!["q_qqq11111".to_string()],
            "Question 目标应进 linked_question_ids"
        );
        assert_eq!(
            loaded.linked_task_ids.clone().unwrap(),
            vec!["task_ttt22222".to_string()],
            "Task 目标应进 linked_task_ids"
        );
    }

    #[test]
    fn test_get_alias_fails_on_unknown_target_prefix() {
        // 防火墙硬线:reader 遇到未知 entity id prefix 必须返 ParseError,
        // 不得 silent drop。
        let conn = test_conn();
        let store = SqliteAliasStore::new(&conn);
        let wb = WhiteboardId::parse("wb_root").unwrap();
        let alias = store.create(&wb, "card_aaa").unwrap();

        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES (?1, 'xyz_garbage', 'alias_link')",
            params![alias.alias_id],
        )
        .unwrap();

        let err = store.get(&alias.alias_id).unwrap_err();
        assert!(
            matches!(err, KeysightError::ParseError(_)),
            "未知 prefix 应让 reader 返 ParseError,实际: {err:?}"
        );
    }
}
