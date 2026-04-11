# Phase 3: Legacy DB Import — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import data from the old Obsidian keysight plugin DB (v1 schema) into the new Tauri app DB (v7 Entity Registry schema).

**Architecture:** LegacyReader trait reads from old DB → intermediate types → LegacyImporter trait writes to new DB. Single transaction, idempotent (INSERT OR REPLACE), old DB backed up before read.

**Tech Stack:** Rust, rusqlite, serde_json (JSON column parsing), Tauri State

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/modules/keysight/models.rs` | Modify | 新增中间类型 + ImportSummary + SkippedItem + WhiteboardMapping |
| `src-tauri/src/modules/keysight/domain/legacy_import.rs` | Create | LegacyReader/LegacyImporter traits + SqliteXxx impls + tests |
| `src-tauri/src/modules/keysight/domain/mod.rs` | Modify | 新增 `pub(in crate::modules::keysight) mod legacy_import;` |
| `src-tauri/src/modules/keysight/state.rs` | Modify | 新增 `db_path: PathBuf` |
| `src-tauri/src/modules/keysight/commands.rs` | Modify | 新增 `import_legacy_db` command |
| `src-tauri/src/lib.rs` | Modify | `init_keysight_state` 改用 `app_data_dir`，collect_commands 新增 |
| `src-tauri/src/modules/keysight/domain/sync.rs` | Modify | 测试 fixtures 路径 `"atomic cards/"` → `"whiteboard/"` |
| `src-tauri/src/modules/keysight/domain/card.rs` | Modify | 测试 fixtures 路径 `"atomic cards/"` → `"whiteboard/"` |
| `src-tauri/src/modules/keysight/domain/overview.rs` | Modify | 测试 fixtures 路径 `"atomic cards/"` → `"whiteboard/"` |

---

### Task 1: 新增中间类型和 ImportSummary 到 models.rs

**Files:**
- Modify: `src-tauri/src/modules/keysight/models.rs`

- [ ] **Step 1: 在 models.rs 末尾添加中间类型**

在 `VaultInfoResponse` 之后追加：

```rust
// ============================================================
// Legacy Import 中间类型
// ============================================================

/// 旧 DB insights 表的一行（v1 schema）。
#[derive(Debug, Clone)]
pub struct LegacyInsight {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub link_to: Vec<String>,
    pub related: Vec<String>,
    pub see_also: Vec<String>,
    pub understanding: String,
    pub source: String,
    pub mtime: f64,
}

/// 旧 DB meta 表中 graph_sections JSON 元素。
#[derive(Debug, Clone)]
pub struct LegacySection {
    pub id: String,
    pub title: String,
    pub color: Option<String>,
    pub card_ids: Vec<String>,
    pub linked_section_ids: Vec<String>,
}

/// 旧 DB meta 表中 graph_notes JSON 元素。
#[derive(Debug, Clone)]
pub struct LegacyNote {
    pub id: String,
    pub title: String,
    pub content: String,
    pub linked_card_ids: Vec<String>,
    pub linked_note_ids: Vec<String>,
    pub linked_section_ids: Vec<String>,
}

/// 旧 DB meta 表中 graph_aliases JSON 元素。
#[derive(Debug, Clone)]
pub struct LegacyAlias {
    pub alias_id: String,
    pub card_id: String,
    pub linked_card_ids: Vec<String>,
    pub linked_section_ids: Vec<String>,
    pub incoming_card_ids: Vec<String>,
}

/// 旧 DB meta 表中 graph_positions JSON 对象的一个 entry。
#[derive(Debug, Clone)]
pub struct LegacyPosition {
    pub entity_id: String,
    pub x: f64,
    pub y: f64,
}

/// meta key 后缀 → 新 whiteboard_id 的映射。
#[derive(Debug, Clone)]
pub struct WhiteboardMapping {
    /// None = root（无后缀的 meta key），Some("chentian") = 子白板
    pub meta_suffix: Option<String>,
    /// 新 DB 中的 whiteboard_id："wb_root" 或子白板名
    pub whiteboard_id: String,
}

/// 导入汇总报告。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub cards: usize,
    pub sections: usize,
    pub notes: usize,
    pub aliases: usize,
    pub edges: usize,
    pub positions: usize,
    pub section_members: usize,
    pub skipped: Vec<SkippedItem>,
}

