#![allow(dead_code)]
use rusqlite::Connection;

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::models::{EdgeRow, EdgeStyle, EdgeType};

/// 实体图谱边操作契约。
pub(in crate::modules::keysight) trait EntityGraph {
    /// 连接两个实体。INSERT OR IGNORE — 重复连接幂等。
    fn connect(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: EdgeType,
        style: Option<EdgeStyle>,
        label: Option<&str>,
    ) -> Result<(), KeysightError>;

    /// 断开两个实体的连接。
    fn disconnect(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: EdgeType,
    ) -> Result<(), KeysightError>;

    /// 查询某实体的所有出边。
    fn edges_from(&self, entity_id: &str) -> Result<Vec<EdgeRow>, KeysightError>;

    /// 查询某实体的所有入边。
    fn edges_to(&self, entity_id: &str) -> Result<Vec<EdgeRow>, KeysightError>;
}

pub(in crate::modules::keysight) struct SqliteEntityGraph<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteEntityGraph<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

/// 把用户从 ⋯ 菜单 Draw connection 画线时 caller 传入的 edge_type
/// 按 source 实体前缀归一化为 Rust 持久化层读写一致的值。
///
/// 背景：`NoteStore::get` / `AliasStore::get` 按 `note_link` / `alias_link`
/// 回读 edges 表，若 entity_connect 写入时仍用 caller 传的 `LinkTo`，
/// 边会真写入 DB 但在 note/alias 侧永远读不回，buildEdges 也就渲染不出。
/// 所以在 Rust 入口按 from_id 前缀强制归一化，TS 侧可无脑传 `LinkTo`。
///
/// - `note_*` → [`EdgeType::NoteLink`]（覆盖 caller）
/// - `alias_*` → [`EdgeType::AliasLink`]（覆盖 caller）
/// - 其它（主要是 `card_*`）→ 保留 caller 传入值（支持 Related picker 的 Related / 历史 SeeAlso）
pub(in crate::modules::keysight) fn resolve_user_drawn_edge_type(
    from_id: &str,
    caller_edge_type: EdgeType,
) -> EdgeType {
    if from_id.starts_with("note_") {
        EdgeType::NoteLink
    } else if from_id.starts_with("alias_") {
        EdgeType::AliasLink
    } else {
        caller_edge_type
    }
}

impl EntityGraph for SqliteEntityGraph<'_> {
    fn connect(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: EdgeType,
        style: Option<EdgeStyle>,
        label: Option<&str>,
    ) -> Result<(), KeysightError> {
        let edge_type_str = edge_type.as_db_str();
        let style_str = style.map(|s| s.as_db_str().to_string());
        self.conn.execute(
            "INSERT OR IGNORE INTO edges (from_id, to_id, edge_type, style, label) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![from_id, to_id, edge_type_str, style_str, label],
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
    use crate::modules::keysight::db::init_db;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_connect_creates_edge() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect("card_aaa", "card_bbb", EdgeType::LinkTo, None, None).unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from_id, "card_aaa");
        assert_eq!(edges[0].to_id, "card_bbb");
        assert_eq!(edges[0].edge_type, "link_to");
    }

    #[test]
    fn test_connect_with_style_and_label() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect("card_aaa", "card_bbb", EdgeType::Related, Some(EdgeStyle::Dashed), Some("参考")).unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert_eq!(edges[0].style, Some("dashed".to_string()));
        assert_eq!(edges[0].label, Some("参考".to_string()));
    }

    #[test]
    fn test_connect_idempotent() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect("card_aaa", "card_bbb", EdgeType::LinkTo, None, None).unwrap();
        graph.connect("card_aaa", "card_bbb", EdgeType::LinkTo, None, None).unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert_eq!(edges.len(), 1, "重复连接应幂等");
    }

    #[test]
    fn test_disconnect_removes_edge() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect("card_aaa", "card_bbb", EdgeType::LinkTo, None, None).unwrap();
        graph.disconnect("card_aaa", "card_bbb", EdgeType::LinkTo).unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert!(edges.is_empty(), "断开后应无边");
    }

    #[test]
    fn test_disconnect_nonexistent_is_ok() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.disconnect("card_aaa", "card_bbb", EdgeType::LinkTo).unwrap();
    }

    #[test]
    fn test_edges_to_returns_incoming() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect("card_aaa", "card_bbb", EdgeType::LinkTo, None, None).unwrap();
        graph.connect("card_ccc", "card_bbb", EdgeType::Related, None, None).unwrap();

        let edges = graph.edges_to("card_bbb").unwrap();
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_different_edge_types_coexist() {
        let conn = test_conn();
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect("card_aaa", "card_bbb", EdgeType::LinkTo, None, None).unwrap();
        graph.connect("card_aaa", "card_bbb", EdgeType::Related, None, None).unwrap();

        let edges = graph.edges_from("card_aaa").unwrap();
        assert_eq!(edges.len(), 2, "不同类型的边应共存");
    }

    // ============================================================
    // resolve_user_drawn_edge_type — 前端 ⋯ 菜单 Draw connection 的归一化
    // ============================================================
    //
    // 背景：TS 侧 onDrawConnectionFrom 不知道 Rust 持久化约定（Note 的
    // linked_note_ids 只从 edge_type='note_link' 的 edge 读回，Alias 同理），
    // 所以在 Rust 入口按 from_id 前缀强制归一化，让 TS 无脑传 LinkTo 即可。

    #[test]
    fn test_resolve_user_drawn_edge_type_note_source_forces_note_link() {
        // note 源 — 不管 caller 传什么都应归一为 NoteLink
        assert_eq!(
            resolve_user_drawn_edge_type("note_aaa11111", EdgeType::LinkTo),
            EdgeType::NoteLink
        );
        assert_eq!(
            resolve_user_drawn_edge_type("note_aaa11111", EdgeType::Related),
            EdgeType::NoteLink
        );
    }

    #[test]
    fn test_resolve_user_drawn_edge_type_alias_source_forces_alias_link() {
        // alias 源 — 归一为 AliasLink
        assert_eq!(
            resolve_user_drawn_edge_type("alias_xxx22222", EdgeType::LinkTo),
            EdgeType::AliasLink
        );
    }

    #[test]
    fn test_resolve_user_drawn_edge_type_card_source_preserves_caller() {
        // card 源 — 保留 caller 传入，支持 Related picker 的 Related / 历史 SeeAlso
        assert_eq!(
            resolve_user_drawn_edge_type("card_xxx33333", EdgeType::LinkTo),
            EdgeType::LinkTo
        );
        assert_eq!(
            resolve_user_drawn_edge_type("card_xxx33333", EdgeType::Related),
            EdgeType::Related
        );
        assert_eq!(
            resolve_user_drawn_edge_type("card_xxx33333", EdgeType::SeeAlso),
            EdgeType::SeeAlso
        );
    }
}
