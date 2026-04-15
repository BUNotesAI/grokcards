# P3 Kanban V1.1 — Subtask + Double-Click Edit Design

> **Scope:** P3 Kanban V1.1(Phase 6),在 V1(commit `80819dc` 收口)的基础上增加 **Subtask 抽象** 和 **双击编辑 modal**。
>
> **Precedent:** 本 feature 的全部架构决策均已由用户在 V1 收口前的对话中确认(见 §1 决策清单)。本文档是一份执行设计,把决策落地为具体的 file-level / function-level / test-level 计划,供 subagent-driven-development 或 executing-plans 流程使用。
>
> **TDD Red/Green 人工确认关卡**: 用户已授权本次 session 跳过人工确认(2026-04-14 授权,session-scoped)。测试本身仍严格执行(Red 测失败 → 写代码 → Green 测通过 → Refactor),但不暂停等确认。
>
> **L0 硬约束提醒**: 本 feature 跨 Rust + TS,必须守 CLAUDE.md 的硬约束 ——「TS 只负责 UI,Rust 独占业务」、SQLite 是 source of truth、建模优先 + 强类型、测试真实代码路径、Trait-First 等。任何"TS parse checklist"、"TS 拼 markdown body"、"checklist 文本作为 stringly-typed blob"都是直接违规。

---

## 1. 背景与产品动机

P3 V1 交付后,用户提出关键语义澄清:

- **Task 粒度 = area** —— 一个 task 不是一个原子 actionable TODO,而是一个粒度偏大的工作方向(area / initiative)。例如"实现 keysight 画布 zoom 支持"是一个 task,而不是"添加 onWheel handler"
- **Body 承载 checklist** —— task 的 markdown body 用 **GFM checklist**(`- [ ]` / `- [x]`)列出这个 area 的具体子步骤
- **子任务语义** —— 每一项 checklist item 对标一个 **subtask**,应在 Kanban 卡片上有进度可见性(`done/total`),并在双击编辑时可勾选/增删/改文本

这个语义改变了 Kanban V1 的隐含假设(每个 task 是单一 actionable 单位),要求 V1.1 同时落地 **Subtask 结构化建模** 和 **双击编辑交互入口**。

非目标(V1.2+):

- ❌ 嵌套 checklist(`- [ ]` 下再缩进 `- [ ]`)
- ❌ Subtask 独立查询 / 跨 task 聚合 / status 扩展
- ❌ Subtask 作为一等 entity(不写入 `entities` 表)
- ❌ Subtask 可拖拽排序(只在编辑器内改)

---

## 2. 用户已确认的 5 项架构决策

**2.1 Parser 放 Rust 侧**

TaskEntity 加 `subtasks: Vec<Subtask>` 字段,在 `domain::task::get / query_all / query_kanban` 的 reader 路径统一 parse body,填充 subtasks。TS 只读不 parse。

**Why**: L0 硬约束 ——「TS 只负责 UI,Rust 独占业务」。Parser 是业务逻辑(决定什么是 subtask、什么格式合法),放 TS 会制造 test theater 温床(TS 复制 Rust parse 规则)和跨语言 drift 风险。

**Also why**: 未来如果要对 subtask 做统计(进度 badge)、跨 task 搜索、backlink,都需要 structured representation。一次在 Rust parse 完、reader 填充,所有上层直接吃,比每个 reader 单独做 TS 解析干净。

**2.2 Subtask 数据结构**

```rust
pub struct Subtask {
    pub text: String,       // checklist item 文本(trim 后,不含 `- [ ]` 前缀)
    pub done: bool,         // `- [ ]` → false,`- [x]` / `- [X]` → true
    pub line_index: usize,  // 在 body 行数组中的 0-based 行索引(用于 write path 精确定位)
}
```

**Why `line_index` 而不是顺序数字 id**:
- Subtask 不是 entity,不写 DB,不做跨会话持久化 id
- `line_index` 让 write path 可以在保留非 checklist 行(注释、标题、普通段落)的前提下,只重写 checklist 相关行
- 如果用抽象顺序 id,write path 必须把整个 body 完全按 subtasks 数组从零重生成,会丢失用户在 checklist 之间写的自由文本(这是**不可接受的 data loss**)

**V1 限制**: 只支持 **顶层** 顶格 GFM checklist(行的起始匹配 `- [ ]` / `- [x]`,允许前置空白但不支持嵌套)。嵌套行(父项的子勾选)在 V1 **不识别为 subtask**,保留为原始文本不动。

**2.3 Write path**

新 command `task_update_subtasks(task_id: String, subtasks: Vec<Subtask>)`,Rust 负责:

1. 读当前 task(含当前 body + file_path)
2. 调 `render_subtasks_into_body(old_body, subtasks)` 生成新 body —— 保留所有非 checklist 行不动,只按 subtasks 数组重写 checklist 行
3. 重新渲染完整 markdown(frontmatter + new body)
4. 写文件 + sync_file 回 DB

**Why Rust re-renders not TS**: L0 硬约束 ——「TS 不构造持久化格式」。TS 拼 markdown body 一旦漂移(frontmatter 格式、行尾空白、escape 规则),SQLite 和 markdown 两边会静默分叉。Rust 是唯一 source of truth for 文件格式。

**2.4 Kanban 卡片显示**

`KanbanCard` 右上显示 `done/total` 徽章(e.g. `2/5`),subtasks 为空时 **不显示徽章**(避免空进度 `0/0` 噪音)。不展开列出 items —— Kanban 列宽有限,展开会让列过拥挤、视觉噪音大。详情在 double-click modal 里。

