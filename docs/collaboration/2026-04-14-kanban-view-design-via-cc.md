---
date: 2026-04-14
topic: Kanban View
status: design-approved
authors: [user, claude-code]
related:
  - docs/progress/backend.md (P3 Kanban)
  - docs/handoff/backend.md
  - 上一份 handoff (含原始 5 决策点 + Task 3 段落): git show 7e2f828:docs/handoff/backend.md
process: brainstormed via superpowers:brainstorming, 9 questions, 5 design sections approved
---

# Kanban View — 需求与设计方案

## 0. 文档定位

本文档是 **P3 Kanban view** 的需求与设计方案,通过 `superpowers:brainstorming` skill 与用户协作产出,需求和方案在同一份文档,作为后续 implementation plan 的输入。

按用户要求,放到 `docs/collaboration/` 而非 `docs/superpowers/specs/`,文件名带 `-via-cc` 后缀(对齐已有 `-via-codex` precedent)。

## 1. 背景

用户的核心业务需求(从 Phase B2 之前就存在,handoff 多次提及):

> "类似 kanban 的地方,能统计项目的任务进展,点击 task 进去就可以对 task 做 note/alias 连线标注。"

Phase B2 已经完成 task 的 backend(CRUD + color schema + IPC commands + canvas Task 节点 + canvas 创建按钮),但 kanban 这一层完全没有实装。本设计是 P3 Kanban view 的 V1 方案。

**当前缺口**: sidebar 没有 Kanban 入口,无法跨项目看 task 概览,无法按 status 列管理。

## 2. 需求清单(brainstorm 决策)

### 2.1 范围内决策

| # | 决策点 | 选择 | 理由 |
|---|---|---|---|
| 1 | UI 形态 | Sidebar 顶级 nav `/kanban`,project 下拉切换 | 一个 UI 通吃跨项目 + 单项目;以后想要 canvas 内 toggle 是 additive |
| 2 | Status 列实现 | 纯视觉列从 `task.status` 派生 | 单一真源 (SQLite is SoT);避免 Phase B2 stringly-typed 漂移坑 |
| 3 | TaskStatus 扩展 | 加 **Inbox** → 5 值 (Inbox / Next / Active / Blocked / Done) | 用户需要"收集需求阶段"的暂存 |
| 4 | Inbox 与 project | 仍绑 project,无特殊化 | 一致性原则;不引入 `Optional<ProjectName>` 复杂度 |
| 5 | Kanban ↔ Canvas 关系 | Co-equal mode,project 内 toolbar 双向跳转("Reveal Graph" / "Show Kanban") | 画板的天然单位是 project 不是 task;reveal 是 project-level 而非 task-level |
| 6 | 列内排序 | V1 不做(按 `created_at desc`),V2 加 `order_in_column` | YAGNI |
| 7 | Kanban 创建入口 | Toolbar "+ New task" + 每列列头小 "+"(列头预设 status) | Inbox 列那个 + 永远可见,贴合 high-frequency Inbox-add workflow |
| 8 | 数据同步 | **无 sync 概念**,SQLite 是单一真源 | L0 数据真源硬约束 |
| 9 | Canvas 自动定位 | Kanban create 立即写 `positions` 行,坐标=最底元素下方 | 用户明确否决"手动 drag-to-canvas",要求两边数据完全一致,无"未登陆 canvas"的 task |
| 10 | 默认 task 创建 status | **Inbox** | 贴合"收集需求阶段"workflow,可在 modal 中改 |
| 11 | "All projects" 模式 Reveal Graph | 隐藏(只有 specific project 选中时显示) | 跨项目 graph 不存在,不能 reveal |

### 2.2 范围外(V1 不做,留 V2 / V1.5)

- ❌ **列内手动排序** — V2 加 `order_in_column` 字段
- ❌ **跨项目拖拽 task**(从 project A 的列拖到 project B) — drag 只改 status,不改 project
- ❌ **多 project 矩阵视图**(行=project,列=status) — V1 用 dropdown 切换
- ❌ **Per-task "Reveal in Graph"**(右键 task → 跳 canvas + 自动 pan/zoom 到 node) — V1.5 additive
- ❌ **Kanban 内做 note/alias 标注** — 标注是 canvas 的能力,kanban 是浅视图
- ❌ **Optimistic update + rollback** — 服务端为准,失败 loud
- ❌ **Drag-over 视觉态**(高亮目标列) — 基础 onDragEnd 即可,以后可 polish
- ❌ **Quick-add 浮动输入框 / 全局快捷键** — toolbar + 列头 + modal 已够

