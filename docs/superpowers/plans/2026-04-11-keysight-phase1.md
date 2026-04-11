# KeySight Phase 1: Rust Domain Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **TDD 人工确认关卡**: 每个 🔴 Red / 🟢 Green 关卡处必须停下来等用户确认。不得跳过。

**Goal:** Port all KeySight Rust domain logic to the Tauri project, reading/writing v7 schema tables exclusively (no legacy `insights` table), with full unit test coverage.

**Architecture:** TDD + Trait-First. Each domain module defines a trait contract, tests express expected behavior (Red), implementation makes tests pass (Green). In-memory SQLite provides the test database. `VaultFs` trait isolates file system I/O for testability.

**Tech Stack:** Rust, rusqlite 0.33 (bundled), serde_yaml 0.9, rand 0.9, specta 2.0.0-rc.24, tauri-specta 2.0.0-rc.24

**Source project:** `~/codes/vibe-coding/obsidian-plugin-keysight/keysight-core/src/`

---

## File Structure

```
src-tauri/src/modules/keysight/
├── mod.rs              # 窄接口: pub use commands, models
├── models.rs           # 数据类型 + specta::Type
├── errors.rs           # KeysightError + impl Into<AppError>
├── db.rs               # V7 schema DDL + init_db
├── id.rs               # ID 工厂函数
├── parser.rs           # Frontmatter YAML 解析
├── vault_fs.rs         # VaultFs trait + RealVaultFs + MockVaultFs
├── domain/
│   ├── mod.rs          # re-export 所有 domain 子模块
│   ├── entity.rs       # EntityGraph trait — edge 连接/断开
│   ├── card.rs         # CardStore trait — 卡片查询 + 编辑
│   ├── section.rs      # SectionStore trait — 分组 CRUD
│   ├── note.rs         # NoteStore trait — 笔记 CRUD
│   ├── alias.rs        # AliasStore trait — 别名 CRUD
│   ├── layout.rs       # LayoutStore trait — 位置管理
│   ├── sync.rs         # sync_file — 文件 → DB 同步
│   ├── task.rs         # TaskStore trait — 任务状态管理
│   ├── question.rs     # QuestionStore trait — 问题状态管理
│   └── overview.rs     # 统计查询
└── commands.rs         # (Phase 2, 本 plan 不涉及)
```

---

## Task 1: Module Skeleton + Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/modules/keysight/mod.rs`
- Create: `src-tauri/src/modules/keysight/domain/mod.rs`
- Modify: `src-tauri/src/modules/mod.rs`
- Modify: `src-tauri/src/app_error.rs`

- [ ] **Step 1: Add dependencies to Cargo.toml**

在 `[dependencies]` 段添加：

```toml
serde_yaml = "0.9"
rand = "0.9"
```

- [ ] **Step 2: Create keysight/mod.rs**

```rust
pub mod models;

mod db;
mod domain;
mod errors;
mod id;
mod parser;
mod vault_fs;

/// 初始化 keysight 模块的数据库表。
pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    db::init_db(conn)
}
```

- [ ] **Step 3: Create keysight/domain/mod.rs**

```rust
pub(super) mod entity;
pub(super) mod card;
pub(super) mod section;
pub(super) mod note;
pub(super) mod alias;
pub(super) mod layout;
pub(super) mod sync;
pub(super) mod task;
pub(super) mod question;
pub(super) mod overview;
```

- [ ] **Step 4: Register in modules/mod.rs**

```rust
pub mod todo;
pub mod keysight;

/// 初始化所有模块的数据库表。
pub fn init_all(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    todo::init(conn)?;
    keysight::init(conn)?;
    Ok(())
}
```

- [ ] **Step 5: Add Keysight variant to AppError**

```rust
#[derive(Debug, Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("{0}")]
    Todo(String),

    #[error("{0}")]
    Keysight(String),
}
```

- [ ] **Step 6: Create placeholder files**

为每个 domain 子模块创建空文件（让编译通过）：

`src-tauri/src/modules/keysight/models.rs`:
```rust
// 占位，Task 2 填充
```

`src-tauri/src/modules/keysight/errors.rs`:
```rust
// 占位，Task 3 填充
```

`src-tauri/src/modules/keysight/db.rs`:
```rust
/// 占位，Task 4 填充
pub(super) fn init_db(_conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    Ok(())
}
```

`src-tauri/src/modules/keysight/id.rs`:
```rust
// 占位，Task 5 填充
```

`src-tauri/src/modules/keysight/parser.rs`:
```rust
// 占位，Task 6 填充
```

`src-tauri/src/modules/keysight/vault_fs.rs`:
```rust
// 占位，Task 7 填充
```

每个 `domain/*.rs` 文件创建为空文件。

- [ ] **Step 7: Verify compilation**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -3`

Expected: `Finished` 无 error。

- [ ] **Step 8: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/modules/ src-tauri/src/app_error.rs
git commit -m "feat(keysight): add module skeleton and dependencies"
```

---

## Task 2: models.rs — Data Types

**Files:**
- Modify: `src-tauri/src/modules/keysight/models.rs`

从源项目 `domain/types.rs` 移植所有类型，`ts_rs::TS` → `specta::Type`，移除 `#[ts(...)]` 属性。

- [ ] **Step 1: Write all type definitions**