**2.5 Modal 内容范围**

`TaskEditModal` 包含:
- **title** (input,可改)
- **content**(textarea,3-5 行初始高度,可改)
- **Checklist 编辑器**(增加 item / 删除 item / 勾选 done / 改 text)
- **status**(select,同 CreateTaskModal,可改)
- **area**(input,可改)
- **color**(color picker 或同 create 的现有方式,可改)
- **project**(只读显示,**不允许改**)

**Why project 只读**: `domain::task::update` 明确"不允许通过 update 改 project"(CLAUDE.md L0 Operation Contract),改 project 需要跨目录迁移文件,是另一个独立操作(V1.1 scope 外)。

---

## 3. 数据模型详细规格

### 3.1 `Subtask` struct(新增)

**位置**: `src-tauri/src/modules/keysight/models.rs`

```rust
/// Task body 中的一项 GFM checklist item(子任务)。
///
/// Parse 规则见 `domain::task::parse_task_checklist`。V1 只识别顶层顶格
/// `- [ ]` / `- [x]` 形式,嵌套和其他标记(`*`、`1.` 带 checkbox)不识别。
///
/// Invariant: 本类型由 `parse_task_checklist` 构造,text 已 trim 两端空白,
/// line_index 指向原始 body 行数组的 0-based 索引。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub text: String,
    pub done: bool,
    pub line_index: usize,
}
```

**Why derive `PartialEq + Eq`**: 测试断言需要(`assert_eq!(parsed, expected_vec)`)。

**Why `serde rename_all = "camelCase"`**: 和 TaskEntity 一致,bindings.ts 生成 `lineIndex` 而不是 `line_index`。

### 3.2 `TaskEntity` 扩展

**位置**: `src-tauri/src/modules/keysight/models.rs:273` (已有 struct)

```rust
pub struct TaskEntity {
    pub id: String,
    pub title: String,
    pub content: String,
    pub whiteboard_id: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub area: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    /// V1.1 新增 —— 从 `content` 中 parse 出的 GFM checklist 子任务列表。
    /// 空 Vec 表示 body 没有任何 checklist item(不是"未解析")。
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
}
```

**`#[serde(default)]` 原因**: 兼容历史 frontmatter-only task(虽然当前全是 file-backed,但规则统一)。

---

## 4. Parser 规格 `parse_task_checklist`

**签名**:

```rust
pub(in crate::modules::keysight) fn parse_task_checklist(body: &str) -> Vec<Subtask>
```

**返回 `Vec<Subtask>` 而非 `Result`**: Parser 是 **lenient** 的 —— 非 checklist 行不是错误,直接跳过。没有任何输入可以让这个函数失败。不合法的 checklist 语法(`-[x]` 无空格、`- [y]` 非法 marker)被当作普通文本跳过,不报错、不 log。

**行级识别规则(正则伪代码)**:

```
前置空白 (空格/tab) → 不影响顶层判定(但 V1 要求真正顶层 —— 见下)
"- " 或 "* "       → list marker
"[ ]" 或 "[x]" 或 "[X]" → checkbox 标记
至少 1 个空白
剩余文本            → subtask.text(trim 两端)
```

**顶层判定(V1 简化)**: V1 不支持嵌套 checklist。实现方式是 **拒绝前置空白超过 0** —— 行必须以 `- [ ]` / `- [x]` / `- [X]` 起始(也接受 `* [ ]` 之类 —— 或者限定只认 `-`,这个细节在 Phase 6.1 的测试里 pin down)。

**建议 V1 规则(最严格):**

```
^- \[( |x|X)\] (.+)$
```

- 只认 `-`(不认 `*`、`+`)
- checkbox 内只认 ` ` / `x` / `X`
- checkbox 和 text 之间必须有空格
- text 必须非空

其他格式视作普通文本。用户写 `- [X]Do something` 会被跳过,这是合法的 V1 行为。测试里需要一条断言。

**Line index**: body split by `\n`,每行的索引就是 `line_index`。保留所有行(包括空行)的索引,所以在有 frontmatter 之外的上下文插入 checklist 时也能精确回写。

**Edge cases(必须测试)**:

| # | 输入 | 期望 |
|---|---|---|
| 1 | 空字符串 `""` | `vec![]` |
| 2 | 纯文本无 checklist | `vec![]` |
| 3 | 单一 `- [ ] Buy milk` | `vec![Subtask { text: "Buy milk", done: false, line_index: 0 }]` |
| 4 | 单一 `- [x] Write tests` | `vec![Subtask { text: "Write tests", done: true, line_index: 0 }]` |
| 5 | 单一 `- [X] Capital X` | `vec![Subtask { text: "Capital X", done: true, line_index: 0 }]` |
| 6 | 混合 done/undone 多行 + 空行 + 自由文本 | 只返回 2 个 Subtask,line_index 指向实际行号 |
| 7 | 前置空白的嵌套 `  - [ ] nested` | 跳过,V1 不识别,返回 `vec![]` |
| 8 | 无空格 `- [x]No space` | 跳过,返回 `vec![]` |
| 9 | 非 `-` marker `* [ ] star marker` | 跳过,返回 `vec![]` |
| 10 | checkbox 内非法字符 `- [y] invalid` | 跳过,返回 `vec![]` |
| 11 | `- [ ]` 后文本为空 `- [ ] ` | 跳过(text 必须非空),返回 `vec![]` |
| 12 | checklist 前后有自由文本 + 行间插入非 checklist 行 | 正确分离,checklist 行的 line_index 精准 |

