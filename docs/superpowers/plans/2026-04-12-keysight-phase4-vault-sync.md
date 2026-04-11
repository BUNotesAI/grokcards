# Phase 4: Vault 同步管道 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 vault 全量增量同步管道，支持手动触发和启动自动同步，将 `whiteboard/` 目录下的 md 文件单向同步到 SQLite DB。

**Architecture:** 扩展 VaultFs trait 增加 `list_md_files` 方法，新增 `sync_vault` domain 纯函数实现 mtime diff + sync + orphan cleanup + ID backfill 全流程，通过 Tauri command 暴露给前端，启动时 setup 阶段自动调用一次。

**Tech Stack:** Rust, rusqlite, tauri-specta, std::fs（递归目录遍历）

**Spec:** `docs/superpowers/specs/2026-04-12-keysight-phase4-vault-sync.md`

---

## 文件映射

| 操作 | 文件 | 职责 |
|------|------|------|
| 修改 | `src-tauri/src/modules/keysight/vault_fs.rs` | VaultFs trait 新增 `list_md_files`，RealVaultFs/MockVaultFs 实现 |
| 修改 | `src-tauri/src/modules/keysight/models.rs` | 新增 `SyncVaultReport` 类型 |
| 修改 | `src-tauri/src/modules/keysight/domain/sync.rs` | 新增 `sync_vault` + `insert_id_into_frontmatter` 函数及测试 |
| 修改 | `src-tauri/src/modules/keysight/commands.rs` | 新增 `sync_vault` command |
| 修改 | `src-tauri/src/lib.rs` | setup 阶段调用启动同步 + collect_commands 注册 |

---

### Task 1: VaultFs trait 扩展 + MockVaultFs 实现

**Files:**
- 修改: `src-tauri/src/modules/keysight/vault_fs.rs`

- [ ] **Step 1: 在 VaultFs trait 中新增 `list_md_files` 方法**

在 `vault_fs.rs` 的 `trait VaultFs` 中，`write_file` 方法后追加：

```rust
    /// 列出指定子目录下所有 .md 文件及其 mtime（epoch ms）。
    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError>;
```

- [ ] **Step 2: 实现 RealVaultFs::list_md_files**

在 `impl VaultFs for RealVaultFs` 中追加：

```rust
    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError> {
        let base = Path::new(&self.vault_path).join(subdir);
        if !base.exists() {
            return Ok(vec![]);
        }
        let mut result = Vec::new();
        Self::walk_md_files(&base, &Path::new(&self.vault_path), &mut result)?;
        Ok(result)
    }
```

在 `impl RealVaultFs` 中追加递归遍历 helper（`abs_path` 方法后）：

```rust
    /// 递归遍历目录，收集 .md 文件的相对路径和 mtime。
    fn walk_md_files(
        dir: &Path,
        vault_root: &Path,
        result: &mut Vec<(String, f64)>,
    ) -> Result<(), KeysightError> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| KeysightError::FileError(format!("读取目录 {}: {e}", dir.display())))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| KeysightError::FileError(format!("遍历目录项: {e}")))?;
            let path = entry.path();
            if path.is_dir() {
                Self::walk_md_files(&path, vault_root, result)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let relative = path
                    .strip_prefix(vault_root)
                    .map_err(|e| KeysightError::FileError(format!("路径前缀计算失败: {e}")))?
                    .to_string_lossy()
                    .to_string();
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .map_err(|e| KeysightError::FileError(format!("读取 mtime {}: {e}", path.display())))?;
                let epoch_ms = mtime
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
                    * 1000.0;
                result.push((relative, epoch_ms));
            }
        }
        Ok(())
    }
```

- [ ] **Step 3: 扩展 MockVaultFs 支持 mtime**

在 `MockVaultFs` 中追加 `mtimes` 字段和 builder 方法：

```rust
#[cfg(test)]
pub(super) struct MockVaultFs {
    files: std::cell::RefCell<std::collections::HashMap<String, String>>,
    mtimes: std::cell::RefCell<std::collections::HashMap<String, f64>>,
}

#[cfg(test)]
impl MockVaultFs {
    pub fn new() -> Self {
        Self {
            files: std::cell::RefCell::new(std::collections::HashMap::new()),
            mtimes: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.files
            .borrow_mut()
            .insert(path.to_string(), content.to_string());
        self
    }

    /// 设置文件的 mtime（用于 sync_vault 测试）。
    pub fn with_mtime(self, path: &str, mtime: f64) -> Self {
        self.mtimes.borrow_mut().insert(path.to_string(), mtime);
        self
    }

    /// 同时设置文件内容和 mtime（便捷方法）。
    pub fn with_file_and_mtime(self, path: &str, content: &str, mtime: f64) -> Self {
        self.with_file(path, content).with_mtime(path, mtime)
    }

    /// 获取 mock 文件系统中的文件内容（用于测试断言）。
    pub fn get_file(&self, path: &str) -> Option<String> {
        self.files.borrow().get(path).cloned()
    }
}
```

