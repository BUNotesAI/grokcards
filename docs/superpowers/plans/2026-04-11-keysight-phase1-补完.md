# KeySight Phase 1 补完 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **TDD 人工确认关卡**: 每个 🔴 Red / 🟢 Green 关卡处必须停下来等用户确认。不得跳过。

**Goal:** 补完 Phase 1 遗漏的 5 项 domain 功能：card mutations、FTS 搜索、card links 查询、section 跨白板迁移、graph overview。

**Architecture:** 扩展现有 trait（CardStore、SectionStore），新增方法。CardStore struct 改为持有 conn + vault_fs。FTS5 虚拟表加入 schema。所有变更遵循 TDD + Red/Green 人工确认关卡。

**Tech Stack:** Rust, rusqlite (bundled, FTS5), serde_yaml, specta

**源项目参考:** `~/codes/vibe-coding/obsidian-plugin-keysight/keysight-core/src/`

---

## File Structure

```
修改:
  src-tauri/src/modules/keysight/db.rs          — SCHEMA_V7_SQL 追加 FTS5
  src-tauri/src/modules/keysight/models.rs       — 新增 CardLinksResponse, WhiteboardOverview, CardSummary, GraphOverviewResponse
  src-tauri/src/modules/keysight/domain/card.rs  — CardStore 扩展: edit_title, edit_body, update_understanding, search, query_links
  src-tauri/src/modules/keysight/domain/sync.rs  — sync_file/remove_file 追加 FTS 同步
  src-tauri/src/modules/keysight/domain/section.rs — SectionStore 扩展: move_to_whiteboard
  src-tauri/src/modules/keysight/domain/overview.rs — 新增 graph_overview
```

---

## Task 1: models.rs — 新增返回类型

**Files:**
- Modify: `src-tauri/src/modules/keysight/models.rs`

- [ ] **Step 1: 添加 CardLinksResponse**

在 `StatsResponse` 之后添加：

```rust
/// 单卡片完整链接图谱。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardLinksResponse {
    /// 出边 link_to 的目标 id
    pub link_to: Vec<String>,
    /// 出边 related 的目标 id
    pub related: Vec<String>,
    /// 出边 see_also 的目标 id
    pub see_also: Vec<String>,
    /// 入边 link_to（谁 link 到我）
    pub linked_from: Vec<String>,
    /// 入边 related（谁和我 related）
    pub related_from: Vec<String>,
    /// 入边 see_also（谁 see_also 我）
    pub see_also_from: Vec<String>,
}
```

- [ ] **Step 2: 添加 GraphOverviewResponse 相关类型**

```rust
/// 卡片摘要（用于 overview）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardSummary {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub incoming_link_count: u64,
}

/// 单个白板的概览。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WhiteboardOverview {
    pub whiteboard_id: String,
    pub cards: u64,
    pub sections: u64,
    pub notes: u64,
    pub aliases: u64,
    pub card_summaries: Vec<CardSummary>,
}

/// 图谱总览。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GraphOverviewResponse {
    pub whiteboards: Vec<WhiteboardOverview>,
}
```

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -3 && cd ..`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/modules/keysight/models.rs
git commit -m "feat(keysight): add CardLinksResponse + GraphOverviewResponse types"
```

---

## Task 2: db.rs — FTS5 虚拟表 (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/db.rs`

- [ ] **Step 1: 在现有 tests 中添加 FTS5 测试**

在 `db.rs` 的 `#[cfg(test)] mod tests` 中添加：

```rust
    #[test]
    fn test_init_db_creates_fts_table() {
        let conn = test_conn();
        // FTS5 虚拟表可以正常 INSERT 和查询
        conn.execute(
            "INSERT INTO entities_fts (id, title, content) VALUES ('test', 'hello', 'world')",
            [],
        )
        .unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'hello'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::db::tests::test_init_db_creates_fts_table -- --nocapture 2>&1 | tail -10 && cd ..`

Expected: FAIL — `no such table: entities_fts`

**⏸ 等待用户确认 Red**

- [ ] **Step 3: 在 SCHEMA_V7_SQL 索引之后追加 FTS5**

