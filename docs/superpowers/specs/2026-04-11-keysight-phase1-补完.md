# KeySight Phase 1 补完 — 遗漏 Domain 功能设计

## 动机

Phase 1 的目标是"全部 Rust domain 逻辑"，但实际遗漏了 5 项功能。不能用的东西写出来等于零——必须先补完 Phase 1 再进 Phase 2。

## 架构决策

### 可见性：Option A

Domain 模块的 trait/struct 从 `pub(super)` 改为 `pub(in crate::modules::keysight)`，commands.rs 直接构造 struct 调用方法。Deep Module 的窄接口在 `keysight/mod.rs` 层（只暴露 `pub mod commands` + `pub mod models`），不在 domain 内部。

### CardStore 读写统一

`CardStore` trait 从只读扩展为读写。`SqliteCardStore` 从只持有 `conn` 改为持有 `conn + vault_fs`。写方法通过 VaultFs 操作文件，测试用 MockVaultFs。

---

## 1. CardStore 扩展 — edit_title / edit_body / update_understanding

### 变更

- `SqliteCardStore` struct：`conn: &'a Connection` + `vault_fs: &'a dyn VaultFs`
- 现有只读方法不变（不碰 vault_fs）
- 新增 3 个 trait 方法

### edit_title(id, new_title)

1. 查 `entities.file_path` WHERE id
2. `vault_fs.read_file(file_path)` 读取 markdown
3. 找到 `# ` 开头的 H1 行，替换为 `# 【ATC】{new_title}`
4. `vault_fs.write_file(file_path, updated_content)` 写回
5. `UPDATE entities SET title = ? WHERE id = ?`

### edit_body(id, new_body)

1. 查 `entities.file_path`
2. 读文件
3. 保留 frontmatter + H1 行，替换 H1 之后的所有内容为 new_body
4. 写回文件
5. `UPDATE entities SET content = ? WHERE id = ?`

### update_understanding(id, text)

1. 查 `entities.file_path`
2. 读文件
3. `parser::write_frontmatter(content, FrontmatterUpdate { understanding: Some(text) })` 更新 frontmatter
4. 写回文件
5. `UPDATE card_fields SET understanding = ? WHERE entity_id = ?`

### 测试

- MockVaultFs 预置文件 → 调用 mutation → 断言 DB 更新 + MockVaultFs 文件内容正确
- 不存在的 id → NotFound error
- 空 title → EmptyTitle error

---

## 2. search_cards — FTS5 全文搜索

### Schema 变更