## 3. 架构总览

### 3.1 路由布局

```
/keysight                             — KeysightView (existing, canvas)
/keysight?wb=projects/super-tauri    — canvas of one project
/kanban                               — KanbanView (NEW), default "All projects"
/kanban?project=super-tauri          — single project kanban
```

### 3.2 层次切分

```
┌─────────────────────────────────────────────────┐
│  AppShell.tsx (existing)                        │
│  ├─ sidebar nav: + "Kanban" item ⬅ NEW          │
│  └─ <Outlet />                                  │
│       ├─ /keysight  → KeysightView (existing)   │
│       └─ /kanban    → KanbanView ⬅ NEW          │
└─────────────────────────────────────────────────┘

Frontend (~5 新组件)
├─ KanbanView          页面壳 + URL state + queries
├─ KanbanToolbar       project 选择 + "+ New task" + "Reveal Graph"
├─ KanbanBoard         5 列 (DndContext + droppables)
├─ KanbanColumn        单列 (header + "+" + 卡片 list)
├─ KanbanCard          单 task 卡片 (draggable)
└─ CreateTaskModal     project + title + status (default Inbox)

Backend (Rust, 增量小)
├─ TaskStatus enum     + Inbox 变体 (单点改动,编译器强制所有 match 穷尽)
├─ task::create        内部调 compute_position_below_bottommost 自动定位
├─ task::compute_position_below_bottommost  ⬅ NEW (纯函数 helper)
├─ task::query_kanban  ⬅ NEW command (project: Option<ProjectName>)
└─ parse_task_status_from_ipc  + 接受 "inbox"

第三方
└─ @dnd-kit/core       5 droppable columns + draggable cards
```

### 3.3 关键边界

- **KanbanView 与 GraphView 完全独立**,不嵌套,不共享父组件
- **共享 SQLite + 共享 React Query cache key 命名约定**,改一处自动 invalidate 另一边
- **"Reveal Graph" 用 `useNavigate` 跳路由**,非组件嵌套
- **没有任何 sync 代码**,两个视图都是同一 SQLite 数据的渲染

## 4. 组件细节

### 4.1 `KanbanView.tsx` — 页面壳

- **URL state**: `?project={name}` 或 missing(后者 = "All projects")
- **Queries**:
  - `useQuery(["tasks-kanban", project], () => commands.taskQueryKanban(project))` — 主数据源
  - `useQuery(["whiteboards"], () => commands.listWhiteboards())` — dropdown 数据源
- **Children**: `<KanbanToolbar>` + `<KanbanBoard>` + `<CreateTaskModal>`
- **Modal open state** 提到这一层(dropdown / 创建 modal 之间共享)

### 4.2 `KanbanToolbar.tsx`

- **Project dropdown**(数据源 = list_whiteboards 过滤 `projects/*`)
- **"+ New task"** 按钮 → 打开 modal,无预设 status
- **"Reveal Graph →"** 按钮 — 仅 specific project 时可见,点击 `navigate('/keysight?wb=projects/' + name)`
- 顶部统计:5 列各多少 task

### 4.3 `KanbanBoard.tsx`

- **接 tasks list,内部按 `task.status` 分 5 组**
- `<DndContext onDragEnd={handleDragEnd}>` 包裹
- 渲染 5 个 `<KanbanColumn>` 顺序固定: Inbox / Next / Active / Blocked / Done
- `handleDragEnd`: 提取 source/target column 的 status → 调 `commands.taskUpdate(id, null, null, newStatus, null, null)` → 集中 `invalidateAllTaskCaches`

### 4.4 `KanbanColumn.tsx`