在 `SCHEMA_V7_SQL` 常量的最末尾（索引之后）追加：

```sql

-- 全文搜索
CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(id UNINDEXED, title, content);
```

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::db::tests -- --nocapture 2>&1 | tail -10 && cd ..`

Expected: 6 tests PASS (含新的 FTS test)

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/db.rs
git commit -m "feat(keysight): add FTS5 virtual table to v7 schema"
```

---

## Task 3: sync.rs — FTS 同步 (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/sync.rs`

- [ ] **Step 1: 添加 FTS 同步测试**

在 sync.rs 的 `#[cfg(test)] mod tests` 中添加：

```rust
    #[test]
    fn test_sync_card_writes_fts() {
        let conn = test_conn();
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();

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
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();

        // 用不同内容重新 sync
        let md2 = "---\ntype: atomic-card\nid: card_test0001\n---\n\n# 【ATC】Updated Title\n\nNew body.\n";
        sync_file(&conn, "atomic cards/test.md", md2, 2000.0).unwrap();

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
        sync_file(&conn, "atomic cards/test.md", CARD_MD, 1000.0).unwrap();
        remove_file(&conn, "atomic cards/test.md").unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities_fts WHERE entities_fts MATCH 'Test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::sync::tests::test_sync_card_writes_fts -- --nocapture 2>&1 | tail -10 && cd ..`

Expected: FAIL — FTS 表为空（sync_file 尚未写 FTS）

**⏸ 等待用户确认 Red**

- [ ] **Step 3: sync_file 中追加 FTS 同步**

在 sync_file 函数的 step 8（edges 同步）之后、step 9（file_mtimes）之前，追加：

```rust
    // 8.5 同步 FTS 索引
    conn.execute("DELETE FROM entities_fts WHERE id = ?1", [&id])?;
    conn.execute(
        "INSERT INTO entities_fts (id, title, content) VALUES (?1, ?2, ?3)",
        params![id, parsed.title, parsed.content],
    )?;
```

在 remove_file 函数中，`for id in &ids` 循环体内追加：

```rust
        conn.execute("DELETE FROM entities_fts WHERE entity_id = ?1", [id])?;
```

**注意**: FTS5 表的 rowid 列名取决于建表语句。`entities_fts` 的第一列是 `id`（UNINDEXED），DELETE 时用 `WHERE id = ?1`（不是 `entity_id`）。remove_file 中也应该是 `DELETE FROM entities_fts WHERE id = ?1`。

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::sync::tests -- --nocapture 2>&1 | tail -20 && cd ..`

Expected: 16 tests PASS（13 原有 + 3 新增）

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/sync.rs
git commit -m "feat(keysight): sync FTS5 index in sync_file and remove_file"
```

---

## Task 4: CardStore 重构 — conn + vault_fs (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/card.rs`

这个 task 分两步：先重构 struct（不加新方法），确保现有 10 个测试仍通过；再加写方法。

- [ ] **Step 1: 修改 SqliteCardStore struct 和 trait**

CardStore trait 追加写方法签名 + search + query_links（全部 `todo!()` 实现）。struct 增加 vault_fs 字段。

**trait 新增方法：**

```rust
    /// 编辑卡片标题（同时写回文件）。
    fn edit_title(&self, id: &str, new_title: &str) -> Result<(), KeysightError>;
    /// 编辑卡片正文（同时写回文件）。
    fn edit_body(&self, id: &str, new_body: &str) -> Result<(), KeysightError>;
    /// 更新理解笔记（同时写回文件 frontmatter）。
    fn update_understanding(&self, id: &str, text: &str) -> Result<(), KeysightError>;
    /// 全文搜索卡片。
    fn search(&self, text: &str) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 查询单卡片完整链接图谱。
    fn query_links(&self, id: &str) -> Result<CardLinksResponse, KeysightError>;
```

**struct 改为：**

```rust
pub(super) struct SqliteCardStore<'a> {
    conn: &'a Connection,
    vault_fs: Option<&'a dyn VaultFs>,
}

impl<'a> SqliteCardStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn, vault_fs: None }
    }

    pub fn with_vault_fs(conn: &'a Connection, vault_fs: &'a dyn VaultFs) -> Self {
        Self { conn, vault_fs: Some(vault_fs) }
    }
}
```

