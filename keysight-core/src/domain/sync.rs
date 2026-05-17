#![allow(dead_code)]
use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::domain::id::WhiteboardId;
use crate::errors::KeysightError;
use crate::id;
use crate::models::{SyncFileResponse, SyncVaultReport};
use crate::parser;
use crate::vault_fs::VaultFs;

/// entity id 的两种来源:沿用文件中已写入的,或本次新生成需 backfill 回写文件。
///
/// 用 enum 表达让"new_id 存在 ↔ needs_backfill=true"成为类型不变式,
/// 避免散落的 `(id, needs_backfill, assigned_id)` 元组组合出非法状态。
enum IdAssignment {
    Existing(String),
    Generated(String),
}

impl IdAssignment {
    fn id(&self) -> &str {
        match self {
            Self::Existing(id) | Self::Generated(id) => id,
        }
    }

    fn needs_backfill(&self) -> bool {
        matches!(self, Self::Generated(_))
    }

    fn assigned(&self) -> String {
        match self {
            Self::Generated(id) => id.clone(),
            Self::Existing(_) => String::new(),
        }
    }
}

/// entities 表 UPSERT 的两种结果。
#[derive(Clone, Copy)]
enum RowOp {
    Inserted,
    Updated,
}

impl RowOp {
    fn inserted_count(self) -> u32 {
        matches!(self, Self::Inserted) as u32
    }

    fn updated_count(self) -> u32 {
        matches!(self, Self::Updated) as u32
    }
}

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
pub fn sync_file(
    conn: &Connection,
    file_path: &str,
    content: &str,
    mtime: f64,
) -> Result<SyncFileResponse, KeysightError> {
    let Some(parsed) = parser::parse_entity(content) else {
        return non_entity_response(conn, file_path, mtime);
    };

    let kind_str = entity_kind_str(&parsed)?;
    let wb_id = derive_whiteboard_id(file_path);
    let id_assignment = resolve_entity_id(&parsed, kind_str);
    let id = id_assignment.id();

    let row_op = upsert_entity_row(conn, id, kind_str, &parsed, &wb_id, file_path)?;
    upsert_kind_specific_fields(conn, id, kind_str, &parsed)?;
    replace_tags(conn, id, &parsed.tags)?;
    replace_edges_for_kind(conn, id, kind_str, &parsed)?;
    refresh_entity_fts(conn, id, &parsed)?;
    record_file_mtime(conn, file_path, mtime)?;

    Ok(SyncFileResponse {
        updated: row_op.updated_count(),
        inserted: row_op.inserted_count(),
        deleted: 0,
        needs_id_backfill: id_assignment.needs_backfill(),
        assigned_id: id_assignment.assigned(),
    })
}

/// 非实体文件路径:只更新 mtime,不动 entities 系表。
fn non_entity_response(
    conn: &Connection,
    file_path: &str,
    mtime: f64,
) -> Result<SyncFileResponse, KeysightError> {
    record_file_mtime(conn, file_path, mtime)?;
    Ok(SyncFileResponse {
        updated: 0,
        inserted: 0,
        deleted: 0,
        needs_id_backfill: false,
        assigned_id: String::new(),
    })
}

/// 从 frontmatter `type` 字段映射到 DB `entities.kind` 列字符串。
fn entity_kind_str(parsed: &parser::ParsedEntity) -> Result<&'static str, KeysightError> {
    match parsed.entity_type.as_str() {
        "atomic-card" => Ok("card"),
        "note" => Ok("note"),
        "project-task" => Ok("task"),
        "question" => Ok("question"),
        other => Err(KeysightError::InvalidEntityType(other.to_string())),
    }
}

/// 决定本次同步使用的 entity id:沿用 frontmatter 已写入的,或按 kind 新生成。
fn resolve_entity_id(parsed: &parser::ParsedEntity, kind_str: &str) -> IdAssignment {
    if let Some(existing) = parsed.id.clone() {
        return IdAssignment::Existing(existing);
    }
    let new_id = match kind_str {
        "card" => id::gen_card_id(),
        "note" => id::gen_note_id(),
        "task" => id::gen_task_id(),
        "question" => id::gen_question_id(),
        _ => unreachable!("entity_kind_str 已穷尽 4 类合法 kind"),
    };
    IdAssignment::Generated(new_id)
}

