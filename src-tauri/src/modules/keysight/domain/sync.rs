#![allow(dead_code)]
use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::id;
use crate::modules::keysight::models::SyncFileResponse;
use crate::modules::keysight::parser;

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
pub(in crate::modules::keysight) fn sync_file(
    conn: &Connection,
    file_path: &str,
    content: &str,
    mtime: f64,
) -> Result<SyncFileResponse, KeysightError> {
    // 1. 解析 markdown
    let parsed = match parser::parse_entity(content) {
        Some(p) => p,
        None => {
            // 非实体文件 — 只记录 mtime
            conn.execute(
                "INSERT OR REPLACE INTO file_mtimes (filePath, mtime) VALUES (?1, ?2)",
                params![file_path, mtime],
            )?;
            return Ok(SyncFileResponse {
                updated: 0,
                inserted: 0,
                deleted: 0,
                needs_id_backfill: false,
                assigned_id: String::new(),
            });
        }
    };

    // 2. 确定 kind 字符串
    let kind_str = match parsed.entity_type.as_str() {
        "atomic-card" => "card",
        "note" => "note",
        "project-task" => "task",
        "question" => "question",
        other => return Err(KeysightError::InvalidEntityType(other.to_string())),
    };

    // 3. 推导 whiteboard_id
    let wb_id = derive_whiteboard_id(file_path);

    // 4. 确定或生成 ID
    let mut needs_id_backfill = false;
    let mut assigned_id = String::new();
    let id = if let Some(ref existing_id) = parsed.id {
        existing_id.clone()
    } else {
        let new_id = match kind_str {
            "card" => id::gen_card_id(),
            "note" => id::gen_note_id(),
            "task" => id::gen_task_id(),
            "question" => id::gen_question_id(),
            _ => unreachable!(),
        };
        needs_id_backfill = true;
        assigned_id = new_id.clone();
        new_id
    };

    // 5. UPSERT entities
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE id = ?1",
        [&id],
        |r| r.get::<_, i64>(0),
    )? > 0;

    let (updated, inserted) = if exists {
        conn.execute(
            "UPDATE entities SET kind = ?1, title = ?2, whiteboard_id = ?3, file_path = ?4, content = ?5 WHERE id = ?6",
            params![kind_str, parsed.title, wb_id, file_path, parsed.content, id],
        )?;
        (1u32, 0u32)
    } else {
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, file_path, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, kind_str, parsed.title, wb_id, file_path, parsed.content],
        )?;
        (0u32, 1u32)
    };

    // 6. UPSERT 类型特定字段
    match kind_str {
        "card" => {
            conn.execute(
                "INSERT OR REPLACE INTO card_fields (entity_id, understanding, source) VALUES (?1, ?2, ?3)",
                params![id, parsed.understanding, parsed.source],
            )?;
        }
        "task" => {
            conn.execute(
                "INSERT OR REPLACE INTO task_fields (entity_id, status, area, project) VALUES (?1, ?2, ?3, ?4)",
                params![
                    id,
                    parsed.task_status.as_deref().unwrap_or("next"),
                    parsed.task_area,
                    parsed.task_project
                ],
            )?;
        }
        "question" => {
            conn.execute(
                "INSERT OR REPLACE INTO question_fields (entity_id, status) VALUES (?1, ?2)",
                params![id, parsed.question_status.as_deref().unwrap_or("pending")],
            )?;
        }
        "note" => { /* note 无类型特定字段表 */ }
        _ => {}
    }

    // 7. 全量替换 entity_tags
    conn.execute("DELETE FROM entity_tags WHERE entity_id = ?1", [&id])?;
    for tag in &parsed.tags {
        conn.execute(
            "INSERT OR IGNORE INTO entity_tags (entity_id, tag) VALUES (?1, ?2)",
            params![id, tag],
        )?;
    }

    // 8. 全量替换 edges（从 frontmatter 的 link_to/related/see_also 重建）
    conn.execute("DELETE FROM edges WHERE from_id = ?1", [&id])?;
    for target in &parsed.link_to {
        conn.execute(
            "INSERT OR IGNORE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'link_to')",
            params![id, target],
        )?;
    }
    for target in &parsed.related {
        conn.execute(
            "INSERT OR IGNORE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'related')",
            params![id, target],
        )?;
    }
    for target in &parsed.see_also {
        conn.execute(
            "INSERT OR IGNORE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'see_also')",
            params![id, target],
        )?;
    }

    // 8.5 同步 FTS 索引
    conn.execute("DELETE FROM entities_fts WHERE id = ?1", [&id])?;
    conn.execute(
        "INSERT INTO entities_fts (id, title, content) VALUES (?1, ?2, ?3)",
        params![id, parsed.title, parsed.content],
    )?;

    // 9. UPSERT file_mtimes
    conn.execute(
        "INSERT OR REPLACE INTO file_mtimes (filePath, mtime) VALUES (?1, ?2)",
        params![file_path, mtime],
    )?;

    Ok(SyncFileResponse {
        updated,
        inserted,
        deleted: 0,
        needs_id_backfill,
        assigned_id,
    })
}