- [ ] **Step 4: 实现 MockVaultFs::list_md_files**

在 `impl VaultFs for MockVaultFs` 中追加：

```rust
    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError> {
        let prefix = format!("{}/", subdir);
        let files = self.files.borrow();
        let mtimes = self.mtimes.borrow();
        let mut result = Vec::new();
        for path in files.keys() {
            if path.starts_with(&prefix) && path.ends_with(".md") {
                let mtime = mtimes.get(path).copied().unwrap_or(1000.0);
                result.push((path.clone(), mtime));
            }
        }
        Ok(result)
    }
```

- [ ] **Step 5: 编译验证**

运行: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
期望: 编译通过，无 warning

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/modules/keysight/vault_fs.rs
git commit -m "feat(keysight): extend VaultFs trait with list_md_files"
```

---

### Task 2: SyncVaultReport 类型

**Files:**
- 修改: `src-tauri/src/modules/keysight/models.rs`

- [ ] **Step 1: 在 models.rs 中新增 SyncVaultReport**

在 `SyncFileResponse` 定义之后追加：

```rust
/// Vault 全量同步结果报告。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncVaultReport {
    /// 文件系统扫描到的 .md 文件总数
    pub scanned: u32,
    /// 实际同步的文件数（new + changed）
    pub synced: u32,
    /// 孤儿清理数（DB 有但文件不存在）
    pub removed: u32,
    /// mtime 未变跳过数
    pub skipped: u32,
    /// ID 回写数
    pub backfilled: u32,
}
```

- [ ] **Step 2: 编译验证**

运行: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
期望: 编译通过（`dead_code` 允许，models.rs 头部已有 `#![allow(dead_code)]`）

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/modules/keysight/models.rs
git commit -m "feat(keysight): add SyncVaultReport type"
```

---

### Task 3: insert_id_into_frontmatter 纯函数（TDD）

**Files:**
- 修改: `src-tauri/src/modules/keysight/domain/sync.rs`

- [ ] **Step 1: 写失败测试**

在 `sync.rs` 底部的 `mod tests` 中追加：

```rust
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
```

- [ ] **Step 2: 运行测试确认 Red**

运行: `cd src-tauri && cargo test --lib modules::keysight::domain::sync::tests::test_insert_id -- -q 2>&1`
期望: 编译失败 — `insert_id_into_frontmatter` 未定义

- [ ] **Step 3: 写最小实现**

在 `sync.rs` 中 `all_file_mtimes` 函数之后、`#[cfg(test)]` 之前追加：

```rust
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
```

- [ ] **Step 4: 运行测试确认 Green**

运行: `cd src-tauri && cargo test --lib modules::keysight::domain::sync::tests::test_insert_id -- -q 2>&1`
期望: 3 passed

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/modules/keysight/domain/sync.rs
git commit -m "feat(keysight): add insert_id_into_frontmatter with TDD"
```

---

### Task 4: sync_vault domain 函数（TDD）

**Files:**
- 修改: `src-tauri/src/modules/keysight/domain/sync.rs`

- [ ] **Step 1: 添加 import**

在 `sync.rs` 顶部的 `use` 区追加：

```rust
use std::collections::HashMap;