```rust
use serde::{Deserialize, Serialize};

/// 白板坐标位置。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// 实体类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum EntityKind {
    Card,
    Note,
    Alias,
    Section,
    Task,
    Question,
}

/// 实体数据来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitySource {
    FileBacked,
    DbOnly,
}

impl EntityKind {
    /// 该实体类型的数据来源。
    pub fn source(&self) -> EntitySource {
        match self {
            EntityKind::Card | EntityKind::Note | EntityKind::Task | EntityKind::Question => {
                EntitySource::FileBacked
            }
            EntityKind::Section | EntityKind::Alias => EntitySource::DbOnly,
        }
    }

    /// ID 前缀。
    pub fn id_prefix(&self) -> &'static str {
        match self {
            EntityKind::Card => "card_",
            EntityKind::Note => "note_",
            EntityKind::Alias => "alias_",
            EntityKind::Section => "sec_",
            EntityKind::Task => "task_",
            EntityKind::Question => "q_",
        }
    }

    /// 从 id 前缀推断实体类型。
    pub fn from_id(id: &str) -> Option<Self> {
        if id.starts_with("card_") {
            Some(EntityKind::Card)
        } else if id.starts_with("note_") {
            Some(EntityKind::Note)
        } else if id.starts_with("alias_") {
            Some(EntityKind::Alias)
        } else if id.starts_with("sec_") {
            Some(EntityKind::Section)
        } else if id.starts_with("task_") {
            Some(EntityKind::Task)
        } else if id.starts_with("q_") {
            Some(EntityKind::Question)
        } else {
            None
        }
    }

    /// 数据库中 kind 列的字符串值。
    pub fn as_db_str(&self) -> &'static str {
        match self {
            EntityKind::Card => "card",
            EntityKind::Note => "note",
            EntityKind::Alias => "alias",
            EntityKind::Section => "section",
            EntityKind::Task => "task",
            EntityKind::Question => "question",
        }
    }

    /// 从数据库 kind 字符串解析。
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "card" => Some(EntityKind::Card),
            "note" => Some(EntityKind::Note),
            "alias" => Some(EntityKind::Alias),
            "section" => Some(EntityKind::Section),
            "task" => Some(EntityKind::Task),
            "question" => Some(EntityKind::Question),
            _ => None,
        }
    }
}

/// 边类型（实体间关系）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum EdgeType {
    LinkTo,
    Related,
    SeeAlso,
    SectionLink,
    NoteLink,
    AliasLink,
    CardToAlias,
}

impl EdgeType {
    /// 数据库中 edge_type 列的字符串值。
    pub fn as_db_str(&self) -> &'static str {
        match self {
            EdgeType::LinkTo => "link_to",
            EdgeType::Related => "related",
            EdgeType::SeeAlso => "see_also",
            EdgeType::SectionLink => "section_link",
            EdgeType::NoteLink => "note_link",
            EdgeType::AliasLink => "alias_link",
            EdgeType::CardToAlias => "card_to_alias",
        }
    }

    /// 从数据库字符串解析。
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "link_to" => Some(EdgeType::LinkTo),
            "related" => Some(EdgeType::Related),
            "see_also" => Some(EdgeType::SeeAlso),
            "section_link" => Some(EdgeType::SectionLink),
            "note_link" => Some(EdgeType::NoteLink),
            "alias_link" => Some(EdgeType::AliasLink),
            "card_to_alias" => Some(EdgeType::CardToAlias),
            _ => None,
        }
    }
}

/// 边的视觉样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum EdgeStyle {
    Solid,
    Dashed,
    Dotted,
}

impl EdgeStyle {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            EdgeStyle::Solid => "solid",
            EdgeStyle::Dashed => "dashed",
            EdgeStyle::Dotted => "dotted",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "solid" => Some(EdgeStyle::Solid),
            "dashed" => Some(EdgeStyle::Dashed),
            "dotted" => Some(EdgeStyle::Dotted),
            _ => None,
        }
    }
}

/// 任务状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Next,
    Active,
    Done,
    Blocked,
}

/// 问题状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum QuestionStatus {
    Pending,
    Doing,
    Done,
    Understood,
}

/// 原子卡片。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AtomicCard {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub link_to: Vec<String>,
    pub related: Vec<String>,
    pub understanding: String,
    pub source: String,
    pub see_also: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime: Option<f64>,
}

/// 图谱分组。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphSection {
    pub id: String,
    pub title: String,
    pub card_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_section_ids: Option<Vec<String>>,
}

/// 图谱笔记。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNote {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_section_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_card_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_note_ids: Option<Vec<String>>,
}

/// 卡片别名。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardAlias {
    pub alias_id: String,
    pub card_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_card_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_section_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_note_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incoming_card_ids: Option<Vec<String>>,
}

/// 边（edges 表的行）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from_id: String,
    pub to_id: String,
    pub edge_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// 文件同步结果。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncFileResponse {
    pub updated: u32,
    pub inserted: u32,
    pub deleted: u32,
    pub needs_id_backfill: bool,
    pub assigned_id: String,
}

/// 统计信息。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub cards: u64,
    pub notes: u64,
    pub sections: u64,
    pub aliases: u64,
    pub tasks: u64,
    pub questions: u64,
    pub edges: u64,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -3`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/modules/keysight/models.rs
git commit -m "feat(keysight): add data types with specta::Type"
```

---

## Task 3: errors.rs — Module Error Type

**Files:**
- Modify: `src-tauri/src/modules/keysight/errors.rs`

- [ ] **Step 1: Write error type**

```rust
use crate::app_error::AppError;

/// KeySight 模块内部错误类型。
#[derive(Debug, thiserror::Error)]
pub(super) enum KeysightError {
    #[error("标题不能为空")]
    EmptyTitle,

    #[error("实体未找到: {0}")]
    NotFound(String),

    #[error("无效的实体类型: {0}")]
    InvalidEntityType(String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("文件操作失败: {0}")]
    FileError(String),

    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
}

impl From<KeysightError> for AppError {
    fn from(e: KeysightError) -> Self {
        AppError::Keysight(e.to_string())
    }
}
```

- [ ] **Step 2: Verify + commit**

```bash
cargo clippy --workspace -- -D warnings
git add src-tauri/src/modules/keysight/errors.rs
git commit -m "feat(keysight): add KeysightError type"
```

---

## Task 4: db.rs — V7 Schema (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/db.rs`

- [ ] **Step 1: Write failing test**

```rust
pub(super) fn init_db(_conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    Ok(()) // 空实现
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_init_db_creates_entities_table() {
        let conn = test_conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_init_db_creates_edges_table() {
        let conn = test_conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_init_db_creates_positions_table() {
        let conn = test_conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM positions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_init_db_creates_section_members_table() {
        let conn = test_conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM section_members", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_init_db_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        init_db(&conn).unwrap(); // 第二次不 panic
    }
}
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test -p super-tauri --lib modules::keysight::db::tests -- --nocapture 2>&1 | tail -10`

Expected: FAIL — `no such table: entities`

**⏸ 等待用户确认 Red**

- [ ] **Step 3: Implement init_db**