- **Props**: `status: TaskStatus`, `tasks: TaskEntity[]`
- **列头**: 标题(中文或英文,如 "收件箱" / "下一步" / "进行中" / "阻塞" / "完成",或 "Inbox" / "Next" / "Active" / "Blocked" / "Done",待用户确认见 §10.3) + 计数 + 小 "+" 按钮
- **小 "+" 按钮**: 打开 modal,status 预设为该列的 status
- `<SortableContext>` 包(为 V2 排序留接口,V1 strategy 用 `verticalListSortingStrategy` 或 disable)
- `map(tasks → <KanbanCard>)`
- **Empty state**: 居中提示文案

### 4.5 `KanbanCard.tsx`

- **Props**: `task: TaskEntity`
- 显示: title / status badge / project tag / 左侧 color 条
- `useDraggable` from dnd-kit
- click → 暂时仅 selection 视觉态(V1 不做"进入" graph)
- "All projects" 模式下 project tag 醒目

### 4.6 `CreateTaskModal.tsx`

- **字段**: project (dropdown,必填) / title (文本,必填) / status (default `Inbox`,可改)
- **submit**: `commands.taskCreate(project, title, null, status, null, null)` → `invalidateAllTaskCaches()` → 关闭 modal
- **错误**: loud `console.error` + toast / inline message
- **校验**: client-side(空 title / 缺 project)+ server-side(canonical: `ProjectName::new` + `parse_task_status_from_ipc`)

### 4.7 `AppShell.tsx` 改动

- sidebar nav 加 `{ icon: LayoutGrid, label: "Kanban", path: "/kanban" }`,放在 KeySight 下方、Notes 上方(沿用现有 nav 顺序惯例)

## 5. 数据流

### 5.1 创建 task(kanban → SQLite → 两边自动 refresh)

```
User clicks "+ New task" in KanbanColumn header (Inbox 列)
  ↓
CreateTaskModal opens with status=Inbox pre-filled
  ↓
User fills title, picks project, submit
  ↓
commands.taskCreate(project, title, null, "inbox", null, null)
  ↓
Rust task::create(conn, ...)
  ├─ ProjectName::new(project)? — 校验
  ├─ insert entities + task_fields rows
  ├─ task::compute_position_below_bottommost(conn, "projects/{name}")
  │   ├─ SELECT MAX(y + node_height) FROM positions WHERE whiteboard_id = ?
  │   └─ return Position { x, y }  [empty 时 return Position { x: 0.0, y: 0.0 }]
  ├─ insert positions row (whiteboard_id, entity_id, x, y)
  └─ render markdown file (existing logic, frontmatter status=inbox)
  ↓
return TaskEntity (含 status + project + ...)
  ↓
Frontend invalidateAllTaskCaches():
  - queryClient.invalidateQueries({ queryKey: ["tasks-kanban"] })
  - queryClient.invalidateQueries({ queryKey: ["tasks", "projects/" + project] })
  - queryClient.invalidateQueries({ queryKey: ["positions", "projects/" + project] })
  ↓
Kanban refetches → 新 task 出现在 Inbox 列
Canvas (如打开) refetches → 新 task 出现在最底部
```

### 5.2 拖拽改 status(kanban 内 drag-and-drop)

```
User drags KanbanCard from Inbox column to Next column
  ↓
@dnd-kit onDragEnd → 提取 source/target droppable id (= column status)
  ↓
parse target id → TaskStatus literal (5 个之一,非法直接 return)
  ↓
commands.taskUpdate(id, null, null, "next", null, null)
  ↓
Rust task::update → SQLite + render markdown 更新 frontmatter status
  ↓
invalidateAllTaskCaches()
  ↓
Canvas (如打开) 不需要重新 layout (位置不变,只 status 字段变),自动重渲染
```

**V1 决定**: 不做 optimistic update(简单 + 失败时 query refetch 自动回滚视觉)。

### 5.3 跨项目 vs 单项目查询

```
"All projects":  task_query_kanban(None)         → SELECT * FROM entities WHERE entity_kind='task'
"super-tauri":   task_query_kanban(Some(name))   → ... AND whiteboard_id = "projects/super-tauri"
```

### 5.4 URL state ↔ React state 同步

- `?project=xxx` 是真源(refresh 不丢 + 可分享 link)
- KanbanView 用 `useSearchParams` 读
- Toolbar dropdown 改 → `navigate('/kanban?project=xxx')` → React Query refetch

