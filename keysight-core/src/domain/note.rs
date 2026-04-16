#![allow(dead_code)]
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};

use crate::domain::edge::EntityId;
use crate::errors::KeysightError;
use crate::id;
use crate::models::{GraphNote, NoteFileMigrationReport};
use crate::parser;
use crate::vault_fs::{RealVaultFs, VaultFs};

use super::sync;

/// 笔记存储契约。
pub trait NoteStore {
    fn create(&self, whiteboard_id: &str, title: &str, content: Option<&str>, color: Option<&str>) -> Result<GraphNote, KeysightError>;
    fn delete(&self, id: &str) -> Result<(), KeysightError>;
    fn update(&self, id: &str, title: Option<&str>, content: Option<&str>, color: Option<&str>) -> Result<(), KeysightError>;
    fn get(&self, id: &str) -> Result<GraphNote, KeysightError>;
    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphNote>, KeysightError>;
}

pub struct SqliteNoteStore<'a> {
    conn: &'a Connection,
    vault_fs: Option<&'a dyn VaultFs>,
}

struct NoteSnapshot {
    whiteboard_id: String,
    title: String,
    color: Option<String>,
    file_path: Option<String>,
    note: GraphNote,
}

struct NoteFileState<'a> {
    id: &'a str,
    whiteboard_id: &'a str,
    title: &'a str,
    content: &'a str,
    color: Option<&'a str>,
    linked_card_ids: &'a [String],
    linked_note_ids: &'a [String],
    linked_section_ids: &'a [String],
    linked_question_ids: &'a [String],
    linked_task_ids: &'a [String],
    file_path: Option<&'a str>,
}

struct NoteRenderInputs<'a> {
    id: &'a str,
    title: &'a str,
    content: &'a str,
    color: Option<&'a str>,
    linked_card_ids: &'a [String],
    linked_note_ids: &'a [String],
    linked_section_ids: &'a [String],
    linked_question_ids: &'a [String],
    linked_task_ids: &'a [String],
}

impl<'a> SqliteNoteStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn, vault_fs: None } }

    pub fn with_vault_fs(conn: &'a Connection, vault_fs: &'a dyn VaultFs) -> Self {
        Self {
            conn,
            vault_fs: Some(vault_fs),
        }
    }

    fn require_vault_fs(&self, op: &str) -> Result<&dyn VaultFs, KeysightError> {
        self.vault_fs
            .ok_or_else(|| KeysightError::FileError(format!("{op} 需要 VaultFs")))
    }

    fn current_snapshot(&self, id: &str) -> Result<NoteSnapshot, KeysightError> {
        let row: Option<(String, String, Option<String>, Option<String>)> = self
            .conn
            .query_row(
                "SELECT whiteboard_id, title, color, file_path FROM entities WHERE id = ?1 AND kind = 'note'",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let (whiteboard_id, title, color, file_path) = row.ok_or_else(|| KeysightError::NotFound(id.to_string()))?;
        let note = self.get(id)?;
        Ok(NoteSnapshot {
            whiteboard_id,
            title,
            color,
            file_path,
            note,
        })
    }

    fn rewrite_note_file_from_state(&self, state: NoteFileState<'_>) -> Result<String, KeysightError> {
        let vault_fs = self.require_vault_fs("rewrite_note_file_from_state")?;
        let relative_path = desired_note_relative_path(
            state.whiteboard_id,
            state.id,
            state.title,
            state.file_path,
        );
        let markdown = render_note_markdown(NoteRenderInputs {
            id: state.id,
            title: state.title,
            content: state.content,
            color: state.color,
            linked_card_ids: state.linked_card_ids,
            linked_note_ids: state.linked_note_ids,
            linked_section_ids: state.linked_section_ids,
            linked_question_ids: state.linked_question_ids,
            linked_task_ids: state.linked_task_ids,
        });
        vault_fs.write_file(&relative_path, &markdown)?;
        sync::sync_file(self.conn, &relative_path, &markdown, current_mtime_ms())?;
        if let Some(previous_path) = state.file_path
            && !previous_path.is_empty()
            && previous_path != relative_path
        {
            vault_fs.delete_file(previous_path)?;
            self.conn
                .execute("DELETE FROM file_mtimes WHERE filePath = ?1", [previous_path])?;
        }
        Ok(relative_path)
    }

    pub fn sync_links_to_file(&self, id: &str) -> Result<(), KeysightError> {
        let snapshot = self.current_snapshot(id)?;
        self.rewrite_note_file_from_state(NoteFileState {
            id,
            whiteboard_id: &snapshot.whiteboard_id,
            title: &snapshot.title,
            content: &snapshot.note.content,
            color: snapshot.color.as_deref(),
            linked_card_ids: snapshot.note.linked_card_ids.as_deref().unwrap_or(&[]),
            linked_note_ids: snapshot.note.linked_note_ids.as_deref().unwrap_or(&[]),
            linked_section_ids: snapshot.note.linked_section_ids.as_deref().unwrap_or(&[]),
            linked_question_ids: snapshot.note.linked_question_ids.as_deref().unwrap_or(&[]),
            linked_task_ids: snapshot.note.linked_task_ids.as_deref().unwrap_or(&[]),
            file_path: snapshot.file_path.as_deref(),
        })?;
        Ok(())
    }
}