```rust
use rusqlite::Connection;

/// V7 schema DDL — 纯新表，无旧表。
const SCHEMA_V7_SQL: &str = "
-- 基础表
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS file_mtimes (filePath TEXT PRIMARY KEY, mtime REAL NOT NULL);

-- Entity Registry
CREATE TABLE IF NOT EXISTS entities (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,
    title         TEXT NOT NULL,
    whiteboard_id TEXT NOT NULL,
    file_path     TEXT,
    content       TEXT,
    color         TEXT
);
CREATE TABLE IF NOT EXISTS card_fields (
    entity_id     TEXT PRIMARY KEY REFERENCES entities(id),
    understanding TEXT NOT NULL DEFAULT '',
    source        TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS task_fields (
    entity_id TEXT PRIMARY KEY REFERENCES entities(id),
    status    TEXT NOT NULL DEFAULT 'next',
    area      TEXT,
    project   TEXT
);
CREATE TABLE IF NOT EXISTS question_fields (
    entity_id TEXT PRIMARY KEY REFERENCES entities(id),
    status    TEXT NOT NULL DEFAULT 'pending'
);
CREATE TABLE IF NOT EXISTS alias_fields (
    entity_id TEXT PRIMARY KEY REFERENCES entities(id),
    card_id   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS entity_tags (
    entity_id TEXT NOT NULL,
    tag       TEXT NOT NULL,
    PRIMARY KEY (entity_id, tag)
);
CREATE TABLE IF NOT EXISTS edges (
    from_id   TEXT NOT NULL,
    to_id     TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    style     TEXT,
    label     TEXT,
    PRIMARY KEY (from_id, to_id, edge_type)
);
CREATE TABLE IF NOT EXISTS positions (
    entity_id     TEXT NOT NULL,
    whiteboard_id TEXT NOT NULL,
    x             REAL NOT NULL,
    y             REAL NOT NULL,
    PRIMARY KEY (entity_id, whiteboard_id)
);
CREATE TABLE IF NOT EXISTS section_members (
    section_id TEXT NOT NULL,
    entity_id  TEXT NOT NULL,
    PRIMARY KEY (section_id, entity_id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_entities_kind ON entities(kind);
CREATE INDEX IF NOT EXISTS idx_entities_wb ON entities(whiteboard_id);
CREATE INDEX IF NOT EXISTS idx_entities_file ON entities(file_path);
CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
CREATE INDEX IF NOT EXISTS idx_task_status ON task_fields(status);
CREATE INDEX IF NOT EXISTS idx_question_status ON question_fields(status);
CREATE INDEX IF NOT EXISTS idx_positions_wb ON positions(whiteboard_id);
CREATE INDEX IF NOT EXISTS idx_section_members_entity ON section_members(entity_id);
";

/// 初始化 keysight 模块的数据库表。可重复调用（IF NOT EXISTS）。
pub(super) fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA_V7_SQL)
}
```

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test -p super-tauri --lib modules::keysight::db::tests -- --nocapture 2>&1 | tail -10`

Expected: all tests PASS

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/db.rs
git commit -m "feat(keysight): implement v7 schema with TDD"
```

---

## Task 5: id.rs — ID Generation (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/id.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_card_id_format() {
        let id = gen_card_id();
        assert!(id.starts_with("card_"), "应以 card_ 开头，实际: {id}");
        assert_eq!(id.len(), 13, "card_ (5) + 8 hex = 13，实际: {}", id.len());
    }

    #[test]
    fn test_gen_sec_id_format() {
        let id = gen_sec_id();
        assert!(id.starts_with("sec_"));
        assert_eq!(id.len(), 12); // sec_ (4) + 8 hex
    }

    #[test]
    fn test_gen_note_id_format() {
        let id = gen_note_id();
        assert!(id.starts_with("note_"));
        assert_eq!(id.len(), 13); // note_ (5) + 8 hex
    }

    #[test]
    fn test_gen_alias_id_format() {
        let id = gen_alias_id();
        assert!(id.starts_with("alias_"));
        assert_eq!(id.len(), 14); // alias_ (6) + 8 hex
    }

    #[test]
    fn test_gen_task_id_format() {
        let id = gen_task_id();
        assert!(id.starts_with("task_"));
        assert_eq!(id.len(), 13); // task_ (5) + 8 hex
    }

    #[test]
    fn test_gen_question_id_format() {
        let id = gen_question_id();
        assert!(id.starts_with("q_"));
        assert_eq!(id.len(), 10); // q_ (2) + 8 hex
    }

    #[test]
    fn test_ids_are_unique() {
        let ids: Vec<String> = (0..100).map(|_| gen_card_id()).collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "100 个 ID 应全部唯一");
    }

    #[test]
    fn test_hex_chars_only() {
        let id = gen_card_id();
        let hex_part = &id[5..]; // 跳过 "card_"
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "hex 部分应全为十六进制字符，实际: {hex_part}"
        );
    }
}
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test -p super-tauri --lib modules::keysight::id::tests -- --nocapture 2>&1 | tail -15`

Expected: FAIL — functions not defined

**⏸ 等待用户确认 Red**

- [ ] **Step 3: Implement ID generation**

```rust
use rand::Rng;

/// 生成 8 字符小写十六进制字符串。
fn gen_hex8() -> String {
    let n: u32 = rand::rng().random();
    format!("{:08x}", n)
}

pub(super) fn gen_card_id() -> String {
    format!("card_{}", gen_hex8())
}

pub(super) fn gen_sec_id() -> String {
    format!("sec_{}", gen_hex8())
}

pub(super) fn gen_note_id() -> String {
    format!("note_{}", gen_hex8())
}

pub(super) fn gen_alias_id() -> String {
    format!("alias_{}", gen_hex8())
}

pub(super) fn gen_task_id() -> String {
    format!("task_{}", gen_hex8())
}

pub(super) fn gen_question_id() -> String {
    format!("q_{}", gen_hex8())
}
```

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

Run: 同 Step 2 命令

Expected: all tests PASS

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/id.rs
git commit -m "feat(keysight): implement id generation with TDD"
```

---

## Task 6: parser.rs — Frontmatter Parsing (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/parser.rs`

从源项目 `infra/parser.rs` 移植。parser 是纯函数（输入 markdown 文本，输出解析结果），非常适合 TDD。

- [ ] **Step 1: Define output types + write failing tests**

