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
