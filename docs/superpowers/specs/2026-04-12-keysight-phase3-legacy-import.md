# Phase 3: 一次性旧 DB 导入

## 目标

将旧 Obsidian 插件的 keysight.db（v1 schema）数据导入新 Tauri app 的 keysight.db（v7 Entity Registry schema）。一次性操作，幂等可重跑。

## 旧 DB 位置与 schema

**生产数据库**：`/Users/alexwang/codes/vibe-coding/obsidian-plugin-keysight/keysight.db`（5.6 MB，v1 schema）

**旧 schema 核心表**：

| 表 | 用途 | 行数 |
|---|------|------|
| `insights` | 卡片数据（id, filePath, title, content, tags/linkTo/related/seeAlso 为 JSON 列） | 145 |
| `meta` | 白板图元数据（JSON blobs：sections/positions/notes/aliases per whiteboard） | 16 keys |
| `insight_links` | 派生表（trigger 自动同步，不需要读） | 24 |
| `insight_related` | 派生表（同上） | 83 |
| `insight_tags` | 派生表（同上） | 581 |
| `whiteboard_cards` | 空表，未使用 | 0 |
| `file_mtimes` | 全 vault 文件 mtime，不导入 | 12,110 |

**白板列表**（从 meta key 推导）：

| meta key 后缀 | 新 whiteboard_id | sections | notes | aliases |
|--------------|-----------------|----------|-------|---------|
| 无（root） | `wb_root` | 1 | 3 | 0 |
| `:chentian` | `chentian` | 11 | 72 | 119 |
| `:rust` | `rust` | 20 | 0 | 6 |
| `:rust-examples` | `rust-examples` | 1 | 5 | 0 |

## 数据质量审计结果

### 干净

- 145 cards 全部 `card_` 前缀，无空 ID/title
- filePaths 已是 `whiteboard/` 格式
- insight_links/insight_related 零悬挂引用
- aliases 的 cardId 全部指向有效 insights
- seeAlso 全部为空

### 已知异常（设计时已处理）

1. **sections.cardIds 含混合前缀** — `card_`(145) + `alias_`(125) + `note_`(80)。不是 bug，直接映射到 `section_members`
2. **positions 含非实体 key** — `agent`、`--no-move`、`builtin_*`、白板名。Reader 层过滤，只保留标准前缀
3. **4 条脏 card position** — 浮点混入 ID、多 ID 粘连、已删除 card。校验后跳过
4. **whiteboard_cards 空表** — 忽略

## 方案：Trait 分层（LegacyReader + LegacyImporter）

```
旧 DB ──LegacyReader──→ 中间类型 ──LegacyImporter──→ 新 DB
```

读写解耦，各自独立测试。

## 中间类型

```rust
/// 旧 DB insights 表的一行
struct LegacyInsight {
    id: String,           // "card_xxxxxxxx"
    file_path: String,
    title: String,
    content: String,
    tags: Vec<String>,    // 从 JSON 数组解析
    link_to: Vec<String>, // card IDs
    related: Vec<String>, // card IDs
    see_also: Vec<String>,// card IDs（实际全空）
    understanding: String,
    source: String,
    mtime: f64,
}

/// 旧 DB meta 表中的 graph_sections JSON 元素
struct LegacySection {
    id: String,                     // "sec_xxxxxxxx"
    title: String,
    color: Option<String>,
    card_ids: Vec<String>,          // 实际含 card_/alias_/note_ 混合
    linked_section_ids: Vec<String>,
}

/// 旧 DB meta 表中的 graph_notes JSON 元素
struct LegacyNote {
    id: String,
    title: String,
    content: String,
    linked_card_ids: Vec<String>,   // 可含 alias_ 前缀
    linked_note_ids: Vec<String>,
    linked_section_ids: Vec<String>,
}

/// 旧 DB meta 表中的 graph_aliases JSON 元素
struct LegacyAlias {
    alias_id: String,
    card_id: String,                // 指向的原始 card
    linked_card_ids: Vec<String>,   // alias 自己的连线目标
    linked_section_ids: Vec<String>,
    incoming_card_ids: Vec<String>, // 指向这个 alias 的 card
}

/// 旧 DB meta 表中的 graph_positions JSON 对象的一个 entry
struct LegacyPosition {
    entity_id: String,  // 已过滤，只有标准前缀
    x: f64,
    y: f64,
}
```

中间类型不做校验、不做 ID 映射 — 那是 Importer 的职责。

## LegacyReader trait