---

## 5. Write Path: `render_subtasks_into_body` + `task_update_subtasks`

### 5.1 `render_subtasks_into_body` helper

**签名**:

```rust
pub(in crate::modules::keysight) fn render_subtasks_into_body(
    old_body: &str,
    new_subtasks: &[Subtask],
) -> String
```

**逻辑**:

1. 把 `old_body` split 成 Vec<String> 行数组
2. 找出所有原 checklist 行的 index(通过 `parse_task_checklist(old_body)` 拿到 `line_index` 集合)
3. 从行数组删掉所有原 checklist 行
4. 把 new_subtasks 的渲染结果(每行 `- [ ] text` 或 `- [x] text`)**插入** 到 —— 这里需要决策(见下)
5. join 回字符串

**关键决策:新 subtasks 插入位置**

两个选项:

- **A. 替换位置**: 在原 checklist 行的**首个 index 位置**按新数组全量写入,保留前后非 checklist 行不动
- **B. 追加末尾**: 删除所有原 checklist 行,在 body 末尾追加新 subtasks

**推荐 A**,因为保留 checklist 与周围自由文本的相对位置语义(用户可能在 checklist 上面写了一段说明,下面写了一段总结)。

**边界**:

- 原 body 没有 checklist 行 + new_subtasks 非空 → 追加到 body 末尾(B 退化),确保末尾有换行
- 原 body 有 checklist 行 + new_subtasks 为空 → 删除所有原 checklist 行,保留周围文本
- 原 body 为空 + new_subtasks 非空 → body 变成 `- [ ] foo\n- [x] bar\n`

**幂等性测试**: `render_subtasks_into_body(body, parse_task_checklist(body)) == body`(**不一定**成立,因为归一化:大小写 `X → x`、末尾空格 trim、`[ ]` 规范化。但再跑一次 parse 后应该完全一致)。二次迭代幂等:

```
let once = render(body, parse(body));
let twice = render(once, parse(once));
assert_eq!(once, twice);  // 归一化后的 body 再 render 一次不变
```

### 5.2 `task_update_subtasks` command

**位置**: `src-tauri/src/modules/keysight/commands.rs`

**签名**(Rust + tauri-specta):

```rust
#[tauri::command]
#[specta::specta]
pub fn task_update_subtasks(
    state: State<'_, KeysightState>,
    id: String,
    subtasks: Vec<Subtask>,
) -> Result<TaskEntity, AppError>
```

**返回 `TaskEntity`** 而不是 `()`,因为 TS 侧需要新的 subtasks 回流(以防万一 parse 结果和前端发送的不完全一致 —— 比如归一化)。

**流程**:

1. `get` 当前 task
2. `render_subtasks_into_body(current.content, &subtasks)` 生成新 body
3. 调 `domain::task::update(TaskUpdateInput { content: Some(&new_body), ..keep })` —— 复用现有 update 逻辑,把新 body 作为 content 传入
4. `get` 更新后的 task 并返回(含重新 parse 出的 subtasks)

**Why 复用 `update` 而非单独实现**: update 已经处理了 markdown 文件重写 + sync_file + file_mtimes 更新 + 防改 project 校验。重新实现一次是 test theater 的另一种形态(copy/paste 业务逻辑)。

### 5.3 副作用矩阵更新

`CLAUDE.md` 副作用矩阵加一行:

| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|---|---|---|---|
| `task_update_subtasks` | 等价 `task_update(content)` + `entities`, `task_fields`, `file_mtimes`, `entities_fts` + markdown 文件 | DB 重写 + 文件 body 重写(保留 frontmatter)+ 可能 rename(但 title 没改所以不会 rename) | domain unit test + TS command test |

---

## 6. TS UI 层规格

### 6.1 `KanbanCard` progress 徽章

**位置**: `src/components/kanban/KanbanCard.tsx:28` 起 JSX 修改

**逻辑**:

```tsx
const total = task.subtasks.length;
const done = task.subtasks.filter((s) => s.done).length;
const showProgress = total > 0;

// JSX 右上角
{showProgress && (
  <div
    className="ml-auto rounded bg-secondary px-1.5 py-0.5 text-xs text-secondary-foreground"
    data-testid="kanban-card-progress"
    aria-label={`${done} of ${total} subtasks done`}
  >
    {done}/{total}
  </div>
)}
```

**布局调整**: 原 card 现在用 `<div className="text-sm font-medium">{task.title}</div>`,加 progress 徽章后需要 flex row 布局(title 左,progress 右)。

**Empty 态**: `subtasks.length === 0` 时**不显示徽章**(避免 `0/0` 噪音)。

**测试**:

- `total === 0` 时不渲染 `kanban-card-progress`
- `total === 3, done === 1` 时显示 `1/3`
- `total === 3, done === 3` 时显示 `3/3`

### 6.2 `TaskEditModal` 组件(新建)

**位置**: `src/components/kanban/TaskEditModal.tsx`(新文件)

**Props**:

```ts
interface TaskEditModalProps {
  task: TaskEntity;  // parent 条件渲染 `{edit.open && <TaskEditModal task={edit.task} ... />}`
  availableProjects: string[];
  onSubmit: (data: {
    id: string;
    title: string;
    content: string;
    subtasks: Subtask[];
    status: TaskStatus;
    area: string | null;
    color: string | null;
  }) => Promise<void>;
  onCancel: () => void;
}
```