```rust
use serde::{Deserialize, Serialize};

/// parse_entity 的输出。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ParsedEntity {
    pub entity_type: String,
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub link_to: Vec<String>,
    pub related: Vec<String>,
    pub see_also: Vec<String>,
    pub understanding: String,
    pub source: String,
    pub task_status: Option<String>,
    pub task_area: Option<String>,
    pub task_project: Option<String>,
    pub question_status: Option<String>,
}

/// frontmatter 更新请求。
#[derive(Debug, Default)]
pub(super) struct FrontmatterUpdate {
    pub id: Option<String>,
    pub link_to: Option<Vec<String>>,
    pub related: Option<Vec<String>>,
    pub understanding: Option<String>,
    pub see_also: Option<Vec<String>>,
}

pub(super) fn parse_entity(_markdown: &str) -> Option<ParsedEntity> {
    todo!() // 空实现
}

pub(super) fn extract_frontmatter(_markdown: &str) -> Option<String> {
    todo!()
}

pub(super) fn skip_frontmatter(_markdown: &str) -> &str {
    todo!()
}

pub(super) fn write_frontmatter(_markdown: &str, _updates: FrontmatterUpdate) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD_MD: &str = "\
---
type: atomic-card
id: card_abc12345
tags:
  - rust
  - concurrency
linkTo:
  - card_def67890
related:
  - card_xyz11111
understanding: 深入理解所有权模型
source: https://doc.rust-lang.org
see-also:
  - card_see11111
---

# 【ATC】Rust Ownership

Body content here.
";

    const NOTE_MD: &str = "\
---
type: note
id: note_abc12345
tags:
  - study
---

# 【NOTE】Study Notes

Note body content.
";

    const TASK_MD: &str = "\
---
type: project-task
id: task_abc12345
status: active
area: backend
project: keysight
---

# 【TASK】Implement parser

Task description.
";

    const QUESTION_MD: &str = "\
---
type: question
id: q_abc12345
status: pending
---

# 【QUE】How does async work?

Question body.
";

    #[test]
    fn test_parse_card() {
        let parsed = parse_entity(CARD_MD).expect("应成功解析 card");
        assert_eq!(parsed.entity_type, "atomic-card");
        assert_eq!(parsed.id, Some("card_abc12345".to_string()));
        assert_eq!(parsed.title, "Rust Ownership");
        assert_eq!(parsed.content, "Body content here.\n");
        assert_eq!(parsed.tags, vec!["rust", "concurrency"]);
        assert_eq!(parsed.link_to, vec!["card_def67890"]);
        assert_eq!(parsed.related, vec!["card_xyz11111"]);
        assert_eq!(parsed.see_also, vec!["card_see11111"]);
        assert_eq!(parsed.understanding, "深入理解所有权模型");
        assert_eq!(parsed.source, "https://doc.rust-lang.org");
    }

    #[test]
    fn test_parse_note() {
        let parsed = parse_entity(NOTE_MD).expect("应成功解析 note");
        assert_eq!(parsed.entity_type, "note");
        assert_eq!(parsed.title, "Study Notes");
    }

    #[test]
    fn test_parse_task() {
        let parsed = parse_entity(TASK_MD).expect("应成功解析 task");
        assert_eq!(parsed.entity_type, "project-task");
        assert_eq!(parsed.task_status, Some("active".to_string()));
        assert_eq!(parsed.task_area, Some("backend".to_string()));
        assert_eq!(parsed.task_project, Some("keysight".to_string()));
    }

    #[test]
    fn test_parse_question() {
        let parsed = parse_entity(QUESTION_MD).expect("应成功解析 question");
        assert_eq!(parsed.entity_type, "question");
        assert_eq!(parsed.question_status, Some("pending".to_string()));
    }

    #[test]
    fn test_parse_unknown_type_returns_none() {
        let md = "---\ntype: unknown\n---\n# Title\nBody\n";
        assert!(parse_entity(md).is_none());
    }

    #[test]
    fn test_parse_no_frontmatter_returns_none() {
        let md = "# Just a title\nNo frontmatter here.\n";
        assert!(parse_entity(md).is_none());
    }

    #[test]
    fn test_parse_no_type_returns_none() {
        let md = "---\ntags:\n  - test\n---\n# Title\nBody\n";
        assert!(parse_entity(md).is_none());
    }

    #[test]
    fn test_extract_frontmatter() {
        let fm = extract_frontmatter(CARD_MD).expect("应提取 frontmatter");
        assert!(fm.contains("type: atomic-card"));
        assert!(fm.contains("id: card_abc12345"));
    }

    #[test]
    fn test_skip_frontmatter() {
        let after = skip_frontmatter(CARD_MD);
        assert!(after.starts_with("\n# 【ATC】Rust Ownership"));
    }

    #[test]
    fn test_card_without_id() {
        let md = "---\ntype: atomic-card\ntags:\n  - test\n---\n\n# 【ATC】No ID Card\n\nBody.\n";
        let parsed = parse_entity(md).expect("应成功解析");
        assert_eq!(parsed.id, None);
        assert_eq!(parsed.title, "No ID Card");
    }
}
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test -p super-tauri --lib modules::keysight::parser::tests -- --nocapture 2>&1 | tail -15`

Expected: FAIL — `not yet implemented`

**⏸ 等待用户确认 Red**

- [ ] **Step 3: Implement parser**

从源项目 `infra/parser.rs` 移植核心逻辑。关键实现点：

