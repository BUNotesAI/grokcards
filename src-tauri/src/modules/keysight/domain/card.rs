#![allow(dead_code)]

use std::collections::HashMap;

use rusqlite::Connection;

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::models::AtomicCard;

/// 卡片存储契约。
pub(super) trait CardStore {
    /// 按 ID 查询单张卡片（含 tags、edges、card_fields）。
    fn get(&self, id: &str) -> Result<AtomicCard, KeysightError>;
    /// 查询所有卡片，按 mtime 降序，支持分页。
    fn query_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 按文件路径查询。
    fn query_by_file(&self, file_path: &str) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 按 ID 列表批量查询。
    fn query_by_ids(&self, ids: &[String]) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 卡片总数。
    fn count(&self) -> Result<i64, KeysightError>;
}

pub(super) struct SqliteCardStore<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteCardStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

/// 从一组 card id 批量加载 tags，返回 id → Vec<tag> 映射。
fn batch_load_tags(conn: &Connection, ids: &[String]) -> Result<HashMap<String, Vec<String>>, KeysightError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT entity_id, tag FROM entity_tags WHERE entity_id IN ({}) ORDER BY entity_id, tag",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        let entity_id: String = row.get(0)?;
        let tag: String = row.get(1)?;
        Ok((entity_id, tag))
    })?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for r in rows {
        let (entity_id, tag) = r?;
        map.entry(entity_id).or_default().push(tag);
    }
    Ok(map)
}

/// 从一组 card id 批量加载出边（link_to/related/see_also），返回 id → (link_to, related, see_also)。
#[allow(clippy::type_complexity)]
fn batch_load_edges(conn: &Connection, ids: &[String]) -> Result<HashMap<String, (Vec<String>, Vec<String>, Vec<String>)>, KeysightError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT from_id, to_id, edge_type FROM edges WHERE from_id IN ({}) AND edge_type IN ('link_to', 'related', 'see_also')",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        let from_id: String = row.get(0)?;
        let to_id: String = row.get(1)?;
        let edge_type: String = row.get(2)?;
        Ok((from_id, to_id, edge_type))
    })?;

    let mut map: HashMap<String, (Vec<String>, Vec<String>, Vec<String>)> = HashMap::new();
    for r in rows {
        let (from_id, to_id, edge_type) = r?;
        let entry = map.entry(from_id).or_default();
        match edge_type.as_str() {
            "link_to" => entry.0.push(to_id),
            "related" => entry.1.push(to_id),
            "see_also" => entry.2.push(to_id),
            _ => {}
        }
    }
    Ok(map)
}

/// 查询卡片基础行（entities + card_fields + file_mtimes），不含 tags/edges。
struct CardRow {
    id: String,
    title: String,
    content: String,
    file_path: String,
    understanding: String,
    source: String,
    mtime: Option<f64>,
}

