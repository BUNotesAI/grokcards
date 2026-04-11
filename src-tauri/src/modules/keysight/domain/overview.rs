#![allow(dead_code)]
use rusqlite::Connection;

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::models::StatsResponse;

/// 查询各实体类型的数量统计。
pub(super) fn stats(conn: &Connection) -> Result<StatsResponse, KeysightError> {
    // 按 kind 分组统计
    let mut stmt = conn.prepare("SELECT kind, COUNT(*) FROM entities GROUP BY kind")?;
    let rows = stmt.query_map([], |r| {
        let kind: String = r.get(0)?;
        let count: u64 = r.get(1)?;
        Ok((kind, count))
    })?;

    let mut resp = StatsResponse {
        cards: 0,
        notes: 0,
        sections: 0,
        aliases: 0,
        tasks: 0,
        questions: 0,
        edges: 0,
    };

    for r in rows {
        let (kind, count) = r?;
        match kind.as_str() {
            "card" => resp.cards = count,
            "note" => resp.notes = count,
            "section" => resp.sections = count,
            "alias" => resp.aliases = count,
            "task" => resp.tasks = count,
            "question" => resp.questions = count,
            _ => {}
        }
    }

    resp.edges = conn.query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?;

    Ok(resp)
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

    #[test]
    fn test_stats_empty() {
        let conn = test_conn();
        let s = stats(&conn).unwrap();
        assert_eq!(s.cards, 0);
        assert_eq!(s.notes, 0);
        assert_eq!(s.edges, 0);
    }

    #[test]
    fn test_stats_with_data() {
        let conn = test_conn();

        let card_md = "---\ntype: atomic-card\nid: card_stat0001\ntags:\n  - test\nlinkTo:\n  - card_other001\n---\n\n# 【ATC】Stat Card\n\nBody.\n";
        sync::sync_file(&conn, "cards/stat.md", card_md, 100.0).unwrap();

        let task_md = "---\ntype: project-task\nid: task_stat0001\nstatus: next\n---\n\n# 【TASK】Stat Task\n\nBody.\n";
        sync::sync_file(&conn, "tasks/stat.md", task_md, 200.0).unwrap();

        let s = stats(&conn).unwrap();
        assert_eq!(s.cards, 1);
        assert_eq!(s.tasks, 1);
        assert_eq!(s.edges, 1); // card_stat0001 → card_other001 (link_to)
        assert_eq!(s.notes, 0);
    }
}