/// 从 file_path 推导 whiteboard_id。
///
/// 规则：`whiteboard/{sub}/...` → `sub`，其他 → `wb_root`
pub(super) fn derive_whiteboard_id(file_path: &str) -> String {
    let path = file_path
        .strip_prefix("whiteboard/")
        .unwrap_or(file_path);
    // 如果 strip 成功，取第一个 / 之前的部分作为子白板 id
    if path.len() < file_path.len() {
        if let Some(slash_pos) = path.find('/') {
            let sub_wb = &path[..slash_pos];
            if !sub_wb.is_empty() {
                return sub_wb.to_string();
            }
        }
    }
    "wb_root".to_string()
}

/// 删除文件对应的实体及 mtime 记录。
pub(in crate::modules::keysight) fn remove_file(conn: &Connection, file_path: &str) -> Result<(), KeysightError> {
    // 先查出该文件对应的 entity id，级联清理关联表
    let mut stmt = conn.prepare("SELECT id FROM entities WHERE file_path = ?1")?;
    let ids: Vec<String> = stmt
        .query_map([file_path], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    for id in &ids {
        conn.execute("DELETE FROM card_fields WHERE entity_id = ?1", [id])?;
        conn.execute("DELETE FROM task_fields WHERE entity_id = ?1", [id])?;
        conn.execute("DELETE FROM question_fields WHERE entity_id = ?1", [id])?;
        conn.execute("DELETE FROM entity_tags WHERE entity_id = ?1", [id])?;
        conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", [id])?;
        conn.execute("DELETE FROM entities_fts WHERE id = ?1", [id])?;
        conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id])?;
        conn.execute("DELETE FROM section_members WHERE entity_id = ?1", [id])?;
    }

    conn.execute("DELETE FROM entities WHERE file_path = ?1", [file_path])?;
    conn.execute("DELETE FROM file_mtimes WHERE filePath = ?1", [file_path])?;
    Ok(())
}