用 `Option<&dyn VaultFs>` + 两个构造函数，这样现有的只读测试（用 `new`）不需要改动。写方法在调用时检查 `vault_fs.is_some()`。

新增方法的实现暂时全部 `todo!()`。

- [ ] **Step 2: 确认现有 10 个 card 测试仍通过**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::card::tests -- --nocapture 2>&1 | tail -15 && cd ..`

Expected: 10 PASS（现有测试不受影响）

- [ ] **Step 3: 添加 imports**

card.rs 顶部添加需要的 imports：

```rust
use crate::modules::keysight::models::CardLinksResponse;
use crate::modules::keysight::parser;
use crate::modules::keysight::vault_fs::VaultFs;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/card.rs
git commit -m "refactor(keysight): CardStore struct holds optional vault_fs, add todo!() method stubs"
```

---

## Task 5: edit_title / edit_body / update_understanding (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/card.rs`

- [ ] **Step 1: 写 mutation 测试**

在 card.rs 的 `#[cfg(test)] mod tests` 中添加：

```rust
    use crate::modules::keysight::vault_fs::MockVaultFs;

    const CARD_FILE_CONTENT: &str = "\
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
understanding: 旧理解
source: https://example.com
see-also:
  - card_other003
---

# 【ATC】Test Card

Body content.
";

    fn seed_card_with_file(conn: &Connection) -> MockVaultFs {
        sync::sync_file(conn, "atomic cards/test.md", CARD_FILE_CONTENT, 1000.0).unwrap();
        MockVaultFs::new().with_file("atomic cards/test.md", CARD_FILE_CONTENT)
    }

    #[test]
    fn test_edit_title() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store.edit_title("card_test0001", "New Title").unwrap();

        // DB 更新
        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.title, "New Title");

        // 文件更新 — H1 行应包含新标题
        let file = vfs.get_file("atomic cards/test.md").unwrap();
        assert!(file.contains("# 【ATC】New Title"));
        assert!(!file.contains("# 【ATC】Test Card"));
    }

    #[test]
    fn test_edit_title_empty_rejected() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        let result = store.edit_title("card_test0001", "  ");
        assert!(matches!(result, Err(KeysightError::EmptyTitle)));
    }

    #[test]
    fn test_edit_title_not_found() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        let result = store.edit_title("card_nonexist", "X");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_edit_body() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store.edit_body("card_test0001", "Brand new body.\n").unwrap();

        // DB 更新
        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.content, "Brand new body.\n");

        // 文件更新 — frontmatter 和 H1 保留，body 替换
        let file = vfs.get_file("atomic cards/test.md").unwrap();
        assert!(file.contains("Brand new body."));
        assert!(file.contains("# 【ATC】Test Card")); // H1 保留
        assert!(file.contains("type: atomic-card")); // frontmatter 保留
        assert!(!file.contains("Body content.")); // 旧 body 消失
    }

    #[test]
    fn test_update_understanding() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store.update_understanding("card_test0001", "新的理解").unwrap();

        // DB 更新
        let understanding: String = conn
            .query_row(
                "SELECT understanding FROM card_fields WHERE entity_id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(understanding, "新的理解");

        // 文件更新 — frontmatter 中 understanding 已变
        let file = vfs.get_file("atomic cards/test.md").unwrap();
        assert!(file.contains("新的理解"));
    }
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::card::tests::test_edit_title -- --nocapture 2>&1 | tail -10 && cd ..`

Expected: FAIL — `not yet implemented`

**⏸ 等待用户确认 Red**

- [ ] **Step 3: 实现 edit_title**