```rust
use serde::Deserialize;

/// 已知的实体类型。
const KNOWN_TYPES: &[&str] = &["atomic-card", "note", "project-task", "question"];

/// 文件标记，从 H1 标题中去掉。
const FILE_MARKERS: &[&str] = &["【ATC】", "【NOTE】", "【TASK】", "【QUE】"];

/// 原始 frontmatter 反序列化中间结构。
#[derive(Debug, Deserialize)]
struct RawEntityFrontmatter {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    understanding: Option<String>,
    #[serde(default, rename = "linkTo")]
    link_to: Vec<String>,
    #[serde(default)]
    related: Vec<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default, rename = "see-also")]
    see_also: Vec<String>,
    // task 字段
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    area: Option<String>,
    #[serde(default)]
    project: Option<String>,
}

/// 提取 --- 之间的 YAML frontmatter 文本。
pub(super) fn extract_frontmatter(markdown: &str) -> Option<String> {
    let trimmed = markdown.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let close = after_first.find("\n---")?;
    Some(after_first[..close].to_string())
}

/// 返回 frontmatter 之后的文本。
pub(super) fn skip_frontmatter(markdown: &str) -> &str {
    let trimmed = markdown.trim_start();
    if !trimmed.starts_with("---") {
        return markdown;
    }
    let after_first = &trimmed[3..];
    match after_first.find("\n---") {
        Some(pos) => {
            let rest = &after_first[pos + 4..]; // 跳过 "\n---"
            rest
        }
        None => markdown,
    }
}

/// 从 frontmatter 之后的文本中提取 H1 标题和正文。
fn extract_title_and_body(text: &str) -> (String, String) {
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            let mut title = trimmed[2..].to_string();
            // 移除文件标记
            for marker in FILE_MARKERS {
                title = title.replace(marker, "");
            }
            let title = title.trim().to_string();
            // 正文 = H1 之后的所有行
            let body_lines: Vec<&str> = text.lines().skip(i + 1).collect();
            let body = body_lines.join("\n");
            let body = body.trim_start_matches('\n').to_string();
            // 保留尾部换行
            let body = if body.is_empty() { body } else { format!("{body}\n") };
            return (title, body);
        }
    }
    // 没有 H1 — 用第一个非空行作标题
    let first_non_empty = text.lines().find(|l| !l.trim().is_empty());
    match first_non_empty {
        Some(line) => (line.trim().to_string(), String::new()),
        None => (String::new(), String::new()),
    }
}

/// 解析 markdown 文件为多类型实体。
pub(super) fn parse_entity(markdown: &str) -> Option<ParsedEntity> {
    let fm_str = extract_frontmatter(markdown)?;
    let raw: RawEntityFrontmatter = serde_yaml::from_str(&fm_str).ok()?;

    let entity_type = raw.r#type.as_deref()?;
    if !KNOWN_TYPES.contains(&entity_type) {
        return None;
    }

    let after_fm = skip_frontmatter(markdown);
    let (title, content) = extract_title_and_body(after_fm);

    // id: 优先用 id 字段，fallback 到 uuid（向后兼容）
    let id = raw.id.or(raw.uuid);

    // task/question 状态字段
    let (task_status, task_area, task_project, question_status) = match entity_type {
        "project-task" => (raw.status.clone(), raw.area.clone(), raw.project.clone(), None),
        "question" => (None, None, None, raw.status.clone()),
        _ => (None, None, None, None),
    };

    Some(ParsedEntity {
        entity_type: entity_type.to_string(),
        id,
        title,
        content,
        tags: raw.tags,
        link_to: raw.link_to,
        related: raw.related,
        see_also: raw.see_also,
        understanding: raw.understanding.unwrap_or_default(),
        source: raw.source.unwrap_or_default(),
        task_status,
        task_area,
        task_project,
        question_status,
    })
}

/// write_frontmatter — 修改 frontmatter 并返回完整 markdown。
pub(super) fn write_frontmatter(markdown: &str, updates: FrontmatterUpdate) -> String {
    let fm_str = extract_frontmatter(markdown);
    let mut map: serde_yaml::Mapping = match &fm_str {
        Some(s) => serde_yaml::from_str(s).unwrap_or_default(),
        None => serde_yaml::Mapping::new(),
    };

    // 应用更新
    if let Some(id) = updates.id {
        // 移除旧 uuid key（向后兼容），写入 id
        map.remove(&serde_yaml::Value::String("uuid".to_string()));
        map.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(id),
        );
    }
    if let Some(link_to) = updates.link_to {
        let val: Vec<serde_yaml::Value> = link_to.into_iter().map(serde_yaml::Value::String).collect();
        map.insert(
            serde_yaml::Value::String("linkTo".to_string()),
            serde_yaml::Value::Sequence(val),
        );
    }
    if let Some(related) = updates.related {
        let val: Vec<serde_yaml::Value> = related.into_iter().map(serde_yaml::Value::String).collect();
        map.insert(
            serde_yaml::Value::String("related".to_string()),
            serde_yaml::Value::Sequence(val),
        );
    }
    if let Some(understanding) = updates.understanding {
        map.insert(
            serde_yaml::Value::String("understanding".to_string()),
            serde_yaml::Value::String(understanding),
        );
    }
    if let Some(see_also) = updates.see_also {
        let val: Vec<serde_yaml::Value> = see_also.into_iter().map(serde_yaml::Value::String).collect();
        map.insert(
            serde_yaml::Value::String("see-also".to_string()),
            serde_yaml::Value::Sequence(val),
        );
    }

    let after_fm = skip_frontmatter(markdown);
    let yaml = serde_yaml::to_string(&map).unwrap_or_default();
    format!("---\n{}---\n{}", yaml, after_fm)
}

/// understanding 字段可以是 string 或 list（旧版兼容）。
fn string_or_list<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrList;
    impl<'de> de::Visitor<'de> for StringOrList {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string or list of strings")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut parts = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                parts.push(s);
            }
            Ok(Some(parts.join("\n")))
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(StringOrList)
}
```

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test -p super-tauri --lib modules::keysight::parser::tests -- --nocapture 2>&1 | tail -15`

Expected: all tests PASS

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/parser.rs
git commit -m "feat(keysight): implement frontmatter parser with TDD"
```

---

## Task 7: vault_fs.rs — File I/O Abstraction

**Files:**
- Modify: `src-tauri/src/modules/keysight/vault_fs.rs`

为文件读写提供 trait 抽象，测试时用 MockVaultFs 替代真实文件系统。

- [ ] **Step 1: Implement VaultFs trait + real + mock implementations**

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use super::errors::KeysightError;

/// 文件系统操作契约 — 测试时用 mock 替代真实文件系统。
pub(super) trait VaultFs {
    /// 读取 vault 内相对路径的文件内容。
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError>;
    /// 写入 vault 内相对路径的文件内容。
    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError>;
}

/// 真实文件系统实现。
pub(super) struct RealVaultFs {
    vault_path: String,
}

impl RealVaultFs {
    pub fn new(vault_path: String) -> Self {
        Self { vault_path }
    }

    fn abs_path(&self, relative: &str) -> String {
        if self.vault_path.ends_with('/') {
            format!("{}{}", self.vault_path, relative)
        } else {
            format!("{}/{}", self.vault_path, relative)
        }
    }
}

impl VaultFs for RealVaultFs {
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError> {
        let abs = self.abs_path(relative_path);
        std::fs::read_to_string(&abs)
            .map_err(|e| KeysightError::FileError(format!("读取 {abs}: {e}")))
    }

    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError> {
        let abs = self.abs_path(relative_path);
        if let Some(parent) = Path::new(&abs).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| KeysightError::FileError(format!("创建目录 {}: {e}", parent.display())))?;
        }
        std::fs::write(&abs, content)
            .map_err(|e| KeysightError::FileError(format!("写入 {abs}: {e}")))
    }
}

/// 测试用 mock 文件系统。
#[cfg(test)]
pub(super) struct MockVaultFs {
    files: RefCell<HashMap<String, String>>,
}