### 5.5 Cache key 命名约定

| Key | 用途 | 由谁 invalidate |
|---|---|---|
| `["tasks-kanban", project?]` | kanban view 主数据 | 所有 task 写操作 |
| `["tasks", whiteboardId]` | canvas view 的 task list | 所有 task 写操作 |
| `["positions", whiteboardId]` | canvas position layer | task create / position 更新 |
| `["whiteboards"]` | sidebar / project dropdown | 白板 CRUD |

**关键约束**: 所有 task 写命令成功后必须调用 `invalidateAllTaskCaches()` helper。该 helper 集中收口,避免每个 mutation 自己拼 cache key 漏掉。

## 6. 错误处理

### 6.1 Rust(typed errors,向上传播)

| 失败点 | 错误处理 |
|---|---|
| `task::create` 文件名非法 / project 不存在 | `KeysightError::InvalidProjectName` / `IO`,`Into<AppError>` 抛 IPC |
| `compute_position_below_bottommost` empty whiteboard | 返回 `Position { x: 0.0, y: 0.0 }`,**不是错误** |
| `compute_position_below_bottommost` SQL fail | 向上传播 `KeysightError::Database`,task 创建失败 |
| `task_query_kanban` DB 错误 | `Result<Vec<TaskEntity>, AppError>` 传播 |
| `task_update` status 字符串非法 | `parse_task_status_from_ipc` 返 `InvalidTaskStatus` |
| TaskStatus 加 Inbox 后旧 DB 数据 | **没有迁移问题** — Inbox 是新值,旧数据不会有,新建才用 |

### 6.2 Frontend(UI 反馈,不解析 error message)

| 状态 | UI |
|---|---|
| Query loading | KanbanBoard 显示 5 列 skeleton,column header `—` 计数 |
| Query error | Board inline error: `加载失败: {message}` + Retry 按钮 |
| Mutation error(create / drag) | toast + console.error + 前端**不修改本地 state**(query 自动 refetch 回滚) |
| 空状态(单 project 无 task) | 列内居中 `还没有 task。点 + 创建第一个` |
| 空状态(无 project) | dropdown 显示 `还没有 project,先去 Canvas 创建一个 project 白板` |
| Modal 表单校验 | client side(title / project) + server side(`ProjectName::new`) |

### 6.3 不做的事(硬红线)

- ❌ 前端**不**解析 error message 字符串做控制流
- ❌ 前端**不**有 fallback / silent catch — 失败要 loud
- ❌ V1 **不**做 optimistic update + rollback
- ❌ V1 **不**做 retry policy,失败显示按钮让用户手动 retry

## 7. 测试策略

### 7.1 Rust 单元测试(domain.rs + in-memory SQLite)

| 测试目标 | 用例 |
|---|---|
| `compute_position_below_bottommost` | empty / 1 row / N rows / 跨 wb 隔离 / 极大 y 值 / 负 y 值 |
| `task::create` 集成 auto-position | 第一次 → (0,0) / 第二次 → 第一个下方 / 跨白板独立 / position 行同步写入 / 失败回滚 |
| `task::query_kanban` | None → 所有 / Some(project) → 仅该项目 / 5 status 都不漏 / 空结果 |
| TaskStatus Inbox 解析 | "inbox" → Ok(Inbox) / 大小写敏感 / serde round-trip |
| `parse_task_status_from_ipc` | 加 inbox case / 非法字符串拒绝 |

### 7.2 Rust 集成测试(`tests/kanban_stories.rs`)

- **Story 1**: 创建 3 个 task in 不同 status → query_kanban 返回 grouped 正确
- **Story 2**: drag (update status) → query 反映新 status + position 不变
- **Story 3**: 跨 project query → All projects 模式正确
- **Story 4**: kanban 创建 task → canvas query 立即可见(模拟两边视图同步)

### 7.3 TS 组件测试(vitest + RTL,mock `commands.*`)