**Internal state**(每次 open 从 task 初始化,靠 conditional render 保证 fresh):

```ts
const [title, setTitle] = useState(task.title);
const [content, setContent] = useState(task.content);
const [subtasks, setSubtasks] = useState<Subtask[]>(task.subtasks);
const [status, setStatus] = useState(task.status);
const [area, setArea] = useState(task.area ?? "");
const [color, setColor] = useState(task.color ?? "");
```

**Checklist 编辑器 sub-component**(`ChecklistEditor`,可以 inline 在同文件):

- 每一项 render 为一行 `[checkbox] [input text] [× delete button]`
- 底部 `+ Add item` 按钮,点击 append 一个 `{ text: "", done: false, line_index: subtasks.length }`
- 勾选 → `setSubtasks(prev.map(s => s.id === x ? {...s, done: !s.done} : s))`(这里要 key 用什么?用 index 够吗?—— 见下)
- 删除 → filter 掉对应 index
- 改文本 → `setSubtasks(prev.map(...))`

**Key 策略**: 编辑器内部用 `idx` 作为 React key(因为 subtasks 无持久 id)。但这会导致中间删除时 React 复用 DOM 节点出现 text state 错位。**解决**: 用 `line_index` 作为 React key(原本就是 usize,唯一且稳定),新增的 item 用递增的临时值(e.g. `subtasks.length + 1`,或 `Math.max(...line_indices) + 1`)。Submit 时 Rust 会重算 line_index。

或者更干净:**在组件内部维护一个 "ui_key" 字段**(用 React `useId` + index 或自增计数器),专门作为 render key,不进持久化。符合 L0 规则"纯 UI 状态 key"(CLAUDE.md 明确允许 `ui_` 前缀)。

**推荐**: 用一个包装类型 `SubtaskWithKey = Subtask & { uiKey: string }`,uiKey 在组件本地生成(`useId()` 或 `crypto.randomUUID()`),**不出组件**,提交给 onSubmit 时剥掉 uiKey 还原 Subtask。

Submit 时 `subtasks: subtasks.map(({uiKey, ...rest}) => rest)`。

**Submit 逻辑**:

```ts
const handleSubmit = async () => {
  await onSubmit({
    id: task.id,
    title: title.trim(),
    content: content,  // content 可能被 re-rendered,但这里 commit TS 视图的 content(由 checklist editor 同步)
    subtasks: subtasks.map(({uiKey, ...rest}) => rest),
    status,
    area: area.trim() || null,
    color: color || null,
  });
};
```

**Critical decision - content vs subtasks 同步**:

用户可以同时:
- 在 content textarea 里手动编辑文本(包括手写 `- [ ]`)
- 在 checklist editor 里勾选/增删

**这是潜在冲突源**。V1 策略:

**推荐 A**: Modal 内部 **content textarea 和 checklist editor 独立操作** —— textarea 只改非 checklist 文本部分,checklist editor 只改 checklist 部分。Submit 时:

1. 先把 TS 视图的 `content`(textarea 内容)和 `subtasks`(editor 状态)合并成一个逻辑 body
2. 合并方式: 取 TS 当前 content,**用 subtasks 替换掉里面的 checklist 行**(模拟 Rust 的 `render_subtasks_into_body` 但在 TS 侧预览)
3. 但**最终提交到 Rust 是分开的** —— title/content 走 `task_update`, subtasks 走 `task_update_subtasks`
4. 两个 command 顺序执行,task_update 先(改非 checklist 文本),task_update_subtasks 后(改 checklist)

**Why 两 command 而非一个大 command**: 职责分离 —— update 管非 subtask 字段,update_subtasks 专管 checklist。两个 command 的业务规则独立演化,不互相污染。

**Why 不单 command 合并**: 会让 command 签名膨胀到 `task_update_everything(id, title, content, status, area, color, subtasks)`,回到"god command"形态,违反 L0「窄接口」原则。

**交易原子性**: 两次 command 中间失败(第一次改完 title/content,第二次改 subtasks 失败)会留下 **部分更新**。V1 接受这个风险,不做跨 command 事务。失败时前端 loud console.error,refetch query 刷新视图,用户看到实际状态。

**Alternative B(更简单,V1 推荐)**: TaskEditModal **只管 title + status + area + color + checklist**,content textarea 里 **不显示 checklist 行**(预先过滤掉),用户只能改 checklist 外的自由文本。checklist 行在提交时由 subtasks 渲染插入。这样 content 和 subtasks **不会冲突**。

选 **B**。它牺牲了"在 textarea 里手写 checklist markdown"的能力(高级用户可能想要),但大大简化 V1 的同步逻辑,消灭交易原子性问题。V1.2 再考虑允许在 textarea 里混写。

**最终 Submit 逻辑(B 方案)**:

```ts
const handleSubmit = async () => {
  // 两个 command 串行:先 update 非 checklist 字段,再 update subtasks
  // content 这里只是"非 checklist 文本",Rust 的 task_update_subtasks 会把 subtasks merge 进去
  await unwrapCommand(
    commands.taskUpdate(
      task.id,
      title.trim() || null,
      content || null,  // 仅非 checklist 部分
      status,
      area.trim() || null,
      color || null,
    ),
  );
  await unwrapCommand(
    commands.taskUpdateSubtasks(
      task.id,
      subtasks.map(({uiKey, ...rest}) => rest),
    ),
  );
  invalidateAllTaskCaches(queryClient);
  onCancel();  // close modal
};
```