impl NoteStore for SqliteNoteStore<'_> {
    fn create(&self, whiteboard_id: &str, title: &str, content: Option<&str>, color: Option<&str>) -> Result<GraphNote, KeysightError> {
        let title = validate_title(title)?;
        let note_id = id::gen_note_id();
        let normalized_content = content
            .map(parser::normalize_legacy_toggle_syntax)
            .unwrap_or_default();
        let relative_path = note_relative_path(whiteboard_id, &note_id, title);
        let markdown = render_note_markdown(NoteRenderInputs {
            id: &note_id,
            title,
            content: &normalized_content,
            color,
            linked_card_ids: &[],
            linked_note_ids: &[],
            linked_section_ids: &[],
            linked_question_ids: &[],
            linked_task_ids: &[],
        });

        self.require_vault_fs("create")?
            .write_file(&relative_path, &markdown)?;
        sync::sync_file(self.conn, &relative_path, &markdown, current_mtime_ms())?;
        self.get(&note_id)
    }

    fn delete(&self, id: &str) -> Result<(), KeysightError> {
        let file_path: Option<String> = self
            .conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'note'",
                [id],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(file_path) = file_path.filter(|path| !path.is_empty()) {
            self.require_vault_fs("delete")?.delete_file(&file_path)?;
            sync::remove_file(self.conn, &file_path)?;
            return Ok(());
        }

        self.conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id])?;
        self.conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", [id])?;
        self.conn.execute("DELETE FROM entities WHERE id = ?1", [id])?;
        Ok(())
    }

    fn update(&self, id: &str, title: Option<&str>, content: Option<&str>, color: Option<&str>) -> Result<(), KeysightError> {
        let snapshot = self.current_snapshot(id)?;
        let next_title = match title {
            Some(value) => validate_title(value)?,
            None => snapshot.title.as_str(),
        };
        let next_content = content
            .map(parser::normalize_legacy_toggle_syntax)
            .unwrap_or_else(|| snapshot.note.content.clone());
        let next_color = match color {
            Some("default") => None,
            Some(other) => Some(other),
            None => snapshot.color.as_deref(),
        };

        self.rewrite_note_file_from_state(NoteFileState {
            id,
            whiteboard_id: &snapshot.whiteboard_id,
            title: next_title,
            content: &next_content,
            color: next_color,
            linked_card_ids: snapshot.note.linked_card_ids.as_deref().unwrap_or(&[]),
            linked_note_ids: snapshot.note.linked_note_ids.as_deref().unwrap_or(&[]),
            linked_section_ids: snapshot.note.linked_section_ids.as_deref().unwrap_or(&[]),
            linked_question_ids: snapshot.note.linked_question_ids.as_deref().unwrap_or(&[]),
            linked_task_ids: snapshot.note.linked_task_ids.as_deref().unwrap_or(&[]),
            file_path: snapshot.file_path.as_deref(),
        })?;
        Ok(())
    }

    fn get(&self, id: &str) -> Result<GraphNote, KeysightError> {
        let (title, raw_content, color): (String, String, Option<String>) = self.conn.query_row(
            "SELECT title, COALESCE(content, ''), color FROM entities WHERE id = ?1 AND kind = 'note'",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;
        let content = parser::normalize_legacy_toggle_syntax(&raw_content)
            .trim_end_matches('\n')
            .to_string();

        let mut stmt = self.conn.prepare(
            "SELECT to_id FROM edges WHERE from_id = ?1 AND edge_type = 'note_link'"
        )?;
        let targets: Vec<String> = stmt.query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // 穷尽 match EntityId 所有 6 个 variant —— 禁止 `_` 通配,踩坑样例 1
        // 的新防御核心:任何未知 kind 让编译器 + 运行时同时抛错,而不是 silent drop。
        // 旧 if-else 链只识别 card/alias/sec/note 前缀,把 q_/task_ 目标静默丢弃,
        // 导致 DB 有 orphan edge 但 UI 读不回来(踩坑样例 1 的元凶)。
        let mut linked_card_ids = Vec::new();
        let mut linked_section_ids = Vec::new();
        let mut linked_note_ids = Vec::new();
        let mut linked_question_ids = Vec::new();
        let mut linked_task_ids = Vec::new();
        for target_str in targets {
            let entity_id = EntityId::parse(&target_str).map_err(|e| {
                KeysightError::ParseError(format!(
                    "note_link target 无法 parse 为 EntityId: {target_str} ({e})"
                ))
            })?;
            match entity_id {
                // card 和 alias 历史上合并在 linked_card_ids 里(alias 视觉按 card 渲染)
                EntityId::Card(_) | EntityId::Alias(_) => linked_card_ids.push(target_str),
                EntityId::Section(_) => linked_section_ids.push(target_str),
                EntityId::Note(_) => linked_note_ids.push(target_str),
                EntityId::Question(_) => linked_question_ids.push(target_str),
                EntityId::Task(_) => linked_task_ids.push(target_str),
            }
        }

        Ok(GraphNote {
            id: id.to_string(),
            title,
            content,
            color,
            linked_section_ids: if linked_section_ids.is_empty() { None } else { Some(linked_section_ids) },
            linked_card_ids: if linked_card_ids.is_empty() { None } else { Some(linked_card_ids) },
            linked_note_ids: if linked_note_ids.is_empty() { None } else { Some(linked_note_ids) },
            linked_question_ids: if linked_question_ids.is_empty() { None } else { Some(linked_question_ids) },
            linked_task_ids: if linked_task_ids.is_empty() { None } else { Some(linked_task_ids) },
        })
    }

    fn query_all(&self, whiteboard_id: &str) -> Result<Vec<GraphNote>, KeysightError> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM entities WHERE kind = 'note' AND whiteboard_id = ?1 ORDER BY title"
        )?;
        let ids: Vec<String> = stmt.query_map([whiteboard_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter().map(|id| self.get(id)).collect()
    }
}