```rust
trait LegacyReader {
    fn read_insights(&self) -> Result<Vec<LegacyInsight>, KeysightError>;
    fn read_sections(&self, whiteboard_key: &str) -> Result<Vec<LegacySection>, KeysightError>;
    fn read_notes(&self, whiteboard_key: &str) -> Result<Vec<LegacyNote>, KeysightError>;
    fn read_aliases(&self, whiteboard_key: &str) -> Result<Vec<LegacyAlias>, KeysightError>;
    fn read_positions(&self, whiteboard_key: &str) -> Result<Vec<LegacyPosition>, KeysightError>;
    fn list_whiteboards(&self) -> Result<Vec<WhiteboardMapping>, KeysightError>;
}

struct WhiteboardMapping {
    meta_suffix: Option<String>,  // None = root, Some("chentian") = sub
    whiteboard_id: String,        // "wb_root" 或 "chentian"
}
```

实现：`SqliteLegacyReader` 持有 `&Connection`（只读连接到旧 DB）。

positions 过滤在 Reader 层 — 只保留 `card_`/`sec_`/`note_`/`alias_` 前缀。

## LegacyImporter trait

```rust
trait LegacyImporter {
    fn import(&self, reader: &dyn LegacyReader) -> Result<ImportSummary, KeysightError>;
}

struct ImportSummary {
    cards: usize,
    sections: usize,
    notes: usize,
    aliases: usize,
    edges: usize,
    positions: usize,
    section_members: usize,
    skipped: Vec<SkippedItem>,
}

struct SkippedItem {
    entity_id: String,
    reason: String,
}
```

实现：`SqliteLegacyImporter` 持有 `&Connection`（新 DB 连接）。

### 导入流程（单事务内）

```
1. 备份旧 DB 文件（事务外，cp → .bak-import-{timestamp}）
2. BEGIN TRANSACTION
3. 导入 cards — insights → entities + card_fields + entity_tags + edges
4. list_whiteboards → 逐白板导入：
   4a. sections → entities + section_members + edges (section_link)
   4b. notes → entities + edges (note_link)
   4c. aliases → entities + alias_fields + edges (alias_link + card_to_alias)
   4d. positions → positions 表
5. 同步 FTS — 每条 card INSERT INTO entities_fts
6. COMMIT
7. 返回 ImportSummary
```

**顺序约束**：cards 先 → sections → notes/aliases → positions。

**幂等**：全部 `INSERT OR REPLACE`。

**跳过策略**：单条解析失败 / 悬挂引用 → 跳过 + 记录到 `skipped`，不中断。

**不导入**：`file_mtimes`（新 app 自行扫描）、`insights_fts`（新 DB 有 `entities_fts`）。

## 数据映射规则

### insights → entities + card_fields + entity_tags + edges

| 旧字段 | 新表.新字段 |
|--------|-----------|
| id | entities.id |
| filePath | entities.file_path |
| title | entities.title |
| content | entities.content |
| — | entities.kind = 'card' |
| — | entities.whiteboard_id = derive_whiteboard_id(filePath) |
| understanding | card_fields.understanding |
| source | card_fields.source |
| tags[] | entity_tags 行 |
| linkTo[] | edges (edge_type = 'link_to') |
| related[] | edges (edge_type = 'related') |
| seeAlso[] | edges (edge_type = 'see_also') |

### meta graph_sections → entities + section_members + edges

| 旧字段 | 新表.新字段 |
|--------|-----------|
| id | entities.id |
| title | entities.title |
| color | entities.color |
| — | entities.kind = 'section' |
| — | entities.whiteboard_id = WhiteboardMapping.whiteboard_id |
| cardIds[] | section_members 行（entity_id = 数组元素） |
| linkedSectionIds[] | edges (edge_type = 'section_link') |

### meta graph_notes → entities + edges

| 旧字段 | 新表.新字段 |
|--------|-----------|
| id | entities.id |
| title | entities.title |
| content | entities.content |
| — | entities.kind = 'note' |
| — | entities.whiteboard_id = WhiteboardMapping.whiteboard_id |
| linkedCardIds[] | edges (edge_type = 'note_link') |
| linkedNoteIds[] | edges (edge_type = 'note_link') |
| linkedSectionIds[] | edges (edge_type = 'note_link') |

### meta graph_aliases → entities + alias_fields + edges

| 旧字段 | 新表.新字段 |
|--------|-----------|
| aliasId | entities.id |
| cardId | entities.title (alias 的 title 就是它指向的 card_id) + alias_fields.card_id |
| — | entities.kind = 'alias' |
| — | entities.whiteboard_id = WhiteboardMapping.whiteboard_id |
| linkedCardIds[] | edges (edge_type = 'alias_link') |
| linkedSectionIds[] | edges (edge_type = 'alias_link') |
| incomingCardIds[] | edges (edge_type = 'card_to_alias', from = incoming, to = aliasId) |

