#![allow(dead_code)]

use std::collections::HashSet;

use rusqlite::{params, Connection};

use super::super::errors::KeysightError;
use super::super::models::{
    ImportSummary, LegacyAlias, LegacyInsight, LegacyNote, LegacyPosition, LegacySection,
    SkippedItem, WhiteboardMapping,
};
use super::sync::derive_whiteboard_id;

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

/// 将旧 DB 数据导入新 DB (v7 schema) 的行为契约。
pub(in crate::modules::keysight) trait LegacyImporter {
    /// 从旧 DB 读取全部数据并写入新 DB。
    ///
    /// ## 执行效果
    /// 1. 在单个事务内完成所有写入
    /// 2. 导入 cards → entities + card_fields + entity_tags + edges
    /// 3. 导入每个 whiteboard 的 sections / notes / aliases / positions
    /// 4. 重建 FTS 索引
    ///
    /// ## 幂等性
    /// 使用 INSERT OR REPLACE，重复调用不会产生重复数据。
    ///
    /// ## 不做的事
    /// - 不删除新 DB 中已有的、旧 DB 中没有的数据
    fn import(&self, reader: &dyn LegacyReader) -> Result<ImportSummary, KeysightError>;
}

/// 基于 SQLite 连接的 LegacyImporter 实现。
pub(in crate::modules::keysight) struct SqliteLegacyImporter<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteLegacyImporter<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> LegacyImporter for SqliteLegacyImporter<'a> {
    fn import(&self, reader: &dyn LegacyReader) -> Result<ImportSummary, KeysightError> {
        let mut summary = ImportSummary {
            cards: 0,
            sections: 0,
            notes: 0,
            aliases: 0,
            edges: 0,
            positions: 0,
            section_members: 0,
            skipped: Vec::new(),
        };

        self.conn.execute("BEGIN", [])?;

        // ── 1. 导入 cards ──────────────────────────────────
        let insights = reader.read_insights()?;
        let mut known_ids: HashSet<String> = HashSet::new();

        for ins in &insights {
            known_ids.insert(ins.id.clone());

            let wb_id = derive_whiteboard_id(&ins.file_path);
            // entities
            self.conn.execute(
                "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id, file_path, content) \
                 VALUES (?1, 'card', ?2, ?3, ?4, ?5)",
                params![ins.id, ins.title, wb_id, ins.file_path, ins.content],
            )?;
            // card_fields
            self.conn.execute(
                "INSERT OR REPLACE INTO card_fields (entity_id, understanding, source) \
                 VALUES (?1, ?2, ?3)",
                params![ins.id, ins.understanding, ins.source],
            )?;
            summary.cards += 1;

            // tags — 先删后插，保证幂等
            self.conn.execute(
                "DELETE FROM entity_tags WHERE entity_id = ?1",
                [&ins.id],
            )?;
            for tag in &ins.tags {
                self.conn.execute(
                    "INSERT OR IGNORE INTO entity_tags (entity_id, tag) VALUES (?1, ?2)",
                    params![ins.id, tag],
                )?;
            }

            // edges — link_to / related / see_also（仅目标在 known_ids 中才写入）
            // 先收集，等全部 card 注册完 known_ids 后统一处理
        }

        // card edges — 需要 known_ids 完整后再处理
        for ins in &insights {
            // 先清理旧 edges
            self.conn.execute(
                "DELETE FROM edges WHERE from_id = ?1 AND edge_type IN ('link_to','related','see_also')",
                [&ins.id],
            )?;

            for target in &ins.link_to {
                if known_ids.contains(target) {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'link_to')",
                        params![ins.id, target],
                    )?;
                    summary.edges += 1;
                } else {
                    summary.skipped.push(SkippedItem {
                        entity_id: ins.id.clone(),
                        reason: format!("dangling link_to target: {target}"),
                    });
                }
            }
            for target in &ins.related {
                if known_ids.contains(target) {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'related')",
                        params![ins.id, target],
                    )?;
                    summary.edges += 1;
                } else {
                    summary.skipped.push(SkippedItem {
                        entity_id: ins.id.clone(),
                        reason: format!("dangling related target: {target}"),
                    });
                }
            }
            for target in &ins.see_also {
                if known_ids.contains(target) {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'see_also')",
                        params![ins.id, target],
                    )?;
                    summary.edges += 1;
                } else {
                    summary.skipped.push(SkippedItem {
                        entity_id: ins.id.clone(),
                        reason: format!("dangling see_also target: {target}"),
                    });
                }
            }
        }

        // ── 2. 导入每个白板的 sections / notes / aliases / positions ──
        let whiteboards = reader.list_whiteboards()?;
        for wb in &whiteboards {
            let meta_key = wb.meta_suffix.as_deref().unwrap_or("");

            // ── sections ──
            let sections = reader.read_sections(meta_key)?;
            for sec in &sections {
                self.conn.execute(
                    "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id, color) \
                     VALUES (?1, 'section', ?2, ?3, ?4)",
                    params![sec.id, sec.title, wb.whiteboard_id, sec.color],
                )?;
                summary.sections += 1;

                // section_members
                self.conn.execute(
                    "DELETE FROM section_members WHERE section_id = ?1",
                    [&sec.id],
                )?;
                for member_id in &sec.card_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO section_members (section_id, entity_id) \
                         VALUES (?1, ?2)",
                        params![sec.id, member_id],
                    )?;
                    summary.section_members += 1;
                }

                // section_link edges
                for target_sec_id in &sec.linked_section_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                         VALUES (?1, ?2, 'section_link')",
                        params![sec.id, target_sec_id],
                    )?;
                    summary.edges += 1;
                }
            }

            // ── notes ──
            let notes = reader.read_notes(meta_key)?;
            for note in &notes {
                self.conn.execute(
                    "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id, content) \
                     VALUES (?1, 'note', ?2, ?3, ?4)",
                    params![note.id, note.title, wb.whiteboard_id, note.content],
                )?;
                summary.notes += 1;

                // note_link edges（linked_card_ids + linked_note_ids + linked_section_ids）
                for target in &note.linked_card_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                         VALUES (?1, ?2, 'note_link')",
                        params![note.id, target],
                    )?;
                    summary.edges += 1;
                }
                for target in &note.linked_note_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                         VALUES (?1, ?2, 'note_link')",
                        params![note.id, target],
                    )?;
                    summary.edges += 1;
                }
                for target in &note.linked_section_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                         VALUES (?1, ?2, 'note_link')",
                        params![note.id, target],
                    )?;
                    summary.edges += 1;
                }
            }

            // ── aliases ──
            let aliases = reader.read_aliases(meta_key)?;
            for alias in &aliases {
                // alias entity — title 存 cardId 以便追溯
                self.conn.execute(
                    "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id) \
                     VALUES (?1, 'alias', ?2, ?3)",
                    params![alias.alias_id, alias.card_id, wb.whiteboard_id],
                )?;
                // alias_fields
                self.conn.execute(
                    "INSERT OR REPLACE INTO alias_fields (entity_id, card_id) \
                     VALUES (?1, ?2)",
                    params![alias.alias_id, alias.card_id],
                )?;
                summary.aliases += 1;

                // alias_link edge（alias → 原 card）
                self.conn.execute(
                    "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                     VALUES (?1, ?2, 'alias_link')",
                    params![alias.alias_id, alias.card_id],
                )?;
                summary.edges += 1;

                // alias_link edge（alias → linked_card_ids）
                for target in &alias.linked_card_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                         VALUES (?1, ?2, 'alias_link')",
                        params![alias.alias_id, target],
                    )?;
                    summary.edges += 1;
                }
                for target in &alias.linked_section_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                         VALUES (?1, ?2, 'alias_link')",
                        params![alias.alias_id, target],
                    )?;
                    summary.edges += 1;
                }

                // card_to_alias edge（原 card → alias）
                self.conn.execute(
                    "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) \
                     VALUES (?1, ?2, 'card_to_alias')",
                    params![alias.card_id, alias.alias_id],
                )?;
                summary.edges += 1;
            }

            // ── positions ──
            let positions = reader.read_positions(meta_key)?;
            for pos in &positions {
                self.conn.execute(
                    "INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![pos.entity_id, wb.whiteboard_id, pos.x, pos.y],
                )?;
                summary.positions += 1;
            }
        }

        // ── 3. 重建 FTS 索引 ──────────────────────────────
        self.conn.execute("DELETE FROM entities_fts", [])?;
        let mut fts_stmt = self.conn.prepare(
            "SELECT id, title, content FROM entities WHERE kind = 'card'",
        )?;
        let fts_rows: Vec<(String, String, String)> = fts_stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(fts_stmt);

        for (id, title, content) in &fts_rows {
            self.conn.execute(
                "INSERT INTO entities_fts (id, title, content) VALUES (?1, ?2, ?3)",
                params![id, title, content],
            )?;
        }

        self.conn.execute("COMMIT", [])?;

        Ok(summary)
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

    // ============================================================
    // LegacyImporter 测试
    // ============================================================

    /// 创建新 DB（v7 schema, in-memory）。
    fn new_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        super::super::super::db::init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_import_cards_entities_and_fields() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Alpha", "[]", "[]", "[]");
        seed_insight(&old, "card_bbb22222", "Beta", "[]", "[]", "[]");

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        assert_eq!(summary.cards, 2);

        // entities 表有 2 条 card
        let count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE kind = 'card'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // card_fields 表有 2 条
        let cf_count: i64 = new
            .query_row("SELECT COUNT(*) FROM card_fields", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cf_count, 2);

        // 验证 understanding 和 source 字段
        let understanding: String = new
            .query_row(
                "SELECT understanding FROM card_fields WHERE entity_id = 'card_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(understanding, "understand");

        // 验证 whiteboard_id 由 derive_whiteboard_id 推导
        let wb: String = new
            .query_row(
                "SELECT whiteboard_id FROM entities WHERE id = 'card_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(wb, "rust"); // file_path = "whiteboard/rust/Alpha.md"
    }

    #[test]
    fn test_import_cards_tags() {
        let old = legacy_conn();
        seed_insight(
            &old,
            "card_aaa11111",
            "TagCard",
            r#"["rust","trait","ownership"]"#,
            "[]",
            "[]",
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        importer.import(&reader).unwrap();

        let tag_count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM entity_tags WHERE entity_id = 'card_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tag_count, 3);
    }

    #[test]
    fn test_import_cards_edges() {
        let old = legacy_conn();
        seed_insight(
            &old,
            "card_aaa11111",
            "Source",
            "[]",
            r#"["card_bbb22222"]"#,
            r#"["card_bbb22222"]"#,
        );
        seed_insight(&old, "card_bbb22222", "Target", "[]", "[]", "[]");

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        assert_eq!(summary.edges, 2); // link_to + related

        let edge_count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE from_id = 'card_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(edge_count, 2);
    }

    #[test]
    fn test_import_sections_with_members() {
        let old = legacy_conn();
        // 需要有 card 才能被 section 引用
        seed_insight(&old, "card_aaa11111", "Member", "[]", "[]", "[]");
        // 根白板 → list_whiteboards 能找到
        seed_meta(&old, "graph_sections", r#"[{"id":"sec_aaa11111","title":"Group1","color":"blue","cardIds":["card_aaa11111"],"linkedSectionIds":[]}]"#);

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        assert_eq!(summary.sections, 1);
        assert_eq!(summary.section_members, 1);

        // entities 表有 section
        let sec_count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE kind = 'section'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sec_count, 1);

        // section_members 表
        let mem_count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM section_members WHERE section_id = 'sec_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mem_count, 1);
    }

    #[test]
    fn test_import_sections_linked() {
        let old = legacy_conn();
        seed_meta(&old, "graph_sections", r#"[{"id":"sec_aaa11111","title":"A","color":null,"cardIds":[],"linkedSectionIds":["sec_bbb22222"]},{"id":"sec_bbb22222","title":"B","color":null,"cardIds":[],"linkedSectionIds":[]}]"#);

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        // sec_aaa11111 → sec_bbb22222 section_link edge
        let edge_count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE edge_type = 'section_link'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(edge_count, 1);
        assert!(summary.edges >= 1);
    }

    #[test]
    fn test_import_notes_with_links() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Target", "[]", "[]", "[]");
        seed_meta(&old, "graph_sections", "[]"); // 根白板存在
        seed_meta(
            &old,
            "graph_notes",
            r#"[{"id":"note_aaa11111","title":"My Note","content":"body","linkedCardIds":["card_aaa11111"],"linkedNoteIds":[],"linkedSectionIds":[]}]"#,
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        assert_eq!(summary.notes, 1);

        // note entity
        let note_title: String = new
            .query_row(
                "SELECT title FROM entities WHERE id = 'note_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(note_title, "My Note");

        // note_link edge
        let link_count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE edge_type = 'note_link' AND from_id = 'note_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(link_count, 1);
    }

    #[test]
    fn test_import_aliases() {
        let old = legacy_conn();
        seed_insight(&old, "card_bbb22222", "RealCard", "[]", "[]", "[]");
        seed_meta(&old, "graph_sections", "[]");
        seed_meta(
            &old,
            "graph_aliases",
            r#"[{"aliasId":"alias_aaa11111","cardId":"card_bbb22222","linkedCardIds":[],"linkedSectionIds":[],"incomingCardIds":[]}]"#,
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        assert_eq!(summary.aliases, 1);

        // alias entity — title = cardId
        let alias_title: String = new
            .query_row(
                "SELECT title FROM entities WHERE id = 'alias_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(alias_title, "card_bbb22222");

        // alias_fields
        let card_id: String = new
            .query_row(
                "SELECT card_id FROM alias_fields WHERE entity_id = 'alias_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(card_id, "card_bbb22222");

        // alias_link edge (alias → 原 card)
        let alias_link: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE edge_type = 'alias_link' AND from_id = 'alias_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(alias_link, 1);

        // card_to_alias edge (原 card → alias)
        let c2a: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE edge_type = 'card_to_alias' AND from_id = 'card_bbb22222' AND to_id = 'alias_aaa11111'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(c2a, 1);
    }

    #[test]
    fn test_import_positions() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "PosCard", "[]", "[]", "[]");
        seed_meta(&old, "graph_sections", "[]");
        seed_meta(
            &old,
            "graph_positions",
            r#"{"card_aaa11111":{"x":100.0,"y":200.0}}"#,
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        assert_eq!(summary.positions, 1);

        let (x, y): (f64, f64) = new
            .query_row(
                "SELECT x, y FROM positions WHERE entity_id = 'card_aaa11111'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!((x - 100.0).abs() < f64::EPSILON);
        assert!((y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_import_fts_sync() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "SearchMe", "[]", "[]", "[]");

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        importer.import(&reader).unwrap();

        // FTS 索引能搜到
        let count: i64 = new
            .query_row(
                "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'SearchMe'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_import_skips_dangling_edge() {
        let old = legacy_conn();
        // card_aaa11111 linkTo card_nonexist（不存在的目标）
        seed_insight(
            &old,
            "card_aaa11111",
            "Dangling",
            "[]",
            r#"["card_nonexist"]"#,
            "[]",
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        // skipped 包含 dangling edge
        assert_eq!(summary.skipped.len(), 1);
        assert!(summary.skipped[0].reason.contains("dangling"));

        // edges 表无记录
        let edge_count: i64 = new
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))
            .unwrap();
        assert_eq!(edge_count, 0);
    }

    #[test]
    fn test_import_idempotent() {
        let old = legacy_conn();
        seed_insight(
            &old,
            "card_aaa11111",
            "Idem",
            r#"["rust","test"]"#,
            "[]",
            "[]",
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);

        // 第一次导入
        importer.import(&reader).unwrap();

        // 第二次导入
        importer.import(&reader).unwrap();

        // entity 数量不翻倍
        let entity_count: i64 = new
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entity_count, 1);

        // tag 数量不翻倍
        let tag_count: i64 = new
            .query_row("SELECT COUNT(*) FROM entity_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tag_count, 2);
    }
}