/// 一次性把 DB-only note 导出为 `whiteboard/` 下的 markdown 文件，并同时备份 DB 与 whiteboard 目录。
pub fn migrate_db_notes_to_files(
    conn: &Connection,
    db_path: &Path,
    vault_path: &Path,
) -> Result<NoteFileMigrationReport, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM entities WHERE kind = 'note' AND (file_path IS NULL OR file_path = '') ORDER BY id",
    )?;
    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if ids.is_empty() {
        return Ok(NoteFileMigrationReport {
            migrated_notes: 0,
            skipped_notes: 0,
            db_backup_path: String::new(),
            whiteboard_backup_path: String::new(),
        });
    }

    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let db_backup_path = backup_file(db_path, &timestamp)?;
    let whiteboard_backup_path = backup_whiteboard_dir(vault_path, &timestamp)?;

    let fs = RealVaultFs::new(vault_path.to_string_lossy().into_owned());
    let store = SqliteNoteStore::with_vault_fs(conn, &fs);
    let mut migrated_notes = 0u32;
    let mut skipped_notes = 0u32;

    for id in ids {
        match store.sync_links_to_file(&id) {
            Ok(()) => migrated_notes += 1,
            Err(KeysightError::NotFound(_)) => skipped_notes += 1,
            Err(err) => return Err(err),
        }
    }

    Ok(NoteFileMigrationReport {
        migrated_notes,
        skipped_notes,
        db_backup_path,
        whiteboard_backup_path,
    })
}

fn current_mtime_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn whiteboard_relative_dir(whiteboard_id: &str) -> String {
    if whiteboard_id == "wb_root" {
        "whiteboard".to_string()
    } else {
        format!("whiteboard/{whiteboard_id}")
    }
}

fn validate_title(title: &str) -> Result<&str, KeysightError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        Err(KeysightError::EmptyTitle)
    } else {
        Ok(trimmed)
    }
}