/// UPSERT entities 主表(共用列:kind/title/whiteboard_id/file_path/content/color)。
fn upsert_entity_row(
    conn: &Connection,
    id: &str,
    kind_str: &str,
    parsed: &parser::ParsedEntity,
    wb_id: &WhiteboardId,
    file_path: &str,
) -> Result<RowOp, KeysightError> {
    let exists = conn
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE id = ?1",
            [id],
            |r| r.get::<_, i64>(0),
        )?
        > 0;

    if exists {
        conn.execute(
            "UPDATE entities SET kind = ?1, title = ?2, whiteboard_id = ?3, file_path = ?4, content = ?5, color = ?6 WHERE id = ?7",
            params![kind_str, parsed.title, wb_id.as_str(), file_path, parsed.content, parsed.color, id],
        )?;
        Ok(RowOp::Updated)
    } else {
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, file_path, content, color) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, kind_str, parsed.title, wb_id.as_str(), file_path, parsed.content, parsed.color],
        )?;
        Ok(RowOp::Inserted)
    }
}

/// 写 kind 特定字段表(card_fields / task_fields / question_fields)。note 无此表。
fn upsert_kind_specific_fields(
    conn: &Connection,
    id: &str,
    kind_str: &str,
    parsed: &parser::ParsedEntity,
) -> Result<(), KeysightError> {
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
    Ok(())
}

/// 全量替换 entity_tags(先 DELETE 再 INSERT,反映 frontmatter 最新状态)。
fn replace_tags(
    conn: &Connection,
    id: &str,
    tags: &[String],
) -> Result<(), KeysightError> {
    conn.execute("DELETE FROM entity_tags WHERE entity_id = ?1", [id])?;
    for tag in tags {
        conn.execute(
            "INSERT OR IGNORE INTO entity_tags (entity_id, tag) VALUES (?1, ?2)",
            params![id, tag],
        )?;
    }
    Ok(())
}

/// 全量替换 edges。kind 决定从 parsed 哪些字段抽出 edge:
/// - note: link_to + see_also → note_link
/// - question: link_to → question_link
/// - card / task: link_to → link_to, related → related, see_also → see_also
fn replace_edges_for_kind(
    conn: &Connection,
    id: &str,
    kind_str: &str,
    parsed: &parser::ParsedEntity,
) -> Result<(), KeysightError> {
    conn.execute("DELETE FROM edges WHERE from_id = ?1", [id])?;
    match kind_str {
        "note" => {
            for target in parsed.link_to.iter().chain(parsed.see_also.iter()) {
                insert_edge(conn, id, target, "note_link")?;
            }
        }
        "question" => {
            for target in &parsed.link_to {
                insert_edge(conn, id, target, "question_link")?;
            }
        }
        _ => {
            for target in &parsed.link_to {
                insert_edge(conn, id, target, "link_to")?;
            }
            for target in &parsed.related {
                insert_edge(conn, id, target, "related")?;
            }
            for target in &parsed.see_also {
                insert_edge(conn, id, target, "see_also")?;
            }
        }
    }
    Ok(())
}

fn insert_edge(
    conn: &Connection,
    from_id: &str,
    to_id: &str,
    edge_type: &str,
) -> Result<(), KeysightError> {
    conn.execute(
        "INSERT OR IGNORE INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, ?3)",
        params![from_id, to_id, edge_type],
    )?;
    Ok(())
}

/// 刷新 FTS 索引(DELETE + INSERT,保持 title/content 与 entities 一致)。
fn refresh_entity_fts(
    conn: &Connection,
    id: &str,
    parsed: &parser::ParsedEntity,
) -> Result<(), KeysightError> {
    conn.execute("DELETE FROM entities_fts WHERE id = ?1", [id])?;
    conn.execute(
        "INSERT INTO entities_fts (id, title, content) VALUES (?1, ?2, ?3)",
        params![id, parsed.title, parsed.content],
    )?;
    Ok(())
}