**但这引入了新问题**: task_update 传入的 content **不包括 checklist 行**,但 Rust 侧的 task_update 会把这个 content **作为新的完整 body 重写文件**,把 checklist 部分抹掉 —— 然后 task_update_subtasks 才插回去。这有一个时间窗口 body 是"无 checklist"状态。

**解决**: TS 侧传给 task_update 的 content **是完整 body(含保留的原 checklist 行)**,而不是过滤后的。TS 不过滤,Rust 的 task_update_subtasks 再做 merge。具体:

```ts
// TS 侧 modal 内部只管 "非 checklist 部分",但 submit 时拼回完整 body
const nonChecklistContent = content; // textarea 状态
// 先读出 task 当前 body 里的 checklist 行(或在 open modal 时抓)
// 然后 content = nonChecklistContent + "\n" + 原 checklist 行的原始字符串
```

**这把问题复杂化了**。更干净的做法:**把 task_update 和 task_update_subtasks 合并** 成一个新 command `task_update_full`,V1.1 scope 接受一个 god-ish command,scope 限定 task 更新。

**最终决策**: V1.1 新增 **一个** 新 command:

```rust
#[tauri::command]
#[specta::specta]
pub fn task_update_with_subtasks(
    state: State<'_, KeysightState>,
    id: String,
    title: Option<String>,
    content: Option<String>,           // 非 checklist 部分的文本,可为 None 保持不变
    subtasks: Option<Vec<Subtask>>,    // None 保持不变
    status: Option<TaskStatus>,
    area: Option<String>,
    color: Option<String>,
) -> Result<TaskEntity, AppError>
```

Rust 内部:
1. `get` current task
2. 如果 `subtasks` 是 Some,调 `render_subtasks_into_body(content.unwrap_or(current.content), subtasks)` 生成新 body
3. 其他字段按 None=保留 / Some=覆盖 原则
4. 调 `domain::task::update` 统一写入

**Why 不是纯粹加字段到 task_update**: 向后兼容性 —— 现有 task_update 已经被 canvas/toolbar 多处调用,改签名会影响其他非 kanban 场景。新 command `task_update_with_subtasks` 只给 kanban modal 用,canvas 的 inline 编辑继续用旧 task_update(不涉及 subtask 操作)。

**副作用矩阵新行**:

| 写操作 | 影响 | 副作用 | 测试 |
|---|---|---|---|
| `task_update_with_subtasks` | 等价 task_update(content 合并 subtasks)+ frontmatter 字段 | DB 重写 + 文件 body 重写(用 render_subtasks_into_body 合并)+ file_mtimes 更新 | domain unit test(✓ 合并 / ✓ 空 subtasks 清空 checklist / ✓ None 字段保留) |

### 6.3 双击 `KanbanCard` 打开 Edit Modal

**位置修改**:

- `KanbanCard.tsx`: 加 `onDoubleClick: () => void` prop,绑定到外层 `<div onDoubleClick={...}>`。避免和 `@dnd-kit` 的 pointer event 冲突 —— 双击和拖拽手势不同,dnd-kit 的 `useDraggable` 用 pointer move 触发,不影响双击。
- `KanbanBoard.tsx`: 加 `onTaskDoubleClick: (task: TaskEntity) => void` prop,透传给 KanbanCard
- `KanbanView.tsx`: 
  - 新 state: `const [editState, setEditState] = useState<{open: boolean, task: TaskEntity | null}>({open: false, task: null});`
  - `const openEditModal = (task: TaskEntity) => setEditState({open: true, task});`
  - `const closeEditModal = () => setEditState((prev) => ({...prev, open: false}));`
  - `handleEditSubmit` 调 `commands.taskUpdateWithSubtasks(...)` + invalidate + close
  - JSX 加 `{editState.open && editState.task && <TaskEditModal task={editState.task} .../>}`

### 6.4 bindings.ts 回流

新增的 Rust 类型 `Subtask` 和新 command `task_update_with_subtasks` 会通过 `cargo test export_bindings` 自动生成到 `src/bindings.ts`。TS 侧 `import type { Subtask, TaskEntity } from "@/bindings"`。

---

## 7. 执行计划(Phase 6 拆解)

本 feature 按 7 个子 phase 执行,每个 phase 独立 commit,保持 bisect 友好。

### Phase 6.1 — Rust: Subtask struct + parse_task_checklist

**文件**:
- `src-tauri/src/modules/keysight/models.rs` — 加 Subtask struct
- `src-tauri/src/modules/keysight/domain/task.rs` — 加 parse_task_checklist

**TDD Red**:
编写 `tests` 模块新增 12 个 parse 单测(见 §4 edge case 表),跑 `cargo test parse_task_checklist`,确认全部失败。

**TDD Green**:
实现 `parse_task_checklist`(正则或逐行 starts_with 比较),跑测试全绿。

**Commit message**:
```
feat(keysight): parse_task_checklist + Subtask struct (V1.1 Phase 6.1)

GFM checklist parser,V1 只识别顶层顶格 `- [ ]` / `- [x]` / `- [X]`。
12 个 edge case 测试覆盖:空 body / 纯文本 / done/undone 混合 / 嵌套跳过 /
非法格式跳过 / line_index 精确定位等。

为 V1.1 Subtask feature 的数据层打基础,后续 Phase 把 TaskEntity 加
subtasks 字段 + reader 填充。
```

### Phase 6.2 — Rust: TaskEntity.subtasks 字段 + reader 填充

