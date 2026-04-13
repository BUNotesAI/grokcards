---
area: backend
last_updated: 2026-04-14T03:00:00+08:00
session_id: current
status: done
stale_check: pnpm test -- --run 2>&1 | tail -5
---

# Handoff: backend

## 状态:无活跃 task

Phase B1(Keysight 节点菜单能力类型化)已于本 session 全部落地,共 2 commit,无工作树残留(除常规文档追加)。Active 为空,下一 session 可以从 `docs/progress/backend.md` > Next 挑一个开始。

## Phase B1 完成时的成果摘要

| 子阶段 | Commit | 改动要点 |
|---|---|---|
| 2 — type sketch | `6a37f25` | 新建 `NodeCapabilityCatalog.ts` 判别联合骨架:NodeKind 5 种 + 10 个 Capability variant + NODE_CAPABILITIES 静态 const + per-NodeKind handlers(Pick)+ NodeMenuConfig 判别联合 + 防火墙类型约束 |
| 3 — 接线 | `29d1fb8` | NodeContextMenu 从 5 份 kind-dispatched 分支改为 catalog 单一渲染路径;EntityNode/GraphView 菜单 wiring 改新 shape;GraphView 3 个 per-kind copy 合并为 `onCopyEntityUuidTitle(id, kind)` + 新增 `onSetSectionColor`;测试文件重写 shape + 6 个新 case |

**防火墙落地点**:
- `NodeKind` union 不含 "task",传 `{kind: "task"}` 编译失败(Task 预留在类型层外)
- 每个 Capability variant 把 `ui_kind`/`destructive`/`visible` 写死,const 初始化非法组合编译失败(如 `set_color.ui_kind` 只能 "custom")
- `NodeCapabilityHandlerMap` 单一声明源,per-NodeKind handlers 用 Pick 裁剪
- 渲染时 `menu.handlers as Partial<NodeCapabilityHandlerMap>` — 结构子类型合法(Pick 子集 → Partial 全集)

**真实 UX 新增**:
- Card/Alias/Section 菜单新增 `Copy UUID + title`(统一 normalized pipeline)
- Section 菜单新增 `Set color`(复用 `section_update` 的 color 参数)
- Delete 标签统一("Delete alias"/"Delete section" → "Delete")

**测试计数**:TS 209 → 215(+6),Rust 不变。

## Code Review 6 项自查结果

1. **测试覆盖** ✓ — 每个新 capability 有 presence 检查 + handler click 检查;visibility 规则(sections 空/节点在/不在 section)全部覆盖;边界 case(Card 无 Delete / Section 不含 draw&move&remove&edit)显式断言
2. **逻辑正确性** ✓ — applicableCaps filter/sort 逻辑直读直算;groupOf 分组插入 separator 覆盖 normal/custom/destructive 三组;GraphView 的 onCopyEntityUuidTitle 所有 entity kind 显式处理(card/alias/note/question/section/task/whiteboard)
3. **回归风险** ✓ — 209 原测试全部通过(label 统一的 3 处在测试中同步更新),新增 6 个 case 锁住 Phase B1 新 UX
4. **I/O 正确性** ✓ — 无 Rust / DB 改动,纯 TS 重构
5. **IPC 类型安全** ✓ — 无新 Tauri command,无 bindings.ts 变化,`onSetSectionColor` 复用已登记的 `section_update`(副作用矩阵不需更新)
6. **建模强度** ✓ — 防火墙六项见上;runtime cast 只在渲染 dispatch 处使用且是 sound 的

## 验证基线

```bash
pnpm test -- --run 2>&1 | tail -5
# 期望: Test Files  29 passed (29)
#       Tests  215 passed (215)
```

```bash
pnpm build 2>&1 | tail -5
# 期望: built in <5s, 无 tsc 错
```

```bash
git log --oneline -4
# 期望 HEAD:
# 29d1fb8 refactor(keysight): Phase B1 完成 — 节点菜单 catalog 驱动渲染
# 6a37f25 feat(keysight): Phase B1 type sketch — node capability catalog 骨架
# d2ade2a docs(backend): Phase A 完成 — progress/handoff/devlog 同步
```

## 下一 session 可以做什么

按 `docs/progress/backend.md` > Next 挑一个开始:

1. **Keysight Task 节点菜单接入** — 纯 TS,把 Task 作为第 6 个 NodeKind 加到 catalog,接入 `copy_uuid_title` / `draw_connection` / `edit_title` / `move_to_section` / `remove_from_group` / `delete`。catalog 增量扩张:
   - `NodeKind` union 加 `"task"`
   - 6 个对应 capability 的 `applies_to` 加 `task`
   - 新增 `TaskNodeHandlers = Pick<...>` + `NodeMenuConfig` 判别联合添加 task 变体
   - EntityNode 的 `case "task"` 装配 menuConfig(目前 TaskNode 没有 contextMenu prop,要补)
   - GraphView.menuHandlers 新增 `onDeleteTask` / `onEditTaskTitle`(复用 `task_update` / `task_delete` 等已有 command)
   - onCopyEntityUuidTitle 的 `case "task"` 查 `data.tasks` 已经预留(无需改动)
   - 测试:新增 TaskNode.test.tsx 菜单 case + NodeContextMenu.test.tsx variant='task' 套

2. **Keysight 节点菜单 Phase B2** — Rust schema 变更,给 Card/Question/Task 加 `color` 字段:
   - `entities` 表加 `color` 列 + migration
   - 新增 `card_set_color` / `question_set_color` / `task_set_color` commands
   - `AtomicCard`/`QuestionEntity`/`TaskEntity` 加 color 字段
   - catalog 的 `set_color.applies_to` 扩到全集
   - EntityNode 的 cardMenu/questionMenu/taskMenu 接入 set_color handlers
   - GraphView.menuHandlers 新增 `onSetCardColor` / `onSetQuestionColor` / `onSetTaskColor`
   - 回归:原 Note/Section 的 set_color 行为不变

3. 其他积压未立 task(参见 progress/frontend.md 或 progress/ipc.md)

## 如果要 Phase B1 回归验证

- 手动 smoke test:打开任意白板,五种节点类型各开一次 ⋯ 菜单,确认 UX 新增的 `Copy UUID + title` 出现在 Card/Alias/Section 上,Section 有 7 色块
- 拷贝一次 card title 验证剪贴板内容:`UUID:{id} {normalized title}`(无 `**bold**`)
- 对一个 Section 改颜色验证 `section_update` 调用成功