#[cfg(test)]
impl MockVaultFs {
    pub fn new() -> Self {
        Self {
            files: RefCell::new(HashMap::new()),
        }
    }

    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.files.borrow_mut().insert(path.to_string(), content.to_string());
        self
    }

    /// 获取 mock 文件系统中的文件内容（用于测试断言）。
    pub fn get_file(&self, path: &str) -> Option<String> {
        self.files.borrow().get(path).cloned()
    }
}

#[cfg(test)]
impl VaultFs for MockVaultFs {
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError> {
        self.files
            .borrow()
            .get(relative_path)
            .cloned()
            .ok_or_else(|| {
                KeysightError::FileError(format!("文件不存在: {relative_path}"))
            })
    }

    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError> {
        self.files
            .borrow_mut()
            .insert(relative_path.to_string(), content.to_string());
        Ok(())
    }
}
```

- [ ] **Step 2: Verify + commit**

```bash
cargo clippy --workspace -- -D warnings
git add src-tauri/src/modules/keysight/vault_fs.rs
git commit -m "feat(keysight): add VaultFs trait for file I/O abstraction"
```

---

## Task 8: entity.rs — Edge Operations (TDD + Trait)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/entity.rs`

entity connect/disconnect 是最简单的 domain 模块，建立 TDD + Trait 的模式。

- [ ] **Step 1: Define EntityGraph trait + write failing tests**

```rust
use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::models::{Edge, EdgeStyle, EdgeType};

/// 实体图谱边操作契约。
pub(super) trait EntityGraph {
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
    fn edges_from(&self, entity_id: &str) -> Result<Vec<Edge>, KeysightError>;

    /// 查询某实体的所有入边。
    fn edges_to(&self, entity_id: &str) -> Result<Vec<Edge>, KeysightError>;
}

pub(super) struct SqliteEntityGraph<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteEntityGraph<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> EntityGraph for SqliteEntityGraph<'a> {
    fn connect(&self, _from_id: &str, _to_id: &str, _edge_type: EdgeType, _style: Option<EdgeStyle>, _label: Option<&str>) -> Result<(), KeysightError> {
        todo!()
    }
    fn disconnect(&self, _from_id: &str, _to_id: &str, _edge_type: EdgeType) -> Result<(), KeysightError> {
        todo!()
    }
    fn edges_from(&self, _entity_id: &str) -> Result<Vec<Edge>, KeysightError> {
        todo!()
    }
    fn edges_to(&self, _entity_id: &str) -> Result<Vec<Edge>, KeysightError> {
        todo!()
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
        graph
            .connect("card_aaa", "card_bbb", EdgeType::LinkTo, None, None)
            .unwrap();

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
        graph
            .connect("card_aaa", "card_bbb", EdgeType::Related, Some(EdgeStyle::Dashed), Some("参考"))
            .unwrap();

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
        // 不存在的边 — 不应报错
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
}
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test -p super-tauri --lib modules::keysight::domain::entity::tests -- --nocapture 2>&1 | tail -15`

Expected: FAIL — `not yet implemented`

**⏸ 等待用户确认 Red**

- [ ] **Step 3: Implement EntityGraph**

替换 `todo!()` 为真实实现：

```rust
impl<'a> EntityGraph for SqliteEntityGraph<'a> {
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
            params![from_id, to_id, edge_type_str, style_str, label],
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
            params![from_id, to_id, edge_type_str],
        )?;
        Ok(())
    }

    fn edges_from(&self, entity_id: &str) -> Result<Vec<Edge>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT from_id, to_id, edge_type, style, label FROM edges WHERE from_id = ?1",
        )?;
        let edges = stmt
            .query_map(params![entity_id], |row| {
                Ok(Edge {
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

    fn edges_to(&self, entity_id: &str) -> Result<Vec<Edge>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT from_id, to_id, edge_type, style, label FROM edges WHERE to_id = ?1",
        )?;
        let edges = stmt
            .query_map(params![entity_id], |row| {
                Ok(Edge {
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
```

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

Run: 同 Step 2 命令

Expected: all tests PASS

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/entity.rs
git commit -m "feat(keysight): implement EntityGraph with TDD"
```

---

## Task 9: sync.rs — File Sync (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/sync.rs`

从源项目移植 `sync_file_v7`。这个函数已经写新表，是迁移中最可直接复用的代码。

- [ ] **Step 1: Write failing tests**