```rust
    fn edit_title(&self, id: &str, new_title: &str) -> Result<(), KeysightError> {
        let new_title = new_title.trim();
        if new_title.is_empty() {
            return Err(KeysightError::EmptyTitle);
        }
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("edit_title 需要 VaultFs".to_string())
        })?;

        // 查 file_path
        let file_path: String = self.conn.query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        // 读文件，替换 H1
        let content = vault_fs.read_file(&file_path)?;
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut found = false;
        for line in &mut lines {
            if line.trim().starts_with("# ") {
                *line = format!("# 【ATC】{new_title}");
                found = true;
                break;
            }
        }
        if !found {
            lines.push(format!("# 【ATC】{new_title}"));
        }
        let updated = lines.join("\n") + "\n";
        vault_fs.write_file(&file_path, &updated)?;

        // 更新 DB
        self.conn.execute(
            "UPDATE entities SET title = ?1 WHERE id = ?2",
            rusqlite::params![new_title, id],
        )?;
        Ok(())
    }
```

- [ ] **Step 4: 实现 edit_body**

```rust
    fn edit_body(&self, id: &str, new_body: &str) -> Result<(), KeysightError> {
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("edit_body 需要 VaultFs".to_string())
        })?;

        let file_path: String = self.conn.query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        let content = vault_fs.read_file(&file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        // 找 H1 行的位置
        let h1_idx = lines.iter().position(|l| l.trim().starts_with("# "));
        let prefix = match h1_idx {
            Some(idx) => lines[..=idx].join("\n"),
            None => content.clone(),
        };

        let updated = format!("{prefix}\n\n{new_body}");
        vault_fs.write_file(&file_path, &updated)?;

        self.conn.execute(
            "UPDATE entities SET content = ?1 WHERE id = ?2",
            rusqlite::params![new_body, id],
        )?;
        Ok(())
    }
```

- [ ] **Step 5: 实现 update_understanding**

```rust
    fn update_understanding(&self, id: &str, text: &str) -> Result<(), KeysightError> {
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("update_understanding 需要 VaultFs".to_string())
        })?;

        let file_path: String = self.conn.query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        let content = vault_fs.read_file(&file_path)?;
        let updated = parser::write_frontmatter(&content, parser::FrontmatterUpdate {
            understanding: Some(text.to_string()),
            ..Default::default()
        });
        vault_fs.write_file(&file_path, &updated)?;

        self.conn.execute(
            "UPDATE card_fields SET understanding = ?1 WHERE entity_id = ?2",
            rusqlite::params![text, id],
        )?;
        Ok(())
    }
```

- [ ] **Step 6: Run test → 🟢 Green checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::card::tests -- --nocapture 2>&1 | tail -20 && cd ..`

Expected: 15 tests PASS（10 原有 + 5 新增 mutation 测试）

**⏸ 等待用户确认 Green**

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/card.rs
git commit -m "feat(keysight): implement card mutations (edit_title, edit_body, update_understanding) with TDD"
```

---

## Task 6: search + query_links (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/card.rs`

- [ ] **Step 1: 写 search 和 query_links 测试**

```rust
    #[test]
    fn test_search_by_title() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let results = store.search("Test Card").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "card_test0001");
    }

    #[test]
    fn test_search_by_content() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let results = store.search("Body content").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_no_results() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let results = store.search("nonexistent_xyzzy").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_links() {
        let conn = test_conn();
        seed_card(&conn); // card_test0001 有 link_to/related/see_also 出边

        // 再插一张卡片，link_to card_test0001（产生入边）
        let md = "---\ntype: atomic-card\nid: card_linker1\nlinkTo:\n  - card_test0001\n---\n\n# 【ATC】Linker\n\nBody.\n";
        sync::sync_file(&conn, "atomic cards/linker.md", md, 2000.0).unwrap();

        let store = SqliteCardStore::new(&conn);
        let links = store.query_links("card_test0001").unwrap();

        // 出边
        assert_eq!(links.link_to, vec!["card_other001"]);
        assert_eq!(links.related, vec!["card_other002"]);
        assert_eq!(links.see_also, vec!["card_other003"]);

        // 入边
        assert_eq!(links.linked_from, vec!["card_linker1"]);
    }

    #[test]
    fn test_query_links_not_found() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        let result = store.query_links("card_nonexist");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_query_links_no_edges() {
        let conn = test_conn();
        // 无 edges 的卡片
        let md = "---\ntype: atomic-card\nid: card_lonely1\n---\n\n# 【ATC】Lonely\n\nNo links.\n";
        sync::sync_file(&conn, "atomic cards/lonely.md", md, 1000.0).unwrap();

        let store = SqliteCardStore::new(&conn);
        let links = store.query_links("card_lonely1").unwrap();
        assert!(links.link_to.is_empty());
        assert!(links.linked_from.is_empty());
    }
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::card::tests::test_search_by_title -- --nocapture 2>&1 | tail -10 && cd ..`