| 组件 | 测试点 |
|---|---|
| KanbanView | URL `?project=xxx` 驱动 query / dropdown 切换更新 URL / loading/error/empty 状态 |
| KanbanBoard | 5 列固定顺序 / tasks 按 status 分组渲染 / DndContext wrapping |
| KanbanColumn | 计数正确 / 列头 + 触发 modal with status pre-fill / empty state 文案 |
| KanbanCard | render title/status/project tag / "All projects" 模式 project tag 醒目 |
| CreateTaskModal | 表单校验 / submit 调 commands.taskCreate / cancel 关闭 / status 默认 Inbox |
| KanbanToolbar | "Reveal Graph" 仅 specific project 显示 / 跳路由正确 |

### 7.4 TDD 流程(人工确认关卡严格执行)

每个新 Rust 函数 / 新 React 组件按 Red → 用户确认 → Green → 用户确认 → Refactor 走。

### 7.5 测试不做

- ❌ 不测 @dnd-kit 内部行为
- ❌ 不测 react-router 跳转细节
- ❌ 不测 React Query refetch 时机
- ✅ 测的是**我们的代码**: 业务规则 / 数据流接线 / UI 状态切换

## 8. L0 防火墙 / 想出错都难 — 类型层强度自查

> 用户明确要求: 整个方案要始终考虑质量标准体系和"想出错都难"理念。

本节是按 default-deny 原则对设计中可能漂移的地方一一过类型护栏。

### 8.1 已经强类型的地方(继承,不退步)

| 元素 | 类型 | 防护机制 |
|---|---|---|
| `ProjectName` | newtype with `::new()` 校验 | 路径字符 / 长度 / 大小写 在构造器拦下 |
| `TaskStatus` (Rust 内) | enum | match 强制穷尽,加 Inbox 后所有 match arm 编译失败必须补 |
| `TaskEntity` | derive Serialize + Deserialize + specta::Type | tauri-specta 编译期保证 TS 同步 |
| `task::create` | 接 `&ProjectName` 而非 `&str` | command 边界 wrap 之后 domain 安全 |

### 8.2 新增的地方(本设计要求)

| 元素 | 类型决定 | 理由 |
|---|---|---|
| `compute_position_below_bottommost` 返回值 | `struct Position { x: f64, y: f64 }` newtype | `(f64, f64)` tuple 是 stringly-typed 的几何形态,容易调用方传反 |
| `KanbanColumn.status` prop (TS) | `TaskStatus` literal union from bindings.ts | 不能用 `string`;tauri-specta 已生成 union |
| `COLUMNS` 常量 (TS) | `Record<TaskStatus, ColumnConfig>` 而非 `TaskStatus[]` | 加新 status 时编译器强制 5 个 case 都补;数组只能运行时才发现漏 |
| `KanbanCard.task.status` (TS) | `TaskStatus` literal union | 同上 |
| `task::query_kanban` 入参 | `project: Option<ProjectName>` | 不接 `Option<String>` |
| `onDragEnd` 的 source/target | 内部用 `TaskStatus` literal,边界 parse(类似 `parse_task_status_from_ipc` 的 TS 版) | dnd-kit 的 droppable id 是 string,parse-don't-validate 一次 |

### 8.3 Phase B3 P2 类型迁移 — 强烈建议作为本设计的 Phase 0

handoff 列了 Phase B3 P2 推迟项,其中**两条直接影响 kanban 类型安全**,本设计强烈建议本次 P3 实施时顺带完成。如果觉得 scope creep 可以单独切 commit,但**不建议推迟**(kanban 是受益方,推迟意味着同样的拼写错误风险持续到 V2)。

#### 8.3.1 TaskStatus at IPC boundary (从 String 升到 enum)

**现状**: `task_create / task_update / task_query_*` 在 IPC 边界用 `status: String`,Rust 内调 `parse_task_status_from_ipc`。

**问题**: kanban 各处传 status 字符串(创建 task / drag-and-drop 改 status / Inbox 列预设),一旦 TS 拼错或大小写不一致就靠运行时拦下。

**改造**: 让 IPC 边界直接用 `TaskStatus` enum(tauri-specta 已 derive Type)。`bindings.ts` 自动生成 TaskStatus literal union,TS 端无法传非法字符串。

**收益**:
- 拼错 status 编译期失败
- 加 Inbox 时所有 IPC 调用点强制更新(编译失败是好事)
- `parse_task_status_from_ipc` 这个 helper 直接删掉

