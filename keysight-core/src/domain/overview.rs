#![allow(dead_code)]
use std::collections::HashMap;

use rusqlite::Connection;

use crate::errors::KeysightError;
use crate::models::StatsResponse;
use crate::models::{CardSummary, GraphOverviewResponse, WhiteboardOverview, WhiteboardSummary};
use crate::vault_fs::VaultFs;

/// 查询各实体类型的数量统计。
pub fn stats(conn: &Connection) -> Result<StatsResponse, KeysightError> {
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

/// 图谱总览 — 按白板聚合统计 + 卡片摘要。
pub fn graph_overview(conn: &Connection) -> Result<GraphOverviewResponse, KeysightError> {
    // 1. 所有白板
    let mut wb_stmt =
        conn.prepare("SELECT DISTINCT whiteboard_id FROM entities ORDER BY whiteboard_id")?;
    let wb_ids: Vec<String> = wb_stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // 2. 全局 incoming link count
    let mut link_stmt = conn.prepare(
        "SELECT to_id, COUNT(*) FROM edges WHERE edge_type = 'link_to' GROUP BY to_id",
    )?;
    let link_counts: std::collections::HashMap<String, u64> = link_stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // 3. 每个白板聚合
    let mut whiteboards = Vec::new();
    for wb_id in &wb_ids {
        // 按 kind 计数
        let mut count_stmt = conn.prepare(
            "SELECT kind, COUNT(*) FROM entities WHERE whiteboard_id = ?1 GROUP BY kind",
        )?;
        let counts: Vec<(String, u64)> = count_stmt
            .query_map([wb_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut cards = 0u64;
        let mut sections = 0u64;
        let mut notes = 0u64;
        let mut aliases = 0u64;
        for (kind, count) in &counts {
            match kind.as_str() {
                "card" => cards = *count,
                "section" => sections = *count,
                "note" => notes = *count,
                "alias" => aliases = *count,
                _ => {}
            }
        }

        // 卡片摘要
        let mut card_stmt = conn.prepare(
            "SELECT id, title, COALESCE(file_path, '') FROM entities WHERE kind = 'card' AND whiteboard_id = ?1 ORDER BY title",
        )?;
        let card_summaries: Vec<CardSummary> = card_stmt
            .query_map([wb_id], |r| {
                let id: String = r.get(0)?;
                let title: String = r.get(1)?;
                let file_path: String = r.get(2)?;
                Ok((id, title, file_path))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|(id, title, file_path)| {
                let incoming_link_count = link_counts.get(&id).copied().unwrap_or(0);
                CardSummary {
                    id,
                    title,
                    file_path,
                    incoming_link_count,
                }
            })
            .collect();

        whiteboards.push(WhiteboardOverview {
            whiteboard_id: wb_id.clone(),
            cards,
            sections,
            notes,
            aliases,
            card_summaries,
        });
    }

    Ok(GraphOverviewResponse { whiteboards })
}

/// 查询所有子白板的轻量统计（排除 wb_root）。
pub fn list_whiteboards(
    conn: &Connection,
    fs: &dyn VaultFs,
) -> Result<Vec<WhiteboardSummary>, KeysightError> {
    // 一级 whiteboard 目录排除字面 "projects":它是嵌套 project 白板的容器,本身不是白板
    let top_level_whiteboards = fs
        .list_first_level_dirs("whiteboard")?
        .into_iter()
        .filter(|name| name != "projects");

    // whiteboard/projects/* 嵌套 project 白板,带 "projects/" 前缀变成完整 wb_id
    let project_whiteboards = fs
        .list_project_whiteboards()?
        .into_iter()
        .map(|name| format!("projects/{name}"));

    let folder_whiteboards: Vec<String> = top_level_whiteboards.chain(project_whiteboards).collect();

    let mut stmt = conn.prepare(
        "SELECT whiteboard_id,
                SUM(CASE WHEN kind = 'card' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'note' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'section' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'alias' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'task' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'question' THEN 1 ELSE 0 END)
         FROM entities
         WHERE whiteboard_id != 'wb_root'
         GROUP BY whiteboard_id
         ORDER BY whiteboard_id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WhiteboardSummary {
                whiteboard_id: r.get(0)?,
                cards: r.get(1)?,
                notes: r.get(2)?,
                sections: r.get(3)?,
                aliases: r.get(4)?,
                tasks: r.get(5)?,
                questions: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut by_id: HashMap<String, WhiteboardSummary> = rows
        .into_iter()
        .map(|summary| (summary.whiteboard_id.clone(), summary))
        .collect();

    for whiteboard_id in folder_whiteboards {
        by_id.entry(whiteboard_id.clone()).or_insert(WhiteboardSummary {
            whiteboard_id,
            cards: 0,
            notes: 0,
            sections: 0,
            aliases: 0,
            tasks: 0,
            questions: 0,
        });
    }

    let mut results = by_id.into_values().collect::<Vec<_>>();
    results.sort_by(|a, b| a.whiteboard_id.cmp(&b.whiteboard_id));
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use crate::domain::sync;
    use crate::vault_fs::MockVaultFs;

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

    // --- graph_overview ---

    #[test]
    fn test_graph_overview_empty() {
        let conn = test_conn();
        let resp = graph_overview(&conn).unwrap();
        assert!(resp.whiteboards.is_empty());
    }

    #[test]
    fn test_graph_overview_with_data() {
        let conn = test_conn();

        let card_md = "---\ntype: atomic-card\nid: card_ov_001\ntags:\n  - test\nlinkTo:\n  - card_ov_002\n---\n\n# 【ATC】Overview Card 1\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/ov1.md", card_md, 100.0).unwrap();

        let card_md2 = "---\ntype: atomic-card\nid: card_ov_002\n---\n\n# 【ATC】Overview Card 2\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/ov2.md", card_md2, 200.0).unwrap();

        // 另一个白板的 task
        let task_md = "---\ntype: project-task\nid: task_ov_001\nstatus: next\n---\n\n# 【TASK】Task\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/myboard/task.md", task_md, 300.0).unwrap();

        let resp = graph_overview(&conn).unwrap();
        assert_eq!(resp.whiteboards.len(), 2); // wb_root + myboard

        // 找 wb_root
        let root = resp.whiteboards.iter().find(|w| w.whiteboard_id == "wb_root").unwrap();
        assert_eq!(root.cards, 2);
        assert_eq!(root.card_summaries.len(), 2);

        // card_ov_002 被 link_to 一次
        let ov2 = root.card_summaries.iter().find(|c| c.id == "card_ov_002").unwrap();
        assert_eq!(ov2.incoming_link_count, 1);

        // 找 myboard
        let myboard = resp.whiteboards.iter().find(|w| w.whiteboard_id == "myboard").unwrap();
        assert_eq!(myboard.cards, 0); // task 不是 card
    }

    // --- list_whiteboards ---

    #[test]
    fn test_list_whiteboards_empty() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let result = list_whiteboards(&conn, &fs).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_whiteboards_excludes_wb_root() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let card_md = "---\ntype: atomic-card\nid: card_wb_r001\n---\n\n# 【ATC】Root Card\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/root_card.md", card_md, 100.0).unwrap();

        let result = list_whiteboards(&conn, &fs).unwrap();
        assert!(result.is_empty(), "wb_root 的实体不应出现在子白板列表中");
    }

    #[test]
    fn test_list_whiteboards_counts_by_kind() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let card1 = "---\ntype: atomic-card\nid: card_wbl_001\n---\n\n# 【ATC】Card 1\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/rust/c1.md", card1, 100.0).unwrap();
        let card2 = "---\ntype: atomic-card\nid: card_wbl_002\n---\n\n# 【ATC】Card 2\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/rust/c2.md", card2, 200.0).unwrap();

        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('note_wbl_001', 'note', 'Note 1', 'rust')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('alias_wbl_001', 'alias', 'Alias 1', 'chentian')",
            [],
        ).unwrap();

        let result = list_whiteboards(&conn, &fs).unwrap();
        assert_eq!(result.len(), 2);

        let rust_wb = result.iter().find(|w| w.whiteboard_id == "rust").unwrap();
        assert_eq!(rust_wb.cards, 2);
        assert_eq!(rust_wb.notes, 1);
        assert_eq!(rust_wb.sections, 0);

        let ct_wb = result.iter().find(|w| w.whiteboard_id == "chentian").unwrap();
        assert_eq!(ct_wb.aliases, 1);
        assert_eq!(ct_wb.cards, 0);
    }

    #[test]
    fn test_list_whiteboards_includes_empty_folder_whiteboard() {
        let conn = test_conn();
        let fs = MockVaultFs::new().with_dir("whiteboard/agent");

        let result = list_whiteboards(&conn, &fs).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].whiteboard_id, "agent");
        assert_eq!(result[0].cards, 0);
        assert_eq!(result[0].notes, 0);
        assert_eq!(result[0].aliases, 0);
    }

    #[test]
    fn test_list_whiteboards_lists_empty_nested_project_whiteboard() {
        // 场景 c:whiteboard/projects/super-tauri 目录存在但 DB 无任何 task
        //       期望:作为 "projects/super-tauri" 白板出现,counts 全 0
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_dir("whiteboard/projects")
            .with_dir("whiteboard/projects/super-tauri");

        let result = list_whiteboards(&conn, &fs).unwrap();

        assert_eq!(result.len(), 1, "应只有 projects/super-tauri 一条");
        assert_eq!(result[0].whiteboard_id, "projects/super-tauri");
        assert_eq!(result[0].tasks, 0);
        assert_eq!(result[0].cards, 0);
    }

    #[test]
    fn test_list_whiteboards_excludes_bare_projects_folder() {
        // 场景 b:whiteboard/projects 目录本身存在,但里面无任何 project 子目录
        //       期望:不应出现 whiteboard_id = "projects" 的白板条目
        let conn = test_conn();
        let fs = MockVaultFs::new().with_dir("whiteboard/projects");

        let result = list_whiteboards(&conn, &fs).unwrap();

        assert!(
            !result.iter().any(|w| w.whiteboard_id == "projects"),
            "裸 projects 目录不应被列为白板"
        );
        assert!(result.is_empty(), "没有嵌套 project 目录,结果应为空");
    }

    #[test]
    fn test_list_whiteboards_merges_db_task_with_nested_project_dir() {
        // 场景 a:whiteboard/projects/super-tauri 目录存在,DB 已有 1 个 task 归属此白板
        //       期望:1 条 "projects/super-tauri" 白板,tasks=1,且不重复出现裸 "projects"
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_dir("whiteboard/projects")
            .with_dir("whiteboard/projects/super-tauri");

        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('task_lw__001', 'task', 'My Task', 'projects/super-tauri')",
            [],
        ).unwrap();

        let result = list_whiteboards(&conn, &fs).unwrap();

        assert_eq!(result.len(), 1);
        let pwb = &result[0];
        assert_eq!(pwb.whiteboard_id, "projects/super-tauri");
        assert_eq!(pwb.tasks, 1);
        assert!(
            !result.iter().any(|w| w.whiteboard_id == "projects"),
            "不应同时存在裸 projects 白板"
        );
    }
}
