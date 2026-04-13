# backend

## Active

（无）

## Next

- [ ] **Keysight 节点菜单 Phase B2** — 扩 Card/Question/Task 的 `color` schema：`entities` 表 color 列 + migration、`card_set_color` / `question_set_color` / `task_set_color` commands，然后把 catalog 的 `set_color` `applies_to` 扩到全集。依赖 B1 完成。涉及 Rust schema 变更,风险面较大,要独立 task 做。
- [ ] **Keysight Task 节点菜单接入** — 把 Task 作为第 6 个 NodeKind 加到 catalog，接入 `copy_uuid_title` / `draw_connection` / `edit_title` / `move_to_section` / `remove_from_group` / `delete`（复用现有 `task_update` / `task_delete` 等 command）。依赖 B1 完成，是纯 TS 工作。

## Done

### 2026-04-14

- [x] **Keysight 节点菜单能力类型化（Phase B1）完成** — 分 2 子阶段落地,共 2 commit:
  - `6a37f25` feat: Phase B1 type sketch — 新建 `src/components/keysight/nodes/NodeCapabilityCatalog.ts`,判别联合 type sketch(NodeKind 5 种 + 10 个 Capability variant + NODE_CAPABILITIES + per-NodeKind handlers via `Pick<NodeCapabilityHandlerMap, ...>` + NodeMenuConfig 判别联合)
  - `29d1fb8` refactor: Phase B1 完成 — NodeContextMenu 从 5 份 kind-dispatched 分支改为单一 catalog 驱动渲染路径(applies_to 过滤 → visibility → order → ui_kind 分派);EntityNode/GraphView 菜单 wiring 改新 shape;GraphView 菜单回调 3 个 per-kind copy 合并为 `onCopyEntityUuidTitle(id, kind)` + 新增 `onSetSectionColor`
  - 防火墙机制:`NodeKind` union 不含 "task"(类型层面不埋技术债)、每个 Capability variant 把 `ui_kind`/`destructive`/`visible` 写死(const 初始化时非法组合编译失败)、`NodeCapabilityHandlerMap` 单一声明源让 per-kind handlers drift 风险集中一处、渲染时 `menu.handlers as Partial<NodeCapabilityHandlerMap>` 是结构子类型 sound cast
  - 真实 UX 新增:Card/Alias/Section 菜单新增 `Copy UUID + title`(统一 normalized pipeline `UUID:{id} {normalizeCardTitleForClipboard(title)}`,把原 Card 专属的 markdown 剥离能力推到其他节点);Section 菜单新增 `Set color`(复用 `section_update` 的 color 参数);`Delete` 标签统一(原 "Delete alias" / "Delete section" → "Delete")
  - 测试计数:TS vitest 209 → 215(+6,含 Card 无 Delete / sections 空态 / Alias/Section copy_uuid_title / Section set_color / Section 不含 draw/move/remove/edit);Rust 不变
  - 后续 task:**Phase B2**(Card/Question/Task color schema 扩张)和 **Task 节点菜单接入** 留在 Next

- [x] **Keysight Edge 判别联合重构（Phase A）完成** — 分 3 子阶段落地，共 4 commit:
  - `cddecc4` refactor: rename `models::Edge` → `models::EdgeRow`（DB 行 DTO，为新判别联合让名字）
  - `90d727a` feat: type sketch — 6 newtype id + `EntityId` + `ObsidianLink` + 8 Edge 变体 + 7 parse 单测
  - `be7ea8b` refactor: 子阶段 2a — `connect` 签名强类型化 + 绷带退场（删除 `resolve_user_drawn_edge_type`）；新 `user_draw_edge` 意图函数 + `entity_relate` 命令；TS GraphView 同步
  - `675f764` feat: 子阶段 2b — reader 穷尽 match（note.rs / alias.rs）+ `GraphNote`/`CardAlias` 扩展 `linked_question_ids` / `linked_task_ids` + `buildEdges.ts` 渲染新链接
  - 防火墙机制:Section/Task 作 from 编译期死;reader 遇未知 prefix 返 `ParseError` 不 silent drop;踩坑样例 1 回归测试锁住 Note/Alias→Q/T 的读取链路
  - 测试计数:Rust 211 lib（原 198 + 13 新,含 user_draw_edge × 6 / reader Q/T × 2 / reader ParseError × 2 / parse × 7 − 4 删除）;TS 209 passed 不变

### 2026-04-13
- [x] Keysight Note↔Note 连线静默失败 bug fix（绷带版） + Question 节点 ⋯ 菜单四层接线（commit d29d6df）
- [x] Keysight Note 改成文件写回链路，并在启动时迁移 DB-only notes 到 `whiteboard/`（含 DB + whiteboard 备份）
- [x] Keysight Question 补齐文件型 create/update/delete 后端与回归测试
- [x] Keysight entity connect/disconnect 写回 card/note 源文件，避免 DB 与 markdown 漂移

### 2026-04-11
- [x] 定义领域模型（entities + value objects）
- [x] 抽取 command handler 业务逻辑到 domain 模块
