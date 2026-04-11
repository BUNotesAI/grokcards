#![allow(dead_code)]

use rusqlite::Connection;

use super::super::errors::KeysightError;
use super::super::models::{
    LegacyAlias, LegacyInsight, LegacyNote, LegacyPosition, LegacySection, WhiteboardMapping,
};

/// 旧 DB v1 schema DDL（测试用，不含 triggers）。
const LEGACY_SCHEMA_V1_SQL: &str = "
CREATE TABLE insights (
    id TEXT PRIMARY KEY,
    filePath TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    linkTo TEXT NOT NULL DEFAULT '[]',
    related TEXT NOT NULL DEFAULT '[]',
    understanding TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT '',
    seeAlso TEXT NOT NULL DEFAULT '[]',
    mtime REAL NOT NULL DEFAULT 0
);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE file_mtimes (filePath TEXT PRIMARY KEY, mtime REAL NOT NULL);
";

/// position entity_id 的合法前缀。
const VALID_POSITION_PREFIXES: &[&str] = &["card_", "sec_", "note_", "alias_"];

/// 从旧 DB (v1 schema) 读取数据的行为契约。
pub(in crate::modules::keysight) trait LegacyReader {
    /// 读取所有 insights 行。
    fn read_insights(&self) -> Result<Vec<LegacyInsight>, KeysightError>;
    /// 读取指定白板的 sections。
    fn read_sections(&self, whiteboard_key: &str) -> Result<Vec<LegacySection>, KeysightError>;
    /// 读取指定白板的 notes。
    fn read_notes(&self, whiteboard_key: &str) -> Result<Vec<LegacyNote>, KeysightError>;
    /// 读取指定白板的 aliases。
    fn read_aliases(&self, whiteboard_key: &str) -> Result<Vec<LegacyAlias>, KeysightError>;
    /// 读取指定白板的 positions（过滤无效前缀）。
    fn read_positions(&self, whiteboard_key: &str) -> Result<Vec<LegacyPosition>, KeysightError>;
    /// 列出所有白板（根据 meta 表 graph_sections* key 推断）。
    fn list_whiteboards(&self) -> Result<Vec<WhiteboardMapping>, KeysightError>;
}

/// 从旧 SQLite DB 读取的 LegacyReader 实现。
pub(in crate::modules::keysight) struct SqliteLegacyReader<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteLegacyReader<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 解析 JSON 数组字符串为 Vec<String>。
    fn parse_json_array(json: &str) -> Vec<String> {
        serde_json::from_str::<Vec<String>>(json).unwrap_or_default()
    }

    /// 构造 meta key：无后缀 → "graph_{kind}"，有后缀 → "graph_{kind}:{suffix}"。
    fn meta_key(kind: &str, whiteboard_key: &str) -> String {
        if whiteboard_key.is_empty() {
            format!("graph_{kind}")
        } else {
            format!("graph_{kind}:{whiteboard_key}")
        }
    }

    /// 从 meta 表读取 JSON blob。不存在则返回空字符串。
    fn read_meta_blob(&self, key: &str) -> Result<String, KeysightError> {
        let result = self.conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [key],
            |r| r.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(String::new()),
            Err(e) => Err(KeysightError::Database(e)),
        }
    }
}

