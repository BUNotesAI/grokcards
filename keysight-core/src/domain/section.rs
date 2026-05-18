#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::domain::id::{SectionId, WhiteboardId};
use crate::errors::KeysightError;
use crate::id;
use crate::models::GraphSection;

/// 分组存储契约。
pub trait SectionStore {
    fn create(&self, whiteboard_id: &WhiteboardId, title: &str, color: Option<&str>) -> Result<GraphSection, KeysightError>;
    fn delete(&self, id: &SectionId) -> Result<(), KeysightError>;
    fn update(&self, id: &SectionId, title: Option<&str>, color: Option<&str>) -> Result<(), KeysightError>;
    fn add_member(&self, section_id: &SectionId, entity_id: &str) -> Result<(), KeysightError>;
    fn remove_member(&self, section_id: &SectionId, entity_id: &str) -> Result<(), KeysightError>;
    fn get(&self, id: &SectionId) -> Result<GraphSection, KeysightError>;
    fn query_all(&self, whiteboard_id: &WhiteboardId) -> Result<Vec<GraphSection>, KeysightError>;
    fn move_to_whiteboard(&self, section_id: &SectionId, target_whiteboard_id: &WhiteboardId) -> Result<(), KeysightError>;
}

pub struct SqliteSectionStore<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteSectionStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }
}

