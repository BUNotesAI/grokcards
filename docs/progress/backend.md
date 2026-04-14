# backend

## Active

（无）

## Next

- [ ] **Keysight 节点菜单 Phase B2** — 扩 Card/Question/Task 的 `color` schema + Task 的 `edit_title` / `delete` domain:`entities` 表 color 列 + migration、`card_set_color` / `question_set_color` / `task_set_color` commands、`task_update` / `task_delete` domain + commands,然后把 catalog 的 `set_color` / `edit_title` / `delete` applies_to 对应扩到 Task。依赖 B1 + Task 菜单接入完成。涉及 Rust schema 变更,风险面较大,要独立 task 做。

## Done

### 2026-04-14

- [x] **Keysight Task 节点菜单接入(Phase B1 后续)完成** — 分 3 commit 落地,纯 TS 增量扩张:
  - `18ba92a` feat: type sketch — NodeCapabilityCatalog NodeKind union 扩 task;copy_uuid_title / move_to_section / remove_from_group 的 applies_to 加 task;新增 TaskNodeHandlers + NodeMenuConfig.task 变体 + TaskMenuConfig 别名;NodeContextMenu re-export TaskMenuConfig;文件头注释从「Task 预留」改为「Task 已接入(3 项能力)」
  - `2ba46c6` feat: 装配 — TaskNode 加 contextMenu/menuSections/currentSectionId 三个 prop + JSX 渲染 NodeContextMenu;EntityNode 新增 taskMenu useMemo + case "task" 传菜单 prop;GraphView.onCopyEntityUuidTitle 的 case "task" 从 return 改为 data.tasks.find,deps 加 data.tasks
  - `68bde5f` test: NodeContextMenu.test.tsx 新增 variant='task' 描述套(8 case):菜单项 presence / sections 空态 / in-section visibility / copy click / submenu move / remove click / **白名单边界锁死(7 个不该出现的菜单项)**
  - `154ceae` test: harness-check-tests 补缺 — TaskNode.test.tsx +2(contextMenu null/非 null);EntityNode.test.tsx +2(kind=task dispatch + 完整菜单接线链路)
  - 能力范围:Task 菜单只含 **3 项能力**(copy_uuid_title / move_to_section / remove_from_group),其余在 applies_to 中排除:
    - `draw_connection` 被 Edge 判别联合编译期禁止(edge.rs:101-102 无 TaskLink 变体)
    - `edit_title` / `delete` 留给后续 task(后端无 task_update / task_delete)
    - `set_color` 留给 Phase B2(Task 模型无 color 字段)
  - 防火墙机制:NodeKind union 扩后 Task 只能搭配白名单中的 3 项能力,添加 Task 到其他 applies_to 需改 catalog + 新增 handler 字段,编译器强制穷尽;白名单边界测试锁死 UI 渲染路径
  - 测试计数:TS vitest 215 → 227(+12:8 NodeContextMenu task variant + 2 TaskNode prop + 2 EntityNode dispatch);Rust 不变

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