Expected: FAIL — `not yet implemented`

**⏸ 等待用户确认 Red**

- [ ] **Step 3: 实现 search**

```rust
    fn search(&self, text: &str) -> Result<Vec<AtomicCard>, KeysightError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // 构造 FTS5 MATCH 查询：按空格拆词，每个加引号，AND 连接
        let words: Vec<String> = text
            .split_whitespace()
            .map(|w| format!("\"{}\"", w.replace('"', "")))
            .collect();
        let fts_query = words.join(" AND ");

        let mut stmt = self.conn.prepare(
            "SELECT id FROM entities_fts WHERE entities_fts MATCH ?1"
        )?;
        let ids: Vec<String> = stmt
            .query_map([&fts_query], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        if !ids.is_empty() {
            return self.query_by_ids(&ids);
        }

        // FTS 无结果 — fallback 到 LIKE
        let like_pattern = format!("%{text}%");
        let rows = query_card_rows(
            self.conn,
            "AND (e.title LIKE ?1 OR e.content LIKE ?1)",
            &[&like_pattern as &dyn rusqlite::types::ToSql],
        )?;
        assemble_cards(self.conn, rows)
    }
```

- [ ] **Step 4: 实现 query_links**

```rust
    fn query_links(&self, id: &str) -> Result<CardLinksResponse, KeysightError> {
        // 验证卡片存在
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get::<_, i64>(0),
        )? > 0;
        if !exists {
            return Err(KeysightError::NotFound(id.to_string()));
        }

        // 出边
        let mut out_stmt = self.conn.prepare(
            "SELECT to_id, edge_type FROM edges WHERE from_id = ?1"
        )?;
        let out_rows = out_stmt.query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;

        let mut link_to = Vec::new();
        let mut related = Vec::new();
        let mut see_also = Vec::new();
        for r in out_rows {
            let (to_id, edge_type) = r?;
            match edge_type.as_str() {
                "link_to" => link_to.push(to_id),
                "related" => related.push(to_id),
                "see_also" => see_also.push(to_id),
                _ => {}
            }
        }

        // 入边
        let mut in_stmt = self.conn.prepare(
            "SELECT from_id, edge_type FROM edges WHERE to_id = ?1"
        )?;
        let in_rows = in_stmt.query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;

        let mut linked_from = Vec::new();
        let mut related_from = Vec::new();
        let mut see_also_from = Vec::new();
        for r in in_rows {
            let (from_id, edge_type) = r?;
            match edge_type.as_str() {
                "link_to" => linked_from.push(from_id),
                "related" => related_from.push(from_id),
                "see_also" => see_also_from.push(from_id),
                _ => {}
            }
        }

        Ok(CardLinksResponse {
            link_to,
            related,
            see_also,
            linked_from,
            related_from,
            see_also_from,
        })
    }
```

- [ ] **Step 5: Run test → 🟢 Green checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::card::tests -- --nocapture 2>&1 | tail -25 && cd ..`

Expected: 21 tests PASS（15 + 6 新增）

**⏸ 等待用户确认 Green**

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/card.rs
git commit -m "feat(keysight): implement search (FTS5 + LIKE fallback) and query_links with TDD"
```

---

## Task 7: section_move_to_whiteboard (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/section.rs`

- [ ] **Step 1: SectionStore trait 追加 move_to_whiteboard + 写测试**

trait 新增：

```rust
    fn move_to_whiteboard(&self, section_id: &str, target_whiteboard_id: &str) -> Result<(), KeysightError>;
```

测试：