/// 基础查询：entities + card_fields + file_mtimes。
fn query_card_rows(conn: &Connection, where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Vec<CardRow>, KeysightError> {
    let sql = format!(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
         COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
         f.mtime \
         FROM entities e \
         LEFT JOIN card_fields c ON e.id = c.entity_id \
         LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
         WHERE e.kind = 'card' {where_clause} \
         ORDER BY f.mtime DESC",
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params, |row| {
        Ok(CardRow {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            file_path: row.get(3)?,
            understanding: row.get(4)?,
            source: row.get(5)?,
            mtime: row.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(KeysightError::from)
}

/// 将 CardRow + tags + edges 组装为 AtomicCard。
fn assemble_cards(conn: &Connection, card_rows: Vec<CardRow>) -> Result<Vec<AtomicCard>, KeysightError> {
    let ids: Vec<String> = card_rows.iter().map(|r| r.id.clone()).collect();
    let tags_map = batch_load_tags(conn, &ids)?;
    let edges_map = batch_load_edges(conn, &ids)?;

    let cards = card_rows
        .into_iter()
        .map(|row| {
            let tags = tags_map.get(&row.id).cloned().unwrap_or_default();
            let (link_to, related, see_also) = edges_map
                .get(&row.id)
                .cloned()
                .unwrap_or_default();
            AtomicCard {
                id: row.id,
                file_path: row.file_path,
                title: row.title,
                content: row.content,
                tags,
                link_to,
                related,
                understanding: row.understanding,
                source: row.source,
                see_also,
                mtime: row.mtime,
            }
        })
        .collect();
    Ok(cards)
}

impl CardStore for SqliteCardStore<'_> {
    fn get(&self, id: &str) -> Result<AtomicCard, KeysightError> {
        let rows = query_card_rows(self.conn, "AND e.id = ?1", &[&id])?;
        match rows.into_iter().next() {
            Some(row) => {
                let mut cards = assemble_cards(self.conn, vec![row])?;
                Ok(cards.remove(0))
            }
            None => Err(KeysightError::NotFound(id.to_string())),
        }
    }

    fn query_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<AtomicCard>, KeysightError> {
        let mut where_clause = String::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(l) = limit {
            where_clause.push_str(" LIMIT ?1");
            param_values.push(Box::new(l));
            if let Some(o) = offset {
                where_clause.push_str(" OFFSET ?2");
                param_values.push(Box::new(o));
            }
        }

        // query_card_rows 内部已有 ORDER BY，LIMIT/OFFSET 追加到末尾
        // 但 query_card_rows 的 where_clause 插在 WHERE 和 ORDER BY 之间
        // 所以这里不能用 where_clause 传 LIMIT — 需要直接构造 SQL
        let sql = if let Some(l) = limit {
            if let Some(o) = offset {
                format!(
                    "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
                     COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
                     f.mtime \
                     FROM entities e \
                     LEFT JOIN card_fields c ON e.id = c.entity_id \
                     LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
                     WHERE e.kind = 'card' \
                     ORDER BY f.mtime DESC \
                     LIMIT {} OFFSET {}", l, o
                )
            } else {
                format!(
                    "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
                     COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
                     f.mtime \
                     FROM entities e \
                     LEFT JOIN card_fields c ON e.id = c.entity_id \
                     LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
                     WHERE e.kind = 'card' \
                     ORDER BY f.mtime DESC \
                     LIMIT {}", l
                )
            }
        } else {
            "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
             COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
             f.mtime \
             FROM entities e \
             LEFT JOIN card_fields c ON e.id = c.entity_id \
             LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
             WHERE e.kind = 'card' \
             ORDER BY f.mtime DESC".to_string()
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(CardRow {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                file_path: row.get(3)?,
                understanding: row.get(4)?,
                source: row.get(5)?,
                mtime: row.get(6)?,
            })
        })?;
        let card_rows: Vec<CardRow> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        assemble_cards(self.conn, card_rows)
    }

    fn query_by_file(&self, file_path: &str) -> Result<Vec<AtomicCard>, KeysightError> {
        let rows = query_card_rows(self.conn, "AND e.file_path = ?1", &[&file_path])?;
        assemble_cards(self.conn, rows)
    }

    fn query_by_ids(&self, ids: &[String]) -> Result<Vec<AtomicCard>, KeysightError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{i}")).collect();
        let where_clause = format!("AND e.id IN ({})", placeholders.join(", "));
        let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let rows = query_card_rows(self.conn, &where_clause, params.as_slice())?;
        assemble_cards(self.conn, rows)
    }

    fn count(&self) -> Result<i64, KeysightError> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM entities WHERE kind = 'card'",
            [],
            |r| r.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keysight::db::init_db;
    use crate::modules::keysight::domain::sync;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    /// 插入一张标准测试卡片。
    fn seed_card(conn: &Connection) {
        let md = "\
---
type: atomic-card
id: card_test0001
tags:
  - rust
  - ownership
linkTo:
  - card_other001
related:
  - card_other002
understanding: 测试理解
source: https://example.com
see-also:
  - card_other003
---

# 【ATC】Test Card

Body content.
";
        sync::sync_file(conn, "atomic cards/test.md", md, 1000.0).unwrap();
    }

    /// 插入第二张卡片用于批量查询测试。
    fn seed_card2(conn: &Connection) {
        let md = "\
---
type: atomic-card
id: card_test0002
tags:
  - concurrency
---

# 【ATC】Second Card

Second body.
";
        sync::sync_file(conn, "atomic cards/test2.md", md, 2000.0).unwrap();
    }

    #[test]
    fn test_get_card_by_id() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.title, "Test Card");
        assert_eq!(card.file_path, "atomic cards/test.md");
        assert_eq!(card.content, "Body content.\n");
        assert_eq!(card.tags, vec!["ownership", "rust"]); // 字母序（DB ORDER BY tag）
        assert_eq!(card.link_to, vec!["card_other001"]);
        assert_eq!(card.related, vec!["card_other002"]);
        assert_eq!(card.see_also, vec!["card_other003"]);
        assert_eq!(card.understanding, "测试理解");
        assert_eq!(card.source, "https://example.com");
        assert!(card.mtime.is_some());
    }

    #[test]
    fn test_get_card_not_found() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        let result = store.get("card_nonexist");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_query_all() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_all(None, None).unwrap();
        assert_eq!(cards.len(), 2);
        // mtime 降序 — card2(2000) 在前，card1(1000) 在后
        assert_eq!(cards[0].id, "card_test0002");
        assert_eq!(cards[1].id, "card_test0001");
    }

    #[test]
    fn test_query_all_with_limit() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_all(Some(1), None).unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn test_query_by_file() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_by_file("atomic cards/test.md").unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "card_test0001");
    }

    #[test]
    fn test_query_by_file_no_match() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        let cards = store.query_by_file("nonexist.md").unwrap();
        assert!(cards.is_empty());
    }

    #[test]
    fn test_query_by_ids() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let ids = vec!["card_test0001".to_string(), "card_test0002".to_string()];
        let cards = store.query_by_ids(&ids).unwrap();
        assert_eq!(cards.len(), 2);
    }

    #[test]
    fn test_query_by_ids_partial() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let ids = vec!["card_test0001".to_string(), "card_nonexist".to_string()];
        let cards = store.query_by_ids(&ids).unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn test_count() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn test_count_empty() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        assert_eq!(store.count().unwrap(), 0);
    }
}
