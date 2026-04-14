# Project 白板 (`projects/{name}`) 上 set_color / draw_connection 失效 — bug 报告与调查上下文

> **交接对象**:下一个 agent / 人类,排查并修复 "在 `projects/super-tauri` 这种嵌套
> project 白板上,Note/Question/Task 的菜单 `Set color` 和 `Draw connection` 都没反应,
> 但同操作在 `wb_root` 上正常"。
>
> **状态**:症状已经在 dev server 复现并截图确认。**根因未定位**。本文档列出
> 已排除的假设、剩余的开放假设、关键代码路径、最近 3 个 commit。**不建议盲目改代码**,
> 先验证开放假设里的哪一个是真正的根因。

---

## 1. 症状

用户在 dev server 里,**当前白板 = `projects/super-tauri`** 时:

- ✅ 在 toolbar 创建的 Task / Note / Question 都能渲染出来
- ✅ 三种节点的 ⋯ 菜单都能打开
- ✅ 三种节点都能 `Edit title`(本 session P1 Task 4 接入,已验证 OK)
- ❌ 三种节点都**不能 `Set color`** —— 菜单里的色块**有出现**(7 个圆形 swatch),
  但点击后节点背景颜色**没有变化**
- ❌ Note / Question 都**不能 `Draw connection`** —— 菜单里有这一项,**点了完全没动静**
  (无 source 高亮、无 cursor 变化、第二次点击其他节点也不画线)

**对照组**: 在 `wb_root`(根白板)上的同操作**完全正常**,用户已确认。

唯一在所有节点上都正常的菜单功能是 `Edit title`(就是本 session 刚加的)。

### 1.1 截图证据(用户提供)

用户截图的是 Task `haha sandbox 2`(`projects/super-tauri` 白板)的 ⋯ 菜单展开状态,
菜单里清晰可见:

```
┌──────────────────┐
│ Copy UUID + title│
│ Edit title       │
│ ───────────────  │  (separator)
│ ● ● ● ● ● ● ●    │  (7 个色块,各自 aria-label="Set color {hex}")
│ ───────────────  │  (separator)
│ Delete           │  (红色 destructive 样式)
└──────────────────┘
```

菜单项**显示完全正常**,所以 bug **不在** `NodeContextMenu` 渲染层(菜单项过滤、可见性
规则、分组分隔符都对得上 `NodeCapabilityCatalog` 的声明)。

### 1.2 用户的原话

> 增加的task, Note, question, 可以 edit title. 但是都不能 set color, 也都不能 draw
> connection
>
> A: 菜单有, 但是点击背景颜色没有变
>
> B: 点了没动静
>
> C: root 功能正常

---

## 2. 触发上下文

bug 是在测试本 session 的 **P1 Task 4**(TaskNode 接入 edit_title inline editor)时
**顺带发现的**。整个 session 一共做了 3 个 task:

| 顺序 | Task | Commit | 触碰的代码层 |
|---|---|---|---|
| 1 | P0 Task 1: `list_whiteboards` 递归 `projects/*` 嵌套白板 | `c91f7f7` | Rust `vault_fs.rs` + `domain/overview.rs`(纯查询;新增 default trait method `list_project_whiteboards`,过滤裸 `"projects"`,union 嵌套 project 白板) |
| 2 | P0 Task 2: GraphToolbar 加 `Task` 创建按钮(project 白板专属) | `c59cf7e` | TS only:`GraphToolbar.tsx` 加 6 个 task-* props + 条件渲染;`GraphView.tsx` 加 `creatingTask`/`taskDraft` state + `handleSubmitCreateTask`(派生 project 后 `commands.taskCreate(...)`)|
| 3 | P1 Task 4: TaskNode `edit_title` inline editor | `7f3e24d` | TS only:`NodeCapabilityCatalog.ts` 把 `task` 加入 `edit_title.applies_to` + `TaskNodeHandlers` Pick 加 `edit_title`;`TaskNode.tsx` 加 inline editor 模式;`EntityNode.tsx` 加 `onEditTaskTitle` handler + `taskEditingField` 计算 + `case task` 渲染传 4 prop;`GraphView.tsx` 本地 `EditingField` union 加 `"task-title"` + `handleCommitEdit` 加 case + `menuHandlers` 加 `onEditTaskTitle` |