```rust
    use crate::modules::keysight::domain::entity::{EntityGraph, SqliteEntityGraph};
    use crate::modules::keysight::models::EdgeType;

    #[test]
    fn test_move_to_whiteboard() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec = store.create("wb_root", "Movable", None).unwrap();
        store.add_member(&sec.id, "card_aaa").unwrap();

        // 设置位置
        conn.execute(
            "INSERT INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, 'wb_root', 10.0, 20.0)",
            [&sec.id],
        ).unwrap();

        store.move_to_whiteboard(&sec.id, "wb_target").unwrap();

        // whiteboard_id 变了
        let loaded = store.get(&sec.id).unwrap();
        assert_eq!(
            conn.query_row("SELECT whiteboard_id FROM entities WHERE id = ?1", [&sec.id], |r| r.get::<_, String>(0)).unwrap(),
            "wb_target"
        );

        // 旧位置被清
        let pos_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM positions WHERE entity_id = ?1 AND whiteboard_id = 'wb_root'",
            [&sec.id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(pos_count, 0);
    }

    #[test]
    fn test_move_cleans_cross_wb_section_links() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let sec1 = store.create("wb_root", "Sec1", None).unwrap();
        let sec2 = store.create("wb_root", "Sec2", None).unwrap();

        // sec1 → sec2 section_link
        let graph = SqliteEntityGraph::new(&conn);
        graph.connect(&sec1.id, &sec2.id, EdgeType::SectionLink, None, None).unwrap();

        // 移 sec1 到另一个白板
        store.move_to_whiteboard(&sec1.id, "wb_other").unwrap();

        // 跨白板 link 被清
        let edges = graph.edges_from(&sec1.id).unwrap();
        assert!(edges.is_empty(), "跨白板 section_link 应被清除");
    }

    #[test]
    fn test_move_nonexistent_section() {
        let conn = test_conn();
        let store = SqliteSectionStore::new(&conn);
        let result = store.move_to_whiteboard("sec_nonexist", "wb_target");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }
```