### meta graph_positions → positions

| 旧字段 | 新表.新字段 |
|--------|-----------|
| key (entity_id) | positions.entity_id |
| x | positions.x |
| y | positions.y |
| — | positions.whiteboard_id = WhiteboardMapping.whiteboard_id |

## Tauri command

```rust
/// # import_legacy_db
///
/// ## 前置条件
/// - old_db_path 指向旧 Obsidian 插件的 keysight.db（v1 schema）
/// - 文件必须存在且可读
///
/// ## 执行效果
/// 1. 备份旧 DB（cp → {old_db_path}.bak-import-{timestamp}）
/// 2. 只读打开旧 DB
/// 3. 单事务写入新 DB（INSERT OR REPLACE）
/// 4. 同步 FTS 索引
///
/// ## 幂等性
/// 可重复执行，结果一致
#[tauri::command]
#[specta::specta]
fn import_legacy_db(
    state: State<KeysightState>,
    old_db_path: String,
) -> Result<ImportSummary, AppError>
```

## DB 位置迁移

`keysight.db` 从相对路径迁移到 `app_data_dir`：

```rust
fn init_keysight_state(app: &tauri::App) -> KeysightState {
    let data_dir = app.path().app_data_dir().expect("无法获取 app_data_dir");
    std::fs::create_dir_all(&data_dir).expect("无法创建 app_data_dir");
    let db_path = data_dir.join("keysight.db");
    // ...
}
```

`KeysightState` 新增 `db_path: PathBuf` 字段。

## 模块结构

```
src-tauri/src/modules/keysight/
├── domain/
│   ├── mod.rs              # 新增 pub(super) mod legacy_import;
│   ├── legacy_import.rs    # LegacyReader + LegacyImporter traits + SqliteXxx impls
│   └── ...
├── models.rs               # 新增中间类型 + ImportSummary + SkippedItem
├── commands.rs             # 新增 import_legacy_db
├── state.rs                # 新增 db_path: PathBuf
└── ...
```

`legacy_import.rs` 预估 ~400 行。超过 500 行再拆。

## 对现有代码的改动

| 文件 | 改动 |
|------|------|
| `state.rs` | 新增 `db_path: PathBuf` 字段 |
| `lib.rs` | `init_keysight_state` 改用 `app_data_dir`，接收 `&tauri::App` |
| `lib.rs` | `collect_commands![]` 新增 `import_legacy_db` |
| `domain/mod.rs` | 新增 `pub(super) mod legacy_import;` |
| `models.rs` | 新增中间类型 + ImportSummary + SkippedItem |
| `commands.rs` | 新增 `import_legacy_db` 薄壳 |

## 附带修正：测试 fixtures 路径

现有测试中 ~30 处用 `"atomic cards/test.md"` 作为文件路径。改为 `"whiteboard/test.md"`（wb_root 场景）。`derive_whiteboard_id` 生产代码不变。

## 测试策略

### Reader 测试

| 测试 | 验证 |
|------|------|
| `read_insights_basic` | 2 条 insight 字段映射，JSON → Vec |
| `read_insights_empty_json` | `[]` → 空 Vec |
| `read_sections_mixed_members` | cardIds 含 card_/alias_/note_ |
| `read_positions_filters_junk` | 过滤 agent/builtin_/白板名 |
| `read_notes_with_links` | linkedCardIds/linkedNoteIds 解析 |
| `list_whiteboards` | root + 子白板 mapping |

### Importer 测试

| 测试 | 验证 |
|------|------|
| `import_cards_entities_and_fields` | entities + card_fields 写入 |
| `import_cards_tags` | entity_tags 写入 |
| `import_cards_edges` | linkTo/related/seeAlso → edges |
| `import_sections_with_members` | entities + section_members |
| `import_sections_linked` | linkedSectionIds → edges |
| `import_notes_with_links` | entities + edges (note_link) |
| `import_aliases` | entities + alias_fields + edges |
| `import_positions` | positions 表写入，whiteboard_id 正确 |
| `import_fts_sync` | entities_fts 可搜 |
| `import_skips_dangling_edge` | 悬挂 edge → skipped |
| `import_idempotent` | 两次执行结果一致 |

测试辅助：`legacy_test_conn()` 建 v1 schema in-memory，`LEGACY_SCHEMA_V1_SQL` 常量（无 triggers）。

## 副作用矩阵新增

| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|--------|-------------|--------|---------|
| `import_legacy_db` | entities, card_fields, alias_fields, entity_tags, edges, positions, section_members, entities_fts + 旧 DB 文件备份 | DB 批量写 + 文件 copy | domain unit test |