use crate::modules::keysight::models::SyncVaultReport;
use crate::modules::keysight::vault_fs::VaultFs;
```

- [ ] **Step 2: 写失败测试 — 空 vault 空 DB**

在 `mod tests` 中追加 import 和测试：

```rust
    use crate::modules::keysight::vault_fs::MockVaultFs;

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
```

- [ ] **Step 3: 运行测试确认 Red**

运行: `cd src-tauri && cargo test --lib modules::keysight::domain::sync::tests::test_sync_vault_empty -- -q 2>&1`
期望: 编译失败 — `sync_vault` 未定义

- [ ] **Step 4: 写 sync_vault 最小实现**

在 `sync.rs` 中 `insert_id_into_frontmatter` 函数之后追加：

```rust
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
pub(in crate::modules::keysight) fn sync_vault(
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
        if let Some(db_mtime) = db_map.remove(path) {
            if (db_mtime - mtime).abs() < f64::EPSILON {
                // mtime 相同 → 跳过
                skipped += 1;
                continue;
            }
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

    // 4. 清理孤儿（db_map 中剩余的 = 文件已删除）
    let mut removed = 0u32;
    for (orphan_path, _) in &db_map {
        remove_file(conn, orphan_path)?;
        removed += 1;
    }

    Ok(SyncVaultReport {
        scanned,
        synced,
        removed,
        skipped,
        backfilled,
    })
}
```

- [ ] **Step 5: 运行测试确认 Green**

运行: `cd src-tauri && cargo test --lib modules::keysight::domain::sync::tests::test_sync_vault_empty -- -q 2>&1`
期望: 1 passed

- [ ] **Step 6: 追加完整测试套件**

在 `mod tests` 中继续追加：

```rust
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
            .with_file_and_mtime("whiteboard/a.md", CARD_MD, 2000.0) // changed
            .with_file_and_mtime(
                "whiteboard/c.md",
                "---\ntype: atomic-card\nid: card_ccc00001\n---\n\n# 【ATC】Card C\n\nBody C.\n",
                1000.0,
            ); // new (b.md 不在了 → orphan)

        let report = sync_vault(&conn, &fs2).unwrap();
        assert_eq!(report.scanned, 2);
        assert_eq!(report.synced, 2);   // a (changed) + c (new)
        assert_eq!(report.skipped, 0);
        assert_eq!(report.removed, 1);  // b (orphan)
    }

    #[test]
    fn test_sync_vault_ignores_non_whiteboard_files() {
        let conn = test_conn();
        // list_md_files("whiteboard") 只返回 whiteboard/ 下的文件
        // 放在其他目录的文件不会出现在 MockVaultFs.list_md_files 结果中
        let fs = MockVaultFs::new()
            .with_file_and_mtime("other/a.md", CARD_MD, 1000.0);

        let report = sync_vault(&conn, &fs).unwrap();
        assert_eq!(report.scanned, 0); // other/ 不在 whiteboard/ 下
    }