/// 查询所有文件的 mtime。
pub(in crate::modules::keysight) fn all_file_mtimes(
    conn: &Connection,
) -> Result<Vec<(String, f64)>, KeysightError> {
    let mut stmt = conn.prepare("SELECT filePath, mtime FROM file_mtimes")?;
    let rows = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let mtime: f64 = row.get(1)?;
            Ok((path, mtime))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// 在 frontmatter 中插入 id 字段。
///
/// 找到第一个 `---\n` 后的位置，插入 `id: {id}\n`。
/// 纯字符串操作，不使用 YAML 解析器。
///
/// ## 前置条件
/// - content 以 `---\n` 开头（有 frontmatter）
///
/// ## 不做的事
/// - 不校验 id 是否已存在（sync_file 已判断）
/// - 不重新格式化 frontmatter
pub(in crate::modules::keysight) fn insert_id_into_frontmatter(
    content: &str,
    id: &str,
) -> Result<String, KeysightError> {
    if !content.starts_with("---") {
        return Err(KeysightError::ParseError(
            "文件没有 frontmatter（不以 --- 开头）".to_string(),
        ));
    }
    // 找到第一个 ---\n 后的位置
    let insert_pos = content
        .find("---\n")
        .map(|p| p + 4) // "---\n" 长度为 4
        .ok_or_else(|| KeysightError::ParseError("无法定位 frontmatter 起始".to_string()))?;

    let mut result = String::with_capacity(content.len() + id.len() + 5);
    result.push_str(&content[..insert_pos]);
    result.push_str(&format!("id: {id}\n"));
    result.push_str(&content[insert_pos..]);
    Ok(result)
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

    // --- sync_file ---

    #[test]
    fn test_sync_card_inserts_entity() {
        let conn = test_conn();
        let resp = sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();
        assert_eq!(resp.inserted, 1);
        assert_eq!(resp.updated, 0);

        let title: String = conn
            .query_row("SELECT title FROM entities WHERE id = 'card_test0001'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "Test Card");
    }

    #[test]
    fn test_sync_card_writes_card_fields() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();

        let understanding: String = conn
            .query_row(
                "SELECT understanding FROM card_fields WHERE entity_id = 'card_test0001'",
                [], |r| r.get(0),
            )
            .unwrap();
        assert_eq!(understanding, "测试理解");
    }

    #[test]
    fn test_sync_card_writes_tags() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entity_tags WHERE entity_id = 'card_test0001'",
                [], |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1); // "rust"
    }

    #[test]
    fn test_sync_card_writes_edges() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE from_id = 'card_test0001'",
                [], |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3); // link_to + related + see_also
    }

    #[test]
    fn test_sync_card_updates_mtime() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();

        let mtime: f64 = conn
            .query_row(
                "SELECT mtime FROM file_mtimes WHERE filePath = 'whiteboard/test.md'",
                [], |r| r.get(0),
            )
            .unwrap();
        assert!((mtime - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sync_idempotent_update() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();
        let resp = sync_file(&conn, "whiteboard/test.md", CARD_MD, 2000.0).unwrap();
        assert_eq!(resp.updated, 1);
        assert_eq!(resp.inserted, 0);
    }

    #[test]
    fn test_sync_without_id_generates_one() {
        let conn = test_conn();
        let md = "---\ntype: atomic-card\ntags:\n  - test\n---\n\n# 【ATC】No ID\n\nBody.\n";
        let resp = sync_file(&conn, "whiteboard/noid.md", md, 1000.0).unwrap();
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
                [], |r| r.get(0),
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

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_mtimes WHERE filePath = 'notes/regular.md'",
                [], |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    // --- derive_whiteboard_id ---

    #[test]
    fn test_derive_whiteboard_id_root() {
        // whiteboard/ 下直接的文件映射到 wb_root
        assert_eq!(derive_whiteboard_id("whiteboard/test.md"), "wb_root");
        // 非 whiteboard 目录也映射到 wb_root
        assert_eq!(derive_whiteboard_id("other/test.md"), "wb_root");
    }

    #[test]
    fn test_derive_whiteboard_id_sub() {
        assert_eq!(derive_whiteboard_id("whiteboard/myboard/test.md"), "myboard");
    }

    // --- remove_file ---

    #[test]
    fn test_remove_file() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();
        remove_file(&conn, "whiteboard/test.md").unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE file_path = 'whiteboard/test.md'",
                [], |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // --- all_file_mtimes ---

    #[test]
    fn test_all_file_mtimes() {
        let conn = test_conn();
        sync_file(&conn, "a.md", CARD_MD, 100.0).unwrap();

        let mtimes = all_file_mtimes(&conn).unwrap();
        assert_eq!(mtimes.len(), 1);
        assert_eq!(mtimes[0].0, "a.md");
    }

    // --- FTS 同步 ---

    #[test]
    fn test_sync_card_writes_fts() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'Test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_sync_update_refreshes_fts() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();

        // 用不同内容重新 sync
        let md2 = "---\ntype: atomic-card\nid: card_test0001\n---\n\n# 【ATC】Updated Title\n\nNew body.\n";
        sync_file(&conn, "whiteboard/test.md", md2, 2000.0).unwrap();

        // 旧标题搜不到
        let old: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'Ownership'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old, 0);

        // 新标题能搜到
        let new: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'Updated'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(new, 1);
    }

    #[test]
    fn test_remove_file_cleans_fts() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/test.md", CARD_MD, 1000.0).unwrap();
        remove_file(&conn, "whiteboard/test.md").unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'Test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // --- insert_id_into_frontmatter ---

    #[test]
    fn test_insert_id_basic() {
        let content = "---\ntype: atomic-card\ntags:\n  - rust\n---\n\n# Title\n\nBody.\n";
        let result = insert_id_into_frontmatter(content, "card_abc12345").unwrap();
        assert!(result.contains("id: card_abc12345\n"));
        // id 应在第一个 --- 之后
        let id_pos = result.find("id: card_abc12345").unwrap();
        let first_sep = result.find("---").unwrap();
        assert!(id_pos > first_sep);
        // 其余 frontmatter 字段不变
        assert!(result.contains("type: atomic-card"));
        assert!(result.contains("tags:"));
    }

    #[test]
    fn test_insert_id_preserves_body() {
        let content = "---\ntype: atomic-card\n---\n\n# Title\n\nBody content.\n";
        let result = insert_id_into_frontmatter(content, "card_xyz99999").unwrap();
        assert!(result.contains("# Title"));
        assert!(result.contains("Body content."));
    }

    #[test]
    fn test_insert_id_no_frontmatter_errors() {
        let content = "# Just a title\n\nNo frontmatter.\n";
        let result = insert_id_into_frontmatter(content, "card_abc12345");
        assert!(result.is_err());
    }
}