- [ ] **Step 2: Run test → 🔴 Red checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::section::tests::test_move_to_whiteboard -- --nocapture 2>&1 | tail -10 && cd ..`

Expected: FAIL — `not yet implemented`

**⏸ 等待用户确认 Red**

- [ ] **Step 3: 实现 move_to_whiteboard**

```rust
    fn move_to_whiteboard(&self, section_id: &str, target_whiteboard_id: &str) -> Result<(), KeysightError> {
        // 1. 验证存在并获取旧 whiteboard_id
        let old_wb: String = self.conn.query_row(
            "SELECT whiteboard_id FROM entities WHERE id = ?1 AND kind = 'section'",
            [section_id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(section_id.to_string()),
            other => KeysightError::Database(other),
        })?;

        // 2. 更新 whiteboard_id
        self.conn.execute(
            "UPDATE entities SET whiteboard_id = ?1 WHERE id = ?2",
            params![target_whiteboard_id, section_id],
        )?;

        // 3. 查成员
        let mut stmt = self.conn.prepare("SELECT entity_id FROM section_members WHERE section_id = ?1")?;
        let members: Vec<String> = stmt.query_map([section_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // 4. 清成员旧位置
        for member_id in &members {
            self.conn.execute(
                "DELETE FROM positions WHERE entity_id = ?1 AND whiteboard_id = ?2",
                params![member_id, old_wb],
            )?;
        }

        // 5. 清 section 自身旧位置
        self.conn.execute(
            "DELETE FROM positions WHERE entity_id = ?1 AND whiteboard_id = ?2",
            params![section_id, old_wb],
        )?;

        // 6. 清跨白板 section_link
        let mut edge_stmt = self.conn.prepare(
            "SELECT from_id, to_id FROM edges WHERE (from_id = ?1 OR to_id = ?1) AND edge_type = 'section_link'"
        )?;
        let edge_pairs: Vec<(String, String)> = edge_stmt.query_map([section_id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        for (from_id, to_id) in &edge_pairs {
            let other_id = if from_id == section_id { to_id } else { from_id };
            let other_wb: Result<String, _> = self.conn.query_row(
                "SELECT whiteboard_id FROM entities WHERE id = ?1",
                [other_id],
                |r| r.get(0),
            );
            if let Ok(wb) = other_wb {
                if wb != target_whiteboard_id {
                    self.conn.execute(
                        "DELETE FROM edges WHERE from_id = ?1 AND to_id = ?2 AND edge_type = 'section_link'",
                        params![from_id, to_id],
                    )?;
                }
            }
        }

        Ok(())
    }
```

- [ ] **Step 4: Run test → 🟢 Green checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::section::tests -- --nocapture 2>&1 | tail -15 && cd ..`

Expected: 9 tests PASS（6 原有 + 3 新增）

**⏸ 等待用户确认 Green**

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/section.rs
git commit -m "feat(keysight): implement section_move_to_whiteboard with TDD"
```

---

## Task 8: graph_overview (TDD)

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/overview.rs`

- [ ] **Step 1: 写 graph_overview 测试**

```rust
    use crate::modules::keysight::models::GraphOverviewResponse;

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
        sync::sync_file(&conn, "atomic cards/ov1.md", card_md, 100.0).unwrap();

        let card_md2 = "---\ntype: atomic-card\nid: card_ov_002\n---\n\n# 【ATC】Overview Card 2\n\nBody.\n";
        sync::sync_file(&conn, "atomic cards/ov2.md", card_md2, 200.0).unwrap();

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
```

- [ ] **Step 2: 添加 graph_overview 空实现**

```rust
pub(super) fn graph_overview(_conn: &Connection) -> Result<GraphOverviewResponse, KeysightError> {
    todo!()
}
```

- [ ] **Step 3: Run test → 🔴 Red checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::overview::tests::test_graph_overview_with_data -- --nocapture 2>&1 | tail -10 && cd ..`

Expected: FAIL — `not yet implemented`

**⏸ 等待用户确认 Red**

- [ ] **Step 4: 实现 graph_overview**

```rust
pub(super) fn graph_overview(conn: &Connection) -> Result<GraphOverviewResponse, KeysightError> {
    // 1. 所有白板
    let mut wb_stmt = conn.prepare("SELECT DISTINCT whiteboard_id FROM entities ORDER BY whiteboard_id")?;
    let wb_ids: Vec<String> = wb_stmt.query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // 2. 全局 incoming link count
    let mut link_stmt = conn.prepare(
        "SELECT to_id, COUNT(*) FROM edges WHERE edge_type = 'link_to' GROUP BY to_id"
    )?;
    let link_counts: std::collections::HashMap<String, u64> = link_stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // 3. 每个白板聚合
    let mut whiteboards = Vec::new();
    for wb_id in &wb_ids {
        // 计数
        let mut count_stmt = conn.prepare(
            "SELECT kind, COUNT(*) FROM entities WHERE whiteboard_id = ?1 GROUP BY kind"
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
            "SELECT id, title, COALESCE(file_path, '') FROM entities WHERE kind = 'card' AND whiteboard_id = ?1 ORDER BY title"
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
                CardSummary { id, title, file_path, incoming_link_count }
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
```

- [ ] **Step 5: Run test → 🟢 Green checkpoint**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight::domain::overview::tests -- --nocapture 2>&1 | tail -15 && cd ..`

Expected: 4 tests PASS（2 原有 + 2 新增）

**⏸ 等待用户确认 Green**

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/overview.rs
git commit -m "feat(keysight): implement graph_overview with TDD"
```

---

## Task 9: Full verification

- [ ] **Step 1: 全量 keysight 测试**

Run: `cd src-tauri && cargo test --color=always --lib modules::keysight -- --nocapture 2>&1 | tail -30 && cd ..`

Expected: 所有测试通过（约 110+ tests）

- [ ] **Step 2: 全量 workspace 测试**

Run: `cd src-tauri && cargo test --color=always --workspace --lib 2>&1 | tail -5 && cd ..`

- [ ] **Step 3: Clippy**

Run: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5 && cd ..`

- [ ] **Step 4: TS build**

Run: `cd /Users/alexwang/codes/vibe-coding/super-tauri && pnpm build 2>&1 | tail -5`

- [ ] **Step 5: Commit + update progress**

更新 `docs/progress/keysight.md`：Phase 1 补完标记 Done。

```bash
git add -A && git commit -m "docs(keysight): Phase 1 补完 — all domain functions complete"
```