```

- [ ] **Step 7: 运行全部 sync_vault 测试确认 Green**

运行: `cd src-tauri && cargo test --lib modules::keysight::domain::sync::tests::test_sync_vault -- -q 2>&1`
期望: 7 passed

- [ ] **Step 8: 运行全部 keysight 测试确认无回归**

运行: `cd src-tauri && cargo test --lib modules::keysight -- -q 2>&1 | tail -3`
期望: 全部通过，无失败

- [ ] **Step 9: 提交**

```bash
git add src-tauri/src/modules/keysight/domain/sync.rs
git commit -m "feat(keysight): add sync_vault with TDD — mtime diff, orphan cleanup, ID backfill"
```

---

### Task 5: Tauri command + 启动同步 + bindings 更新

**Files:**
- 修改: `src-tauri/src/modules/keysight/commands.rs`
- 修改: `src-tauri/src/modules/keysight/mod.rs`
- 修改: `src-tauri/src/lib.rs`

- [ ] **Step 1: 新增 sync_vault command**

在 `commands.rs` 的 `sync_all_file_mtimes` 函数之后追加：

```rust
/// # sync_vault
///
/// ## 前置条件
/// - KEYSIGHT_VAULT_PATH 已设置且目录存在
///
/// ## 执行效果
/// 1. 扫描 whiteboard/ 下所有 .md 文件
/// 2. mtime diff → 同步变更文件到 DB
/// 3. 清理孤儿（DB 有但文件不存在的实体）
/// 4. 回写缺少 id 的文件 frontmatter
///
/// ## 幂等性
/// 幂等 — mtime 未变的文件不重复同步
///
/// ## 关联操作
/// - [`sync_file`] — 单文件同步（内部调用）
/// - [`sync_remove_file`] — 删除文件（内部调用）
#[tauri::command]
#[specta::specta]
pub fn sync_vault(state: State<'_, KeysightState>) -> Result<SyncVaultReport, AppError> {
    let conn = state.db.lock().unwrap();
    let fs = RealVaultFs::new(state.vault_path.to_string_lossy().to_string());
    sync::sync_vault(&conn, &fs).map_err(Into::into)
}
```

`commands.rs` 已有 `use super::vault_fs::RealVaultFs;` 和 `use super::domain::{overview, sync};`，无需新增。

在已有的 `use super::models::{...}` 行中追加 `SyncVaultReport`：

```rust
use super::models::{
    AtomicCard, CardAlias, CardLinksResponse, Edge, EdgeStyle, EdgeType, GraphNote,
    GraphOverviewResponse, GraphSection, ImportSummary, Position, StatsResponse, SyncFileResponse,
    SyncVaultReport, VaultInfoResponse,
};
```

- [ ] **Step 2: 在 keysight mod.rs 新增 startup_sync 公开函数**

`lib.rs` 无法直接访问 `domain::sync` 和 `vault_fs`（它们是 `pub(in crate::modules::keysight)` 可见性）。正确做法：在 `mod.rs` 中暴露一个窄接口供 `lib.rs` 调用，不破坏模块封装。

在 `src-tauri/src/modules/keysight/mod.rs` 的 `init` 函数之后追加：

```rust
/// 启动时同步 vault 到 DB。
///
/// 供 lib.rs setup 阶段调用。内部委托 domain::sync::sync_vault。
pub fn startup_sync(state: &KeysightState) -> Result<models::SyncVaultReport, String> {
    let conn = state.db.lock().unwrap();
    let fs = vault_fs::RealVaultFs::new(state.vault_path.to_string_lossy().to_string());
    domain::sync::sync_vault(&conn, &fs).map_err(|e| e.to_string())
}
```

同时在 `mod.rs` 顶部追加 `use` 引用 `state`（如果未导入）：

确认 `mod.rs` 已有 `pub mod state;`，`KeysightState` 路径为 `state::KeysightState`，需在函数签名中使用完整路径或导入。使用完整路径更清晰：

```rust
pub fn startup_sync(state: &state::KeysightState) -> Result<models::SyncVaultReport, String> {
```

- [ ] **Step 3: 在 lib.rs collect_commands 中注册 sync_vault**

在 `lib.rs` 的 `collect_commands![]` 中，`sync_all_file_mtimes` 行后追加：

```rust
        modules::keysight::commands::sync_vault,
```

- [ ] **Step 4: 在 lib.rs setup 中调用 startup_sync**

在 `lib.rs` 的 `.setup(move |app| { ... })` 闭包中，`app.manage(keysight_state);` 之后、`builder.mount_events(app);` 之前追加：

```rust
            // 启动时全量同步 vault → DB
            let ks = app.state::<modules::keysight::state::KeysightState>();
            match modules::keysight::startup_sync(&ks) {
                Ok(report) => eprintln!(
                    "[keysight] startup sync: scanned={}, synced={}, removed={}, skipped={}, backfilled={}",
                    report.scanned, report.synced, report.removed, report.skipped, report.backfilled
                ),
                Err(e) => eprintln!("[keysight] startup sync failed: {e}"),
            }
```

- [ ] **Step 5: 编译验证**

运行: `cd src-tauri && cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
期望: 编译通过

- [ ] **Step 6: 重新生成 bindings.ts**

运行: `cd src-tauri && cargo test export_bindings 2>&1 | tail -3`
期望: test passed，bindings.ts 更新

- [ ] **Step 7: 验证 bindings.ts 中包含 syncVault**

运行: `grep -n 'syncVault' src/bindings.ts`
期望: 能找到 `syncVault` 函数签名和 `SyncVaultReport` 类型

- [ ] **Step 8: 运行全部测试确认无回归**

运行: `cd src-tauri && cargo test --workspace -- -q 2>&1 | tail -3`
期望: 全部通过

- [ ] **Step 9: 提交**

```bash
git add src-tauri/src/modules/keysight/commands.rs src-tauri/src/modules/keysight/mod.rs src-tauri/src/lib.rs src/bindings.ts
git commit -m "feat(keysight): add sync_vault command + startup sync + bindings"
```

---

### Task 6: 副作用矩阵更新 + 提交前验证

**Files:**
- 修改: `CLAUDE.md`（副作用矩阵）

- [ ] **Step 1: 在 CLAUDE.md 副作用矩阵中登记 sync_vault**

在副作用矩阵表格末尾追加：

```markdown
| `sync_vault` | `entities`, `card_fields`, `task_fields`, `question_fields`, `entity_tags`, `edges`, `entities_fts`, `file_mtimes`, `positions`, `section_members` + vault md 文件 | 全量 DB 同步 + 孤儿清理 + 文件 ID 回写 | domain unit test |
```

- [ ] **Step 2: 提交前全量验证**

```bash
cd src-tauri && cargo clippy --workspace -- -D warnings    # Rust lint
cd src-tauri && cargo test --workspace                      # Rust 测试
pnpm build                                                  # TS 类型检查 + 构建
```

期望: 三项全部通过

- [ ] **Step 3: 提交**

```bash
git add CLAUDE.md
git commit -m "docs(keysight): register sync_vault in side-effect matrix"
```
