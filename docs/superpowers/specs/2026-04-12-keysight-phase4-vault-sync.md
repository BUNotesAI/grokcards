# Phase 4: Vault 同步管道

## 概述

将旧项目（Obsidian 插件）的文件同步管道迁移到 Tauri 独立应用。md 文件是 source of truth，单向同步到 SQLite DB。变更来源包括 Obsidian 编辑、Tauri app 行内编辑、CLI/agent 外部写入。

## 范围

### 做

- 扩展 VaultFs trait，新增 `list_md_files` 方法
- `sync_vault` domain 函数：全量增量扫描管道
- mtime-based 增量 diff（复用已有 `all_file_mtimes`）
- ID backfill 回写（`insert_id_into_frontmatter` 纯函数 + `VaultFs.write_file`）
- 孤儿检测（DB 有记录但文件已删除 → `remove_file`）
- `SyncVaultReport` 返回类型（scanned/synced/removed/skipped/backfilled）
- Tauri command `sync_vault` 暴露给前端
- 启动时自动同步（setup 阶段阻塞调用）

### 不做

- ❌ notify 文件监听（手动触发够用，未来按需加）
- ❌ 定时扫描（旧项目 30min 间隔不迁移）
- ❌ modify 事件实时处理
- ❌ 冲突解决（md 是 source of truth，单向覆盖 DB）
- ❌ 前端 Sync 按钮 UI（Phase 6 范围）

## 设计

### 1. VaultFs trait 扩展

```rust
pub(in crate::modules::keysight) trait VaultFs {
    /// 读取 vault 中指定相对路径的文件内容
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError>;

    /// 写入内容到 vault 中指定相对路径的文件
    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError>;

    /// 列出指定子目录下所有 .md 文件及其 mtime
    /// 返回 (相对路径, mtime_epoch_ms) 列表
    /// subdir 示例: "whiteboard"
    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError>;
}
```

**RealVaultFs**：`std::fs::read_dir` 递归遍历 `vault_path/{subdir}/`，过滤 `.md` 扩展名，`metadata().modified()` 转 epoch ms。

**MockVaultFs**：从已有 `RefCell<HashMap<String, String>>` 派生文件列表。mtime 用额外 `HashMap<String, f64>` 或固定值。

### 2. sync_vault 管道

```rust
pub(in crate::modules::keysight) fn sync_vault(
    conn: &Connection,
    fs: &dyn VaultFs,
) -> Result<SyncVaultReport, KeysightError>
```

**执行流程**：

```
1. fs.list_md_files("whiteboard")        → fs_files: Vec<(path, mtime)>
2. sync::all_file_mtimes(conn)           → db_files: Vec<(path, mtime)>
3. 构建 db_map: HashMap<path, mtime> 从 db_files
4. 遍历 fs_files:
   ├ db_map 中不存在       → new，加入 to_sync
   ├ db_map 中存在但 mtime 不同 → changed，加入 to_sync
   └ db_map 中存在且 mtime 相同 → skip
5. 遍历 db_map 中剩余项（fs_files 中不存在的）→ orphan，加入 to_remove
6. 对每个 to_sync:
   ├ content = fs.read_file(path)
   ├ response = sync::sync_file(conn, path, content, mtime)
   └ 如果 response.needs_id_backfill:
     ├ new_content = insert_id_into_frontmatter(content, response.assigned_id)
     └ fs.write_file(path, new_content)
7. 对每个 to_remove:
   └ sync::remove_file(conn, path)
8. 返回 SyncVaultReport
```

### 3. SyncVaultReport

```rust
#[derive(Debug, Serialize, Deserialize, specta::Type)]
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

### 4. insert_id_into_frontmatter

```rust
/// 在 frontmatter 中插入 id 字段
///
/// 找到第一个 `---` 后的换行位置，插入 `id: {assigned_id}\n`。
/// 纯字符串操作，不使用 YAML 解析器，避免重排字段顺序。
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
) -> Result<String, KeysightError>
```

### 5. Tauri command

```rust
#[tauri::command]
#[specta::specta]
pub fn sync_vault(state: State<KeysightState>) -> Result<SyncVaultReport, AppError> {
    let conn = state.db.lock().unwrap();
    let fs = RealVaultFs::new(&state.vault_path);
    domain::sync::sync_vault(&conn, &fs).map_err(Into::into)
}
```

### 6. 启动集成

在 `lib.rs` 的 `setup` 阶段，KeysightState 初始化后立即调用：

```rust
.setup(|app| {
    // ... init KeysightState, init_db ...
    let state = app.state::<KeysightState>();
    let conn = state.db.lock().unwrap();
    let fs = RealVaultFs::new(&state.vault_path);
    let report = domain::sync::sync_vault(&conn, &fs)?;
    log::info!("startup sync: {:?}", report);
    Ok(())
})
```

阻塞同步，app 窗口显示前数据已就绪。145 卡片量级无需异步。

## 测试策略

所有测试使用 MockVaultFs + in-memory SQLite，TDD Red-Green-Refactor。

| 用例 | 验证点 |
|------|--------|
| 空 vault 空 DB | scanned=0, synced=0, removed=0 |
| 3 个新文件，DB 空 | synced=3, skipped=0 |
| 3 个文件已同步，mtime 不变 | skipped=3, synced=0 |
| 1 个文件 mtime 变了 | synced=1, skipped=2 |
| DB 有记录但文件已删除 | removed=1（孤儿清理） |
| 文件无 id → backfill 回写 | backfilled=1, MockVaultFs 中内容更新 |
| insert_id_into_frontmatter | 各种 frontmatter 格式正确插入 |
| 混合场景 | 新增 + 变更 + 删除 + 跳过，Report 各字段正确 |

## 依赖关系

- 复用：`sync::sync_file`、`sync::remove_file`、`sync::all_file_mtimes`（Phase 1 已实现 + 测试）
- 复用：`VaultFs` trait + `RealVaultFs` + `MockVaultFs`（已实现）
- 扩展：VaultFs 新增 `list_md_files`
- 新增：`sync_vault` 函数、`insert_id_into_frontmatter` 函数、`SyncVaultReport` 类型
- 新增：`sync_vault` Tauri command
- 修改：`lib.rs` setup 阶段加启动同步