**这 3 个 commit 都没有触碰 set_color 或 draw_connection 的任何代码路径。** 它们只新增/
修改了 task 创建入口 + edit_title 链路。

所以 bug **要么早就存在(只是用户今天才发现)**,**要么由更早的某个 commit 引入但只在
project 白板上表现**。

### 2.1 不能假设是新 bug

git log 显示 Phase B2 阶段(commit `bf96491`/`d6c6f18`/`874ae95`)给 Task 加了 color
schema + commands.taskSetColor,以及 sync 路径识别 `projects/{name}` 二级 wb_id。这些
变更涉及 set_color **直接路径** 和 **wb_id 派生路径**,有可能藏着相关 regression。

调查时 **必须先在历史 commit 上 bisect**,而不是想当然地认为是这次的 3 个 task 引入的。

---

## 3. 已排除的假设

以下假设在写这份报告时已经手动 grep + 阅读代码排除:

### 3.1 ❌ "菜单渲染层有 wb_id 守卫"

`src/components/keysight/nodes/NodeContextMenu.tsx` 完全 wb-agnostic。它只看:

```ts
NODE_CAPABILITIES
  .filter((cap) => cap.applies_to.has(menu.kind))      // 只看 NodeKind
  .filter((cap) => isCapabilityVisible(cap, ctx))      // 只看 sections / currentSectionId
  .sort((a, b) => a.order - b.order)
```

`isCapabilityVisible` 只看 `currentSectionId !== null`(in_section 规则)和
`sectionsCount > 0`(sections_not_empty 规则)。`set_color` 和 `draw_connection`
都没有 `visible` 字段,**永远应当渲染**。截图也证实它们确实渲染了。

### 3.2 ❌ "applies_to 漏了 task / note / question"

`NodeCapabilityCatalog.ts:154-228` 的 `NODE_CAPABILITIES` 数组里:

```ts
{
  kind: "set_color",
  applies_to: new Set<NodeKind>(["card", "note", "question", "section", "task"]),
  ...
},
{
  kind: "draw_connection",
  applies_to: new Set<NodeKind>(["card", "alias", "note", "question"]),
  ...
},
```

note/question/task 都在 set_color 的 applies_to 里;note/question 都在 draw_connection
的 applies_to 里(task 不在,**这是预期**:Edge 判别联合编译期禁 Task 作 from)。

### 3.3 ❌ "menuHandlers 在 GraphView 漏了某个 callback"

`GraphView.tsx` 的 `menuHandlers` useMemo(line ~1002 起)定义齐全,包含
`onDrawConnectionFrom`、`onSetCardColor`、`onSetNoteColor`、`onSetQuestionColor`、
`onSetSectionColor`、`onSetTaskColor` 全部 6 个。

EntityNode 的 `noteMenu` / `questionMenu` / `taskMenu` useMemo 也都正确把这些 callback
封进对应的 NodeMenuConfig。

### 3.4 ❌ "TaskNode 不渲染 color 字段"

实际查阅 `src/components/keysight/nodes/TaskNode.tsx` 后发现它**确实不读
`task.color`** 来设置背景色。但这无法解释 **Note / Question** 的 `Set color` 也失效
—— 它们的渲染层和 task 是独立组件。所以 TaskNode 的 color 渲染缺失最多只能解释 task
那一栏,不能作为整体根因。

(**附带建议**: 即便不是这个 bug 的根因,TaskNode.tsx 不读 `task.color` 本身也是个
独立缺口,Phase B2 加了 schema 但渲染层没接,可以单独修。)

### 3.5 ❌ "draw_connection 第二次点击有问题"

`GraphView.handleSelectEntity`(line ~1269 起)的 drawing 模式判断逻辑:

```tsx
if (drawingState && drawingState.fromId !== selection.id) {
  const { fromId } = drawingState;
  setDrawingState(null);
  unwrapCommand(commands.entityConnect(fromId, selection.id))
    .then(() => queryClient.invalidateQueries())
    .catch((err) => console.error("建立连线失败:", err));
  return;
}
```

逻辑没有 wb_id 参与,且 `commands.entityConnect(fromId, toId)` Rust 侧也是按 entity id
派发的。从代码上看不出 wb_id 区别。**但这一段没有手动 bisect 验证过**,只是阅读代码后
"看起来对"。

### 3.6 ❌ "drawingState 从来没被 set"(即第一次点 Draw connection 就没动)

`onDrawConnectionFrom` 的实现就一行:

```tsx
onDrawConnectionFrom: (id) => {
  setRelatedPickerCardId(null);
  setRelatedSearch("");
  setDrawingState({ fromId: id });
},
```

完全 wb-agnostic。但用户描述 "B: 点了没动静" 意味着**第二次点击**也没触发画线 —— 这
**不能区分** "第一次点没生效" 和 "第二次点击被吞" 两种情况。需要在 dev server 里开
React DevTools 看 `drawingState` 这个 useState 的实际值变化。

---

## 4. 开放假设(按可能性排,需要逐一验证)

### 4.1 ⭐ H1 — Rust 写入路径用 wb_id 派生文件路径,但派生方式不支持嵌套 `projects/{name}`

**论点**:Phase B2 给 task 设计了固定路径 `whiteboard/projects/{project}/{id} 【TASK】{title}.md`,
**通过 `ProjectName` newtype 保证**。但 **note / question 的文件路径** 是怎么决定的?
特别是当用户在 `projects/super-tauri` 白板上**新建** note 时,note 的文件落到哪里?

如果 `domain::note::create` / `domain::question::create` 在写文件时**无视 wb_id**(还是
落到平铺 `whiteboard/{name}.md` 或 `whiteboard/notes/{name}.md`),那:

1. 节点能在 DB 中创建 + 在画布渲染(因为 entities 表的 whiteboard_id 存的是 `projects/super-tauri`)
2. **后续 update 按文件路径找文件时找不到**,或找的是错的文件 → 静默失败 / 写错地方
3. 即便 update 成功,query 时按 `whiteboard_id = 'projects/super-tauri'` SELECT,**仍然
   能查回原节点**(因为 entities 表行没动)
4. **但 color 字段没真的更新到 DB**,所以前端 invalidate 后看到的还是旧值 → "颜色没变"

**为什么这个假设排第一**:
- 它能同时解释 Note / Question 的 set_color 失效(走文件写回路径)
- 它能解释为什么 `wb_root` 上正常(`wb_root` 的文件路径派生是老逻辑、稳定)
- 它能解释为什么 task 的 set_color 也失效(task 的 update 也走文件写回,如果路径有问题
  也可能挂掉 —— 虽然 task 有 ProjectName 强类型保护)
- 它**不能直接解释** draw_connection 失效 —— `entity_connect` 只写 `edges` 表,跟文件
  无关 → **draw_connection 是另一个 bug**(或者另一种解释,见 H4)

**怎么验证**:
1. 在 dev server 上 project 白板里建一个 Note,看磁盘上 `whiteboard/projects/super-tauri/`
   下有没有对应的 .md 文件,还是落在 `whiteboard/` 下了
2. 改 note 标题(edit_title)、看文件名变了没
3. 设 color、看 frontmatter 里的 `color` 字段更新了没
4. 看 Rust `note::create` / `question::create` 里 wb_id 怎么决定文件路径

### 4.2 ⭐ H2 — `entityConnect` 写入 edges 后,某种 sync 把 note/question 从 project 白板"驱赶"到了 wb_root

**论点**: 注意 Phase B2 的 `sync::derive_whiteboard_id` 修复(handoff 提到)把
`projects/loose.md` 这种"裸 project 文件"quarantine 到 `wb_root`。如果 note/question 的
文件**没正确落在 `whiteboard/projects/super-tauri/` 子目录里**(见 H1),那它们对应的
DB 行可能在下次 sync 时被 derive 到 `wb_root` 而**不是** `projects/super-tauri`。

