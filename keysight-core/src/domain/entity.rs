#![allow(dead_code)]
use rusqlite::Connection;

use crate::domain::edge::Edge;
use crate::errors::KeysightError;
use crate::models::{EdgeRow, EdgeType};

/// 实体图谱边操作契约。
pub trait EntityGraph {
    /// 按强类型 [`Edge`] 插入 edges 表。INSERT OR IGNORE — 重复连接幂等。
    ///
    /// 替代旧 `connect(from: &str, to: &str, edge_type: EdgeType, style, label)`
    /// 五参数逃生舱口。参数已经是合法 [`Edge`]，调用方通过
    /// [`crate::domain::edge::user_draw_edge`] 或直接构造
    /// 变体拿到 Edge。style / label 统一写 NULL —— 旧 API 的这两个参数已整体
    /// 退役(TS 从未使用,生产代码从未设值,只有一条单测在测它们)。
    fn connect(&self, edge: &Edge) -> Result<(), KeysightError>;

    /// 断开两个实体的连接(按 DB 行原始键定位)。
    ///
    /// 子阶段 2a 保留旧字符串签名 —— disconnect 的语义是「删除一行 DB 记录」
    /// 而不是「表达一个合法 Edge」,且 TS 侧目前无 disconnect 调用,类型化
    /// 投入收益比不高。子阶段 2b 的 reader 能力到位后,本方法可统一升级到
    /// `disconnect(edge: &Edge)`。
    fn disconnect(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: EdgeType,
    ) -> Result<(), KeysightError>;

    /// 查询某实体的所有出边(返回 DB 行投影,未做类型安全校验)
    fn edges_from(&self, entity_id: &str) -> Result<Vec<EdgeRow>, KeysightError>;

    /// 查询某实体的所有入边(返回 DB 行投影,未做类型安全校验)
    fn edges_to(&self, entity_id: &str) -> Result<Vec<EdgeRow>, KeysightError>;
}

pub struct SqliteEntityGraph<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteEntityGraph<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl EntityGraph for SqliteEntityGraph<'_> {
    fn connect(&self, edge: &Edge) -> Result<(), KeysightError> {
        let (from_id, to_id, edge_type_str) = edge.db_insert_values();
        self.conn.execute(
            "INSERT OR IGNORE INTO edges (from_id, to_id, edge_type, style, label) VALUES (?1, ?2, ?3, NULL, NULL)",
            rusqlite::params![from_id, to_id, edge_type_str],
        )?;
        Ok(())
    }

    fn disconnect(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: EdgeType,
    ) -> Result<(), KeysightError> {
        let edge_type_str = edge_type.as_db_str();
        self.conn.execute(
            "DELETE FROM edges WHERE from_id = ?1 AND to_id = ?2 AND edge_type = ?3",
            rusqlite::params![from_id, to_id, edge_type_str],
        )?;
        Ok(())
    }

    fn edges_from(&self, entity_id: &str) -> Result<Vec<EdgeRow>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT from_id, to_id, edge_type, style, label FROM edges WHERE from_id = ?1",
        )?;
        let edges = stmt
            .query_map(rusqlite::params![entity_id], |row| {
                Ok(EdgeRow {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    edge_type: row.get(2)?,
                    style: row.get(3)?,
                    label: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(edges)
    }

    fn edges_to(&self, entity_id: &str) -> Result<Vec<EdgeRow>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT from_id, to_id, edge_type, style, label FROM edges WHERE to_id = ?1",
        )?;
        let edges = stmt
            .query_map(rusqlite::params![entity_id], |row| {
                Ok(EdgeRow {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    edge_type: row.get(2)?,
                    style: row.get(3)?,
                    label: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use crate::domain::edge::{user_draw_edge, CardId, EntityId};

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    /// 测试辅助:把字符串 id 解析成 [`CardId`],用于构造 `CardRelated` 等
    /// 不经过 [`user_draw_edge`] 的 Edge 变体。
    fn card_id(s: &str) -> CardId {
        match EntityId::parse(s).unwrap() {
            EntityId::Card(c) => c,
            other => panic!("期望 card id,实际: {other:?}"),
        }
    }

    /// 测试辅助:构造一条 card → card 的 [`Edge::CardLink`] fixture。
    fn card_link(from: &str, to: &str) -> Edge {
        user_draw_edge(
            EntityId::parse(from).unwrap(),
            EntityId::parse(to).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn test_connect_creates_edge() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect(&card_link("card_aaa", "card_bbb")).unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from_id, "card_aaa");
        assert_eq!(edges[0].to_id, "card_bbb");
        assert_eq!(edges[0].edge_type, "link_to");
    }

    #[test]
    fn test_connect_idempotent() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        let edge = card_link("card_aaa", "card_bbb");
        graph.connect(&edge).unwrap();
        graph.connect(&edge).unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert_eq!(edges.len(), 1, "重复连接应幂等");
    }

    #[test]
    fn test_disconnect_removes_edge() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect(&card_link("card_aaa", "card_bbb")).unwrap();
        graph
            .disconnect("card_aaa", "card_bbb", EdgeType::LinkTo)
            .unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert!(edges.is_empty(), "断开后应无边");
    }

    #[test]
    fn test_disconnect_nonexistent_is_ok() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph
            .disconnect("card_aaa", "card_bbb", EdgeType::LinkTo)
            .unwrap();
    }

    #[test]
    fn test_edges_to_returns_incoming() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect(&card_link("card_aaa", "card_bbb")).unwrap();
        // card_ccc → card_bbb 通过 CardRelated(非 link_to,走独立变体)
        let related = Edge::CardRelated {
            from: card_id("card_ccc"),
            to: card_id("card_bbb"),
        };
        graph.connect(&related).unwrap();

        let edges = graph.edges_to("card_bbb").unwrap();
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_different_edge_types_coexist() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        let card_a = card_id("card_aaa");
        let card_b = card_id("card_bbb");

        graph
            .connect(&Edge::CardLink {
                from: card_a.clone(),
                to: EntityId::Card(card_b.clone()),
            })
            .unwrap();
        graph
            .connect(&Edge::CardRelated {
                from: card_a,
                to: card_b,
            })
            .unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert_eq!(edges.len(), 2, "不同类型的边应共存");
    }
}