```rust
use rusqlite::{params, Connection};

use crate::modules::keysight::db::init_db;
use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::models::SyncFileResponse;

/// 将 markdown 文件内容同步到数据库。
///
/// ## 执行效果
/// 1. 解析 frontmatter → 确定实体类型
/// 2. 生成或使用已有 ID
/// 3. UPSERT entities 表 + 类型特定字段表
/// 4. 全量替换 entity_tags 和 edges（从 frontmatter 重建）
/// 5. UPSERT file_mtimes
///
/// ## 幂等性
/// 相同内容重复调用 → updated=1, inserted=0
pub(super) fn sync_file(
    _conn: &Connection,
    _file_path: &str,
    _content: &str,
    _mtime: f64,
) -> Result<SyncFileResponse, KeysightError> {
    todo!()
}

/// 从 file_path 推导 whiteboard_id。
pub(super) fn derive_whiteboard_id(_file_path: &str) -> String {
    todo!()
}

/// 删除文件对应的实体及 mtime 记录。
pub(super) fn remove_file(_conn: &Connection, _file_path: &str) -> Result<(), KeysightError> {
    todo!()
}

/// 查询所有文件的 mtime。
pub(super) fn all_file_mtimes(
    _conn: &Connection,
) -> Result<Vec<(String, f64)>, KeysightError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    const CARD_MD: &str = "\
---
type: atomic-card
id: card_test0001
tags:
  - rust
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

Body content here.
";

    #[test]
    fn test_sync_card_inserts_entity() {
        let conn = test_conn();
        let resp = sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();
        assert_eq!(resp.inserted, 1);
        assert_eq!(resp.updated, 0);

        // 验证 entities 表
        let title: String = conn
            .query_row("SELECT title FROM entities WHERE id = 'card_test0001'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "Test Card");
    }

    #[test]
    fn test_sync_card_writes_card_fields() {
        let conn = test_conn();
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();

        let understanding: String = conn
            .query_row(
                "SELECT understanding FROM card_fields WHERE entity_id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(understanding, "测试理解");
    }

    #[test]
    fn test_sync_card_writes_tags() {
        let conn = test_conn();
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entity_tags WHERE entity_id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1); // "rust" tag
    }

    #[test]
    fn test_sync_card_writes_edges() {
        let conn = test_conn();
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE from_id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // link_to(1) + related(1) + see_also(1) = 3
        assert_eq!(count, 3);
    }

    #[test]
    fn test_sync_card_updates_mtime() {
        let conn = test_conn();
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();

        let mtime: f64 = conn
            .query_row(
                "SELECT mtime FROM file_mtimes WHERE filePath = 'atomic cards/test.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!((mtime - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sync_idempotent_update() {
        let conn = test_conn();
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();
        let resp = sync_file(&conn, "atomic cards/test.md", CARD_MD, 2000.0).unwrap();
        assert_eq!(resp.updated, 1);
        assert_eq!(resp.inserted, 0);
    }

    #[test]
    fn test_sync_without_id_generates_one() {
        let conn = test_conn();
        let md = "---\ntype: atomic-card\ntags:\n  - test\n---\n\n# 【ATC】No ID\n\nBody.\n";
        let resp = sync_file(&conn, "atomic cards/noid.md", md, 1000.0).unwrap();
        assert!(resp.needs_id_backfill);
        assert!(resp.assigned_id.starts_with("card_"));
    }

    #[test]
    fn test_sync_task() {
        let conn = test_conn();
        let md = "---\ntype: project-task\nid: task_test0001\nstatus: active\narea: backend\nproject: keysight\n---\n\n# 【TASK】Test Task\n\nTask body.\n";
        sync_file(&conn, "tasks/test.md", md, 1000.0).unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM task_fields WHERE entity_id = 'task_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "active");
    }

    #[test]
    fn test_sync_non_entity_file_only_updates_mtime() {
        let conn = test_conn();
        let md = "# Just a regular file\n\nNo frontmatter.\n";
        let resp = sync_file(&conn, "notes/regular.md", md, 1000.0).unwrap();
        assert_eq!(resp.inserted, 0);
        assert_eq!(resp.updated, 0);

        // mtime 仍应记录
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_mtimes WHERE filePath = 'notes/regular.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_derive_whiteboard_id_root() {
        assert_eq!(derive_whiteboard_id("atomic cards/test.md"), "wb_root");
    }

    #[test]
    fn test_derive_whiteboard_id_sub() {
        assert_eq!(derive_whiteboard_id("whiteboard/myboard/test.md"), "myboard");
    }

    #[test]
    fn test_remove_file() {
        let conn = test_conn();
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();
        remove_file(&conn, "atomic cards/test.md").unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities WHERE file_path = 'atomic cards/test.md'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_all_file_mtimes() {
        let conn = test_conn();
        sync_file(&conn, "a.md", CARD_MD, 100.0).unwrap();

        let mtimes = all_file_mtimes(&conn).unwrap();
        assert_eq!(mtimes.len(), 1);
        assert_eq!(mtimes[0].0, "a.md");
    }
}
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

**⏸ 等待用户确认 Red**

- [ ] **Step 3: Implement sync_file + helpers**

从源项目 `sync_file_v7` 移植，适配新架构（`&Connection` 替代 `&Store`，使用 `parser::parse_entity`，使用 `id::gen_*_id`）。

关键实现要点：
1. `parse_entity` 解析 markdown
2. 确定 kind_str（`"atomic-card"` → `"card"`）
3. `derive_whiteboard_id` 从 file_path 推导
4. 如无 id 则 `gen_*_id()` 生成
5. UPSERT `entities` 表
6. UPSERT 类型特定字段表（`card_fields` / `task_fields` / `question_fields`）
7. 全量替换 `entity_tags`（DELETE + INSERT）
8. 全量替换 `edges`（DELETE from_id + INSERT link_to/related/see_also）
9. UPSERT `file_mtimes`

详细 SQL 见源项目 `sync_file_v7` 函数（已在探索报告中完整记录），逐行移植并改 `store.conn()` → `conn`。

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/sync.rs
git commit -m "feat(keysight): implement sync_file with TDD"
```

---

## Task 10: card.rs — Card Queries + Mutations (TDD + Trait)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/card.rs`

这是最大的改写——旧代码读 `insights` 表，新代码必须读 `entities + card_fields + entity_tags + edges`。

- [ ] **Step 1: Define CardStore trait**

```rust
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
```

- [ ] **Step 2: Write failing tests**

关键测试：先通过 `sync_file` 插入测试数据，再验证 CardStore 查询。

```rust
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

    #[test]
    fn test_get_card_by_id() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.title, "Test Card");
        assert_eq!(card.tags, vec!["rust", "ownership"]);
        assert_eq!(card.link_to, vec!["card_other001"]);
        assert_eq!(card.related, vec!["card_other002"]);
        assert_eq!(card.see_also, vec!["card_other003"]);
        assert_eq!(card.understanding, "测试理解");
        assert_eq!(card.source, "https://example.com");
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
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_all(None, None).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "card_test0001");
    }

    #[test]
    fn test_query_by_file() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_by_file("atomic cards/test.md").unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn test_count() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);
        assert_eq!(store.count().unwrap(), 1);
    }
}
```

- [ ] **Step 3: Run test → 🔴 Red checkpoint**

**⏸ 等待用户确认 Red**

- [ ] **Step 4: Implement CardStore**

核心改写——从 `insights` 单表查询改为 3-query batch：

```sql
-- Q1: 实体 + card_fields + mtime
SELECT e.id, e.title, e.content, e.file_path,
       COALESCE(c.understanding, '') AS understanding,
       COALESCE(c.source, '') AS source,
       COALESCE(f.mtime, 0) AS mtime
FROM entities e
LEFT JOIN card_fields c ON e.id = c.entity_id
LEFT JOIN file_mtimes f ON e.file_path = f.filePath
WHERE e.kind = 'card'
ORDER BY f.mtime DESC

-- Q2: 批量 tags
SELECT entity_id, tag FROM entity_tags WHERE entity_id IN (?)

-- Q3: 批量 edges（link_to/related/see_also）
SELECT from_id, to_id, edge_type FROM edges
WHERE from_id IN (?) AND edge_type IN ('link_to', 'related', 'see_also')
```

`get()` 方法使用 `WHERE e.id = ?1` 版本的 Q1，加单独查 tags 和 edges。

`query_all` / `query_by_file` / `query_by_ids` 使用带条件的 Q1 + 批量 Q2/Q3，然后在 Rust 侧组装 `AtomicCard`。

- [ ] **Step 5: Run test → 🟢 Green checkpoint**