impl SectionStore for SqliteSectionStore<'_> {
    fn create(&self, whiteboard_id: &WhiteboardId, title: &str, color: Option<&str>) -> Result<GraphSection, KeysightError> {
        let sec_id = id::gen_sec_id();
        self.conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, color) VALUES (?1, 'section', ?2, ?3, ?4)",
            params![sec_id, title, whiteboard_id.as_str(), color],
        )?;
        // auto-position: 在 canvas 最底元素下方一个 node 高度 + spacing 处,
        // 避免新建 section 因无 positions 行而在画布不可见(对标 task::create 的 auto-position)。
        let position = crate::domain::task::compute_position_below_bottommost(
            self.conn,
            whiteboard_id,
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, ?2, ?3, ?4)",
            params![sec_id, whiteboard_id.as_str(), position.x, position.y],
        )?;
        Ok(GraphSection {
            id: sec_id,
            title: title.to_string(),
            card_ids: Vec::new(),
            color: color.map(|s| s.to_string()),
            linked_section_ids: None,
        })
    }

    fn delete(&self, id: &SectionId) -> Result<(), KeysightError> {
        let id_str = id.as_str();
        self.conn.execute("DELETE FROM section_members WHERE section_id = ?1", [id_str])?;
        self.conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id_str])?;
        self.conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", [id_str])?;
        self.conn.execute("DELETE FROM entities WHERE id = ?1", [id_str])?;
        Ok(())
    }

    fn update(&self, id: &SectionId, title: Option<&str>, color: Option<&str>) -> Result<(), KeysightError> {
        let id_str = id.as_str();
        if let Some(t) = title {
            self.conn.execute("UPDATE entities SET title = ?1 WHERE id = ?2", params![t, id_str])?;
        }
        if let Some(c) = color {
            let c_val: Option<&str> = if c == "default" { None } else { Some(c) };
            self.conn.execute("UPDATE entities SET color = ?1 WHERE id = ?2", params![c_val, id_str])?;
        }
        Ok(())
    }

    fn add_member(&self, section_id: &SectionId, entity_id: &str) -> Result<(), KeysightError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO section_members (section_id, entity_id) VALUES (?1, ?2)",
            params![section_id.as_str(), entity_id],
        )?;
        Ok(())
    }

    fn remove_member(&self, section_id: &SectionId, entity_id: &str) -> Result<(), KeysightError> {
        self.conn.execute(
            "DELETE FROM section_members WHERE section_id = ?1 AND entity_id = ?2",
            params![section_id.as_str(), entity_id],
        )?;
        Ok(())
    }

    fn get(&self, id: &SectionId) -> Result<GraphSection, KeysightError> {
        let id_str = id.as_str();
        let (title, color): (String, Option<String>) = self.conn.query_row(
            "SELECT title, color FROM entities WHERE id = ?1 AND kind = 'section'",
            [id_str],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id_str.to_string()),
            other => KeysightError::Database(other),
        })?;

        let mut stmt = self.conn.prepare("SELECT entity_id FROM section_members WHERE section_id = ?1")?;
        let card_ids: Vec<String> = stmt.query_map([id_str], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut stmt2 = self.conn.prepare(
            "SELECT to_id FROM edges WHERE from_id = ?1 AND edge_type = 'section_link'"
        )?;
        let linked: Vec<String> = stmt2.query_map([id_str], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let linked_section_ids = if linked.is_empty() { None } else { Some(linked) };

        Ok(GraphSection { id: id_str.to_string(), title, card_ids, color, linked_section_ids })
    }

    fn query_all(&self, whiteboard_id: &WhiteboardId) -> Result<Vec<GraphSection>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM entities WHERE kind = 'section' AND whiteboard_id = ?1 ORDER BY title"
        )?;
        let ids: Vec<String> = stmt.query_map([whiteboard_id.as_str()], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        // DB read 路径,kind='section' 已保证 sec_ 前缀,走 new_unchecked
        ids.into_iter()
            .map(|id| self.get(&SectionId::new_unchecked(id)))
            .collect()
    }

    fn move_to_whiteboard(&self, section_id: &SectionId, target_whiteboard_id: &WhiteboardId) -> Result<(), KeysightError> {
        let section_id_str = section_id.as_str();
        // 1. 验证存在并获取旧 whiteboard_id
        let old_wb: String = self.conn.query_row(
            "SELECT whiteboard_id FROM entities WHERE id = ?1 AND kind = 'section'",
            [section_id_str],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(section_id_str.to_string()),
            other => KeysightError::Database(other),
        })?;

        // 2. 更新 whiteboard_id
        self.conn.execute(
            "UPDATE entities SET whiteboard_id = ?1 WHERE id = ?2",
            params![target_whiteboard_id.as_str(), section_id_str],
        )?;

        // 3. 查成员
        let mut stmt = self.conn.prepare("SELECT entity_id FROM section_members WHERE section_id = ?1")?;
        let members: Vec<String> = stmt.query_map([section_id_str], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // 4. 清成员旧位置
        for member_id in &members {
            self.conn.execute(
                "DELETE FROM positions WHERE entity_id = ?1 AND whiteboard_id = ?2",
                params![member_id, old_wb],
            )?;
        }

        // 5. 清 section 自身旧位置
        self.conn.execute(
            "DELETE FROM positions WHERE entity_id = ?1 AND whiteboard_id = ?2",
            params![section_id_str, old_wb],
        )?;

        // 6. 清跨白板 section_link
        let mut edge_stmt = self.conn.prepare(
            "SELECT from_id, to_id FROM edges WHERE (from_id = ?1 OR to_id = ?1) AND edge_type = 'section_link'"
        )?;
        let edge_pairs: Vec<(String, String)> = edge_stmt.query_map([section_id_str], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        for (from_id, to_id) in &edge_pairs {
            let other_id = if from_id == section_id_str { to_id } else { from_id };
            let other_wb: Result<String, _> = self.conn.query_row(
                "SELECT whiteboard_id FROM entities WHERE id = ?1",
                [other_id],
                |r| r.get(0),
            );
            if let Ok(wb) = other_wb
                && wb.as_str() != target_whiteboard_id.as_str()
            {
                self.conn.execute(
                    "DELETE FROM edges WHERE from_id = ?1 AND to_id = ?2 AND edge_type = 'section_link'",
                    params![from_id, to_id],
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use crate::domain::entity::{EntityGraph, SqliteEntityGraph};

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    /// 测试 fixture 桥接:把 String / &str id 包成 SectionId。W4 渗透后 trait
    /// 方法签名要求 `&SectionId`,fixture 数据来自 `store.create(...).id`
    /// (String),这里集中转换避免每处重复 SectionId::parse(...).unwrap()。
    fn sid(s: &str) -> SectionId {
        SectionId::parse(s).expect("test fixture section id 应合法")
    }

    #[test]
    fn test_create_section() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create(&WhiteboardId::parse("wb_root").unwrap(),"My Section", Some("blue")).unwrap();
        assert!(sec.id.starts_with("sec_"));
        assert_eq!(sec.title, "My Section");
        assert_eq!(sec.color, Some("blue".to_string()));
        assert!(sec.card_ids.is_empty());
    }

    #[test]
    fn test_delete_section() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create(&WhiteboardId::parse("wb_root").unwrap(),"To Delete", None).unwrap();
        store.delete(&sid(&sec.id)).unwrap();
        assert!(matches!(store.get(&sid(&sec.id)), Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_update_section_title() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create(&WhiteboardId::parse("wb_root").unwrap(),"Old", None).unwrap();
        store.update(&sid(&sec.id), Some("New"), None).unwrap();
        let updated = store.get(&sid(&sec.id)).unwrap();
        assert_eq!(updated.title, "New");
    }

    #[test]
    fn test_add_remove_member() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create(&WhiteboardId::parse("wb_root").unwrap(),"Sec", None).unwrap();
        store.add_member(&sid(&sec.id), "card_aaa").unwrap();
        store.add_member(&sid(&sec.id), "card_bbb").unwrap();

        let loaded = store.get(&sid(&sec.id)).unwrap();
        assert_eq!(loaded.card_ids.len(), 2);

        store.remove_member(&sid(&sec.id), "card_aaa").unwrap();
        let loaded2 = store.get(&sid(&sec.id)).unwrap();
        assert_eq!(loaded2.card_ids.len(), 1);
    }

    #[test]
    fn test_add_member_idempotent() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create(&WhiteboardId::parse("wb_root").unwrap(),"Sec", None).unwrap();
        store.add_member(&sid(&sec.id), "card_aaa").unwrap();
        store.add_member(&sid(&sec.id), "card_aaa").unwrap(); // 重复
        let loaded = store.get(&sid(&sec.id)).unwrap();
        assert_eq!(loaded.card_ids.len(), 1);
    }

    #[test]
    fn test_query_all_sections() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        store.create(&WhiteboardId::parse("wb_root").unwrap(),"Alpha", None).unwrap();
        store.create(&WhiteboardId::parse("wb_root").unwrap(),"Beta", None).unwrap();
        store.create(&WhiteboardId::parse("wb_other").unwrap(), "Gamma", None).unwrap();

        let sections = store.query_all(&WhiteboardId::parse("wb_root").unwrap()).unwrap();
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn test_move_to_whiteboard() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create(&WhiteboardId::parse("wb_root").unwrap(),"Movable", None).unwrap();
        store.add_member(&sid(&sec.id), "card_aaa").unwrap();

        // 设置位置(create 已经写入 auto-position row,这里用 OR REPLACE 覆盖成固定坐标供断言)
        conn.execute(
            "INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, 'wb_root', 10.0, 20.0)",
            [&sec.id],
        ).unwrap();

        store.move_to_whiteboard(&sid(&sec.id), &WhiteboardId::parse("wb_target").unwrap()).unwrap();

        // whiteboard_id 变了
        let new_wb: String = conn.query_row(
            "SELECT whiteboard_id FROM entities WHERE id = ?1",
            [&sec.id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(new_wb, "wb_target");

        // 旧位置被清
        let pos_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM positions WHERE entity_id = ?1 AND whiteboard_id = 'wb_root'",
            [&sec.id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(pos_count, 0);
    }

    #[test]
    fn test_move_cleans_cross_wb_section_links() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec1 = store.create(&WhiteboardId::parse("wb_root").unwrap(),"Sec1", None).unwrap();
        let sec2 = store.create(&WhiteboardId::parse("wb_root").unwrap(),"Sec2", None).unwrap();

        // sec1 → sec2 section_link ——通过原始 SQL 模拟历史遗留数据。新 Edge
        // 枚举不含 SectionLink 变体(业务:section 不主动发边),所以无法通过
        // graph.connect 建立,但 move_to_whiteboard 仍需清理旧数据。
        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'section_link')",
            rusqlite::params![&sec1.id, &sec2.id],
        ).unwrap();
        let graph = SqliteEntityGraph::new(&conn);

        // 移 sec1 到另一个白板
        store.move_to_whiteboard(&sid(&sec1.id), &WhiteboardId::parse("wb_other").unwrap()).unwrap();

        // 跨白板 link 被清
        let edges = graph.edges_from(&sec1.id).unwrap();
        assert!(edges.is_empty(), "跨白板 section_link 应被清除");
    }

    /// 新建 section 应当自动写 positions 行 —— 空白板 (0, 0),
    /// 后续节点 y = max_y + DEFAULT_NODE_HEIGHT(140) + NODE_SPACING(40) = 180。
    /// 没有 auto-position 的话前端画布拿不到坐标,section 不渲染。
    #[test]
    fn test_create_writes_auto_position() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);

        let first = store.create(&WhiteboardId::parse("wb_root").unwrap(),"First", None).unwrap();
        let (x0, y0): (f64, f64) = conn
            .query_row(
                "SELECT x, y FROM positions WHERE entity_id = ?1 AND whiteboard_id = 'wb_root'",
                [&first.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((x0, y0), (0.0, 0.0), "空白板第一个 section 应在 (0, 0)");

        let second = store.create(&WhiteboardId::parse("wb_root").unwrap(),"Second", None).unwrap();
        let (x1, y1): (f64, f64) = conn
            .query_row(
                "SELECT x, y FROM positions WHERE entity_id = ?1 AND whiteboard_id = 'wb_root'",
                [&second.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(x1, 0.0);
        assert_eq!(y1, 180.0, "第二个 section 落在 DEFAULT_NODE_HEIGHT + NODE_SPACING 下方");
    }

    #[test]
    fn test_move_nonexistent_section() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let result = store.move_to_whiteboard(&sid("sec_nonexist"), &WhiteboardId::parse("wb_target").unwrap());
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }
}