**文件**:
- `models.rs` — TaskEntity 加 `subtasks: Vec<Subtask>` 字段
- `domain/task.rs` — `get / query_all / query_kanban` 三处 mapper 调 `parse_task_checklist(&content)` 填充

**Red**:
- 写测试:create 一个含 checklist body 的 task,call `get`,断言 `task.subtasks` 长度和内容符合预期
- 写测试:create 无 checklist 的 task,断言 `task.subtasks.is_empty()`
- 写测试:query_all / query_kanban 的结果里所有 task 都正确填充 subtasks

**Green**:
- Mapper 里加 `let content = r.get::<_, String>(2)?.trim_end_matches('\n').to_string()` 之后立即 `let subtasks = parse_task_checklist(&content);`,填到 TaskEntity。

**重新生成 bindings**:
```
cargo test --manifest-path src-tauri/Cargo.toml export_bindings
```
验证 `src/bindings.ts` 出现 `Subtask` 类型和 `TaskEntity.subtasks`。

**Commit message**:
```
feat(keysight): TaskEntity.subtasks + reader 填充 (V1.1 Phase 6.2)

TaskEntity 加 subtasks: Vec<Subtask>,三处 SQL reader(get / query_all /
query_kanban)mapper 统一调 parse_task_checklist 填充。bindings.ts 自动
生成 Subtask type,TS 侧可直接读 task.subtasks 无需额外 parse。

测试:含/不含 checklist body 的 roundtrip 验证,跨 reader 一致性。
```

### Phase 6.3 — Rust: render_subtasks_into_body + task_update_with_subtasks command

**文件**:
- `domain/task.rs` — 加 `render_subtasks_into_body` helper
- `commands.rs` — 加 `task_update_with_subtasks` command
- `lib.rs` — `collect_commands![]` 注册新 command
- `CLAUDE.md` — 副作用矩阵加一行

**Red**:
- `render_subtasks_into_body` 测试:
  - 空 body + 空 subtasks → `""`
  - 空 body + 非空 subtasks → `- [ ] foo\n- [x] bar\n`
  - 有 checklist + 非空 subtasks → 原 checklist 行被替换,非 checklist 行保留原位置
  - 有 checklist + 空 subtasks → 原 checklist 行全部删除,非 checklist 行保留
  - 幂等: `render(render(body, parse(body)), parse(...)) == render(body, parse(body))`
  - 大小写归一化: `- [X]` input 经 render 后变 `- [x]`(如果决定归一化;否则保留大小写)
- `task_update_with_subtasks` command 测试:
  - Create task with 3 subtasks → call update_with_subtasks(新 vec) → 断言 task.subtasks 变
  - 同上但只传 subtasks,其他字段 None → 保留 title/status/area/color
  - 传空 subtasks Vec(清空)→ task.content 里 checklist 行消失,非 checklist 文本保留

**Green**:
- 实现 render helper + command + lib.rs 注册
- 重新生成 bindings

**Commit message**:
```
feat(keysight): task_update_with_subtasks command (V1.1 Phase 6.3)

新 command 专管 task 含 subtasks 的更新,内部通过 render_subtasks_into_body
把 subtasks Vec 按 line_index 合并回 markdown body(保留非 checklist 行的
相对位置),然后复用 domain::task::update 的文件重写 + sync 路径。

不改现有 task_update 签名(向后兼容 canvas inline edit 等非 kanban 调用点)。
副作用矩阵新增一行,CLAUDE.md 同步更新。

测试:幂等 / 空 subtasks 清空 / None 字段保留 / 归一化 大小写。
```

### Phase 6.4 — TS: KanbanCard progress 徽章

**文件**:
- `src/components/kanban/KanbanCard.tsx` — JSX 调整 + 加 progress UI
- `src/__tests__/components/kanban/KanbanCard.test.tsx` — 加 3 个新测试

**Red**:
- 测试 `total === 0` 时不渲染 progress
- 测试 `total === 3, done === 1` 显示 `1/3`
- 测试 `total === 3, done === 3` 显示 `3/3`(和 class 颜色不在 V1 scope 区分)

**Green**:
- 实现 flex row 布局 + 条件渲染 progress 徽章

**Commit message**:
```
feat(kanban): KanbanCard 加 subtask progress 徽章 (V1.1 Phase 6.4)

卡片右上显示 done/total 进度(e.g. 2/5),subtasks 为空时不显示徽章避免
噪音。布局改 flex row(title 左 / progress 右)。

测试 +3:空 subtasks / 部分 done / 全 done。
```

### Phase 6.5 — TS: TaskEditModal 组件

**文件**:
- `src/components/kanban/TaskEditModal.tsx` — 新建
- `src/components/kanban/ChecklistEditor.tsx` — 新建(子组件,可 inline 也可单独)
- `src/__tests__/components/kanban/TaskEditModal.test.tsx` — 新建

**Red**:
- 给定 task 渲染 modal,断言 title input 预填 `task.title`
- content textarea 预填 `task.content`(非 checklist 部分)
- checklist editor 显示所有 subtasks,勾选状态正确
- 点击 subtask checkbox → toggle done(内部 state)
- 点 "+ Add item" → checklist 多一行
- 点 "×" delete → checklist 少一行
- 改 subtask text → 内部 state 同步
- Submit → onSubmit 被调用,payload 包含所有字段
- Escape → onCancel