这意味着用户看到的 "在 project 白板上的 Note"其实是 DB 里 `whiteboard_id = 'wb_root'`
的 entity,前端的查询 `["notes", "projects/super-tauri"]` 可能根本不会查到它(取决于
TanStack Query 的 cache 状态什么时候被 invalidate)。

不过这个假设比较绕,而且和"创建 + 渲染都正常"的事实有点冲突 —— 如果 sync 真的把节点
驱赶到了 wb_root,project 白板上的查询应该返回空,节点根本不会渲染。**优先级低于 H1**。

### 4.3 ⭐ H3 — 事件冒泡 / dropdown 关闭时机问题(纯前端)

**论点**: 在 project 白板上,某种 DOM 事件冒泡链路被改了,比如 `withStopBubble` 没起
作用,导致点击色块或菜单项时,事件被外面的 canvas / wrapper 捕获,触发了 close-on-outside
而吞掉 click。这能解释:

- set_color: 色块 click 没触发 onClick 处理器
- draw_connection: 菜单项 click 没触发 onDrawConnectionFrom

但**不能解释** edit_title 为什么在 project 白板上正常 —— 它是同一个 dropdown 的同一
个 plain ui_kind 菜单项。如果是事件冒泡问题,edit_title 也应该挂掉。

**优先级低**,除非发现某种 ui_kind 区分(比如 plain 和 custom 走不同的 click 路径)。

### 4.4 ⭐ H4 — Rust `entity_connect` 拒绝 `projects/{name}` 上下文里的 from/to id

**论点**: Phase A 的 Edge 判别联合重构(commit `90d727a` 等)把 `EntityGraph::connect`
改成强类型的 `Edge` 判别联合。可能某个变体的合法性校验依赖 entity 的 whiteboard_id,
而当 from 和 to 都在 `projects/super-tauri` 时,某个 SQL 查询失败或 enum 解析失败,
但错误被吞掉了(没在前端 console 出现 —— 因为 unwrapCommand 的 catch 只 console.error)。

**怎么验证**:
1. dev server 打开浏览器 DevTools console
2. 在 project 白板上点 Draw connection → 点另一个节点
3. 看 console 有没有 "建立连线失败:" 的 error log
4. 看 DB `edges` 表里有没有新行写入

### 4.5 ⭐ H5 — `commands.noteUpdate` / `questionUpdate` / `taskSetColor` 的某个参数序列化失败

**论点**: bindings.ts 的 typed wrapper 接受 string,但 wb_id 含 `/` 时某层(serde / specta /
Tauri IPC)处理不当 → 命令调用静默失败。

**优先级低**,因为:
- task_create 在 project 白板上明确成功了(用户 Task 2 验证截图)
- edit_title 在 project 白板上明确成功了(本次 P1 Task 4 验证)
- 它们走的是 noteUpdate / questionUpdate / taskUpdate **同一组命令**,只是参数不同
- 如果是序列化挂了,edit_title 也会挂

排除掉的可能性比较高,但保留这一项是因为它最容易验证(打印 IPC 错误就够了)。

### 4.6 H6 — TaskNode 的 color 渲染本身缺失(只解释 task 单一路径)

`src/components/keysight/nodes/TaskNode.tsx` 没有任何 `task.color` → background style
的代码。所以即便 Rust 成功更新了 color,TaskNode 也不会显示出来。

但**不能解释** Note / Question 的 set_color 也失效 —— 这两个组件的 color 渲染是独立
的(NoteNode.tsx / QuestionNode.tsx),需要单独看。

**这一项是单独的 cleanup task**(Phase B2 漏给 TaskNode 接 color 渲染),不是本 bug 的
完整解释。

---

## 5. 推荐排查顺序

按从最容易验证到最难的顺序:

1. **打开 DevTools console**(Cmd+Opt+I 在 Tauri webview),在 project 白板上重复操作:
   - 点色块 → 看 console 有没有 "更新 note 颜色失败:" / "更新 task 颜色失败:" / "更新 question 颜色失败:" 之类的 error log
   - 点 Draw connection → 点第二个节点 → 看有没有 "建立连线失败:" log
   - **如果有 error**:打印 stack trace,把 Rust 错误内容贴出来 → 直接定位
   - **如果没 error**:进入 step 2(说明命令"成功"了但效果没出来)

2. **检查磁盘和 DB** 看哪一层数据没真的更新:
   - 在 project 白板上设一次 color
   - 看 vault 里对应的 .md 文件 frontmatter `color` 字段变了没
   - 看 `keysight.db` 的 entities 表对应行的 color 列变了没
   - **如果 DB 变了 + 文件变了**: 前端 invalidate / 缓存 / 渲染问题
   - **如果 DB 没变 / 文件没变**: Rust update 路径有 bug,从 H1/H2 入手

3. **bisect git 历史** 确认 bug 引入时间:
   - 把 commit 切到 Phase B2 之前(比如 `2026-04-13` 那批)
   - 复现是否依然存在
   - 二分查找定位首个表现 bug 的 commit
   - 这能告诉你是不是 Phase B2 的 sync / wb_id 派生改动引入的

4. **手动写 Rust 单元测试** 验证 H1:
   - 写一个测试: `domain::note::create(conn, "projects/super-tauri", title, content, color)`
     → 期望 .md 文件落在 `whiteboard/projects/super-tauri/` 子目录下
   - 写一个测试: 然后 `domain::note::update(conn, note_id, ..., color)` → 期望 color 字段
     在 frontmatter 里
   - 这两个测试可能 Red,直接揭示 H1 是否成立

---

## 6. 关键代码路径(供下一个 agent 入口阅读)

### 6.1 前端

| 文件 | 关注点 |
|---|---|
| `src/components/keysight/nodes/NodeContextMenu.tsx` | 菜单渲染、color 色块、`CapabilityItem` 的 dispatch |
| `src/components/keysight/nodes/NodeCapabilityCatalog.ts` | applies_to / handlers Pick / NODE_CAPABILITIES 数组 |
| `src/components/keysight/nodes/EntityNode.tsx` | `noteMenu`/`questionMenu`/`taskMenu` useMemo 的 handlers 闭包 |
| `src/components/keysight/GraphView.tsx` (line ~1000-1210) | `menuHandlers` useMemo:`onSetNoteColor`/`onSetQuestionColor`/`onSetTaskColor`/`onDrawConnectionFrom` 实现 |
| `src/components/keysight/GraphView.tsx` (line ~1269-1298) | `handleSelectEntity` 的 drawing mode 第二次点击逻辑 |
| `src/components/keysight/nodes/TaskNode.tsx` | **未读取 `task.color`** —— 单独缺口 |
| `src/components/keysight/nodes/NoteNode.tsx` | 检查它读不读 `note.color` 设置 background |
| `src/components/keysight/nodes/QuestionNode.tsx` | 同上 |
| `src/components/keysight/hooks/useWhiteboardData.ts` | tanstack query 的 query key 设计:`["notes", whiteboardId]` 等 |

### 6.2 后端

| 文件 | 关注点 |
|---|---|
| `src-tauri/src/modules/keysight/domain/note.rs` | `create/update` 怎么决定 .md 文件路径,wb_id 怎么用 |
| `src-tauri/src/modules/keysight/domain/question.rs` | 同上 |
| `src-tauri/src/modules/keysight/domain/task.rs` | task 的 create / update / set_color 的文件路径派生(已知用 ProjectName) |
| `src-tauri/src/modules/keysight/domain/sync.rs` | `derive_whiteboard_id` 的逻辑 —— "loose" 文件如何 quarantine |
| `src-tauri/src/modules/keysight/commands.rs` | `note_update` / `question_update` / `task_set_color` / `entity_connect` 的薄壳 |
| `src-tauri/src/modules/keysight/parser.rs` | `write_color_frontmatter` 的实现(Phase B2 加的) |

---

## 7. 关键 commit / 上下文

最近的相关历史(从最旧到最新):

