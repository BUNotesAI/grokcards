#![allow(dead_code)]
use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::domain::id::{EntityId, WhiteboardId};
use crate::errors::KeysightError;
use crate::models::Position;

/// 位置管理契约。
pub trait LayoutStore {
    fn set_position(&self, whiteboard_id: &WhiteboardId, entity_id: &EntityId, x: f64, y: f64) -> Result<(), KeysightError>;
    fn query_positions(&self, whiteboard_id: &WhiteboardId) -> Result<HashMap<String, Position>, KeysightError>;
    fn remove_position(&self, whiteboard_id: &WhiteboardId, entity_id: &EntityId) -> Result<(), KeysightError>;
}

pub struct SqliteLayoutStore<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteLayoutStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }
}

impl LayoutStore for SqliteLayoutStore<'_> {
    fn set_position(&self, whiteboard_id: &WhiteboardId, entity_id: &EntityId, x: f64, y: f64) -> Result<(), KeysightError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, ?2, ?3, ?4)",
            params![entity_id.as_str(), whiteboard_id.as_str(), x, y],
        )?;
        Ok(())
    }

    fn query_positions(&self, whiteboard_id: &WhiteboardId) -> Result<HashMap<String, Position>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id, x, y FROM positions WHERE whiteboard_id = ?1"
        )?;
        let rows = stmt.query_map([whiteboard_id.as_str()], |r| {
            let id: String = r.get(0)?;
            let x: f64 = r.get(1)?;
            let y: f64 = r.get(2)?;
            Ok((id, Position { x, y }))
        })?;
        let mut map = HashMap::new();
        for r in rows {
            let (id, pos) = r?;
            map.insert(id, pos);
        }
        Ok(map)
    }

    fn remove_position(&self, whiteboard_id: &WhiteboardId, entity_id: &EntityId) -> Result<(), KeysightError> {
        self.conn.execute(
            "DELETE FROM positions WHERE entity_id = ?1 AND whiteboard_id = ?2",
            params![entity_id.as_str(), whiteboard_id.as_str()],
        )?;
        Ok(())
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

    /// 测试 fixture 桥接:把字面量 id 包成 EntityId。W5 渗透后 trait 方法签名
    /// 要求 `&EntityId`,字面量来自 test code,这里集中转换。
    fn eid(s: &str) -> EntityId {
        EntityId::parse(s).expect("test fixture entity id 应合法")
    }

    #[test]
    fn test_set_and_query_position() {
        let conn = test_conn();
        let store = SqliteLayoutStore::new(&conn);
        let wb = WhiteboardId::parse("wb_root").unwrap();
        store.set_position(&wb, &eid("card_aaa"), 100.0, 200.0).unwrap();

        let positions = store.query_positions(&wb).unwrap();
        assert_eq!(positions.len(), 1);
        let pos = &positions["card_aaa"];
        assert!((pos.x - 100.0).abs() < f64::EPSILON);
        assert!((pos.y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_position_upsert() {
        let conn = test_conn();
        let store = SqliteLayoutStore::new(&conn);
        let wb = WhiteboardId::parse("wb_root").unwrap();
        store.set_position(&wb, &eid("card_aaa"), 100.0, 200.0).unwrap();
        store.set_position(&wb, &eid("card_aaa"), 300.0, 400.0).unwrap();

        let positions = store.query_positions(&wb).unwrap();
        assert_eq!(positions.len(), 1);
        assert!((positions["card_aaa"].x - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_remove_position() {
        let conn = test_conn();
        let store = SqliteLayoutStore::new(&conn);
        let wb = WhiteboardId::parse("wb_root").unwrap();
        store.set_position(&wb, &eid("card_aaa"), 100.0, 200.0).unwrap();
        store.remove_position(&wb, &eid("card_aaa")).unwrap();

        let positions = store.query_positions(&wb).unwrap();
        assert!(positions.is_empty());
    }

    #[test]
    fn test_positions_scoped_by_whiteboard() {
        let conn = test_conn();
        let store = SqliteLayoutStore::new(&conn);
        let wb_root = WhiteboardId::parse("wb_root").unwrap();
        let wb_other = WhiteboardId::parse("wb_other").unwrap();
        store.set_position(&wb_root, &eid("card_aaa"), 10.0, 20.0).unwrap();
        store.set_position(&wb_other, &eid("card_bbb"), 30.0, 40.0).unwrap();

        let root = store.query_positions(&wb_root).unwrap();
        assert_eq!(root.len(), 1);
        assert!(root.contains_key("card_aaa"));
    }
}