**改动量**: ~30 行 Rust + 自动生成 TS,kanban 是直接受益方。

#### 8.3.2 WhiteboardId newtype + WhiteboardId::for_project(&ProjectName)

**现状**: 各处用 `String` 传白板 id (`"projects/super-tauri"`),拼接散落在 caller。

**问题**: kanban URL state `?project=` 解析 + Reveal Graph 跳转 + cache key 命名 都要拼字符串,一处 typo 全链路漂移。

**改造**: `WhiteboardId` newtype + 构造器,`WhiteboardId::for_project(&ProjectName) → WhiteboardId(format!("projects/{}", name.as_str()))`。canvas 当前 `currentWhiteboardId.startsWith("projects/")` 这种字符串前缀逻辑收敛到一处。

**改动量**: ~50 行 Rust + 调用点更新,canvas 和 kanban 共同受益。

### 8.4 Default-deny 检查

| 添加项 | 是否 default-deny? |
|---|---|
| TaskStatus 加 Inbox | ✅ 编译器强制所有 match 穷尽 |
| compute_position 在 empty whiteboard | ✅ 显式返 (0, 0),不是 None / panic |
| @dnd-kit drag drop 在非法 target | ✅ onDragEnd parse 后非 5 列直接 return,不调 task_update |
| URL `?project=invalid-name` | ✅ ProjectName::new 拒绝,显示 toast 并 redirect 到 `?project=` 空 |
| Modal submit 空 title | ✅ client trim() + Rust 端 `.is_empty()` 拒绝 |
| 加新 TaskStatus variant (未来) | ✅ Record<TaskStatus, ...> 强制所有 case;match 强制穷尽 |
| Kanban / Canvas 数据漂移 | ✅ SQLite SoT,无副本 |

### 8.5 不允许的逃生舱口

- ❌ TS 端 `as TaskStatus` cast(用 type guard / parse 函数替代)
- ❌ Rust 端 `_ => ...` 通配 match arm(除非 `// 例外:` 注释 + 充分理由)
- ❌ 任何字符串拼接生成 wb_id(一律走 `WhiteboardId::for_project` 或 `::root`)
- ❌ Frontend 解析 Rust error message 字符串做控制流
- ❌ Optimistic update with state mutation(V1 直接禁止)
- ❌ React Query 用 `keepPreviousData` 在 status 切换时显示 stale 数据(可能造成"看到 Inbox 实际是 Next"的错觉)

## 9. 副作用矩阵更新

CLAUDE.md 副作用矩阵新增 / 修改:

| 写操作 | 影响的表 | 副作用 | 测试覆盖 |
|---|---|---|---|
| `task::create` (修改) | `entities`, `task_fields`, `file_mtimes`, `entities_fts`, **`positions`** ⬅ NEW | 自动写入 (x, y) below bottommost | compute_position 单测 + create 集成测 |
| `task::compute_position_below_bottommost` (新增,纯查询) | (read only) | — | 单测 (空 / 1 行 / N 行 / 跨 wb / 极值) |
| `task::query_kanban` (新增,纯查询) | (read only) | — | 单测 |

## 10. 风险与开放问题

### 10.1 风险

- **R1**: @dnd-kit 学习曲线(团队没用过)。**缓解**: 用最小 API(`useDraggable + useDroppable + DndContext`),不上 `SortableContext` 直到 V2 排序需要
- **R2**: invalidate cache key 漏掉导致两边视图不同步。**缓解**: 集中 `invalidateAllTaskCaches()` helper,所有 task 写命令统一调用
- **R3**: `compute_position_below_bottommost` SQL 在大白板上效率(全表扫 positions)。**缓解**: positions 表加 index `(whiteboard_id, y)`,几千行不会慢
- **R4**: 8.3 Phase 0 类型迁移 scope creep。**缓解**: 切独立 commits,可以分阶段 ship

### 10.2 已解决的开放问题

- ✅ **默认 task 创建 status**: Inbox(贴合"收集需求"用法)
- ✅ **"All projects" 模式 Reveal Graph**: 隐藏(只有 specific project 时显示)

### 10.3 仍待用户确认