db.rs 的 `SCHEMA_V7_SQL` 末尾追加：

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(id UNINDEXED, title, content);
```

### sync_file 变更

sync_file 中 entity upsert 之后，同步维护 FTS 表：

```sql
DELETE FROM entities_fts WHERE id = ?;
INSERT INTO entities_fts (id, title, content) VALUES (?, ?, ?);
```

和 entity_tags 的 DELETE + INSERT 模式一致。

remove_file 中也需要清理 FTS：

```sql
DELETE FROM entities_fts WHERE id = ?;
```

### CardStore 新增方法

`search(text: &str) -> Result<Vec<AtomicCard>, KeysightError>`

1. 按空格拆词，每个词加双引号，AND 连接
2. 先尝试 `SELECT id FROM entities_fts WHERE entities_fts MATCH ?`
3. 用匹配到的 id 列表调用现有的 `query_by_ids` 组装完整 AtomicCard
4. FTS 无结果时 fallback 到 `SELECT id FROM entities WHERE kind = 'card' AND (title LIKE ? OR content LIKE ?)`

### 测试

- sync 两张卡片 → search 关键词 → 返回匹配的
- search 不存在的词 → 空列表
- search 中文 → FTS5 支持

---

## 3. query_card_links — 单卡片完整链接图谱

### models.rs 新增

```rust
pub struct CardLinksResponse {
    pub link_to: Vec<String>,        // 出边 link_to
    pub related: Vec<String>,        // 出边 related
    pub see_also: Vec<String>,       // 出边 see_also
    pub linked_from: Vec<String>,    // 入边 link_to（谁 link 到我）
    pub related_from: Vec<String>,   // 入边 related（谁和我 related）
    pub see_also_from: Vec<String>,  // 入边 see_also（谁 see_also 我）
}
```

### CardStore 新增方法

`query_links(id: &str) -> Result<CardLinksResponse, KeysightError>`

1. `SELECT to_id, edge_type FROM edges WHERE from_id = ?` → 按 edge_type 分组到 link_to/related/see_also
2. `SELECT from_id, edge_type FROM edges WHERE to_id = ?` → 按 edge_type 分组到 linked_from/related_from/see_also_from

### 测试

- sync 3 张卡片，建立 edges → query_links → 验证出入边正确
- 无边的卡片 → 6 个空列表
- 不存在的 id → NotFound

---

## 4. section_move_to_whiteboard — 跨白板迁移

### SectionStore 新增方法

`move_to_whiteboard(section_id: &str, target_whiteboard_id: &str) -> Result<(), KeysightError>`

单事务执行：

1. 验证 section 存在（`entities WHERE id = ? AND kind = 'section'`）
2. `UPDATE entities SET whiteboard_id = ? WHERE id = section_id`
3. 查 `section_members WHERE section_id` 拿到成员 id 列表
4. 删除成员在旧白板的位置：`DELETE FROM positions WHERE entity_id IN (members) AND whiteboard_id = old_wb`
5. 删除 section 自身在旧白板的位置：`DELETE FROM positions WHERE entity_id = section_id AND whiteboard_id = old_wb`
6. 删除跨白板 section_link edges：查 `edges WHERE from_id = section_id AND edge_type = 'section_link'`，如果 to_id 的 whiteboard_id != target，则删除该 edge（反向同理）

### 测试

- 创建 section + 成员 + positions → move → 验证 whiteboard_id 变了、旧位置清了
- 跨白板 section_link 被清理
- 同白板 section_link 保留
- 不存在的 section → NotFound

---

## 5. query_graph_overview — 图谱总览

### models.rs 新增

```rust
pub struct WhiteboardOverview {
    pub whiteboard_id: String,
    pub cards: u64,
    pub sections: u64,
    pub notes: u64,
    pub aliases: u64,
    pub card_summaries: Vec<CardSummary>,
}

pub struct CardSummary {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub incoming_link_count: u64,
}

pub struct GraphOverviewResponse {
    pub whiteboards: Vec<WhiteboardOverview>,
}
```

### overview.rs 新增函数

`graph_overview(conn: &Connection) -> Result<GraphOverviewResponse, KeysightError>`

1. `SELECT DISTINCT whiteboard_id FROM entities` → 白板列表
2. 每个白板：`SELECT kind, COUNT(*) FROM entities WHERE whiteboard_id = ? GROUP BY kind`
3. 每个白板的卡片：`SELECT id, title, file_path FROM entities WHERE kind = 'card' AND whiteboard_id = ?`
4. 反向引用计数：`SELECT to_id, COUNT(*) FROM edges WHERE edge_type = 'link_to' GROUP BY to_id` → 全局一次查询，按 card 分配

### 测试

- 空 DB → 空 whiteboards 列表
- 多白板多类型实体 → 验证各白板计数正确
- incoming_link_count 正确

---

## 质量要求

所有 5 项遵循项目质量体系：

- **TDD**：先写测试（Red）→ 用户确认 → 实现（Green）→ 用户确认
- **Trait-First**：CardStore/SectionStore 扩展的新方法先加到 trait 定义
- **Anti-Test-Theater**：测试调用真实 domain 函数，用 in-memory SQLite + MockVaultFs
- **Operation Contract**：write 方法写 doc comment 描述副作用