**Green**:
- 实现 modal 和 checklist editor
- uiKey 用 `crypto.randomUUID()` 生成(L0 允许纯 UI key,submit 时剥掉不传 Rust)

**Commit message**:
```
feat(kanban): TaskEditModal 组件 + ChecklistEditor (V1.1 Phase 6.5)

Modal 内含 title input + content textarea + ChecklistEditor(增删勾改)+
status/area/color 编辑。project 只读展示(domain::task::update 明确不允许
改 project)。

ChecklistEditor 用 crypto.randomUUID 本地生成 uiKey 作为 React key,submit
时剥掉不进持久化(符合 L0"纯 UI 状态 key"例外)。

测试 +多个:预填 / 勾选 / 增删 / 改 text / submit payload / Escape cancel。
```

### Phase 6.6 — TS: 双击 KanbanCard 打开 modal 接线

**文件**:
- `src/components/kanban/KanbanCard.tsx` — 加 `onDoubleClick` prop
- `src/components/kanban/KanbanBoard.tsx` — 加 `onTaskDoubleClick` prop 透传
- `src/components/kanban/KanbanView.tsx` — 加 `editState` + handleEditSubmit + 条件渲染 TaskEditModal
- `src/__tests__/components/kanban/KanbanView.test.tsx` — 加双击 → modal 打开 → submit → taskUpdateWithSubtasks 被调用的端到端测试

**Red**:
- 测试 `fireEvent.doubleClick(card)` → modal 打开
- 测试 modal submit → mock `commands.taskUpdateWithSubtasks` 被调用 with 正确 payload
- 测试 invalidateAllTaskCaches 被调用
- 测试 cancel → modal 关闭

**Green**:
- 实现 wiring

**Commit message**:
```
feat(kanban): 双击 KanbanCard 打开 TaskEditModal + 数据流 (V1.1 Phase 6.6)

KanbanCard onDoubleClick prop 透传到 KanbanView editState,Board 不关心
modal 状态保持职责单一。handleEditSubmit 串一个 command 调用 taskUpdateWithSubtasks
+ invalidateAllTaskCaches + close modal。

双击手势和 @dnd-kit 的 pointer drag 不冲突(dnd 用 pointer move,双击用
click interval 检测)。

测试:端到端双击 → 打开 → submit → command 调用 → cache invalidate → 关闭。
```

### Phase 6.7 — 收口:Docs + Walkthrough + Commit

**文件**:
- `docs/progress/backend.md` — V1.1 Next → Done(2026-04-15 分组)
- `docs/devlog/2026-04-15.md`(追加 V1.1 session 段)
- slipbox changelog
- 用户手动 walkthrough 后 commit

**用户手动 walkthrough 场景(Phase 6.7 blocker)**:
1. 打开 /kanban,任意 task 双击 → modal 打开,title/content/status/area/color 正确预填
2. checklist editor 显示 body 中的 `- [ ]` / `- [x]` 项
3. 勾选一个 undone → done,保存 → KanbanCard 上的 progress 从 `X/Y` 变成 `X+1/Y`
4. 添加新 subtask "test item" → 保存 → markdown 文件里出现新 `- [ ] test item` 行
5. 删除一个 subtask → 保存 → markdown 文件里对应行消失,非 checklist 文本保留
6. 改 title → 保存 → 文件 rename(走 task_update_with_subtasks 的 rename 路径,应该和 task_update 等价)
7. 改 content 非 checklist 部分 → 保存 → 文件里非 checklist 部分被更新,checklist 行不变
8. 改 status → 保存 → 卡片跳到新列(invalidate cache 生效)

**Commit message**:
```
docs(backend): P3 Kanban V1.1 Subtask + 双击编辑完成收口 (V1.1 Phase 6.7)

V1.1 scope: Subtask 结构化建模 + 双击 modal 编辑 + 卡片 progress 徽章。
V1 Kanban 的每个 task 从"单一 actionable 单位"升级为"area + checklist 子
任务"的语义。

commits 7 个:
  6.1: parse_task_checklist + Subtask struct
  6.2: TaskEntity.subtasks + reader 填充
  6.3: task_update_with_subtasks command + render_subtasks_into_body
  6.4: KanbanCard progress 徽章
  6.5: TaskEditModal + ChecklistEditor
  6.6: 双击接线 + 端到端测试
  6.7: 本收口 commit

metrics: Rust +N / TS +N / tests files +N
```

---

## 8. 测试矩阵总览

| 层 | 文件 | 新增测试 | 测试类型 |
|---|---|---|---|
| Rust | `domain/task.rs::tests` | 12 parse_task_checklist + 3 TaskEntity.subtasks 填充 + 6 render_subtasks_into_body + 4 task_update_with_subtasks | unit |
| TS | `__tests__/components/kanban/KanbanCard.test.tsx` | 3 progress 徽章 | RTL |
| TS | `__tests__/components/kanban/TaskEditModal.test.tsx`(新) | ~10 modal 行为 | RTL |
| TS | `__tests__/components/kanban/KanbanView.test.tsx` | 3 双击端到端 | RTL |

**预期新增**: Rust ~25 / TS ~16,V1.1 结束 Rust **~289** / TS **~305**。

---

## 9. 风险与非目标

### 风险