fn record_file_mtime(
    conn: &Connection,
    file_path: &str,
    mtime: f64,
) -> Result<(), KeysightError> {
    conn.execute(
        "INSERT OR REPLACE INTO file_mtimes (filePath, mtime) VALUES (?1, ?2)",
        params![file_path, mtime],
    )?;
    Ok(())
}

/// 从 file_path 推导 whiteboard_id。
///
/// 规则:
/// - `whiteboard/projects/{name}/...` → `projects/{name}`(reserved 命名空间,
///   用于 task kanban whiteboard,wb_id 是一个二级路径)
/// - `whiteboard/{sub}/...` → `sub`(普通 whiteboard,wb_id 是扁平单级,
///   其中 `sub != "projects"`,因为 `projects` 是 reserved 父目录)
/// - `whiteboard/projects/loose.md`(`projects/` 下直接放文件,没有 project 子目录)
///   → `wb_root`(quarantine 到根,不创建 `projects` 影子 whiteboard)
/// - 其他 → `wb_root`(根白板,文件直接在 `whiteboard/` 下)
///
/// P1-7 修复:之前的实现对 `whiteboard/projects/loose.md` 会返回 `"projects"`,
/// 但 `projects` 本身不是合法 whiteboard(是命名空间),这会创建一个影子 whiteboard
/// 污染枚举结果。现在统一归到 wb_root。
///
/// 走 `new_unchecked`(trusted 派生):路径解析后的 wb_id 字符串保证非空且不会
/// 以 6 种 entity prefix 起头,信任 WhiteboardId invariant。
pub(super) fn derive_whiteboard_id(file_path: &str) -> WhiteboardId {
    let raw = derive_whiteboard_id_str(file_path);
    WhiteboardId::new_unchecked(raw)
}

fn derive_whiteboard_id_str(file_path: &str) -> String {
    let Some(path) = file_path.strip_prefix("whiteboard/") else {
        return "wb_root".to_string();
    };
    // 优先识别 reserved `projects/{name}/...` 作为二级 wb_id
    if let Some(rest) = path.strip_prefix("projects/") {
        if let Some(slash_pos) = rest.find('/') {
            let project_name = &rest[..slash_pos];
            if !project_name.is_empty() {
                return format!("projects/{project_name}");
            }
        }
        // `whiteboard/projects/loose.md` 或 `whiteboard/projects/` —— 没有 project
        // 子目录,禁止创建 `projects` 影子 whiteboard,quarantine 到 wb_root
        return "wb_root".to_string();
    }
    // 其余是普通扁平 whiteboard:取第一个 / 之前的部分作为 wb_id
    if let Some(slash_pos) = path.find('/') {
        let sub_wb = &path[..slash_pos];
        if !sub_wb.is_empty() {
            return sub_wb.to_string();
        }
    }
    "wb_root".to_string()
}

