---
date: 2026-04-14
topic: Kanban View implementation plan review
status: review-complete
authors: [codex]
related:
  - docs/collaboration/2026-04-14-kanban-view-implementation-plan-via-cc.md
  - docs/collaboration/2026-04-14-kanban-view-design-via-cc.md
---

# P3 Kanban View 实施方案 Review

## 总结

我同意这份方案的主方向:

- `/kanban` 做成和 canvas 并列的独立路由,而不是嵌到 `KeysightView` 里
- `TaskStatus` 扩成 5 列(`Inbox/Next/Active/Blocked/Done`)
- SQLite 继续做单一真源,前端只做 query/mutation
- 先把静态 board + create 跑通,再上 drag-and-drop

但我不同意它按当前文本直接执行。核心问题不是“小代码示例不严谨”,而是有几条实现前提和当前仓库不一致,会导致计划做到一半才发现底座没铺好。

## 主要问题

### 1. `created_at desc` 这个排序前提当前不存在

这是我最不同意的一点。

- 方案在 `docs/collaboration/2026-04-14-kanban-view-implementation-plan-via-cc.md:813-835` 明确要求 `query_kanban` 按 `created_at desc` 排序
- 但当前 schema 里的 `entities` 表只有 `id/kind/title/whiteboard_id/file_path/content/color`，没有 `created_at`
- 见 `src-tauri/src/modules/keysight/db.rs:10-18`

这意味着当前计划里的 SQL 不是“要改一下字段名”，而是依赖了一个根本不存在的列。

我的建议:

1. 不要在 P3 里口头承诺 “created_at desc”
2. V1 先明确成一个当前可实现的排序
3. 如果产品真的在意“最新创建优先”，那要单独加 migration，而不是在 `query_kanban` 里假设它已经存在

### 2. `/keysight?wb=...` 不是现有契约，双向跳转任务顺序错了

- 方案在 `...via-cc.md:1819-1822` 和 `2289-2317` 里，把 “Reveal Graph / Show Kanban” 建立在 URL 跳转上
- 但当前 `KeysightView` 的白板状态是本地 `useState`
- 见 `src/components/keysight/KeysightView.tsx:41-43`
- 它也没有从 `location.search` 读取 `wb`
- 见同文件 `:113-115`，白板切换是纯本地状态

所以现在加按钮本身不够。按钮能跳到 `/keysight?...`，但 `KeysightView` 不会因此切到目标白板。

我的建议:

1. 先补 `KeysightView` 的 URL state <-> `currentWhiteboardId` 同步
2. 再做 `Reveal Graph / Show Kanban`
3. `GraphToolbar` 继续保持 dumb component，优先走 callback prop，不要直接在组件里塞 `useNavigate`

否则现在的 Task 3.6 / 3.8 顺序是反的。

### 3. `WhiteboardId` 这条 Phase 0 不够增量，而且并没有形成真实防火墙

- 方案把 `WhiteboardId` 提到 Phase 0 核心位置
- 见 `...via-cc.md:23-25`, `75`, `208-354`

我不同意把它放进这次 P3 的主路径，原因有两个:

第一，它现在并不会真正收敛边界。当前仓库大量白板流转还是 `String`:

- `task_query_all` / `layout_query_positions` 等 command 仍然接 `String`
- TS hooks 仍然用 `string` 当 whiteboard id
- 见 `src-tauri/src/modules/keysight/commands.rs:466-475`
- 见 `src/components/keysight/hooks/useWhiteboardData.ts:52-124`

第二，方案里这个 newtype 其实很弱。`parse` 只校验非空，并没有把 `projects/{name}` 进一步约束到 `ProjectName`，更谈不上真正把非法状态关在类型外面。

这会带来一个问题: 改动很多，但收益主要停留在“看起来更强类型”。

我的建议:

1. P3 的 Phase 0 只做 `TaskStatus IPC String -> enum`
2. `WhiteboardId` 单独起一个后续重构 task
3. 真要做，就一次性决定它覆盖哪些 command/model/hook；不要只在 `models.rs` 里加一个 newtype 就算完成

### 4. `CreateTaskModal` 的示例代码会保留过期默认值

- 方案在 `...via-cc.md:1997-2008` 里这样初始化 modal 内部 state:
  - `useState(defaultProject ?? ...)`
  - `useState(defaultStatus)`
- 但打开 modal 的地方又希望按列预设状态
- 见 `...via-cc.md:2212-2228`

这个写法在 React 里会有 stale default 问题:

- 第一次从 Inbox 打开后，内部 `status` state 建好了
- 关闭
- 再从 Blocked 列打开，如果组件没有被真正卸载重建，内部 state 不会跟 `defaultStatus` 自动同步

`defaultProject` 同理。

我的建议:

1. 把 modal 做成受控组件
2. 或者至少在 `open/defaultStatus/defaultProject` 变化时显式 reset state

不然列头 `+` 的“按列预设 status”会失真。