1. **content ↔ subtasks 同步** —— Modal 内部如果允许在 textarea 里手动写 `- [ ]`,会和 checklist editor state 冲突。**V1 决策**: textarea 不显示 checklist 行,用户只能改 checklist 之外的自由文本。V1.2 再考虑 "markdown preview" 模式
2. **Parser 漏 edge case** —— GFM 实际比 V1 支持的更复杂(嵌套、lazy continuation、任务中的 code span 等)。V1 只做最严格的 `^- \[( |x|X)\] text$`,其他情况视为普通文本。用户可能写的格式被 V1 silently 跳过,体感像"bug"。**缓解**: Phase 6.4 卡片 progress 徽章直接反映 parser 的识别结果,用户立即可见 —— 如果他写的东西没被识别,徽章不出现,他会发现
3. **L0 违规温床 —— TS 侧解析 checklist** —— 如果未来为了性能在 TS 侧 preview parse,会引入 drift 风险。**守门**: Code Review 时严查 TS 代码有没有 `.match(/- \[/)` 之类 pattern,发现必须要求走 Rust 新 command
4. **两次 command 非原子** —— 见 §6.2 的 Critical decision。V1 用单 command `task_update_with_subtasks` 解决,Rust 内部原子。**余留风险**: Rust 调用链里 sync_file 后紧跟 query 刷新前若程序 crash,会留下一致的 DB 但 invalid cache(但这和 V1 已有路径一样,不是新风险)
5. **`line_index` 失效** —— 用户在 Rust 侧 get → TS 编辑 → 回传 update 之间,如果用户通过别的路径改了 body(例如直接编辑 markdown 文件触发 file watcher),TS 拿到的 subtasks 的 line_index 就 stale 了。**V1 决策**: 接受,因为 V1 没有 file watcher 自动同步 task body;如果 V1.2 加 watcher 需要 subtasks 加 last_modified 或乐观锁

### 非目标(V1.2+)

- ❌ 嵌套 checklist
- ❌ Subtask 拖拽排序(只在编辑器内通过增删位置改)
- ❌ Subtask 独立 entity_id / 跨 task 引用
- ❌ Subtask 作为 edge 目标(无 `entity_connect(Note → Subtask)`)
- ❌ Progress 徽章点击展开列表
- ❌ TS 侧 preview parse(让 textarea 实时 syntax highlight checklist)
- ❌ 快捷键勾选(e.g. Space 切换当前行 checkbox)
- ❌ Drag-and-drop 在卡片间移动 subtask
- ❌ 跨 task subtask 查询 / 聚合(e.g. "这个 project 下所有 subtask 进度")
- ❌ Task body 里非 checklist 行的富文本编辑(V1 只有 textarea)

---

## 10. Executor 交接 checklist

**开始前须知**:

- [ ] 确认 V1 收口 commit `80819dc` 已落地(含 progress/backend.md 的 V1→Done)
- [ ] 确认 `cargo test --workspace --manifest-path src-tauri/Cargo.toml` green (264 passed / 0 failed / 1 ignored)
- [ ] 确认 `pnpm test -- --run` green (36 files / 289 passed)
- [ ] 确认 `pnpm build` clean
- [ ] 确认 `cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings` clean
- [ ] 阅读本文档 §2 5 项架构决策,任何分歧先 raise 不要开写

**执行期间**:

- [ ] 每个 Phase 6.x 独立 commit,不 bundle
- [ ] TDD 流程:先写失败测试 → 确认 Red → 写实现 → 确认 Green → Refactor。Red/Green 人工确认关卡本 session 已授权跳过
- [ ] 每个 Phase 结束后跑全量测试(cargo test + pnpm test + pnpm build + clippy)
- [ ] 遇到 LSP stale diagnostic → 不要被误导,跑一次 clippy + build 验证
- [ ] 如发现本文档有和仓库现状不一致的地方(API 漂移、类型变化、新 drift)→ 立刻停下,raise 问题 / 补 handoff,不要猜

**Phase 6.7 收口前**:

- [ ] 跑 `/harness-check-tests` 自查测试覆盖
- [ ] 跑 `/harness-type-safety-check` 对照 L0 防火墙
- [ ] Code Review 6 项(测试 / 逻辑 / 回归 / IO / IPC / 建模)
- [ ] **用户手动 walkthrough**(见 §7 Phase 6.7 的 8 个场景)
- [ ] 通过后提交 docs collect commit

---

## 11. 术语表

| 术语 | 含义 |
|---|---|
| **Subtask** | Task body 中的一项 GFM checklist item,结构化为 `{text, done, line_index}` |
| **Body** | Task 的 markdown 文件中 frontmatter 之后的正文部分(含 checklist 和非 checklist 文本) |
| **Checklist line** | Body 里符合 `- [ ] / - [x] / - [X]` 格式的行(parser 识别为 Subtask) |
| **Non-checklist line** | Body 里其他所有行(标题、段落、空行、未识别格式的 list 等) |
| **Progress 徽章** | KanbanCard 右上显示的 `done/total` 进度 UI |
| **line_index** | Body split by `\n` 后的 0-based 行索引,Subtask 指向对应位置 |
| **render_subtasks_into_body** | Rust 侧把 Subtask Vec 合并回 body 的 helper,保留非 checklist 行的相对位置 |
| **Task area 粒度** | 用户语义:一个 task 不是原子 TODO,而是一个工作方向(area),包含多个 subtask |

---

**Document status**: `ready for execution`

**Approval**: 用户已在 V1 收口前的对话中批准 §2 5 项架构决策和 Phase 6 拆解思路(7 个子 task)。

**Next action**: Executor(subagent-driven-development 或 executing-plans)从 Phase 6.1 开始,第一步写 parse_task_checklist 的 12 个单元测试(Red),跑测试确认失败。