fn filename_title_component(text: &str) -> String {
    text.replace("**", "")
}

fn sanitize_file_component(text: &str) -> String {
    let cleaned = filename_title_component(text)
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        "Untitled".to_string()
    } else {
        cleaned
    }
}

fn desired_note_relative_path(
    whiteboard_id: &str,
    note_id: &str,
    title: &str,
    current_file_path: Option<&str>,
) -> String {
    let desired = note_relative_path(whiteboard_id, note_id, title);
    match current_file_path {
        Some(path) if !path.is_empty() => {
            if path == desired {
                path.to_string()
            } else {
                desired
            }
        }
        _ => desired,
    }
}

fn note_relative_path(whiteboard_id: &str, note_id: &str, title: &str) -> String {
    format!(
        "{}/{} 【NOTE】{}.md",
        whiteboard_relative_dir(whiteboard_id),
        note_id,
        sanitize_file_component(title),
    )
}

fn yaml_list(items: &[String]) -> String {
    if items.is_empty() {
        " []".to_string()
    } else {
        format!(
            "\n{}",
            items
                .iter()
                .map(|item| format!("  - {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn render_note_markdown(input: NoteRenderInputs<'_>) -> String {
    let normalized_body = parser::normalize_legacy_toggle_syntax(input.content)
        .trim_end()
        .to_string();
    let mut link_to = input.linked_card_ids.to_vec();
    link_to.extend(input.linked_note_ids.iter().cloned());
    link_to.extend(input.linked_question_ids.iter().cloned());
    link_to.extend(input.linked_task_ids.iter().cloned());

    let mut lines = vec![
        "---".to_string(),
        "type: note".to_string(),
        format!("id: {}", input.id),
        "tags: []".to_string(),
        format!("linkTo:{}", yaml_list(&link_to)),
        "related: []".to_string(),
        format!("see-also:{}", yaml_list(input.linked_section_ids)),
    ];
    if let Some(color) = input.color.filter(|value| !value.is_empty()) {
        if color.starts_with('#') {
            lines.push(format!("color: \"{color}\""));
        } else {
            lines.push(format!("color: {color}"));
        }
    }
    lines.push("---".to_string());
    lines.push(String::new());
    lines.push(format!("# 【NOTE】{}", input.title));
    lines.push(String::new());
    if !normalized_body.is_empty() {
        lines.push(normalized_body);
    }
    lines.push(String::new());
    lines.join("\n")
}

fn backup_file(source: &Path, timestamp: &str) -> Result<String, KeysightError> {
    let backup_path = PathBuf::from(format!(
        "{}.bak-notes-to-files-{}",
        source.display(),
        timestamp
    ));
    std::fs::copy(source, &backup_path)
        .map_err(|e| KeysightError::FileError(format!("备份 DB 失败 {} -> {}: {e}", source.display(), backup_path.display())))?;
    Ok(backup_path.to_string_lossy().into_owned())
}

fn backup_whiteboard_dir(vault_path: &Path, timestamp: &str) -> Result<String, KeysightError> {
    let source = vault_path.join("whiteboard");
    let destination = vault_path.join(format!("whiteboard.bak-notes-to-files-{timestamp}"));

    if destination.exists() {
        std::fs::remove_dir_all(&destination)
            .map_err(|e| KeysightError::FileError(format!("清理旧 whiteboard 备份失败 {}: {e}", destination.display())))?;
    }
    copy_dir_recursive(&source, &destination)?;
    Ok(destination.to_string_lossy().into_owned())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), KeysightError> {
    std::fs::create_dir_all(destination)
        .map_err(|e| KeysightError::FileError(format!("创建目录 {} 失败: {e}", destination.display())))?;

    if !source.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(source)
        .map_err(|e| KeysightError::FileError(format!("读取目录 {} 失败: {e}", source.display())))?
    {
        let entry = entry.map_err(|e| KeysightError::FileError(format!("遍历目录项失败: {e}")))?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).map_err(|e| {
                KeysightError::FileError(format!(
                    "复制文件 {} -> {} 失败: {e}",
                    path.display(),
                    target.display()
                ))
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use crate::vault_fs::MockVaultFs;
    use rusqlite::params;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    fn temp_root(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "super_tauri_note_tests_{}_{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_create_note_writes_markdown_file_and_syncs_db() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);

        let note = store.create("wb_root", "My Note", Some("Content"), Some("yellow")).unwrap();

        assert!(note.id.starts_with("note_"));
        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();
        let file = vfs.get_file(&file_path).unwrap();
        assert!(file.contains("type: note"));
        assert!(file.contains("color: yellow"));
        assert!(file.contains("# 【NOTE】My Note"));
    }

    #[test]
    fn test_delete_note_removes_markdown_file_and_db_rows() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store.create("wb_root", "Del", None, None).unwrap();
        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();

        store.delete(&note.id).unwrap();

        assert!(vfs.get_file(&file_path).is_none());
        assert!(matches!(store.get(&note.id), Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_update_note_rewrites_markdown_file() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store.create("wb_root", "Old", Some("old"), None).unwrap();

        store.update(&note.id, Some("New"), Some("new content"), Some("blue")).unwrap();

        let loaded = store.get(&note.id).unwrap();
        assert_eq!(loaded.title, "New");
        assert_eq!(loaded.content, "new content");
        assert_eq!(loaded.color, Some("blue".to_string()));

        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();
        let file = vfs.get_file(&file_path).unwrap();
        assert!(file.contains("# 【NOTE】New"));
        assert!(file.contains("new content"));
        assert!(file.contains("color: blue"));
    }

    #[test]
    fn test_create_note_sanitizes_filename_without_markdown_bold_or_special_chars() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);

        let note = store
            .create("wb_root", "**My** / Note", Some("Content"), None)
            .unwrap();

        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();
        assert!(file_path.ends_with("【NOTE】My _ Note.md"));
    }

    #[test]
    fn test_update_note_renames_file_when_title_changes() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store.create("wb_root", "Old", Some("body"), None).unwrap();
        let old_file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();

        store
            .update(&note.id, Some("**Renamed** / Note"), None, None)
            .unwrap();

        let new_file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();
        assert_ne!(new_file_path, old_file_path);
        assert!(new_file_path.ends_with("【NOTE】Renamed _ Note.md"));
        assert!(vfs.get_file(&old_file_path).is_none());
        assert!(vfs.get_file(&new_file_path).is_some());
    }

    #[test]
    fn test_update_note_normalizes_legacy_details_summary_to_toggle_syntax() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store.create("wb_root", "Old", Some("old"), None).unwrap();
        store
            .update(
                &note.id,
                None,
                Some("<details>\n<summary>折叠标题</summary>\n\n这里是详细内容\n</details>"),
                None,
            )
            .unwrap();

        let loaded = store.get(&note.id).unwrap();
        assert_eq!(loaded.content, "?>> 折叠标题\n这里是详细内容\n?<<");
    }

    #[test]
    fn test_get_note_normalizes_legacy_details_summary_to_toggle_syntax() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, content) VALUES (?1, 'note', ?2, ?3, ?4)",
            params![
                "note_toggle001",
                "Legacy Toggle",
                "wb_root",
                "<details>\n<summary>折叠标题</summary>\n\n这里是详细内容\n</details>"
            ],
        ).unwrap();

        let store = SqliteNoteStore::new(&conn);
        let loaded = store.get("note_toggle001").unwrap();
        assert_eq!(loaded.content, "?>> 折叠标题\n这里是详细内容\n?<<");
    }

    #[test]
    fn test_query_all_notes() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        store.create("wb_root", "A", None, None).unwrap();
        store.create("wb_root", "B", None, None).unwrap();
        store.create("other", "C", None, None).unwrap();
        let notes = store.query_all("wb_root").unwrap();
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn test_sync_links_to_file_writes_note_link_frontmatter() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store.create("wb_root", "Links", Some("body"), None).unwrap();
        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'note_link')",
            params![note.id, "card_target"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'note_link')",
            params![note.id, "sec_target"],
        )
        .unwrap();

        store.sync_links_to_file(&note.id).unwrap();

        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();
        let file = vfs.get_file(&file_path).unwrap();
        assert!(file.contains("linkTo:\n  - card_target"));
        assert!(file.contains("see-also:\n  - sec_target"));
    }

    #[test]
    fn test_sync_links_to_file_preserves_question_and_task_targets() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store.create("projects/super-tauri", "Links", Some("body"), None).unwrap();
        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'note_link')",
            params![note.id, "q_target"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES (?1, ?2, 'note_link')",
            params![note.id, "task_target"],
        )
        .unwrap();

        store.sync_links_to_file(&note.id).unwrap();

        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();
        let file = vfs.get_file(&file_path).unwrap();
        assert!(file.contains("linkTo:"));
        assert!(file.contains("- q_target"));
        assert!(file.contains("- task_target"));
    }

    #[test]
    fn test_update_note_writes_hex_color_with_quotes() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store.create("wb_root", "Color", Some("body"), None).unwrap();

        store
            .update(&note.id, None, None, Some("#fff8b3"))
            .unwrap();

        let loaded = store.get(&note.id).unwrap();
        assert_eq!(loaded.color, Some("#fff8b3".to_string()));
        let file_path: String = conn
            .query_row("SELECT file_path FROM entities WHERE id = ?1", [&note.id], |r| r.get(0))
            .unwrap();
        let file = vfs.get_file(&file_path).unwrap();
        assert!(file.contains("color: \"#fff8b3\""));
    }

    #[test]
    fn test_migrate_db_notes_to_files_creates_backups_and_files() {
        let root = temp_root("migrate");
        let vault_path = root.join("vault");
        std::fs::create_dir_all(vault_path.join("whiteboard").join("rust")).unwrap();
        std::fs::write(vault_path.join("whiteboard").join("existing.md"), "# existing").unwrap();

        let db_path = root.join("keysight.db");
        let conn = Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, content, color) VALUES ('note_migrate01', 'note', 'Migrated', 'rust', 'body', 'amber')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES ('note_migrate01', 'card_aaa', 'note_link')",
            [],
        )
        .unwrap();

        let report = migrate_db_notes_to_files(&conn, &db_path, &vault_path).unwrap();

        assert_eq!(report.migrated_notes, 1);
        assert!(Path::new(&report.db_backup_path).exists());
        assert!(Path::new(&report.whiteboard_backup_path).exists());

        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = 'note_migrate01'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let abs_note_file = vault_path.join(&file_path);
        assert!(abs_note_file.exists());
        let note_content = std::fs::read_to_string(abs_note_file).unwrap();
        assert!(note_content.contains("# 【NOTE】Migrated"));
        assert!(note_content.contains("color: amber"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_get_note_reads_question_and_task_linked_ids() {
        // Phase A 2b 防火墙验证:reader 穷尽 match EntityId 6 个 variant,
        // Note→Question / Note→Task 目标不再 silent drop(踩坑样例 1 的回归锁)
        use crate::domain::edge::{user_draw_edge, EntityId};
        use crate::domain::entity::{EntityGraph, SqliteEntityGraph};

        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store
            .create("wb_root", "Note With Q/T", Some("body"), None)
            .unwrap();

        let graph = SqliteEntityGraph::new(&conn);
        let to_question = user_draw_edge(
            EntityId::parse(&note.id).unwrap(),
            EntityId::parse("q_abc12345").unwrap(),
        )
        .unwrap();
        let to_task = user_draw_edge(
            EntityId::parse(&note.id).unwrap(),
            EntityId::parse("task_def67890").unwrap(),
        )
        .unwrap();
        graph.connect(&to_question).unwrap();
        graph.connect(&to_task).unwrap();

        let loaded = store.get(&note.id).unwrap();
        assert_eq!(
            loaded.linked_question_ids.clone().unwrap(),
            vec!["q_abc12345".to_string()],
            "Question 目标应进 linked_question_ids"
        );
        assert_eq!(
            loaded.linked_task_ids.clone().unwrap(),
            vec!["task_def67890".to_string()],
            "Task 目标应进 linked_task_ids"
        );
        assert!(loaded.linked_card_ids.is_none());
        assert!(loaded.linked_note_ids.is_none());
        assert!(loaded.linked_section_ids.is_none());
    }

    #[test]
    fn test_get_note_fails_on_unknown_target_prefix() {
        // 防火墙硬线:reader 遇到未知 entity id prefix 必须返 ParseError,
        // 不得 silent drop。模拟 DB 脏数据/未来新 entity kind 的场景。
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteNoteStore::with_vault_fs(&conn, &vfs);
        let note = store
            .create("wb_root", "Note Parse Err", Some("body"), None)
            .unwrap();

        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES (?1, 'xyz_garbage', 'note_link')",
            params![note.id],
        )
        .unwrap();

        let err = store.get(&note.id).unwrap_err();
        assert!(
            matches!(err, KeysightError::ParseError(_)),
            "未知 prefix 应让 reader 返 ParseError 不允许 silent drop,实际: {err:?}"
        );
    }
}
