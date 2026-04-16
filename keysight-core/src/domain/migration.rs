#![allow(dead_code)]

use rusqlite::{params, Connection};

use crate::errors::KeysightError;
use crate::models::ToggleSyntaxMigrationReport;
use crate::parser;
use crate::vault_fs::VaultFs;

use super::sync;

/// 把 legacy `<details>/<summary>` 批量迁移为 `?>> / ?<<`。
///
/// ## 执行效果
/// 1. 扫描 `whiteboard/` 下全部 markdown 文件
/// 2. 若文件内容含 legacy toggle 语法，则写回为 `?>> / ?<<`
/// 3. 立即同步更新后的文件到 DB/FTS
/// 4. 同时清洗 DB 中非文件型 note（`file_path` 为空）
///
/// ## 不做的事
/// - 不修改没有 legacy 语法的内容
/// - 不自动处理 whiteboard 目录外的 markdown 文件
///
/// ## 幂等性
/// 幂等：再次执行不会产生额外修改。
pub fn migrate_legacy_toggle_syntax(
    conn: &Connection,
    fs: &dyn VaultFs,
) -> Result<ToggleSyntaxMigrationReport, KeysightError> {
    let mut report = ToggleSyntaxMigrationReport {
        files_updated: 0,
        db_notes_updated: 0,
    };

    for (path, mtime) in fs.list_md_files("whiteboard")? {
        let content = fs.read_file(&path)?;
        let normalized = parser::normalize_legacy_toggle_syntax(&content);
        if normalized == content {
            continue;
        }

        fs.write_file(&path, &normalized)?;
        sync::sync_file(conn, &path, &normalized, mtime)?;
        report.files_updated += 1;
    }

    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(content, '') FROM entities WHERE kind = 'note' AND COALESCE(file_path, '') = ''",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);

    for (id, content) in rows {
        let normalized = parser::normalize_legacy_toggle_syntax(&content);
        if normalized == content {
            continue;
        }

        conn.execute(
            "UPDATE entities SET content = ?1 WHERE id = ?2",
            params![normalized, id],
        )?;
        conn.execute(
            "UPDATE entities_fts SET content = ?1 WHERE id = ?2",
            params![normalized, id],
        )?;
        report.db_notes_updated += 1;
    }

    Ok(report)
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

    #[test]
    fn test_migrate_legacy_toggle_syntax_rewrites_markdown_files_and_syncs_db() {
        let conn = test_conn();
        let fs = MockVaultFs::new()
            .with_file_and_mtime(
                "whiteboard/test.md",
                "---\ntype: atomic-card\nid: card_test0001\n---\n\n# 【ATC】Test\n\n<details>\n<summary>折叠标题</summary>\n\n这里是详细内容\n</details>\n",
                1234.0,
            );

        let report = migrate_legacy_toggle_syntax(&conn, &fs).unwrap();
        assert_eq!(report.files_updated, 1);
        assert_eq!(report.db_notes_updated, 0);

        let file = fs.get_file("whiteboard/test.md").unwrap();
        assert!(file.contains("?>> 折叠标题"));
        assert!(file.contains("?<<"));
        assert!(!file.contains("<details>"));

        let content: String = conn
            .query_row(
                "SELECT content FROM entities WHERE id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(content, "?>> 折叠标题\n这里是详细内容\n?<<\n");
    }

    #[test]
    fn test_migrate_legacy_toggle_syntax_rewrites_db_notes_without_files() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, content) VALUES (?1, 'note', ?2, ?3, ?4)",
            params![
                "note_toggle001",
                "Legacy Toggle",
                "wb_root",
                "<details>\n<summary>折叠标题</summary>\n\n这里是详细内容\n</details>"
            ],
        )
        .unwrap();

        let fs = MockVaultFs::new();
        let report = migrate_legacy_toggle_syntax(&conn, &fs).unwrap();
        assert_eq!(report.files_updated, 0);
        assert_eq!(report.db_notes_updated, 1);

        let content: String = conn
            .query_row(
                "SELECT content FROM entities WHERE id = 'note_toggle001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(content, "?>> 折叠标题\n这里是详细内容\n?<<");
    }
}