**⏸ 等待用户确认 Green**

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/card.rs
git commit -m "feat(keysight): implement CardStore with v7 queries (TDD)"
```

---

## Task 11-15: Remaining Domain Modules (TDD + Trait)

以下模块均遵循 Task 8 建立的模式：定义 trait → 写测试 → 🔴 → 实现 → 🟢 → commit。

### Task 11: section.rs — Section CRUD

**Trait:** `SectionStore`

```rust
pub(super) trait SectionStore {
    fn create(&self, whiteboard_id: &str, title: &str, color: Option<&str>) -> Result<GraphSection, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn update(&self, id: &str, title: Option<&str>, color: Option<&str>) -> Result<(), KeysightError>;
    fn add_member(&self, section_id: &str, entity_id: &str) -> Result<(), KeysightError>;
    fn remove_member(&self, section_id: &str, entity_id: &str) -> Result<(), KeysightError>;
    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphSection>, KeysightError>;
    fn get(&self, id: &str) -> Result<GraphSection, KeysightError>;
}
```

**关键改写**：旧代码操作 `meta` blob JSON → 新代码操作 `entities(kind='section')` + `section_members` + `edges(edge_type='section_link')` 表。

**核心 SQL**：
- `INSERT INTO entities (id, kind, title, whiteboard_id) VALUES (?, 'section', ?, ?)`
- `INSERT INTO section_members (section_id, entity_id) VALUES (?, ?)`
- `SELECT e.id, e.title, e.color FROM entities e WHERE e.kind = 'section' AND e.whiteboard_id = ?`
- `SELECT entity_id FROM section_members WHERE section_id = ?`
- `SELECT to_id FROM edges WHERE from_id = ? AND edge_type = 'section_link'`
- `GraphSection.card_ids` 从 `section_members` 查询组装
- `GraphSection.linked_section_ids` 从 `edges(section_link)` 查询组装

### Task 12: note.rs — Note CRUD

**Trait:** `NoteStore`

```rust
pub(super) trait NoteStore {
    fn create(&self, whiteboard_id: &str, title: &str, content: Option<&str>, color: Option<&str>) -> Result<GraphNote, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn update(&self, id: &str, title: Option<&str>, content: Option<&str>, color: Option<&str>) -> Result<(), KeysightError>;
    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphNote>, KeysightError>;
}
```

**核心 SQL**：
- `INSERT INTO entities (id, kind, title, whiteboard_id, content, color) VALUES (?, 'note', ?, ?, ?, ?)`
- `SELECT id, title, content, color FROM entities WHERE kind = 'note' AND whiteboard_id = ?`
- `GraphNote.linked_*` 从 `edges(edge_type='note_link')` 查询组装

### Task 13: alias.rs — Alias CRUD

**Trait:** `AliasStore`

```rust
pub(super) trait AliasStore {
    fn create(&self, whiteboard_id: &str, card_id: &str) -> Result<CardAlias, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<CardAlias>, KeysightError>;
}
```

**核心 SQL**：
- `INSERT INTO entities (id, kind, title, whiteboard_id) VALUES (?, 'alias', ?, ?)`
- `INSERT INTO alias_fields (entity_id, card_id) VALUES (?, ?)`
- `SELECT e.id, a.card_id FROM entities e JOIN alias_fields a ON e.id = a.entity_id WHERE e.kind = 'alias' AND e.whiteboard_id = ?`
- `CardAlias.linked_*` 从 `edges(edge_type='alias_link')` 查询组装
- `CardAlias.incoming_card_ids` 从 `edges(edge_type='card_to_alias', to_id=alias_id)` 查询组装

### Task 14: layout.rs — Position Management

**Trait:** `LayoutStore`

```rust
pub(super) trait LayoutStore {
    fn set_position(&self, whiteboard_id: &str, entity_id: &str, x: f64, y: f64) -> Result<(), KeysightError>;
    fn query_positions(&self, whiteboard_id: &str) -> Result<std::collections::HashMap<String, Position>, KeysightError>;
    fn remove_position(&self, whiteboard_id: &str, entity_id: &str) -> Result<(), KeysightError>;
}
```

**核心 SQL**：
- `INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) VALUES (?, ?, ?, ?)`
- `SELECT entity_id, x, y FROM positions WHERE whiteboard_id = ?`
- `DELETE FROM positions WHERE entity_id = ? AND whiteboard_id = ?`

### Task 15: task.rs + question.rs + overview.rs

**task.rs** — 从源项目原样移植（已用 v7 表）：
- `transition_status(conn, id, status)` → `UPDATE task_fields SET status = ? WHERE entity_id = ?`
- `update_area(conn, id, area)` → `UPDATE task_fields SET area = ? WHERE entity_id = ?`
- `update_project(conn, id, project)` → `UPDATE task_fields SET project = ? WHERE entity_id = ?`
- `by_status(conn, status)` → `SELECT ... FROM entities e JOIN task_fields t ON e.id = t.entity_id WHERE t.status = ?`

**question.rs** — 同模式：
- `transition_status(conn, id, status)` → `UPDATE question_fields SET status = ? WHERE entity_id = ?`
- `by_status(conn, status)` → `SELECT ... FROM entities e JOIN question_fields q ON e.id = q.entity_id WHERE q.status = ?`

**overview.rs** — 重写为 v7 查询：
- `stats(conn)` → `SELECT kind, COUNT(*) FROM entities GROUP BY kind` + `SELECT COUNT(*) FROM edges`

每个模块完整 TDD 流程：trait → test → 🔴 → impl → 🟢 → commit。

---

## Task 16: Full Verification + Commit

- [ ] **Step 1: Run all Rust tests**

```bash
cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test --workspace 2>&1 | tail -20
```

Expected: all tests PASS

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace -- -D warnings 2>&1 | tail -5
```

Expected: no warnings

- [ ] **Step 3: Run TS build**

```bash
cd /Users/alexwang/codes/vibe-coding/super-tauri && pnpm build 2>&1 | tail -5
```

Expected: build succeeds

- [ ] **Step 4: Export bindings (verify no regression)**

```bash
cd /Users/alexwang/codes/vibe-coding/super-tauri/src-tauri && cargo test export_bindings 2>&1 | tail -5
```

- [ ] **Step 5: Final commit if any uncommitted changes**

```bash
git status
# 如有未提交的改动，整理后 commit
```

- [ ] **Step 6: Update progress**

将 `docs/progress/keysight.md` 中 Phase 1 从 Next 移到 Done。