```
90d727a feat: Phase A type sketch — 6 newtype id + EntityId + ObsidianLink + 8 Edge 变体
be7ea8b refactor: 子阶段 2a — connect 签名强类型化 + 绷带退场
675f764 feat: 子阶段 2b — reader 穷尽 match (note.rs / alias.rs)
6a37f25 feat: Phase B1 type sketch — NodeCapabilityCatalog
29d1fb8 refactor: Phase B1 完成 — NodeContextMenu 单一渲染路径
18ba92a feat: Task 节点菜单 type sketch
2ba46c6 feat: Task 节点菜单装配
68bde5f test: Task 节点菜单 NodeContextMenu task variant
154ceae test: harness-check-tests 补缺
a4290e5 feat: Phase B2 sub-stage 1 — models.rs 加 color 字段
16d91df feat: Phase B2 sub-stage 2+3 — task domain 重写 (ProjectName, parse_task_status)
133e2b9 feat: Phase B2 sub-stage 4a — question color
d6c6f18 feat: Phase B2 sub-stage 5 — Commands 层 (task CRUD + card_set_color)
bf96491 feat: Phase B2 sub-stage 6+7 — TS catalog 扩张 + 测试扩展
f8adb76 docs: Phase B2 完成
874ae95 fix: Phase B2 post-review — P0 数据安全 + P1 沉默错误修复
7e2f828 docs: handoff 预置 4 个 task
c91f7f7 fix: list_whiteboards 递归 projects/* 嵌套白板         ← 本 session
c59cf7e feat: toolbar 加 Task 创建按钮(project 白板专属)        ← 本 session
7f3e24d feat: TaskNode 接入 edit_title inline editor (P1 Task 4) ← 本 session
```

### 7.1 重点检查的 commit

- **`a4290e5`** — 给 models.rs 加 color 字段。问题: 是不是只给 schema 加了字段但对应的
  query 路径 / serializer / file roundtrip 不完整?
- **`16d91df`** — task domain 完全重写,给 task 加了 ProjectName 强约束。问题: note /
  question 的对应路径**没有同等处理**,可能在 wb_id 含 `/` 时悄悄走到错的代码分支
- **`133e2b9`** — question color 加 default sentinel。问题: 这个改动有没有可能让正常
  color 写入也走到了 sentinel 路径,被当成 "清空" 处理?
- **`bf96491`** — TS catalog 扩张,把 set_color 加到 task/note/question 的 applies_to。
  这个 commit 让 set_color 的菜单项**首次出现**在 task 上 —— 说不定 Note / Question
  的 set_color 在更早的 commit 上就已经有 bug,只是 task 进来后用户才想去测它
- **`874ae95`** — Phase B2 post-review 修复。这里改了 ProjectName::new、parser::write_frontmatter
  / write_color_frontmatter 的失败路径(从 silent 改 loud)。**值得检查** write_color_frontmatter
  在 wb_id 含 `/` 时的行为

---

## 8. 排查时的硬约束(项目规则,不要绕开)

1. **TDD Red→Green 人工确认关卡**: CLAUDE.md L0 明确要求 `domain` 层任何修改都按
   Red→人工确认→Green→人工确认 走。**不要跳过用户确认就直接改 Rust domain 代码**,
   否则下次提交会被 review 退回
2. **不要为了"修 bug"破坏 deep module 边界**: 如果根因是 H1(note/question 的文件路径
   派生不识别嵌套 wb_id),应该在 `domain::note` / `domain::question` 内部修(把路径
   派生函数收紧),而不是在 GraphView 加前端 patch
3. **L0 防火墙原则**: 如果发现 `domain::note::create(wb_id: &str, ...)` 用 `&str` 接
   wb_id,而且函数内部按 `/` 切割 —— 这就是 stringly typed 逃生舱口,根因之一就是
   wb_id 没建模成 `WhiteboardId` newtype。修复时**优先**新增 `WhiteboardId` 强类型(
   handoff 已经把它列在 Phase B3 P2 推迟项里,这次顺手做掉合理)
