# backend

## Active

- [ ] **Keysight 节点菜单能力类型化（Phase B1）** — Card/Alias/Note/Question/Section 五类节点的 ⋯ 菜单能力做成强类型 catalog：判别联合 + applies_to: Set&lt;NodeKind&gt; + ui_kind (plain/submenu/custom) + 可见性规则 + order。当前 `NodeContextMenu` 的菜单项按 kind 分散硬编码在五个分支里，能力空白（Task 是渲染节点但完全无菜单）和能力重复（Move to Section / Remove from group 散在四份）都看不出来。**B1 scope 纯 TS**：复用现有 Rust command，不动 schema。**B1 真实 UX 新增**：Card/Alias/Section 菜单新增 `Copy UUID + title`（统一 normalized pipeline `UUID:{id} {normalize(title)}`），砍掉 Card 原有纯 `Copy title`。Task 是第 6 个渲染节点但 B1 不接线——NodeKind union 不含 "task"，类型层面预留，接入留给后续独立 task。`set_color` 扩张到 Card/Question/Task 留 B2（Rust schema 变更）。

## Next

- [ ] **Keysight 节点菜单 Phase B2** — 扩 Card/Question/Task 的 `color` schema：`entities` 表 color 列 + migration、`card_set_color` / `question_set_color` / `task_set_color` commands，然后把 catalog 的 `set_color` `applies_to` 扩到全集。依赖 B1 完成。涉及 Rust schema 变更,风险面较大,要独立 task 做。
- [ ] **Keysight Task 节点菜单接入** — 把 Task 作为第 6 个 NodeKind 加到 catalog，接入 `copy_uuid_title` / `draw_connection` / `edit_title` / `move_to_section` / `remove_from_group` / `delete`（复用现有 `task_update` / `task_delete` 等 command）。依赖 B1 完成，是纯 TS 工作。

## Done

### 2026-04-14

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