/// 导入时跳过的条目。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkippedItem {
    pub entity_id: String,
    pub reason: String,
}
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
Expected: `Finished`，无 error

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/modules/keysight/models.rs
git commit -m "feat(keysight): add legacy import intermediate types and ImportSummary"
```

---

### Task 2: LegacyReader trait + SqliteLegacyReader — 测试先行

**Files:**
- Create: `src-tauri/src/modules/keysight/domain/legacy_import.rs`
- Modify: `src-tauri/src/modules/keysight/domain/mod.rs`

- [ ] **Step 1: 注册模块**

在 `domain/mod.rs` 末尾添加：

```rust
pub(in crate::modules::keysight) mod legacy_import;
```

- [ ] **Step 2: 创建 legacy_import.rs 骨架 + 旧 schema 常量 + LegacyReader trait**

```rust
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
    /// 读取全部 insights（cards），解析 JSON 列。
    fn read_insights(&self) -> Result<Vec<LegacyInsight>, KeysightError>;
    /// 读取指定白板的 sections。
    fn read_sections(&self, whiteboard_key: &str) -> Result<Vec<LegacySection>, KeysightError>;
    /// 读取指定白板的 notes。
    fn read_notes(&self, whiteboard_key: &str) -> Result<Vec<LegacyNote>, KeysightError>;
    /// 读取指定白板的 aliases。
    fn read_aliases(&self, whiteboard_key: &str) -> Result<Vec<LegacyAlias>, KeysightError>;
    /// 读取指定白板的 positions，已过滤非标准前缀。
    fn read_positions(&self, whiteboard_key: &str) -> Result<Vec<LegacyPosition>, KeysightError>;
    /// 列出所有白板（从 meta 表 graph_sections* keys 推导）。
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
```

- [ ] **Step 3: 写 Reader 测试（全部 Red）**

在文件末尾添加测试模块：

```rust
#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use super::*;
    use super::super::super::db::init_db;

    /// 建旧 DB v1 schema（in-memory）。
    fn legacy_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(LEGACY_SCHEMA_V1_SQL).unwrap();
        conn
    }

    /// 建新 DB v7 schema（in-memory）。
    fn new_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    /// 插入一条 insight 测试数据。
    fn seed_insight(conn: &Connection, id: &str, title: &str, tags: &str, link_to: &str, related: &str) {
        conn.execute(
            "INSERT INTO insights (id, filePath, title, content, tags, linkTo, related, understanding, source, seeAlso, mtime)
             VALUES (?1, ?2, ?3, 'body', ?4, ?5, ?6, 'understand', 'src', '[]', 1000.0)",
            rusqlite::params![id, format!("whiteboard/rust/{title}.md"), title, tags, link_to, related],
        ).unwrap();
    }

    /// 插入一条 meta JSON blob。
    fn seed_meta(conn: &Connection, key: &str, value: &str) {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)",
            [key, value],
        ).unwrap();
    }

    // ==================== Reader tests ====================

    #[test]
    fn test_read_insights_basic() {
        let conn = legacy_conn();
        seed_insight(&conn, "card_aaa11111", "Test Card", r#"["rust","trait"]"#, r#"["card_bbb22222"]"#, r#"["card_ccc33333"]"#);
        seed_insight(&conn, "card_bbb22222", "Second Card", r#"[]"#, r#"[]"#, r#"[]"#);

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
        seed_meta(&conn, "graph_sections:rust",
            r#"[{"id":"sec_aaa11111","title":"Section 1","color":"blue","cardIds":["card_aaa11111","alias_bbb22222","note_ccc33333"],"linkedSectionIds":["sec_ddd44444"]}]"#
        );

        let reader = SqliteLegacyReader::new(&conn);
        let sections = reader.read_sections("rust").unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "sec_aaa11111");
        assert_eq!(sections[0].title, "Section 1");
        assert_eq!(sections[0].color, Some("blue".to_string()));
        assert_eq!(sections[0].card_ids, vec!["card_aaa11111", "alias_bbb22222", "note_ccc33333"]);
        assert_eq!(sections[0].linked_section_ids, vec!["sec_ddd44444"]);
    }

    #[test]
    fn test_read_positions_filters_junk() {
        let conn = legacy_conn();
        seed_meta(&conn, "graph_positions:rust",
            r#"{"card_aaa11111":{"x":10.0,"y":20.0},"agent":{"x":0,"y":0},"--no-move":{"x":1,"y":1},"builtin_java":{"x":2,"y":2},"sec_bbb22222":{"x":30.0,"y":40.0}}"#
        );

        let reader = SqliteLegacyReader::new(&conn);
        let positions = reader.read_positions("rust").unwrap();
        assert_eq!(positions.len(), 2); // card_ + sec_ only
        assert!(positions.iter().any(|p| p.entity_id == "card_aaa11111"));
        assert!(positions.iter().any(|p| p.entity_id == "sec_bbb22222"));
    }

    #[test]
    fn test_read_notes_with_links() {
        let conn = legacy_conn();
        seed_meta(&conn, "graph_notes:chentian",
            r#"[{"id":"note_aaa11111","title":"Note 1","content":"body","linkedCardIds":["alias_bbb22222"],"linkedNoteIds":["note_ccc33333"],"linkedSectionIds":[]}]"#
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
        seed_meta(&conn, "graph_aliases:chentian",
            r#"[{"aliasId":"alias_aaa11111","cardId":"card_bbb22222","linkedCardIds":["card_ccc33333"],"linkedSectionIds":[],"incomingCardIds":["card_ddd44444"]}]"#
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
        seed_meta(&conn, "schema_version", "1"); // 非 graph_ key，不应出现

        let reader = SqliteLegacyReader::new(&conn);
        let wbs = reader.list_whiteboards().unwrap();
        assert_eq!(wbs.len(), 3);
        assert!(wbs.iter().any(|w| w.whiteboard_id == "wb_root" && w.meta_suffix.is_none()));
        assert!(wbs.iter().any(|w| w.whiteboard_id == "chentian" && w.meta_suffix == Some("chentian".to_string())));
        assert!(wbs.iter().any(|w| w.whiteboard_id == "rust" && w.meta_suffix == Some("rust".to_string())));
    }
}
```

- [ ] **Step 4: 运行测试确认全部 Red**

Run: `cd src-tauri && cargo test --lib modules::keysight::domain::legacy_import -- -q 2>&1`
Expected: 7 tests FAIL（`read_insights` 等 trait 方法未实现）

- [ ] **Step 5: 实现 LegacyReader 的全部方法**

在 `impl<'a> LegacyReader for SqliteLegacyReader<'a>` 块中：

```rust
impl<'a> LegacyReader for SqliteLegacyReader<'a> {
    fn read_insights(&self) -> Result<Vec<LegacyInsight>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filePath, title, content, tags, linkTo, related, understanding, source, seeAlso, mtime FROM insights"
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
        rows.collect::<Result<Vec<_>, _>>().map_err(KeysightError::Database)
    }

    fn read_sections(&self, whiteboard_key: &str) -> Result<Vec<LegacySection>, KeysightError> {
        let key = Self::meta_key("sections", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() { return Ok(vec![]); }

        let arr: Vec<serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(arr.into_iter().map(|v| LegacySection {
            id: v["id"].as_str().unwrap_or_default().to_string(),
            title: v["title"].as_str().unwrap_or_default().to_string(),
            color: v["color"].as_str().map(|s| s.to_string()),
            card_ids: Self::parse_json_array(&v["cardIds"].to_string()),
            linked_section_ids: Self::parse_json_array(&v["linkedSectionIds"].to_string()),
        }).collect())
    }

    fn read_notes(&self, whiteboard_key: &str) -> Result<Vec<LegacyNote>, KeysightError> {
        let key = Self::meta_key("notes", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() { return Ok(vec![]); }

        let arr: Vec<serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(arr.into_iter().map(|v| LegacyNote {
            id: v["id"].as_str().unwrap_or_default().to_string(),
            title: v["title"].as_str().unwrap_or_default().to_string(),
            content: v["content"].as_str().unwrap_or_default().to_string(),
            linked_card_ids: Self::parse_json_array(&v["linkedCardIds"].to_string()),
            linked_note_ids: Self::parse_json_array(&v["linkedNoteIds"].to_string()),
            linked_section_ids: Self::parse_json_array(&v["linkedSectionIds"].to_string()),
        }).collect())
    }

    fn read_aliases(&self, whiteboard_key: &str) -> Result<Vec<LegacyAlias>, KeysightError> {
        let key = Self::meta_key("aliases", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() { return Ok(vec![]); }

        let arr: Vec<serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(arr.into_iter().map(|v| LegacyAlias {
            alias_id: v["aliasId"].as_str().unwrap_or_default().to_string(),
            card_id: v["cardId"].as_str().unwrap_or_default().to_string(),
            linked_card_ids: Self::parse_json_array(&v["linkedCardIds"].to_string()),
            linked_section_ids: Self::parse_json_array(&v["linkedSectionIds"].to_string()),
            incoming_card_ids: Self::parse_json_array(&v["incomingCardIds"].to_string()),
        }).collect())
    }

    fn read_positions(&self, whiteboard_key: &str) -> Result<Vec<LegacyPosition>, KeysightError> {
        let key = Self::meta_key("positions", whiteboard_key);
        let blob = self.read_meta_blob(&key)?;
        if blob.is_empty() { return Ok(vec![]); }

        let obj: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&blob)
            .map_err(|e| KeysightError::ParseError(format!("{key}: {e}")))?;

        Ok(obj.into_iter()
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
            "SELECT key FROM meta WHERE key LIKE 'graph_sections%' ORDER BY key"
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
```

- [ ] **Step 6: 运行测试确认全部 Green**

Run: `cd src-tauri && cargo test --lib modules::keysight::domain::legacy_import -- -q 2>&1`
Expected: 7 tests passed

- [ ] **Step 7: clippy 检查**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -3`
Expected: `Finished`

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/legacy_import.rs src-tauri/src/modules/keysight/domain/mod.rs
git commit -m "feat(keysight): LegacyReader trait + SqliteLegacyReader with 7 tests"
```

---

### Task 3: LegacyImporter trait + SqliteLegacyImporter — 测试先行

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/legacy_import.rs`

- [ ] **Step 1: 在 Reader impl 之后添加 Importer trait + struct 骨架**

```rust
use super::super::models::{
    ImportSummary, SkippedItem,
    LegacyAlias, LegacyInsight, LegacyNote, LegacyPosition, LegacySection, WhiteboardMapping,
};

/// 将旧数据写入新 DB 的行为契约。
pub(in crate::modules::keysight) trait LegacyImporter {
    /// 执行完整导入流程，返回汇总报告。
    fn import(&self, reader: &dyn LegacyReader) -> Result<ImportSummary, KeysightError>;
}

/// 写入 SQLite (v7 schema) 的 LegacyImporter 实现。
pub(in crate::modules::keysight) struct SqliteLegacyImporter<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteLegacyImporter<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}
```

- [ ] **Step 2: 在 tests 模块中添加 Importer 测试（全部 Red）**

在现有 `mod tests` 内追加：

```rust
    // ==================== Importer tests ====================

    #[test]
    fn test_import_cards_entities_and_fields() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"["rust"]"#, r#"[]"#, r#"[]"#);

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);
        let summary = importer.import(&reader).unwrap();

        assert_eq!(summary.cards, 1);

        // entities 表
        let kind: String = new.query_row(
            "SELECT kind FROM entities WHERE id = 'card_aaa11111'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(kind, "card");

        // card_fields 表
        let understanding: String = new.query_row(
            "SELECT understanding FROM card_fields WHERE entity_id = 'card_aaa11111'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(understanding, "understand");
    }

    #[test]
    fn test_import_cards_tags() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"["rust","trait"]"#, r#"[]"#, r#"[]"#);

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        let count: i64 = new.query_row(
            "SELECT COUNT(*) FROM entity_tags WHERE entity_id = 'card_aaa11111'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_import_cards_edges() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"[]"#, r#"["card_bbb22222"]"#, r#"["card_ccc33333"]"#);
        seed_insight(&old, "card_bbb22222", "Card B", r#"[]"#, r#"[]"#, r#"[]"#);
        seed_insight(&old, "card_ccc33333", "Card C", r#"[]"#, r#"[]"#, r#"[]"#);

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        let link_count: i64 = new.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id = 'card_aaa11111' AND edge_type = 'link_to'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(link_count, 1);

        let related_count: i64 = new.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id = 'card_aaa11111' AND edge_type = 'related'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(related_count, 1);
    }

    #[test]
    fn test_import_sections_with_members() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"[]"#, r#"[]"#, r#"[]"#);
        seed_meta(&old, "graph_sections", "[]");
        seed_meta(&old, "graph_sections:rust",
            r#"[{"id":"sec_aaa11111","title":"Sec 1","color":null,"cardIds":["card_aaa11111"],"linkedSectionIds":[]}]"#
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let summary = SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        assert_eq!(summary.sections, 1);
        assert_eq!(summary.section_members, 1);

        let kind: String = new.query_row(
            "SELECT kind FROM entities WHERE id = 'sec_aaa11111'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(kind, "section");

        let wb: String = new.query_row(
            "SELECT whiteboard_id FROM entities WHERE id = 'sec_aaa11111'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(wb, "rust");
    }

    #[test]
    fn test_import_sections_linked() {
        let old = legacy_conn();
        seed_meta(&old, "graph_sections",
            r#"[{"id":"sec_aaa11111","title":"A","color":null,"cardIds":[],"linkedSectionIds":["sec_bbb22222"]},{"id":"sec_bbb22222","title":"B","color":null,"cardIds":[],"linkedSectionIds":[]}]"#
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        let edge_count: i64 = new.query_row(
            "SELECT COUNT(*) FROM edges WHERE edge_type = 'section_link'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(edge_count, 1);
    }

    #[test]
    fn test_import_notes_with_links() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"[]"#, r#"[]"#, r#"[]"#);
        seed_meta(&old, "graph_sections", "[]");
        seed_meta(&old, "graph_notes",
            r#"[{"id":"note_aaa11111","title":"Note 1","content":"body","linkedCardIds":["card_aaa11111"],"linkedNoteIds":[],"linkedSectionIds":[]}]"#
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let summary = SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        assert_eq!(summary.notes, 1);

        let edge_count: i64 = new.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id = 'note_aaa11111' AND edge_type = 'note_link'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(edge_count, 1);
    }

    #[test]
    fn test_import_aliases() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"[]"#, r#"[]"#, r#"[]"#);
        seed_meta(&old, "graph_sections", "[]");
        seed_meta(&old, "graph_aliases",
            r#"[{"aliasId":"alias_aaa11111","cardId":"card_aaa11111","linkedCardIds":["card_aaa11111"],"linkedSectionIds":[],"incomingCardIds":["card_aaa11111"]}]"#
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let summary = SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        assert_eq!(summary.aliases, 1);

        let card_id: String = new.query_row(
            "SELECT card_id FROM alias_fields WHERE entity_id = 'alias_aaa11111'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(card_id, "card_aaa11111");

        // alias_link edge
        let alias_link: i64 = new.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id = 'alias_aaa11111' AND edge_type = 'alias_link'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(alias_link, 1);

        // card_to_alias edge
        let c2a: i64 = new.query_row(
            "SELECT COUNT(*) FROM edges WHERE to_id = 'alias_aaa11111' AND edge_type = 'card_to_alias'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(c2a, 1);
    }

    #[test]
    fn test_import_positions() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"[]"#, r#"[]"#, r#"[]"#);
        seed_meta(&old, "graph_sections", "[]");
        seed_meta(&old, "graph_positions",
            r#"{"card_aaa11111":{"x":100.0,"y":200.0}}"#
        );

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let summary = SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        assert_eq!(summary.positions, 1);

        let (x, y): (f64, f64) = new.query_row(
            "SELECT x, y FROM positions WHERE entity_id = 'card_aaa11111' AND whiteboard_id = 'wb_root'",
            [], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap();
        assert!((x - 100.0).abs() < 0.01);
        assert!((y - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_import_fts_sync() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Searchable Title", r#"[]"#, r#"[]"#, r#"[]"#);

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        let count: i64 = new.query_row(
            "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'Searchable'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_import_skips_dangling_edge() {
        let old = legacy_conn();
        // card_aaa11111 的 linkTo 引用不存在的 card_nonexist
        seed_insight(&old, "card_aaa11111", "Card A", r#"[]"#, r#"["card_nonexist"]"#, r#"[]"#);

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let summary = SqliteLegacyImporter::new(&new).import(&reader).unwrap();

        assert_eq!(summary.skipped.len(), 1);
        assert!(summary.skipped[0].reason.contains("dangling"));

        // edge 不应写入
        let count: i64 = new.query_row(
            "SELECT COUNT(*) FROM edges", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_import_idempotent() {
        let old = legacy_conn();
        seed_insight(&old, "card_aaa11111", "Card A", r#"["rust"]"#, r#"[]"#, r#"[]"#);
        seed_meta(&old, "graph_sections", "[]");

        let new = new_conn();
        let reader = SqliteLegacyReader::new(&old);
        let importer = SqliteLegacyImporter::new(&new);

        let s1 = importer.import(&reader).unwrap();
        let s2 = importer.import(&reader).unwrap();

        assert_eq!(s1.cards, s2.cards);

        // 数据不重复
        let count: i64 = new.query_row(
            "SELECT COUNT(*) FROM entities", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 1);

        let tag_count: i64 = new.query_row(
            "SELECT COUNT(*) FROM entity_tags", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(tag_count, 1);
    }
```

- [ ] **Step 3: 运行测试确认 Importer 测试全部 Red**

Run: `cd src-tauri && cargo test --lib modules::keysight::domain::legacy_import -- -q 2>&1`
Expected: 7 Reader tests pass, 11 Importer tests FAIL（`import` 未实现）

- [ ] **Step 4: 实现 SqliteLegacyImporter::import 方法**

```rust
impl<'a> LegacyImporter for SqliteLegacyImporter<'a> {
    fn import(&self, reader: &dyn LegacyReader) -> Result<ImportSummary, KeysightError> {
        let mut summary = ImportSummary {
            cards: 0, sections: 0, notes: 0, aliases: 0,
            edges: 0, positions: 0, section_members: 0,
            skipped: Vec::new(),
        };

        self.conn.execute("BEGIN", [])?;

        // 收集所有已导入实体 id，用于悬挂引用检查
        let mut known_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        // --- 1. Cards ---
        let insights = reader.read_insights()?;
        for ins in &insights {
            self.conn.execute(
                "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id, file_path, content) VALUES (?1, 'card', ?2, ?3, ?4, ?5)",
                rusqlite::params![ins.id, ins.title, derive_whiteboard_id(&ins.file_path), ins.file_path, ins.content],
            )?;
            self.conn.execute(
                "INSERT OR REPLACE INTO card_fields (entity_id, understanding, source) VALUES (?1, ?2, ?3)",
                rusqlite::params![ins.id, ins.understanding, ins.source],
            )?;
            // tags — 先删后插（幂等）
            self.conn.execute("DELETE FROM entity_tags WHERE entity_id = ?1", [&ins.id])?;
            for tag in &ins.tags {
                self.conn.execute(
                    "INSERT INTO entity_tags (entity_id, tag) VALUES (?1, ?2)",
                    rusqlite::params![ins.id, tag],
                )?;
            }
            known_ids.insert(ins.id.clone());
            summary.cards += 1;
        }

        // card edges（linkTo / related / seeAlso）
        for ins in &insights {
            for (targets, edge_type) in [
                (&ins.link_to, "link_to"),
                (&ins.related, "related"),
                (&ins.see_also, "see_also"),
            ] {
                for target in targets {
                    if known_ids.contains(target) {
                        self.conn.execute(
                            "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, ?3)",
                            rusqlite::params![ins.id, target, edge_type],
                        )?;
                        summary.edges += 1;
                    } else {
                        summary.skipped.push(SkippedItem {
                            entity_id: format!("{}→{}", ins.id, target),
                            reason: format!("dangling {edge_type} edge: target {target} not found"),
                        });
                    }
                }
            }
        }

        // --- 2. 逐白板导入 sections / notes / aliases / positions ---
        let whiteboards = reader.list_whiteboards()?;
        for wb in &whiteboards {
            let wb_key = wb.meta_suffix.as_deref().unwrap_or("");
            let wb_id = &wb.whiteboard_id;

            // Sections
            let sections = reader.read_sections(wb_key)?;
            for sec in &sections {
                self.conn.execute(
                    "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id, color) VALUES (?1, 'section', ?2, ?3, ?4)",
                    rusqlite::params![sec.id, sec.title, wb_id, sec.color],
                )?;
                // section_members — 先删后插
                self.conn.execute("DELETE FROM section_members WHERE section_id = ?1", [&sec.id])?;
                for member_id in &sec.card_ids {
                    self.conn.execute(
                        "INSERT INTO section_members (section_id, entity_id) VALUES (?1, ?2)",
                        rusqlite::params![sec.id, member_id],
                    )?;
                    summary.section_members += 1;
                }
                // section_link edges
                for linked in &sec.linked_section_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'section_link')",
                        rusqlite::params![sec.id, linked],
                    )?;
                    summary.edges += 1;
                }
                known_ids.insert(sec.id.clone());
                summary.sections += 1;
            }

            // Notes
            let notes = reader.read_notes(wb_key)?;
            for note in &notes {
                self.conn.execute(
                    "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id, content) VALUES (?1, 'note', ?2, ?3, ?4)",
                    rusqlite::params![note.id, note.title, wb_id, note.content],
                )?;
                // note_link edges（linkedCardIds + linkedNoteIds + linkedSectionIds 统一处理）
                for target in note.linked_card_ids.iter()
                    .chain(note.linked_note_ids.iter())
                    .chain(note.linked_section_ids.iter())
                {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'note_link')",
                        rusqlite::params![note.id, target],
                    )?;
                    summary.edges += 1;
                }
                known_ids.insert(note.id.clone());
                summary.notes += 1;
            }

            // Aliases
            let aliases = reader.read_aliases(wb_key)?;
            for alias in &aliases {
                self.conn.execute(
                    "INSERT OR REPLACE INTO entities (id, kind, title, whiteboard_id) VALUES (?1, 'alias', ?2, ?3)",
                    rusqlite::params![alias.alias_id, alias.card_id, wb_id],
                )?;
                self.conn.execute(
                    "INSERT OR REPLACE INTO alias_fields (entity_id, card_id) VALUES (?1, ?2)",
                    rusqlite::params![alias.alias_id, alias.card_id],
                )?;
                // alias_link edges
                for target in alias.linked_card_ids.iter().chain(alias.linked_section_ids.iter()) {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'alias_link')",
                        rusqlite::params![alias.alias_id, target],
                    )?;
                    summary.edges += 1;
                }
                // card_to_alias edges
                for incoming in &alias.incoming_card_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'card_to_alias')",
                        rusqlite::params![incoming, alias.alias_id],
                    )?;
                    summary.edges += 1;
                }
                known_ids.insert(alias.alias_id.clone());
                summary.aliases += 1;
            }

            // Positions
            let positions = reader.read_positions(wb_key)?;
            for pos in &positions {
                self.conn.execute(
                    "INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![pos.entity_id, wb_id, pos.x, pos.y],
                )?;
                summary.positions += 1;
            }
        }

        // --- 3. FTS 同步 ---
        // 先清空再重建（幂等）
        self.conn.execute("DELETE FROM entities_fts", [])?;
        let mut fts_stmt = self.conn.prepare(
            "SELECT id, title, COALESCE(content, '') FROM entities WHERE kind = 'card'"
        )?;
        let fts_rows: Vec<(String, String, String)> = fts_stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        for (id, title, content) in &fts_rows {
            self.conn.execute(
                "INSERT INTO entities_fts (id, title, content) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, title, content],
            )?;
        }

        self.conn.execute("COMMIT", [])?;

        Ok(summary)
    }
}
```

注意：需要引用 `derive_whiteboard_id`，在文件顶部 use 区添加：

```rust
use super::sync::derive_whiteboard_id;
```

- [ ] **Step 5: 运行测试确认全部 Green**

Run: `cd src-tauri && cargo test --lib modules::keysight::domain::legacy_import -- -q 2>&1`
Expected: 18 tests passed (7 reader + 11 importer)

- [ ] **Step 6: clippy 检查**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -3`
Expected: `Finished`

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/legacy_import.rs
git commit -m "feat(keysight): LegacyImporter trait + SqliteLegacyImporter with 11 tests"
```

---

### Task 4: State 改造 + DB 位置迁移到 app_data_dir

**Files:**
- Modify: `src-tauri/src/modules/keysight/state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 修改 state.rs 新增 db_path 字段**

```rust
use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::Connection;

/// KeySight 模块的共享状态，由 Tauri managed state 注入。
pub struct KeysightState {
    /// SQLite 连接（keysight 专用 DB）。
    pub db: Mutex<Connection>,
    /// Obsidian vault 根目录路径。
    pub vault_path: PathBuf,
    /// keysight.db 文件的绝对路径。
    pub db_path: PathBuf,
}
```

- [ ] **Step 2: 修改 lib.rs 的 init_keysight_state**

将当前的：

```rust
fn init_keysight_state() -> modules::keysight::state::KeysightState {
    let vault_path = std::env::var("KEYSIGHT_VAULT_PATH")
        .expect("环境变量 KEYSIGHT_VAULT_PATH 未设置，请设置为 Obsidian vault 根目录路径");
    let conn = Connection::open("keysight.db").expect("无法打开 keysight 数据库");
    modules::keysight::init(&conn).expect("keysight 建表失败");
    modules::keysight::state::KeysightState {
        db: Mutex::new(conn),
        vault_path: PathBuf::from(vault_path),
    }
}
```

替换为：

```rust
fn init_keysight_state(app: &tauri::App) -> modules::keysight::state::KeysightState {
    let vault_path = std::env::var("KEYSIGHT_VAULT_PATH")
        .expect("环境变量 KEYSIGHT_VAULT_PATH 未设置，请设置为 Obsidian vault 根目录路径");

    let data_dir = app.path().app_data_dir().expect("无法获取 app_data_dir");
    std::fs::create_dir_all(&data_dir).expect("无法创建 app_data_dir");
    let db_path = data_dir.join("keysight.db");

    let conn = Connection::open(&db_path).expect("无法打开 keysight 数据库");
    modules::keysight::init(&conn).expect("keysight 建表失败");
    modules::keysight::state::KeysightState {
        db: Mutex::new(conn),
        vault_path: PathBuf::from(vault_path),
        db_path,
    }
}
```

- [ ] **Step 3: 修改 lib.rs 的 run() 函数中的调用**

将：

```rust
let keysight_state = init_keysight_state();
```

改为在 `setup` 闭包中初始化（因为需要 `&tauri::App`）：

```rust
pub fn run() {
    let builder = make_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    let todo_conn = init_todo_database();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(todo_conn))
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let keysight_state = init_keysight_state(app);
            app.manage(keysight_state);
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: 验证编译**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
Expected: `Finished`

- [ ] **Step 5: 运行全部测试**

Run: `cd src-tauri && cargo test --workspace -- -q 2>&1 | tail -3`
Expected: 所有测试通过（测试不走 `run()`，不受 app_data_dir 影响）

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/state.rs src-tauri/src/lib.rs
git commit -m "refactor(keysight): move DB to app_data_dir, add db_path to KeysightState"
```

---

### Task 5: import_legacy_db command 薄壳

**Files:**
- Modify: `src-tauri/src/modules/keysight/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 commands.rs 末尾添加 import command**

在 `get_vault_info` 之后添加：

```rust
use super::domain::legacy_import::{LegacyImporter, LegacyReader, SqliteLegacyImporter, SqliteLegacyReader};
use super::models::ImportSummary;
```

（合并到文件顶部的 use 区）

```rust
// ============================================================
// Legacy Import
// ============================================================

/// # import_legacy_db
///
/// ## 前置条件
/// - old_db_path 指向旧 Obsidian 插件的 keysight.db（v1 schema）
/// - 文件必须存在且可读
///
/// ## 执行效果
/// 1. 备份旧 DB（cp → {old_db_path}.bak-import-{timestamp}）
/// 2. 只读打开旧 DB
/// 3. 单事务写入新 DB（INSERT OR REPLACE）
/// 4. 同步 FTS 索引
///
/// ## 幂等性
/// 可重复执行，结果一致
///
/// ## 关联操作
/// - [`overview_stats`] — 导入后验证数据量
#[tauri::command]
#[specta::specta]
pub fn import_legacy_db(
    state: State<'_, KeysightState>,
    old_db_path: String,
) -> Result<ImportSummary, AppError> {
    let path = std::path::Path::new(&old_db_path);
    if !path.exists() {
        return Err(AppError::Keysight(format!("旧 DB 文件不存在: {old_db_path}")));
    }

    // 1. 备份旧 DB
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let backup_path = format!("{old_db_path}.bak-import-{timestamp}");
    std::fs::copy(path, &backup_path)
        .map_err(|e| AppError::Keysight(format!("备份旧 DB 失败: {e}")))?;

    // 2. 只读打开旧 DB
    let old_conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ).map_err(|e| AppError::Keysight(format!("打开旧 DB 失败: {e}")))?;

    // 3. 导入
    let new_conn = state.db.lock().unwrap();
    let reader = SqliteLegacyReader::new(&old_conn);
    let importer = SqliteLegacyImporter::new(&new_conn);
    importer.import(&reader).map_err(Into::into)
}
```

注意：commands.rs 顶部需要添加 `use rusqlite::Connection;`。

- [ ] **Step 2: 在 lib.rs 的 collect_commands![] 中注册**

在 `modules::keysight::commands::get_vault_info,` 之后添加：

```rust
        // keysight: legacy import
        modules::keysight::commands::import_legacy_db,
```

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
Expected: `Finished`

- [ ] **Step 4: 重新生成 bindings.ts**

Run: `cd src-tauri && cargo test export_bindings 2>&1 | tail -3`
Expected: test passed

- [ ] **Step 5: 验证 bindings.ts 包含新 command**

Run: `grep "importLegacyDb" ../src/bindings.ts`
Expected: 能找到 `importLegacyDb` 函数

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/commands.rs src-tauri/src/lib.rs src/bindings.ts
git commit -m "feat(keysight): add import_legacy_db Tauri command"
```

---

### Task 6: 修正测试 fixtures 路径 — atomic cards → whiteboard

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/sync.rs`
- Modify: `src-tauri/src/modules/keysight/domain/card.rs`
- Modify: `src-tauri/src/modules/keysight/domain/overview.rs`

- [ ] **Step 1: sync.rs — 替换所有 `"atomic cards/` 为 `"whiteboard/`**

在 sync.rs 的 `#[cfg(test)] mod tests` 中，全局替换：
- `"atomic cards/test.md"` → `"whiteboard/test.md"`
- `"atomic cards/noid.md"` → `"whiteboard/noid.md"`
- `'atomic cards/test.md'` → `'whiteboard/test.md'`（SQL 字符串中）

同时更新 `test_derive_whiteboard_id_root` 测试：

```rust
#[test]
fn test_derive_whiteboard_id_root() {
    // whiteboard/ 下直接的文件映射到 wb_root
    assert_eq!(derive_whiteboard_id("whiteboard/test.md"), "wb_root");
    // 非 whiteboard 目录也映射到 wb_root
    assert_eq!(derive_whiteboard_id("other/test.md"), "wb_root");
}
```

- [ ] **Step 2: card.rs — 替换所有 `"atomic cards/` 为 `"whiteboard/`**

在 card.rs 的 `#[cfg(test)] mod tests` 中，全局替换：
- `"atomic cards/test.md"` → `"whiteboard/test.md"`
- `"atomic cards/test2.md"` → `"whiteboard/test2.md"`
- `"atomic cards/linker.md"` → `"whiteboard/linker.md"`
- `"atomic cards/lonely.md"` → `"whiteboard/lonely.md"`

同时更新 `test_get_card_by_id` 中的断言：

```rust
assert_eq!(card.file_path, "whiteboard/test.md");
```

- [ ] **Step 3: overview.rs — 替换 `"atomic cards/` 为 `"whiteboard/`**

在 overview.rs 的 `#[cfg(test)] mod tests` 中，全局替换：
- `"atomic cards/ov1.md"` → `"whiteboard/ov1.md"`
- `"atomic cards/ov2.md"` → `"whiteboard/ov2.md"`

- [ ] **Step 4: 运行全部 keysight 测试**

Run: `cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3`
Expected: 所有测试通过（数量 = 104 + 18 legacy import = 122）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/sync.rs src-tauri/src/modules/keysight/domain/card.rs src-tauri/src/modules/keysight/domain/overview.rs
git commit -m "fix(keysight): rename test fixture paths from 'atomic cards/' to 'whiteboard/'"
```

---

### Task 7: 更新副作用矩阵 + progress 标记

**Files:**
- Modify: `CLAUDE.md`
- Modify: `docs/progress/keysight.md`

- [ ] **Step 1: 在 CLAUDE.md 副作用矩阵中新增一行**

在 `sync_remove_file` 行之后添加：

```markdown
| `import_legacy_db` | `entities`, `card_fields`, `alias_fields`, `entity_tags`, `edges`, `positions`, `section_members`, `entities_fts` + 旧 DB 文件备份 | DB 批量写 + 文件 copy | domain unit test |
```

- [ ] **Step 2: 在 progress 文件中将 Phase 3 移到 Active**

将 `docs/progress/keysight.md` 中 Phase 3 从 Next 移到 Active。

- [ ] **Step 3: 运行完整验证**

```bash
cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -3
cd src-tauri && cargo test --workspace -- -q 2>&1 | tail -3
cd .. && pnpm build 2>&1 | tail -3
```

Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md docs/progress/keysight.md
git commit -m "docs(keysight): Phase 3 — update side effect matrix and mark Active"
```