4. **bug 复现测试要保留**: TDD 修 bug 的标准流程是先写一个 Red 测试精确复现 bug,然后
   修复,然后该测试变 Green 永久保留为回归防护。**不要修了就删测试**
5. **修完后必须手动 dev server 验证**: 这是 UI bug,纯测试 Green 不代表用户体验 OK。
   必须实际在 dev server 里点一遍 set_color 和 draw_connection,看到 UI 真的有反馈

---

## 9. 用户已经验证的事实(可信前提)

下列是用户在本 session 里**亲自跑 dev server** 验证的 ground truth,排查时可以直接当
前提用,**不需要重新验证**:

- ✅ `projects/super-tauri` 白板能从 Boards 下拉里看到(Task 1 验证)
- ✅ Toolbar 的 Task 按钮能在 project 白板上创建 task,文件落到磁盘正确位置,
  card 渲染正确(Task 2 验证,有截图)
- ✅ Task 的 `Edit title` inline editor 能进入编辑、Enter 提交、文件 rename(Task 4 验证,
  通过本 bug 报告间接确认)
- ✅ wb_root 上 set_color 和 draw_connection 工作正常

下列是用户**报告的 bug 现象**,排查时要确认是否复现:

- ❌ `projects/super-tauri` 白板上,Note / Question / Task 的 set_color 点击色块**菜单
  正常打开 + 色块出现** 但**节点背景颜色不变化**
- ❌ `projects/super-tauri` 白板上,Note / Question 的 Draw connection **菜单正常打开
  + 点击 Draw connection 项**但**点了完全没反应**(无 source 高亮、无 cursor 变化、第
  二次点击其他节点不画线)

---

## 10. 不要做的事

1. **不要假设是本 session 的 3 个 task 引入的** —— 它们都没碰 set_color / draw_connection
   代码路径。先用 git bisect 确认 bug 引入时间,再下结论
2. **不要只看前端** —— 前端代码已经反复 grep,没有 wb_id 守卫。问题更可能在 Rust
   写回路径或 sync 路径
3. **不要用 `_ => {}` 通配** 来吞 enum 错误 —— 见 CLAUDE.md L0 的 keysight 踩坑样例 1
4. **不要修了 Note 就忘 Question 和 Task** —— 三种 entity 都有同样症状,根因可能是共享
   的工具函数;要么三个一起修,要么用一个统一抽象修一处
5. **不要在 GraphView 里加 wb_id 特例分支** —— 那是把 stringly typed 病灶进一步扩散。
   要修就在数据建模层修(WhiteboardId newtype / 路径派生函数)
6. **不要在没有用户确认的情况下提交 commit** —— TDD Red/Green 关卡是人工确认机制,
   尤其是涉及 Rust domain 修改时

---

## 11. 完成标准

修复完成需要:

1. 用户在 dev server `projects/super-tauri` 白板上点色块 → 节点背景颜色立即变化(Note /
   Question / Task 三种全部生效)
2. 用户在 `projects/super-tauri` 白板上点 Note / Question 的 Draw connection → 点另一
   个节点 → 画布上出现连线(和 wb_root 上行为一致)
3. 至少 3 个新的回归测试(domain unit test 或组件测试)精确锁住 bug 不复发
4. `cargo test --workspace` + `cargo clippy -- -D warnings` + `pnpm test` + `pnpm build`
   全绿
5. 修复 commit 单独成 commit(不和这次的 3 个 task commit 混淆),commit message 引用
   本文档路径
6. 如果根因涉及 wb_id 建模问题(H1 / H2),顺手做 `WhiteboardId` newtype 落地(handoff
   Phase B3 P2 推迟项),根除一类 bug 而不是 patch 单点

---

## 12. 给排查 agent 的一句话总结

> 三种节点同时挂掉、edit_title 单独 OK、wb_root 完全正常 —— **指向数据写回路径里
> wb_id 派生的某个分支在 `/` 出现时退化或失效**。优先看 H1(`domain::note`/`question`
> 的文件路径派生),其次 H4(`entity_connect` 的 enum 解析)。**不要先看前端**,前端
> 已经穷举过了。
