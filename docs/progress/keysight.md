# keysight

KeySight 功能从 Obsidian 插件迁移到 Tauri 独立应用。

旧项目：`~/codes/vibe-coding/obsidian-plugin-keysight`（Obsidian 插件 + keysight-core Rust sidecar）

## 旧项目功能全景

迁移进度参照。旧项目完整功能映射到新项目各 Phase。

### 已迁移（Phase 1-3）

| 旧项目功能 | 新项目对应 | Phase |
|------------|-----------|-------|
| keysight-core 全部 domain 逻辑 | `modules/keysight/domain/` 10 个子模块 | 1 |
| RPC dispatch 40+ 操作 | 40 个 `#[tauri::command]` + specta 绑定 | 2 |
| SQLite schema v7 + WAL | `db.rs` init_db | 1 |
| Markdown/YAML frontmatter 解析 | `parser.rs` | 1 |
| 实体类型：card/note/section/alias/task/question | `models.rs` + 各 domain 模块 | 1 |
| Edge 系统（LinkTo/Related/SeeAlso） | `domain/entity.rs` EntityGraph trait | 1 |
| FTS5 全文搜索 | `domain/card.rs` search 方法 | 1 |
| 旧 DB 数据导入 | `domain/legacy_import.rs` LegacyReader/Importer | 3 |
| ts-rs 类型生成 | tauri-specta 替代（编译期类型安全） | 2 |

### 待迁移

| 旧项目功能 | 要点 | 目标 Phase |
|------------|------|-----------|
| **文件同步管道** | vault 事件监听 → mtime diff → sync_file/remove_file → 通知 UI | 4 |
| ├ mtime-based 增量扫描 | `fileMtimeCache` + `all_file_mtimes()` 对比 | 4 |
| ├ ID backfill 回写 | sync_file 返回 `needs_id_backfill` → 写回 frontmatter（旧项目不监听 modify 事件避免循环） | 4 |
| ├ 孤儿检测 | DB 中有但文件系统不存在的实体 → 清理 | 4 |
| ├ 救援扫描 | 有 mtime 但 DB 中无对应实体 → 重新同步 | 4 |
| └ 定时全量扫描 | 旧项目 30min 间隔，可能改为按需 | 4 |
| **GraphView 白板** | 3500+ 行 React 组件，纯 CSS transform 渲染 | 5 |
| ├ 画布渲染 | translate + scale CSS transform，绝对定位节点 | 5 |
| ├ 拖拽定位 | mouseDown/Move/Up，4px 阈值区分 click/drag | 5 |
| ├ Zoom/Pan | 0.05x-3x 缩放，Cmd+wheel 缩放，普通 scroll 平移 | 5 |
| ├ LOD 分级 | >0.4 完整 / 0.1-0.4 精简 / <0.1 最小 | 5 |
| ├ Quadtree 视口裁剪 | 空间索引 + 300px buffer，只渲染可见节点 | 5 |
| ├ 多白板切换 | folder-based 白板模型，Root 画布 + 子白板预览卡 | 5 |
| ├ Section 分组渲染 | 7 色背景，成员位置计算 bounds，拖动联动 | 5 |
| ├ Edge 渲染 | 卡片中心连线 + clipToRect 矩形裁剪，跨白板 edge 替换 | 5 |
| ├ 视口持久化 | localStorage per whiteboard，500ms debounce | 5 |
| └ 自动布局 | 无保存位置的新卡片网格排列（5列） | 5 |
| **侧边栏 UI** | | 6 |
| ├ Follow 模式 | 跟随当前活动文件显示对应卡片 | 6 |
| ├ Cards 列表 | 可搜索+可筛选列表，folder/tag 过滤 | 6 |
| ├ Review 模式 | 随机卡片复习（闪卡风格） | 6 |
| ├ InsightCard 详情 | understanding 编辑、links、see-also | 6 |
| ├ 行内编辑 | 双击标题/正文行内编辑 + markdown 快捷键 | 6 |
| ├ ContextPanel | 展开的卡片详情面板 | 6 |
| ├ FilterBar | folder + tag pill 过滤器 | 6 |
| ├ ExportPanel | 导出为 Markdown/JSON | 6 |
| └ NoteEditor | 笔记编辑器 + markdown 渲染 | 6 |
| **架构替换** | | 各 Phase |
| ├ RPC → Tauri IPC | Unix socket JSON-RPC → `#[tauri::command]` | ✅ 2 |
| ├ Obsidian vault API → notify crate | `vault.on("create/delete/rename")` → Rust `notify` | 4 |
| ├ Obsidian markdown 渲染 → remark/rehype | `RenderedMarkdown.tsx` 需要独立渲染器 | 6 |
| └ Sidecar 进程 → Tauri 内嵌 | 不再需要子进程管理 | ✅ 1 |

## Active

## Next
- [ ] Phase 5c: 交互（拖拽定位 + context menu + containment 检测）
- [ ] Phase 5d: Edge 渲染（实体间连线 + clipToRect）
- [ ] Phase 5e: 多白板（白板切换 + Root 画布预览卡 + viewport 持久化）— 用户反馈：根白板应显示 sub-whiteboard 预览卡而非直接显示卡片，参照 Obsidian 旧版行为
- [ ] Phase 5f: 性能（Quadtree 视口裁剪 + LOD 分级）
- [ ] Phase 6: 侧边栏 UI（Follow 模式 + Cards 列表 + Review + 行内编辑 + FilterBar + ExportPanel）

## Done

- [x] Phase 5b: 实体渲染 — 6 种实体节点 + GraphToolbar + TanStack Query + 视口裁剪, 47 新 TS tests (56 total), 4 新 Rust tests (160 total), 8 commits
- [x] Phase 5a: 画布基础设施 — useViewport hook + GraphCanvas CSS transform + pan/zoom + 键盘快捷键 + 9 TS tests, 手动验证通过
- [x] Phase 4: Vault 同步管道 — VaultFs.list_md_files + sync_vault + insert_id_into_frontmatter + 启动同步 + 11 tests (156 total), Code Review passed
- [x] Phase 3: 一次性旧 DB 导入 — LegacyReader/Importer traits, 23 tests, app_data_dir 迁移, 真实数据验证 (145 cards, 0 skipped)
- [x] Phase 2: Tauri commands + specta 绑定 — 40 commands, KeysightState, Operation Contracts, 副作用矩阵

### 2026-04-11

- [x] Phase 1 补完: card mutations, FTS5 搜索, card links, section 跨白板迁移, graph overview（+20 tests, 104 total）
- [x] Phase 1: Rust domain 移植（102 tests, 10 domain modules, TDD + Trait-First）