- **列名翻译**: 5 列用中文还是英文? 中文方案: 收件箱 / 下一步 / 进行中 / 阻塞 / 已完成。英文方案: Inbox / Next / Active / Blocked / Done。**待用户决定。**(对应 enum 内部仍是 lowercase 英文,只影响 UI 文案)
- **节点高度常量**: `compute_position_below_bottommost` 需要假设一个节点高度(用于算 bottom = y + height)。Canvas 上 task 节点目前是动态高度。**两个方案**: (a) 用 fixed 80px 默认值(简单但可能略 overlap); (b) JOIN entities 表读 height_hint 字段(如有)。**待用户决定** — 推荐 (a)。
- **Phase 0 类型迁移是否本次做**: 8.3.1 + 8.3.2 是强烈建议但要 +1 个 Phase 工作量。**待用户决定**。

## 11. 实施顺序建议(分 Phase,每 Phase 一组 commits 可独立 ship)

### Phase 0 (类型基础,8.3 强类型迁移) — 强烈建议

- TaskStatus IPC boundary `String → enum` + TS bindings 自动生成
- WhiteboardId newtype + `for_project` + 各 caller 收敛
- 跑 `cargo test export_bindings` 验证 bindings.ts 无 break
- 已有测试全绿

### Phase 1 (Rust 数据层)

- TaskStatus enum 加 Inbox 变体 + parse_task_status 加 case + 所有 match 补 arm
- `Position` newtype + `compute_position_below_bottommost` helper + 单测
- `task::create` 集成 auto-position
- `task::query_kanban` 命令 + 单测
- 副作用矩阵更新

### Phase 2 (Frontend 基础组件)

- AppShell sidebar Kanban nav item
- /kanban 路由 + KanbanView 壳(空板)
- KanbanToolbar(project dropdown + Reveal Graph 按钮)
- KanbanBoard + 5 个静态空 KanbanColumn(无 dnd)

### Phase 3 (Frontend 数据接线)

- KanbanCard render + tasks 接入 query
- CreateTaskModal + "+ New task" button + 新建数据流
- 列头 "+" 按钮 + status 预设
- TS 组件测试

### Phase 4 (Drag-and-drop)

- @dnd-kit 引入 + DndContext + Draggable + Droppable
- onDragEnd 集中 handler + parse + task_update mutation
- TS 组件测试(mock dnd events)

### Phase 5 (Polish + dev 验证)

- Empty / loading / error 状态完善
- Reveal Graph 双向跳转(canvas 反向加 "Show Kanban" 按钮)
- `/harness-check-tests` + `/harness-type-safety-check`
- Code Review(6 项检查)
- Dev server 手动 walkthrough(创建 / 拖拽 / 切 project / Reveal Graph 双向)
- Commit + changelog ✅

## 12. 完成标准

- [ ] Phase 0~5 全部完成
- [ ] Rust 测试全绿(预计 +20 个新测试)
- [ ] TS 测试全绿(预计 +30 个新测试)
- [ ] cargo clippy / cargo build / pnpm build 全绿
- [ ] dev server 手动 walkthrough 通过:
  - [ ] sidebar 看到 Kanban 项,点击进入 default "All projects" 视图
  - [ ] dropdown 切换 project,5 列正确展示该 project 的 tasks
  - [ ] 创建 task 默认进 Inbox 列,再次创建出现在 Inbox 第二个
  - [ ] 拖拽 task 从 Inbox 到 Next,canvas 同时打开能看到 status 变化
  - [ ] kanban 创建的 task 在 canvas 上出现在最底部
  - [ ] Reveal Graph 跳到 canvas 同 project,Show Kanban 跳回
  - [ ] All projects 模式跨项目展示
- [ ] Code Review 6 项检查通过
- [ ] 副作用矩阵更新
- [ ] devlog / changelog 同步

## 13. 一句话总结

Kanban view 是 sidebar 顶级独立路由,与 Canvas 是 co-equal 视图模式,共享 SQLite 单一真源,通过 React Query cache 命名约定实现两边自动同步;TaskStatus 加 Inbox,kanban 创建的 task 自动在 canvas 最底部分配坐标,无 sync 代码无两份数据;全程遵循 L0 防火墙模型,enum 强制穷尽、newtype 包 wb_id、Phase 0 顺手做 IPC boundary TaskStatus 类型化。