/// 删除文件对应的实体及 mtime 记录。
pub fn remove_file(conn: &Connection, file_path: &str) -> Result<(), KeysightError> {
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
pub fn all_file_mtimes(
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
pub fn insert_id_into_frontmatter(
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

/// 全量增量同步 vault 到 DB。
///
/// ## 执行效果
/// 1. 扫描 whiteboard/ 下所有 .md 文件
/// 2. 对比 DB 中已知的 file_mtimes
/// 3. 同步变更文件（new + changed）
/// 4. 清理孤儿（DB 有但文件不存在）
/// 5. 回写缺少 id 的文件
///
/// ## 幂等性
/// 幂等 — mtime 未变的文件不重复同步
pub fn sync_vault(
    conn: &Connection,
    fs: &dyn VaultFs,
) -> Result<SyncVaultReport, KeysightError> {
    // 1. 扫描文件系统
    let fs_files = fs.list_md_files("whiteboard")?;
    let scanned = fs_files.len() as u32;

    // 2. 查询 DB 已知 mtime
    let db_files = all_file_mtimes(conn)?;
    let mut db_map: HashMap<String, f64> = db_files.into_iter().collect();

    let mut synced = 0u32;
    let mut skipped = 0u32;
    let mut backfilled = 0u32;

    // 3. 遍历文件系统文件
    for (path, mtime) in &fs_files {
        if let Some(db_mtime) = db_map.remove(path)
            && (db_mtime - mtime).abs() < f64::EPSILON
        {
            // mtime 相同 → 跳过
            skipped += 1;
            continue;
        }
        // new 或 changed → 同步
        let content = fs.read_file(path)?;
        let resp = sync_file(conn, path, &content, *mtime)?;
        synced += 1;

        // ID backfill
        if resp.needs_id_backfill {
            let new_content = insert_id_into_frontmatter(&content, &resp.assigned_id)?;
            fs.write_file(path, &new_content)?;
            backfilled += 1;
        }
    }

    // 4. 清理孤儿（db_map 中剩余的且在 whiteboard/ 下的 = 文件已删除）
    // 只清理 whiteboard/ 前缀的文件，避免误删通过 sync_file command 同步的非 whiteboard 文件
    let mut removed = 0u32;
    for orphan_path in db_map.keys() {
        if orphan_path.starts_with("whiteboard/") {
            remove_file(conn, orphan_path)?;
            removed += 1;
        }
    }

    Ok(SyncVaultReport {
        scanned,
        synced,
        removed,
        skipped,
        backfilled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use crate::vault_fs::MockVaultFs;

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

    const NOTE_MD: &str = "\
---
type: note
id: note_test0001
linkTo:
  - card_other001
  - note_other001
see-also:
  - sec_other001
color: amber
---

# 【NOTE】Test Note

Note body.
";

    const QUESTION_MD: &str = "\
---
type: question
id: q_test00001
status: doing
---

# 【QUE】Test Question

Question body.
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
    fn test_sync_note_maps_link_fields_to_note_link_edges_and_color() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/rust/note.md", NOTE_MD, 1000.0).unwrap();

        let edge_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE from_id = 'note_test0001' AND edge_type = 'note_link'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let color: Option<String> = conn
            .query_row(
                "SELECT color FROM entities WHERE id = 'note_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(edge_count, 3);
        assert_eq!(color, Some("amber".to_string()));
    }

    #[test]
    fn test_sync_question_writes_question_fields() {
        let conn = test_conn();
        sync_file(&conn, "whiteboard/rust/question.md", QUESTION_MD, 1000.0).unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM question_fields WHERE entity_id = 'q_test00001'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(status, "doing");
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
        assert_eq!(derive_whiteboard_id("whiteboard/test.md").as_str(), "wb_root");
        // 非 whiteboard 目录也映射到 wb_root
        assert_eq!(derive_whiteboard_id("other/test.md").as_str(), "wb_root");
    }

    #[test]
    fn test_derive_whiteboard_id_sub() {
        assert_eq!(derive_whiteboard_id("whiteboard/myboard/test.md").as_str(), "myboard");
    }

    #[test]
    fn test_derive_whiteboard_id_project() {
        // projects/ 下二级目录作为 project whiteboard id
        assert_eq!(
            derive_whiteboard_id("whiteboard/projects/super-tauri/task_abc 【TASK】Add login.md").as_str(),
            "projects/super-tauri"
        );
        assert_eq!(
            derive_whiteboard_id("whiteboard/projects/agent-slipbox/note_xyz 【NOTE】Thoughts.md").as_str(),
            "projects/agent-slipbox"
        );
    }

    #[test]
    fn test_derive_whiteboard_id_projects_no_subdir_quarantines_to_wb_root() {
        // P1-7 修复后:projects/ 下直接放文件(没有 project 子目录)
        // 不再创建 `projects` 影子 whiteboard,而是 quarantine 到 wb_root。
        // `projects` 是 reserved 父目录命名空间,不是合法 whiteboard。
        assert_eq!(
            derive_whiteboard_id("whiteboard/projects/loose.md").as_str(),
            "wb_root"
        );
        // 空 project 名也一样归 wb_root
        assert_eq!(derive_whiteboard_id("whiteboard/projects/").as_str(), "wb_root");
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

    // --- sync_vault ---

    #[test]
    fn test_sync_vault_empty() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let report = sync_vault(&conn, &fs).unwrap();
        assert_eq!(report.scanned, 0);
        assert_eq!(report.synced, 0);
        assert_eq!(report.removed, 0);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.backfilled, 0);
    }

    #[test]
    fn test_sync_vault_new_files() {
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 1000.0)
            .with_file_and_mtime(
                "whiteboard/b.md",
                "---\ntype: atomic-card\nid: card_bbb00001\n---\n\n# 【ATC】Card B\n\nBody B.\n",
                2000.0,
            );
        let report = sync_vault(&conn, &fs).unwrap();
        assert_eq!(report.scanned, 2);
        assert_eq!(report.synced, 2);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.removed, 0);

        // 验证 DB 中有 2 个实体
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_sync_vault_skips_unchanged() {
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 1000.0);

        // 先同步一次
        sync_vault(&conn, &fs).unwrap();

        // 再同步一次，mtime 不变 → 跳过
        let report = sync_vault(&conn, &fs).unwrap();
        assert_eq!(report.scanned, 1);
        assert_eq!(report.synced, 0);
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn test_sync_vault_resyncs_changed_mtime() {
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 1000.0);

        // 先同步
        sync_vault(&conn, &fs).unwrap();

        // 模拟文件变更：mtime 变了
        let fs2 = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 2000.0);
        let report = sync_vault(&conn, &fs2).unwrap();
        assert_eq!(report.synced, 1);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn test_sync_vault_removes_orphans() {
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 1000.0);
        sync_vault(&conn, &fs).unwrap();

        // 文件消失 → 孤儿清理
        let fs_empty = MockVaultFs::new();
        let report = sync_vault(&conn, &fs_empty).unwrap();
        assert_eq!(report.removed, 1);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_sync_vault_backfills_id() {
        let conn = test_conn();
        let md_no_id = "---\ntype: atomic-card\ntags:\n  - test\n---\n\n# 【ATC】No ID Card\n\nBody.\n";
        let fs = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/noid.md", md_no_id, 1000.0);

        let report = sync_vault(&conn, &fs).unwrap();
        assert_eq!(report.backfilled, 1);

        // 验证文件内容已回写 id
        let updated_content = fs.get_file("whiteboard/noid.md").unwrap();
        assert!(updated_content.contains("id: card_"));
    }

    #[test]
    fn test_sync_vault_mixed_scenario() {
        let conn = test_conn();

        // 初始同步 2 个文件
        let fs1 = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 1000.0)
            .with_file_and_mtime(
                "whiteboard/b.md",
                "---\ntype: atomic-card\nid: card_bbb00001\n---\n\n# 【ATC】Card B\n\nBody B.\n",
                1000.0,
            );
        sync_vault(&conn, &fs1).unwrap();

        // 第二次同步：a.md mtime 变了，b.md 删了，c.md 新增
        let fs2 = MockVaultFs::new()
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 2000.0)
            .with_file_and_mtime(
                "whiteboard/c.md",
                "---\ntype: atomic-card\nid: card_ccc00001\n---\n\n# 【ATC】Card C\n\nBody C.\n",
                1000.0,
            );

        let report = sync_vault(&conn, &fs2).unwrap();
        assert_eq!(report.scanned, 2);
        assert_eq!(report.synced, 2);   // a (changed) + c (new)
        assert_eq!(report.skipped, 0);
        assert_eq!(report.removed, 1);  // b (orphan)
    }

    #[test]
    fn test_sync_vault_ignores_non_whiteboard_files() {
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_file_and_mtime("other/a.md", CARD_MD, 1000.0);

        let report = sync_vault(&conn, &fs).unwrap();
        assert_eq!(report.scanned, 0);
    }
}