impl<'a> LegacyReader for SqliteLegacyReader<'a> {
    fn read_insights(&self) -> Result<Vec<LegacyInsight>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filePath, title, content, tags, linkTo, related, understanding, source, seeAlso, mtime FROM insights",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(LegacyInsight {
                id: r.get(0)?,
                file_path: r.get(1)?,
                title: r.get(2)?,
                content: r.get(3)?,
                tags: Self::parse_json_array(&r.get::<_, String>(4)?),
                link_to: Self::parse_json_array(&r.get::<_, String>(5)?),
                related: Self::parse_json_array(&r.get::<_, String>(6)?),
                understanding: r.get(7)?,
                source: r.get(8)?,
                see_also: Self::parse_json_array(&r.get::<_, String>(9)?),
                mtime: r.get(10)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(KeysightError::Database)
    }

    fn read_sections(&self, whiteboard_key: &str) -> Result<Vec<LegacySection>, KeysightError> {
        let key = Self::meta_key("sections", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() {
            return Ok(vec![]);
        }

        let arr: Vec<serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(arr
            .into_iter()
            .map(|v| LegacySection {
                id: v["id"].as_str().unwrap_or_default().to_string(),
                title: v["title"].as_str().unwrap_or_default().to_string(),
                color: v["color"].as_str().map(|s| s.to_string()),
                card_ids: Self::parse_json_array(&v["cardIds"].to_string()),
                linked_section_ids: Self::parse_json_array(
                    &v["linkedSectionIds"].to_string(),
                ),
            })
            .collect())
    }

    fn read_notes(&self, whiteboard_key: &str) -> Result<Vec<LegacyNote>, KeysightError> {
        let key = Self::meta_key("notes", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() {
            return Ok(vec![]);
        }

        let arr: Vec<serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(arr
            .into_iter()
            .map(|v| LegacyNote {
                id: v["id"].as_str().unwrap_or_default().to_string(),
                title: v["title"].as_str().unwrap_or_default().to_string(),
                content: v["content"].as_str().unwrap_or_default().to_string(),
                linked_card_ids: Self::parse_json_array(&v["linkedCardIds"].to_string()),
                linked_note_ids: Self::parse_json_array(&v["linkedNoteIds"].to_string()),
                linked_section_ids: Self::parse_json_array(
                    &v["linkedSectionIds"].to_string(),
                ),
            })
            .collect())
    }

    fn read_aliases(&self, whiteboard_key: &str) -> Result<Vec<LegacyAlias>, KeysightError> {
        let key = Self::meta_key("aliases", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() {
            return Ok(vec![]);
        }

        let arr: Vec<serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(arr
            .into_iter()
            .map(|v| LegacyAlias {
                alias_id: v["aliasId"].as_str().unwrap_or_default().to_string(),
                card_id: v["cardId"].as_str().unwrap_or_default().to_string(),
                linked_card_ids: Self::parse_json_array(&v["linkedCardIds"].to_string()),
                linked_section_ids: Self::parse_json_array(
                    &v["linkedSectionIds"].to_string(),
                ),
                incoming_card_ids: Self::parse_json_array(
                    &v["incomingCardIds"].to_string(),
                ),
            })
            .collect())
    }

    fn read_positions(&self, whiteboard_key: &str) -> Result<Vec<LegacyPosition>, KeysightError> {
        let key = Self::meta_key("positions", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() {
            return Ok(vec![]);
        }

        let obj: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(obj
            .into_iter()
            .filter(|(k, _)| VALID_POSITION_PREFIXES.iter().any(|p| k.starts_with(p)))
            .map(|(k, v)| LegacyPosition {
                entity_id: k,
                x: v["x"].as_f64().unwrap_or(0.0),
                y: v["y"].as_f64().unwrap_or(0.0),
            })
            .collect())
    }

    fn list_whiteboards(&self) -> Result<Vec<WhiteboardMapping>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT key FROM meta WHERE key LIKE 'graph_sections%' ORDER BY key",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;

        let mut mappings = Vec::new();
        for key in rows {
            let key = key?;
            if let Some(suffix) = key.strip_prefix("graph_sections:") {
                mappings.push(WhiteboardMapping {
                    meta_suffix: Some(suffix.to_string()),
                    whiteboard_id: suffix.to_string(),
                });
            } else if key == "graph_sections" {
                mappings.push(WhiteboardMapping {
                    meta_suffix: None,
                    whiteboard_id: "wb_root".to_string(),
                });
            }
        }
        Ok(mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(LEGACY_SCHEMA_V1_SQL).unwrap();
        conn
    }

    fn seed_insight(
        conn: &Connection,
        id: &str,
        title: &str,
        tags: &str,
        link_to: &str,
        related: &str,
    ) {
        conn.execute(
            "INSERT INTO insights (id, filePath, title, content, tags, linkTo, related, understanding, source, seeAlso, mtime)
             VALUES (?1, ?2, ?3, 'body', ?4, ?5, ?6, 'understand', 'src', '[]', 1000.0)",
            rusqlite::params![
                id,
                format!("whiteboard/rust/{title}.md"),
                title,
                tags,
                link_to,
                related
            ],
        )
        .unwrap();
    }

    fn seed_meta(conn: &Connection, key: &str, value: &str) {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)",
            [key, value],
        )
        .unwrap();
    }

    #[test]
    fn test_read_insights_basic() {
        let conn = legacy_conn();
        seed_insight(
            &conn,
            "card_aaa11111",
            "Test Card",
            r#"["rust","trait"]"#,
            r#"["card_bbb22222"]"#,
            r#"["card_ccc33333"]"#,
        );
        seed_insight(
            &conn,
            "card_bbb22222",
            "Second Card",
            r#"[]"#,
            r#"[]"#,
            r#"[]"#,
        );

        let reader = SqliteLegacyReader::new(&conn);
        let insights = reader.read_insights().unwrap();

        assert_eq!(insights.len(), 2);
        let first = insights.iter().find(|i| i.id == "card_aaa11111").unwrap();
        assert_eq!(first.title, "Test Card");
        assert_eq!(first.tags, vec!["rust", "trait"]);
        assert_eq!(first.link_to, vec!["card_bbb22222"]);
        assert_eq!(first.related, vec!["card_ccc33333"]);
        assert_eq!(first.understanding, "understand");
        assert_eq!(first.source, "src");
        assert!(first.file_path.contains("whiteboard/rust/"));
    }

    #[test]
    fn test_read_insights_empty_json() {
        let conn = legacy_conn();
        seed_insight(&conn, "card_aaa11111", "Empty", "[]", "[]", "[]");

        let reader = SqliteLegacyReader::new(&conn);
        let insights = reader.read_insights().unwrap();
        let card = &insights[0];
        assert!(card.tags.is_empty());
        assert!(card.link_to.is_empty());
        assert!(card.related.is_empty());
    }

    #[test]
    fn test_read_sections_mixed_members() {
        let conn = legacy_conn();
        seed_meta(
            &conn,
            "graph_sections:rust",
            r#"[{"id":"sec_aaa11111","title":"Section 1","color":"blue","cardIds":["card_aaa11111","alias_bbb22222","note_ccc33333"],"linkedSectionIds":["sec_ddd44444"]}]"#,
        );

        let reader = SqliteLegacyReader::new(&conn);
        let sections = reader.read_sections("rust").unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "sec_aaa11111");
        assert_eq!(sections[0].title, "Section 1");
        assert_eq!(sections[0].color, Some("blue".to_string()));
        assert_eq!(
            sections[0].card_ids,
            vec!["card_aaa11111", "alias_bbb22222", "note_ccc33333"]
        );
        assert_eq!(sections[0].linked_section_ids, vec!["sec_ddd44444"]);
    }

    #[test]
    fn test_read_positions_filters_junk() {
        let conn = legacy_conn();
        seed_meta(
            &conn,
            "graph_positions:rust",
            r#"{"card_aaa11111":{"x":10.0,"y":20.0},"agent":{"x":0,"y":0},"--no-move":{"x":1,"y":1},"builtin_java":{"x":2,"y":2},"sec_bbb22222":{"x":30.0,"y":40.0}}"#,
        );

        let reader = SqliteLegacyReader::new(&conn);
        let positions = reader.read_positions("rust").unwrap();
        assert_eq!(positions.len(), 2);
        assert!(positions.iter().any(|p| p.entity_id == "card_aaa11111"));
        assert!(positions.iter().any(|p| p.entity_id == "sec_bbb22222"));
    }

    #[test]
    fn test_read_notes_with_links() {
        let conn = legacy_conn();
        seed_meta(
            &conn,
            "graph_notes:chentian",
            r#"[{"id":"note_aaa11111","title":"Note 1","content":"body","linkedCardIds":["alias_bbb22222"],"linkedNoteIds":["note_ccc33333"],"linkedSectionIds":[]}]"#,
        );

        let reader = SqliteLegacyReader::new(&conn);
        let notes = reader.read_notes("chentian").unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, "note_aaa11111");
        assert_eq!(notes[0].linked_card_ids, vec!["alias_bbb22222"]);
        assert_eq!(notes[0].linked_note_ids, vec!["note_ccc33333"]);
    }

    #[test]
    fn test_read_aliases() {
        let conn = legacy_conn();
        seed_meta(
            &conn,
            "graph_aliases:chentian",
            r#"[{"aliasId":"alias_aaa11111","cardId":"card_bbb22222","linkedCardIds":["card_ccc33333"],"linkedSectionIds":[],"incomingCardIds":["card_ddd44444"]}]"#,
        );

        let reader = SqliteLegacyReader::new(&conn);
        let aliases = reader.read_aliases("chentian").unwrap();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].alias_id, "alias_aaa11111");
        assert_eq!(aliases[0].card_id, "card_bbb22222");
        assert_eq!(aliases[0].linked_card_ids, vec!["card_ccc33333"]);
        assert_eq!(aliases[0].incoming_card_ids, vec!["card_ddd44444"]);
    }

    #[test]
    fn test_list_whiteboards() {
        let conn = legacy_conn();
        seed_meta(&conn, "graph_sections", "[]");
        seed_meta(&conn, "graph_sections:chentian", "[]");
        seed_meta(&conn, "graph_sections:rust", "[]");
        seed_meta(&conn, "schema_version", "1");

        let reader = SqliteLegacyReader::new(&conn);
        let wbs = reader.list_whiteboards().unwrap();
        assert_eq!(wbs.len(), 3);
        assert!(wbs
            .iter()
            .any(|w| w.whiteboard_id == "wb_root" && w.meta_suffix.is_none()));
        assert!(wbs.iter().any(
            |w| w.whiteboard_id == "chentian" && w.meta_suffix == Some("chentian".to_string())
        ));
        assert!(wbs
            .iter()
            .any(|w| w.whiteboard_id == "rust" && w.meta_suffix == Some("rust".to_string())));
    }
}