### 5. 现有仓库 API 形状和计划示例代码已经漂移，计划需要先校正再执行

这点不是产品方向问题，但它会直接拖慢实施。

几个明显例子:

- 方案把 `task::create` 当成参数展开函数调用
  - 见 `...via-cc.md:113-122`, `766-781`
- 但当前真实接口已经是 `TaskCreateInput` / `TaskUpdateInput`
  - 见 `src-tauri/src/modules/keysight/commands.rs:508-518`

- 方案里多次写 `commands.listWhiteboards()`
  - 见 `...via-cc.md:1931-1937`, `2362`
- 当前 bindings 对应的是 `commands.whiteboardList()`
  - 见 `src/components/keysight/hooks/useWhiteboardData.ts:172-180`

- 方案里从 whiteboard summary 读 `wb.id`
  - 见 `...via-cc.md:1940-1942`
- 当前字段是 `whiteboardId`
  - 见 `src-tauri/src/modules/keysight/models.rs:433-444`

- 方案里的 query 代码自己手拆 `{ status, data }`
- 当前项目已经有统一的 `unwrapCommand`
  - 见 `src/lib/commandResult.ts:1-12`

我的建议:

1. 在真正 implementation 前，先把计划里的代码片段全部对齐当前仓库 API
2. 尤其是 TS 侧统一走 `unwrapCommand`
3. 否则这份计划会变成“方向对，但示例不能抄”

### 6. `compute_position_below_bottommost` 用 `80` 作为默认节点高度，我不同意

- 方案在 `...via-cc.md:585-610` 里把 `DEFAULT_NODE_HEIGHT` 设成 `80.0`
- 但当前前端对 task 的估计高度是 `140`
- 见 `src/components/keysight/types.ts:47-50`

如果目标是“kanban 新建后自动落到最底元素下方，不重叠”，那 `80` 明显偏小，会让这个算法过于乐观。

我的建议:

1. 至少和当前 task 的典型高度对齐到 `140`
2. 或者更保守一点，宁可多留白，不要赌不重叠
3. 文档里明确说明: GraphView 当前创建任务仍会用 `layoutSetPosition` 覆盖到视口中心，这是另一个入口的策略，不是 bug

### 7. “默认创建状态是 Inbox” 目前只落在 kanban，没有覆盖现有 graph 创建入口

- 方案把 modal 默认值定成 `inbox`
- 见 `...via-cc.md:2212-2224`
- 但当前 GraphView 里 project whiteboard 上直接创建 task 仍然写 `"next"`
- 见 `src/components/keysight/GraphView.tsx:952-959`

如果产品决策是“系统内默认新 task 都应该先到 Inbox”，那现在计划没有把这个改完整。

如果决策只是“Kanban 入口默认 Inbox，Graph 入口仍然 Next”，那也应该在方案里明确写成例外，而不是默认读者自己推断。

我倾向于先明确决策，再改代码；不要让两个入口默默分叉。

## 我同意保留的部分

- `TaskStatus IPC String -> enum` 这条值得做，而且应该放在 kanban 前面
- `task_query_kanban` 作为新 command 是合理的，跨项目聚合不能靠前端 N+1 拼
- `KanbanBoard` 固定 5 列、前端按 `task.status` 分组是对的
- `All projects` 模式隐藏 `Reveal Graph` 是对的
- DnD 放到静态 board / create flow 后面做，这个节奏也对

## 我建议的增量重排

### A. 先做最小可落地 backend

1. `TaskStatus` IPC 边界改 enum
2. `TaskStatus` 增加 `Inbox`
3. 新增 `task_query_kanban(project?: string)`
4. 明确 V1 排序，不再写 `created_at desc`

这里先不要塞 `WhiteboardId`。

### B. 再做最小可用 frontend

1. `/kanban` 路由
2. `KanbanView` + `KanbanBoard` + `KanbanColumn` + `KanbanCard`
3. `KanbanToolbar`
4. `CreateTaskModal`
5. 统一复用现有 `whiteboardList()` 和 `unwrapCommand()`

### C. 单独补路由状态同步

1. `KanbanView` 的 `?project=...`
2. `KeysightView` 的 `?wb=...`
3. URL 与本地 state 双向同步稳定后，再做 cross-route 按钮和测试

### D. 最后再上 DnD

1. card/column 基本结构稳定后接 `@dnd-kit`
2. mutation 只改 status，不改 project
3. 失败时直接回落到 refetch，不做 optimistic update

### E. 明确延期项

- `WhiteboardId` newtype
- 更精确的 auto-layout
- 列内排序
- per-task reveal

## 结论

这份方案的产品方向基本正确，但实现计划还需要一次“贴着当前代码库现实重写”。

如果只问我一句话结论:

**可以继续沿这个方向做，但我会删掉 `WhiteboardId` 这一段，补上 `KeysightView` URL 同步，重写 query/toolbar/modal 的示例代码，并把 `created_at desc` 从方案里拿掉。**
