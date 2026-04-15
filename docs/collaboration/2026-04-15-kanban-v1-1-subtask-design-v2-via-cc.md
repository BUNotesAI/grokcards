---
date: 2026-04-15
topic: Kanban V1.1 Subtask + Double-Click Edit Design (V2 — 两轮 codex review 修订定稿)
status: ready for execution
authors: [cc]
related:
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-cc.md
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-codex.md
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-review-via-cc.md
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-review-via-codex.md
---

# P3 Kanban V1.1 — Subtask + Double-Click Edit Design (V2)

> **文档性质**: 本文档是 V1.1 feature 的 **完整、独立、可执行** 设计方案,经过两轮 codex review + 两轮 CC 响应后定稿。Executor 只需读本文档即可执行 Phase 6.0 → 6.7,**不需要** 读 V1 设计或任何 review 文档。
>
> **与 V1 的关系**: V1 的方向基本正确,但有 6 处具体技术问题(详见 §0 修订历史),V2 逐条修正后形成定稿。V1 的文档保留作历史记录,执行时以 V2 为唯一权威来源。
>
> **TDD 人工确认关卡**: 用户已授权本次 session 跳过 Red/Green 人工确认(2026-04-14 session-scoped)。测试仍严格 Red → Green → Refactor 流程,但不暂停等确认。
>
> **L0 硬约束**: 本 feature 跨 Rust + TS,必须守 CLAUDE.md 的硬约束 ——「TS 只负责 UI,Rust 独占业务」、SQLite 是 source of truth、建模优先 + 强类型(判别联合 + typed error 穷尽 match)、测试真实代码路径、Trait-First。任何"TS parse checklist"/"TS 拼 markdown body"/"TS 字符串匹配 Rust error message"都是直接违规。

---

## 0. 修订历史 —— 从 V1 到 V2 的 6 处修正

| # | Codex 发现的问题 | V1 原方案 | V2 定稿方案 |
|---|---|---|---|
| 1 | `Subtask.line_index` 不应成 IPC 写契约 | `Subtask { text, done, line_index }` 暴露给 TS | `Subtask { text, done }` 单一 IPC 类型,parser 内部用私有 `ParsedItem` 管位置 |
| 2 | `render_subtasks_into_body` 的"全删 + 首位插回"算法会挪动中间自由文本 | 单次 replace all | **只支持单连续 checklist block**;多 block 时 Rust 返回 typed error,TS fail-closed |
| 3 | `task_update_with_subtasks` 的 `content` 参数语义前后矛盾(既说"非 checklist 文本"又当完整 body 用)| command 含 `content: Option<String>` | **删除 content 参数**,Rust 内部从 `current.content` 作 merge base 继承所有非 checklist 文本 |
| 4 | 双击 vs `@dnd-kit` 冲突没被证明(listeners 铺满 card root)| hand-wave "move vs click 不冲突" | `PointerSensor` + `activationConstraint: { distance: 8 }`(官方推荐)+ 行为测试 |
| 5 | Parser 规格 3 处自相矛盾 | "允许前置空白不影响顶层" / "拒绝前置空白 > 0" / "认不认 `*`" 并存 | 单一严格规则 `^- \[( \|x\|X)\] .+$`,模糊表述全删 |
| A+B | V1.1 scope 太大,从 subtask 滑向半结构化 markdown 编辑器 | modal 含 `content textarea` | **V1.1 modal 无 content textarea**,free-text body 编辑延 V1.2;自然消解 content ↔ subtasks 同步复杂度 |
| **6 (V2 新)** | `AppError` 现状只有 `Todo(String) / Keysight(String)`,typed error 通路不存在,TS 无法结构化 catch `MultiBlockChecklist` | 假设 TS 可 `switch (err.kind)` 穷尽 | **新增 Phase 6.0**:AppError 从 tuple variants 重构成 struct variants,加 `MultiBlockChecklist { task_id, block_count }` variant,作为 V1.1 第一步 |

**结论**: V2 和 V1 方向完全一致(parser 放 Rust / TaskEntity 填 subtasks / kanban 专用写命令 / 只显示 progress 徽章 / V1.1 不做嵌套),但 6 处具体技术落地被修正。Phase 拆解从 7 步(6.1-6.7)变 8 步(6.0-6.7)。

---

## 1. 背景与产品动机

P3 Kanban V1 交付后,用户提出关键语义澄清:

- **Task 粒度 = area** —— 一个 task 不是原子 actionable TODO,而是一个粒度偏大的工作方向。例如"实现 keysight 画布 zoom 支持"是一个 task
- **Body 承载 checklist** —— task 的 markdown body 用 **GFM checklist**(`- [ ]` / `- [x]`)列出 area 的具体子步骤
- **子任务语义** —— 每项 checklist item 对标一个 **subtask**,应在 Kanban 卡片上有进度可见性(`done/total`),并在双击编辑时可勾选/增删/改文本

V1.1 目标:把"task 就是一个单一 TODO"的隐含假设升级为"task 是 area + 一组子任务",端到端落地 Rust + TS。

---

## 2. 核心架构决策 —— 8 项

1. **Parser 放 Rust 侧** —— `TaskEntity.subtasks: Vec<Subtask>` 字段,`domain::task::get / query_all / query_kanban` 三个 reader 路径统一 parse body 填充。TS 只读不 parse(L0 硬约束)
2. **Subtask 单一 IPC 类型** —— `Subtask { text: String, done: bool }`,无 `line_index`。Rust parser 内部用私有 `ParsedItem` 管位置信息,不跨 IPC 泄漏
3. **单连续 checklist block 约束** —— V1.1 只支持"body 中 checklist 行之间不夹杂非 checklist 行"的简单情形。多 block 时写路径返回 typed error `AppError::MultiBlockChecklist`,读路径不受影响(progress 徽章照常)
4. **Write path 无 content 参数** —— 新 command `task_update_with_subtasks(id, title?, subtasks, status?, area?, color?)`,Rust 拿 `current.content` 作 merge base 继承非 checklist 自由文本
5. **V1.1 modal 无 content textarea** —— 用户不能在 modal 里改自由文本 body,只能改 title / status / area / color + checklist。自由文本编辑延 V1.2
6. **dnd / 双击冲突用 `activationConstraint`** —— `@dnd-kit` 的 `PointerSensor` 配 `{distance: 8}`,drag 只在 pointer 移动 > 8px 后激活,纯 click / double-click 永不触发 drag
7. **Parser 严格规则** —— `^- \[( |x|X)\] .+$`,禁前置空白 / 禁 `*` `+` marker / 禁 checkbox 内非法字符 / 禁空 text。模糊语法视作普通文本跳过
8. **AppError struct variant 重构 + MultiBlockChecklist variant** —— Phase 6.0 作为 V1.1 第一步。app_error.rs 从 tuple variants 改 struct variants,加 `MultiBlockChecklist { task_id, block_count }` 让 TS 侧可用 `err.kind === "MultiBlockChecklist"` 穷尽 match

---

## 3. 数据模型

### 3.1 `Subtask` (IPC 公开类型)

**位置**: `src-tauri/src/modules/keysight/models.rs`

```rust
/// Task body 中的一项 GFM checklist item(子任务)。
///
/// Parse 规则见 `domain::task::parse_task_checklist`(V1 只识别顶层顶格
/// `- [ ] / - [x] / - [X]` 形式,嵌套和其他标记不识别)。
///
/// Invariant: text 已 trim 两端空白且非空。
///
/// V2 决策: 不暴露 `line_index`。位置信息是 parser 实现细节,由 Rust 内部
/// 的私有 `ParsedItem { subtask, line_index }` 局部类型管理,不跨 IPC 泄漏
/// (否则写契约会依赖 parser 位置,天然 stale)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub text: String,
    pub done: bool,
}
```

### 3.2 `TaskEntity` 扩展

**位置**: `src-tauri/src/modules/keysight/models.rs:273` 已有 struct,加一个字段

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
    /// 空 Vec 表示 body 没有任何 checklist item。
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
}
```

### 3.3 `AppError` 重构 (Phase 6.0)

**位置**: `src-tauri/src/app_error.rs`

**V1 现状**(只有 tuple variants):

```rust
#[derive(Debug, Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("{0}")]
    Todo(String),
    #[error("{0}")]
    Keysight(String),
}
```

TS 现在看到: `{ kind: "Todo"; message: string } | { kind: "Keysight"; message: string }`。

**V2 定稿** (struct variants + 新 variant):

```rust
#[derive(Debug, Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "kind")]
pub enum AppError {
    #[error("{message}")]
    Todo { message: String },

    #[error("{message}")]
    Keysight { message: String },

    /// V1.1 新增 —— checklist body 有多个不连续的 block,无法结构化编辑。
    /// 读路径不受影响(progress 徽章仍能工作),只在写路径(task_update_with_subtasks)
    /// fail-closed 返回此错误。TS 侧按 `err.kind === "MultiBlockChecklist"` 穷尽 match,
    /// 显示友好提示要求用户直接编辑 markdown 文件。
    #[error("task {task_id} 的 checklist 有 {block_count} 个不连续 block,无法结构化编辑")]
    MultiBlockChecklist {
        task_id: String,
        block_count: usize,
    },
}
```

TS 生成(`src/bindings.ts` 自动 regen):

```ts
export type AppError =
  | { kind: "Todo"; message: string }
  | { kind: "Keysight"; message: string }
  | { kind: "MultiBlockChecklist"; taskId: string; blockCount: number };
```

**Why serde 从 `tag + content` 改为仅 `tag`**:
- `tag + content` 强制所有 variant 的 payload 都塞进同一个字段名(`message`),对 struct variant 会变成嵌套对象 `{kind, message: {task_id, block_count}}`,命名误导
- 仅 `tag` 让每个 variant 的字段在顶层展开,`{kind: "MultiBlockChecklist", taskId: "...", blockCount: 2}`,符合惯用 tagged union 形态
- 代价: 需要把现有 tuple variants 改成 struct variants 并给字段命名为 `message`,这样 TS 仍看到 `{kind: "Todo", message: "..."}`/`{kind: "Keysight", message: "..."}`,外部 consumer 不受影响

**From impl 同步更新**:

```rust
// src-tauri/src/modules/todo/errors.rs
impl From<TodoError> for AppError {
    fn from(e: TodoError) -> Self {
        AppError::Todo { message: e.to_string() }
    }
}

// src-tauri/src/modules/keysight/errors.rs
impl From<KeysightError> for AppError {
    fn from(e: KeysightError) -> Self {
        match e {
            KeysightError::MultiBlockChecklist { task_id, block_count } => {
                AppError::MultiBlockChecklist { task_id, block_count }
            }
            other => AppError::Keysight { message: other.to_string() },
        }
    }
}
```

**所有直接构造 `AppError::Todo(...)` / `AppError::Keysight(...)` 的点同步改 syntax**(grep 找出,预计 2-5 处)。

### 3.4 `KeysightError` 新 variant

**位置**: `src-tauri/src/modules/keysight/errors.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub(in crate::modules::keysight) enum KeysightError {
    // ...已有 variants...

    /// V1.1 新增 —— checklist body 有多个不连续的 block。
    /// 由 `render_subtasks_into_body` 检测多 block 时抛出,via `From<KeysightError> for AppError`
    /// 转成 `AppError::MultiBlockChecklist { task_id, block_count }` 透传给 TS。
    #[error("task {task_id} 的 checklist 有 {block_count} 个不连续 block")]
    MultiBlockChecklist { task_id: String, block_count: usize },
}
```

---

## 4. Parser 规格 `parse_task_checklist`

**签名**:

```rust
pub(in crate::modules::keysight) fn parse_task_checklist(body: &str) -> Vec<Subtask>
```

**返回 `Vec<Subtask>` 而非 `Result`**: Parser 是 **lenient** 的 —— 非 checklist 行不是错误,跳过。没有任何输入可以让这个函数失败。不合法语法被视作普通文本,不报错不 log。

**单一严格规则**:

```
^- \[( |x|X)\] .+$
```

**禁项清单**(V2 钉死,无讨论空间):

- ❌ 禁前置空白(任何 ` ` / `\t` 开头 → 视作嵌套或段落,跳过)
- ❌ 禁 `*` / `+` marker(只认 `-`)
- ❌ 禁 checkbox 内非法字符(只认 ` ` / `x` / `X`,其他如 `y` / `-` / `/` 一律跳过)
- ❌ 禁 checkbox 后无空格(`- [x]foo` 跳过)
- ❌ 禁空 text(`- [ ]` 后只有空白 → 跳过)
- ❌ V1 不支持嵌套(前置空白即视作嵌套 → 跳过)

**内部数据结构**(**不跨 IPC**,仅 parser/render 私用):

```rust
// domain/task.rs 内部私有
struct ParsedItem {
    subtask: Subtask,
    line_index: usize,
}

fn parse_task_checklist_with_positions(body: &str) -> Vec<ParsedItem> { ... }

pub(in crate::modules::keysight) fn parse_task_checklist(body: &str) -> Vec<Subtask> {
    parse_task_checklist_with_positions(body)
        .into_iter()
        .map(|p| p.subtask)
        .collect()
}
```

Render path 用 `parse_task_checklist_with_positions` 拿到位置信息做 merge,公开 API 只暴露 `parse_task_checklist`。

**Edge case 测试表**(Phase 6.1 Red 阶段写这些测试):

| # | 输入 | 期望 |
|---|---|---|
| 1 | `""` | `vec![]` |
| 2 | `"pure text no checklist"` | `vec![]` |
| 3 | `"- [ ] Buy milk"` | `vec![{text: "Buy milk", done: false}]` |
| 4 | `"- [x] Write tests"` | `vec![{text: "Write tests", done: true}]` |
| 5 | `"- [X] Capital X"` | `vec![{text: "Capital X", done: true}]` |
| 6 | 多行含 done/undone 混合 + 空行 + 自由文本 | 只返回 checklist items 的 Subtask |
| 7 | `"  - [ ] nested"`(前置空白)| `vec![]`(V1 不识别嵌套)|
| 8 | `"- [x]No space"` | `vec![]`(checkbox 后必须空格)|
| 9 | `"* [ ] star marker"` | `vec![]`(V1 不认 `*`)|
| 10 | `"- [y] invalid"` | `vec![]`(checkbox 内只认 ` ` / `x` / `X`)|
| 11 | `"- [ ]   "`(空白 text)| `vec![]`(text 必须非空)|
| 12 | `"说明\n- [ ] a\n说明\n- [x] b\n总结"` | 2 个 Subtask(用 parse_task_checklist_with_positions 验 line_index 精准)|

---

## 5. Write Path

### 5.1 `render_subtasks_into_body` helper

**签名**:

```rust
pub(in crate::modules::keysight) fn render_subtasks_into_body(
    task_id: &str,
    old_body: &str,
    new_subtasks: &[Subtask],
) -> Result<String, KeysightError>
```

**为什么需要 `task_id`**: 多 block 时返回 `KeysightError::MultiBlockChecklist { task_id, block_count }`,需要 task_id 让 TS 侧知道是哪个 task 出问题(上下文信息)。

**算法**:

```
1. 调 parse_task_checklist_with_positions(old_body) → Vec<ParsedItem>
2. 若 Vec 为空 (old body 无 checklist):
   a. 若 new_subtasks 也为空 → 返 old_body 原样
   b. 若 new_subtasks 非空 → 在 body 末尾追加 new_subtasks 渲染结果(确保末尾换行)
3. 若 Vec 非空:
   a. 检查是否 "单连续 block" —— 所有 ParsedItem 的 line_index 必须形成连续整数序列(最小 line_index 到最大 line_index 之间没有缺口)
   b. 若 **非连续**(发现 block_count >= 2 个块)→ 返 Err(KeysightError::MultiBlockChecklist { task_id, block_count })
   c. 若 **连续**:
      - 记录 block 的起止 line_index(min, max)
      - 把 old_body split by '\n' 成 Vec<String>
      - 删除 [min..=max] 范围的所有行
      - 在 min 位置按 new_subtasks 顺序插入渲染行(`- [ ] text` 或 `- [x] text`)
      - join 成 String
4. 返回 Ok(new_body)
```

**连续性检测**伪代码:

```rust
fn is_single_block(positions: &[usize]) -> (bool, usize) {
    if positions.is_empty() {
        return (true, 0);
    }
    let min = positions[0];
    let max = *positions.last().unwrap();
    // 单 block 的充要条件: items 数量 == (max - min + 1),即中间没有空隙
    let is_contig = positions.len() == (max - min + 1);
    let block_count = if is_contig { 1 } else {
        // 粗略计算 block 数(相邻 index 差 > 1 时开启新 block)
        let mut count = 1;
        for w in positions.windows(2) {
            if w[1] - w[0] > 1 { count += 1; }
        }
        count
    };
    (is_contig, block_count)
}
```

注意 positions 必须是升序(parser 按行遍历自然有序)。

**幂等性测试**: `render(id, render(id, body, parse(body)).unwrap(), parse(render...)).unwrap() == 第一次 render 的结果`。二次 render 归一化后完全一致(大小写归一化 `X → x`、行尾空白 trim)。

**多 block 场景**:

```
说明 A
- [ ] one
说明 B
- [x] two
总结 C
```

parse 返回 2 个 ParsedItem,line_index = [1, 3]。positions.len() = 2,max - min + 1 = 3 - 1 + 1 = 3。2 != 3 → 不连续 → `Err(MultiBlockChecklist { task_id, block_count: 2 })`。

### 5.2 `task_update_with_subtasks` command

**位置**: `src-tauri/src/modules/keysight/commands.rs`

**签名**(V2 终稿,无 content 参数):

```rust
#[tauri::command]
#[specta::specta]
pub fn task_update_with_subtasks(
    state: State<'_, KeysightState>,
    id: String,
    title: Option<String>,
    subtasks: Vec<Subtask>,
    status: Option<TaskStatus>,
    area: Option<String>,
    color: Option<String>,
) -> Result<TaskEntity, AppError>
```

**内部逻辑**:

```rust
let _t = ScopedTimer::new("cmd:task_update_with_subtasks");
let conn = lock_db(&state.db, "task_update_with_subtasks");
let vault_fs = RealVaultFs::new(state.vault_path.to_string_lossy().into_owned());

// 1. 读 current 作为 merge base
let current = task::get(&conn, &id).map_err(Into::<AppError>::into)?;

// 2. 把新 subtasks 合并进 current.content(保留所有非 checklist 自由文本)
//    多 block 时 render_subtasks_into_body 返 Err(MultiBlockChecklist),
//    经 From<KeysightError> for AppError 转成 AppError::MultiBlockChecklist
let new_body = task::render_subtasks_into_body(&id, &current.content, &subtasks)
    .map_err(Into::<AppError>::into)?;

// 3. 调 domain::task::update 统一写入(复用现有 rename / sync_file 逻辑)
task::update(
    &conn,
    &vault_fs,
    &id,
    task::TaskUpdateInput {
        title: title.as_deref(),
        content: Some(&new_body),  // 注意: 这是"完整 new body",不是 partial
        status,
        area: area.as_deref(),
        color: color.as_deref(),
    },
)
.map_err(Into::<AppError>::into)?;

// 4. 返回 fresh TaskEntity(含重新 parse 的 subtasks)
task::get(&conn, &id).map_err(Into::into)
```

**Why 复用 `domain::task::update`**: update 已处理 markdown 文件重写 + sync_file + file_mtimes + rename 逻辑。重新实现一次是 test theater 的温床。

**为什么在 command 层做 render 而不是在 domain::task::update 内部**: 保持 `task::update` 签名不变(V1 canvas inline edit 等调用点不受影响),新逻辑集中在 `task_update_with_subtasks` 这一个 command 里。单一职责。

### 5.3 副作用矩阵(CLAUDE.md 更新)

`CLAUDE.md` 副作用矩阵加一行:

| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|---|---|---|---|
| `task_update_with_subtasks` | 等价 task_update(content 用 render_subtasks_into_body 合并后的新 body)+ 可能 rename + file_mtimes | DB 重写 + 文件 body 重写(保留非 checklist 行)+ 多 block 时 fail-closed | domain unit test(合并 / 多 block error / 空 subtasks 清空 / None 字段保留)|

---

## 6. TS UI 规格

### 6.1 `KanbanCard` progress 徽章

**位置**: `src/components/kanban/KanbanCard.tsx`

**改动**:

1. 布局从单行 title 改为 flex row(title 左,progress 徽章右)
2. 从 `task.subtasks` 派生 `total` 和 `done`
3. 条件渲染:`total > 0` 时显示 `{done}/{total}` 徽章

```tsx
const total = task.subtasks.length;
const done = task.subtasks.filter((s) => s.done).length;
const showProgress = total > 0;

return (
  <div /* ... existing dnd props ... */>
    <div className="flex items-start justify-between gap-2">
      <div className="text-sm font-medium">{task.title}</div>
      {showProgress && (
        <div
          className="shrink-0 rounded bg-secondary px-1.5 py-0.5 text-xs text-secondary-foreground"
          data-testid={`kanban-card-progress-${task.id}`}
          aria-label={`${done} of ${total} subtasks done`}
        >
          {done}/{total}
        </div>
      )}
    </div>
    {showProjectTag && task.project && (
      <div className="...">{task.project}</div>
    )}
  </div>
);
```

**测试**(Phase 6.4 写):

- `total === 0` 不渲染徽章
- `total === 3, done === 1` 显示 `1/3`
- `total === 3, done === 3` 显示 `3/3`

### 6.2 `TaskEditModal` 组件(V2 无 content textarea)

**位置**: `src/components/kanban/TaskEditModal.tsx`(新文件)

**Props**:

```ts
interface TaskEditModalProps {
  /** parent 条件渲染 `{editState.open && <TaskEditModal task={editState.task!} .../>}` 防 stale state */
  task: TaskEntity;
  availableProjects: string[];
  onSubmit: (data: {
    id: string;
    title: string;
    subtasks: Subtask[];
    status: TaskStatus;
    area: string | null;
    color: string | null;
  }) => Promise<void>;
  onCancel: () => void;
}
```

**注意**: onSubmit payload **没有 content 字段** —— V1.1 modal 不让用户改自由文本 body,非 checklist 文本由 Rust 从 current.content 继承。

**Internal state**:

```ts
// 每次 open 从 task 初始化(parent 条件渲染保证 fresh,无 stale default)
const [title, setTitle] = useState(task.title);
const [subtasks, setSubtasks] = useState<SubtaskWithUiKey[]>(
  task.subtasks.map(s => ({...s, uiKey: crypto.randomUUID()}))
);
const [status, setStatus] = useState(task.status);
const [area, setArea] = useState(task.area ?? "");
const [color, setColor] = useState(task.color ?? "");
const [submitting, setSubmitting] = useState(false);
const [error, setError] = useState<string | null>(null);
```

**`SubtaskWithUiKey` 本地类型**:

```ts
type SubtaskWithUiKey = Subtask & { uiKey: string };
```

**为什么本地加 uiKey**:
- Subtask 无持久化 id,React 渲染 list 需要稳定 key
- 用 index 作 key 会在中间删除时错位
- 用 `crypto.randomUUID()` 生成纯 UI key,**符合 CLAUDE.md L0 硬约束的例外**(明确允许 `ui_` 前缀的纯 UI state key,不写持久存储)
- submit 时 `subtasks.map(({uiKey, ...rest}) => rest)` 剥掉,还原 `Subtask`

**`ChecklistEditor` 子组件**(同文件内联):

```tsx
interface ChecklistEditorProps {
  subtasks: SubtaskWithUiKey[];
  onChange: (next: SubtaskWithUiKey[]) => void;
}

function ChecklistEditor({ subtasks, onChange }: ChecklistEditorProps) {
  const toggle = (uiKey: string) =>
    onChange(subtasks.map(s => s.uiKey === uiKey ? {...s, done: !s.done} : s));

  const updateText = (uiKey: string, text: string) =>
    onChange(subtasks.map(s => s.uiKey === uiKey ? {...s, text} : s));

  const remove = (uiKey: string) =>
    onChange(subtasks.filter(s => s.uiKey !== uiKey));

  const add = () =>
    onChange([...subtasks, { text: "", done: false, uiKey: crypto.randomUUID() }]);

  return (
    <div className="space-y-1">
      {subtasks.map(s => (
        <div key={s.uiKey} className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={s.done}
            onChange={() => toggle(s.uiKey)}
            data-testid={`checklist-item-checkbox-${s.uiKey}`}
          />
          <input
            type="text"
            value={s.text}
            onChange={e => updateText(s.uiKey, e.target.value)}
            className="flex-1 rounded border px-2 py-1 text-sm"
            placeholder="subtask text"
          />
          <button
            type="button"
            onClick={() => remove(s.uiKey)}
            aria-label="remove subtask"
            className="text-muted-foreground hover:text-destructive"
          >
            ×
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={add}
        className="mt-2 text-xs text-primary"
      >
        + Add subtask
      </button>
    </div>
  );
}
```

**Submit 逻辑(单 command 调用 + MultiBlockChecklist catch)**:

```ts
const handleSubmit = async (e: React.FormEvent) => {
  e.preventDefault();
  if (!title.trim()) return;
  setSubmitting(true);
  setError(null);
  try {
    // 剥掉 uiKey,还原为 IPC 的 Subtask 类型
    const cleanSubtasks: Subtask[] = subtasks
      .filter(s => s.text.trim().length > 0)  // 过滤空白 text,Rust 侧会 reject
      .map(({uiKey, ...rest}) => ({...rest, text: rest.text.trim()}));

    await onSubmit({
      id: task.id,
      title: title.trim(),
      subtasks: cleanSubtasks,
      status,
      area: area.trim() || null,
      color: color || null,
    });
  } catch (err) {
    // catch typed MultiBlockChecklist error via discriminated union
    if (
      typeof err === "object" &&
      err !== null &&
      "kind" in err &&
      (err as { kind: string }).kind === "MultiBlockChecklist"
    ) {
      setError(
        "This task has multiple non-contiguous checklist blocks. " +
        "Please edit the markdown file directly."
      );
    } else {
      setError(err instanceof Error ? err.message : String(err));
    }
  } finally {
    setSubmitting(false);
  }
};
```

**Modal 字段渲染顺序**:

1. `title` input
2. `project` readonly display(显示 `task.project ?? "(no project)"`,不可改)
3. `status` select(复用 COLUMN_ORDER / COLUMNS)
4. `area` input
5. `color` input(复用 CreateTaskModal 的 color 选择方式)
6. `ChecklistEditor`(动态列表)
7. Cancel / Save buttons

### 6.3 双击 KanbanCard + PointerSensor 防冲突

**KanbanCard.tsx 改动**:

```tsx
interface KanbanCardProps {
  task: TaskEntity;
  showProjectTag?: boolean;
  onDoubleClick: (task: TaskEntity) => void;  // 新增 prop
}

// 在 JSX 的外层 div 绑定
<div
  ref={setNodeRef}
  {...listeners}
  {...attributes}
  onDoubleClick={() => onDoubleClick(task)}  // 新增 handler
  // ... existing props
>
```

**KanbanBoard.tsx 改动**: 加 `onTaskDoubleClick` prop 透传给 KanbanCard。

**KanbanView.tsx 改动**:

```ts
// 新 state
const [editState, setEditState] = useState<{
  open: boolean;
  task: TaskEntity | null;
}>({ open: false, task: null });

const openEdit = (task: TaskEntity) => setEditState({ open: true, task });
const closeEdit = () => setEditState((prev) => ({ ...prev, open: false }));

const handleEditSubmit = async (data: {
  id: string;
  title: string;
  subtasks: Subtask[];
  status: TaskStatus;
  area: string | null;
  color: string | null;
}) => {
  await unwrapCommand(
    commands.taskUpdateWithSubtasks(
      data.id,
      data.title,
      data.subtasks,
      data.status,
      data.area,
      data.color,
    ),
  );
  invalidateAllTaskCaches(queryClient);
  closeEdit();
};

// JSX
{editState.open && editState.task && (
  <TaskEditModal
    task={editState.task}
    availableProjects={projects}
    onSubmit={handleEditSubmit}
    onCancel={closeEdit}
  />
)}
```

**DndContext 配置 `activationConstraint`**:

`KanbanBoard.tsx` 引入 PointerSensor:

```tsx
import {
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";

export function KanbanBoard({ /* ... */ }: KanbanBoardProps) {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      // drag 只在 pointer 移动 > 8px 后激活,纯 click / double-click 永不触发 drag
      // 防止双击编辑和拖拽状态变更冲突
      activationConstraint: { distance: 8 },
    }),
  );

  const handleDragEnd = (event: DragEndEvent) => { /* ... */ };

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      {/* ... existing JSX ... */}
    </DndContext>
  );
}
```

**冲突防护测试**(Phase 6.6 写):

```tsx
it("drag 不触发 edit modal", async () => {
  const onDoubleClick = vi.fn();
  render(
    <KanbanBoard
      tasks={[taskFixture]}
      showProjectTags={false}
      onAddTask={vi.fn()}
      onTaskMove={vi.fn()}
      onTaskDoubleClick={onDoubleClick}
    />,
  );
  const card = screen.getByTestId(`kanban-card-${taskFixture.id}`);
  // 模拟 drag 手势: pointerDown → pointerMove(10px) → pointerUp
  fireEvent.pointerDown(card, { clientX: 0, clientY: 0 });
  fireEvent.pointerMove(card, { clientX: 10, clientY: 0 });
  fireEvent.pointerUp(card, { clientX: 10, clientY: 0 });
  // drag 完成后不应该打开 edit modal
  expect(onDoubleClick).not.toHaveBeenCalled();
});

it("双击触发 onDoubleClick", () => {
  const onDoubleClick = vi.fn();
  render(
    <KanbanBoard
      tasks={[taskFixture]}
      showProjectTags={false}
      onAddTask={vi.fn()}
      onTaskMove={vi.fn()}
      onTaskDoubleClick={onDoubleClick}
    />,
  );
  const card = screen.getByTestId(`kanban-card-${taskFixture.id}`);
  fireEvent.doubleClick(card);
  expect(onDoubleClick).toHaveBeenCalledWith(taskFixture);
});
```

---

## 7. 执行计划 —— 8 个 Phase

**TDD 流程**: 每个 Phase 先写失败测试(Red)→ 跑测试确认红色 → 写实现(Green)→ 跑测试确认全绿 → Refactor 保持全绿 → commit。用户已授权跳过 Red/Green 人工确认关卡。

### Phase 6.0 — AppError struct variant 重构 + MultiBlockChecklist variant

**为什么放第一步**: Phase 6.3 的 `render_subtasks_into_body` 要用 `KeysightError::MultiBlockChecklist`,需要 AppError 能 typed 穿越 IPC,这是所有后续 Phase 的硬前置。

**文件**:
- `src-tauri/src/app_error.rs` — enum 从 tuple → struct variants + 新 variant
- `src-tauri/src/modules/todo/errors.rs` — From impl syntax 同步
- `src-tauri/src/modules/keysight/errors.rs` — From impl match MultiBlockChecklist + 新 KeysightError variant
- 所有直接构造 `AppError::Todo(...)` / `AppError::Keysight(...)` 的点(grep 找出)
- `src/bindings.ts` — 自动 regen

**TDD Red**:
- 新测试: `#[test] fn app_error_multi_block_checklist_serializes_with_task_id_and_block_count()`
  - 构造 `AppError::MultiBlockChecklist { task_id: "task_abc", block_count: 2 }`
  - 用 serde_json 序列化
  - 断言结果含 `"kind": "MultiBlockChecklist"` + `"task_id": "task_abc"` + `"block_count": 2`
- 原有测试全部保持绿色(验证 refactor 无回归)

**TDD Green**:
- 改 `AppError` 定义
- 改所有 `AppError::Todo(s)` / `AppError::Keysight(s)` → `AppError::Todo { message: s }` / `AppError::Keysight { message: s }`
- 加 `KeysightError::MultiBlockChecklist` variant
- 改 `impl From<KeysightError> for AppError` 做 match 分流
- 跑 `cargo test export_bindings` 重新生成 bindings.ts
- 验证 bindings.ts 含 `MultiBlockChecklist` 变体
- 全量 `cargo test --workspace` 全绿(264 应该不变)
- 全量 `pnpm test -- --run` + `pnpm build` 全绿(TS 不应该受影响,因为现存 variant 的 `message: string` 字段不变)

**Commit message**:

```
refactor(app_error): struct variants + MultiBlockChecklist variant (V1.1 Phase 6.0)

为 V1.1 Kanban Subtask feature 的 typed error 通路铺路。

- AppError 从 tuple variants (Todo(String) / Keysight(String)) 改成 struct variants
  (Todo { message } / Keysight { message }),serde 从 tag+content 改为仅 tag
- 加 AppError::MultiBlockChecklist { task_id, block_count },让 TS 侧可以按
  err.kind === "MultiBlockChecklist" 穷尽 match(不是字符串 parse)
- KeysightError 加对应 MultiBlockChecklist variant,From<KeysightError> for AppError
  做 match 分流(MultiBlockChecklist 走独立路径保留结构化字段,其他 flatten 为
  AppError::Keysight { message })
- Todo 模块 From impl syntax 同步改 struct variant

bindings.ts 自动 regen。现存 TS 代码读 err.message 不受影响(Todo/Keysight 仍
有 message: string 字段)。新 MultiBlockChecklist 的 taskId/blockCount 将在
Phase 6.3 render_subtasks_into_body 抛出,6.5 TaskEditModal submit catch。

测试 +1 (AppError MultiBlockChecklist 序列化形状验证),全量回归 264 unchanged。
```

### Phase 6.1 — parse_task_checklist + Subtask struct

**文件**:
- `src-tauri/src/modules/keysight/models.rs` — 加 `Subtask { text, done }`
- `src-tauri/src/modules/keysight/domain/task.rs` — 加 parser + 私有 `ParsedItem`

**TDD Red**: 写 12 个 edge case 测试(§4 表),跑 `cargo test parse_task_checklist`,全部失败(函数不存在)。

**TDD Green**: 实现 parser(正则或 starts_with 逐行判断),跑测试全绿。

**Commit message**:

```
feat(keysight): parse_task_checklist + Subtask struct (V1.1 Phase 6.1)

GFM checklist parser,V1 严格规则 ^- \[( |x|X)\] .+$,禁前置空白 / 禁 *+
marker / 禁 checkbox 内非法字符 / 禁空 text / V1 不支持嵌套。

Subtask 是 {text, done} 单一 IPC 类型,位置信息由私有 ParsedItem 内部管理,
不跨 IPC 泄漏(V2 决策:line_index 不进写契约)。

12 个 edge case 测试:空 body / 纯文本 / done/undone / capital X / 嵌套跳过 /
无空格跳过 / * marker 跳过 / 非法字符跳过 / 空 text 跳过 / 混排 line_index 验证。
```

### Phase 6.2 — TaskEntity.subtasks 字段 + 3 reader 填充

**文件**:
- `src-tauri/src/modules/keysight/models.rs` — TaskEntity 加 subtasks 字段
- `src-tauri/src/modules/keysight/domain/task.rs` — `get / query_all / query_kanban` 三处 mapper 调 `parse_task_checklist(&content)` 填充

**TDD Red**:
- 测试 create task 含 checklist body → `get` → 断言 subtasks 正确填充
- 测试 create task 无 checklist → `get` → subtasks.is_empty()
- 测试 query_all / query_kanban 返回的 tasks 都正确填 subtasks

**TDD Green**:
- mapper 里在 content 读出后 `let subtasks = parse_task_checklist(&content);`
- 跑 `cargo test export_bindings` 重新生成 bindings.ts,验证 `TaskEntity.subtasks: Subtask[]` 出现

**Commit message**:

```
feat(keysight): TaskEntity.subtasks + 三 reader 填充 (V1.1 Phase 6.2)

TaskEntity 加 subtasks: Vec<Subtask>,在 get / query_all / query_kanban 三个
SQL reader 的 mapper 里统一调 parse_task_checklist(&content) 填充。bindings.ts
自动生成 TaskEntity.subtasks,TS 侧可直接读 task.subtasks 无需额外 parse。

测试 +3:含 checklist 的 task roundtrip / 空 checklist 断言 / query_all 批量正确。
```

### Phase 6.3 — render_subtasks_into_body + task_update_with_subtasks command

**文件**:
- `src-tauri/src/modules/keysight/domain/task.rs` — `render_subtasks_into_body` + 内部 `parse_task_checklist_with_positions`
- `src-tauri/src/modules/keysight/commands.rs` — `task_update_with_subtasks` command
- `src-tauri/src/lib.rs` — `collect_commands![]` 注册新 command
- `CLAUDE.md` — 副作用矩阵加一行

**TDD Red**:
- `render_subtasks_into_body` 测试(7+ 个):
  - (1) 空 body + 空 subtasks → `Ok("")`
  - (2) 空 body + 非空 subtasks → `Ok("- [ ] a\n- [x] b\n")`
  - (3) 单 block + 替换 → 前后非 checklist 行保留,checklist 行被替换
  - (4) 单 block + 清空 subtasks → checklist 行全删,非 checklist 行保留
  - (5) **多 block** → `Err(MultiBlockChecklist { task_id, block_count: 2 })`(核心 regression)
  - (6) 多 block 超过 2 → block_count 正确
  - (7) 幂等: `render(render(body, parse(body)), parse(once)) == once`
- `task_update_with_subtasks` 测试(5+ 个):
  - (a) Create task with checklist → update_with_subtasks 新 subtasks → get 断言 subtasks 变
  - (b) 只传 subtasks,其他 None → title/status/area/color 保留
  - (c) 传空 subtasks → body 里 checklist 消失,非 checklist 文本保留
  - (d) 多 block task → update_with_subtasks → 断言 `AppError::MultiBlockChecklist` 返回
  - (e) 改 title + subtasks 同时 → title rename + subtasks 更新原子完成

**TDD Green**:
- 实现 render helper(§5.1 算法)
- 实现 command(§5.2 内部逻辑)
- 注册 command
- 更新 CLAUDE.md 副作用矩阵
- `cargo test export_bindings` 重新生成 bindings.ts(出现 `taskUpdateWithSubtasks`)

**Commit message**:

```
feat(keysight): task_update_with_subtasks + render_subtasks_into_body (V1.1 Phase 6.3)

新 command 专管 task 含 subtasks 的更新。Rust 内部通过 current.content 作
merge base,把 subtasks Vec 按单连续 block 合并回 markdown body(保留非
checklist 行原位),然后复用 domain::task::update 的文件重写 + sync 路径。

render_subtasks_into_body 检测多 block 时返回 KeysightError::MultiBlockChecklist
{ task_id, block_count },经 From impl 转成 AppError::MultiBlockChecklist 透传
TS(Phase 6.0 已铺路)。

不改现有 task_update 契约(向后兼容 canvas inline edit 等调用点)。
CLAUDE.md 副作用矩阵新增一行。

测试 +N:render 幂等 / 单 block 替换 / 空 subtasks 清空 / 多 block error /
command 原子 title + subtasks 同时更新 / None 字段保留。
```

### Phase 6.4 — KanbanCard progress 徽章

**文件**:
- `src/components/kanban/KanbanCard.tsx` — JSX flex row + 条件渲染徽章
- `src/__tests__/components/kanban/KanbanCard.test.tsx` — +3 测试

**TDD Red**: 3 个渲染测试(§6.1 末尾),跑 `pnpm test -- --run KanbanCard`,失败。

**TDD Green**: 实现 JSX 改动,测试全绿。

**Commit message**:

```
feat(kanban): KanbanCard progress 徽章 (V1.1 Phase 6.4)

卡片右上显示 done/total 进度(e.g. 2/5),subtasks 为空时不显示避免噪音。
布局改 flex row (title 左 / progress 右)。

测试 +3: 空 subtasks / 部分 done / 全 done。
```

### Phase 6.5 — TaskEditModal + ChecklistEditor(无 content textarea)

**文件**:
- `src/components/kanban/TaskEditModal.tsx` — 新建(含 ChecklistEditor 子组件)
- `src/__tests__/components/kanban/TaskEditModal.test.tsx` — 新建

**TDD Red**:
- 渲染 modal 断言 title 预填 `task.title`,status/area/color 预填正确
- project 显示但不可改(readonly 属性或 disabled)
- ChecklistEditor 显示所有 subtasks,勾选状态正确
- 点 checkbox → toggle done(内部 state 变化)
- 点 "+ Add subtask" → 列表多一行
- 点 "×" delete → 列表少一行
- 改 text → state 同步
- Submit → onSubmit 被调用 with 正确 payload(无 content 字段)
- Cancel / Escape → onCancel 被调用
- catch MultiBlockChecklist error → 显示友好提示

**TDD Green**: 实现组件。

**Commit message**:

```
feat(kanban): TaskEditModal + ChecklistEditor (V1.1 Phase 6.5)

Modal 内含 title / project(只读)/ status / area / color + ChecklistEditor
(增删勾改 subtask)。V2 决策:无 content textarea(自由文本 body 编辑延 V1.2)。

ChecklistEditor 用 crypto.randomUUID 本地生成 uiKey 作为 React key(纯 UI 状态,
不进持久化,符合 L0 "ui_" 前缀例外)。Submit 时剥掉 uiKey 还原 Subtask。

Submit catch MultiBlockChecklist typed error → 显示 "edit markdown file directly"
友好提示。

测试 +多个:预填 / 勾选 / 增删 / 改 text / submit payload / cancel / multi-block
错误显示。
```

### Phase 6.6 — 双击接线 + PointerSensor 配置 + 冲突防护测试

**文件**:
- `src/components/kanban/KanbanCard.tsx` — 加 `onDoubleClick` prop + handler
- `src/components/kanban/KanbanBoard.tsx` — 透传 prop + 引入 PointerSensor 配置
- `src/components/kanban/KanbanView.tsx` — editState + handleEditSubmit + 条件渲染 TaskEditModal
- `src/__tests__/components/kanban/KanbanBoard.test.tsx` — +2 dnd vs 双击冲突测试
- `src/__tests__/components/kanban/KanbanView.test.tsx` — +1 端到端双击 → modal → submit 流程

**TDD Red**:
- (Board) 双击卡片触发 `onTaskDoubleClick` with 正确 task
- (Board) pointerDown → pointerMove(10px) → pointerUp 不触发 `onTaskDoubleClick`(drag 手势)
- (View) 双击 → TaskEditModal 出现(测试 modal 的 testid 存在)
- (View) modal submit → `commands.taskUpdateWithSubtasks` 被调用 with 正确参数 → `invalidateAllTaskCaches` 被调用 → modal 关闭

**TDD Green**:
- 加 KanbanCard `onDoubleClick` prop
- 加 Board `onTaskDoubleClick` 透传 + PointerSensor sensors 配置
- View 加 editState + 条件渲染

**Commit message**:

```
feat(kanban): 双击 KanbanCard 打开 TaskEditModal + PointerSensor 防 dnd 冲突 (V1.1 Phase 6.6)

- KanbanCard onDoubleClick prop 透传到 KanbanView editState
- KanbanBoard 用 PointerSensor + activationConstraint {distance: 8},drag 只在
  pointer 移动 > 8px 后激活,纯 click / double-click 永不触发 drag
- KanbanView editState + handleEditSubmit,串 commands.taskUpdateWithSubtasks +
  invalidateAllTaskCaches + close modal
- 端到端测试:双击 → 打开 → submit → command 调用 → cache invalidate → 关闭
- 冲突防护测试:pointer drag 手势不触发 doubleClick handler
```

### Phase 6.7 — V1.1 收口(docs + walkthrough + commit)

**文件**:
- `docs/progress/backend.md` — V1.1 Next → Done(2026-04-15 分组)
- `docs/devlog/2026-04-15.md` — 追加 V1.1 session 段
- slipbox changelog
- 用户手动 walkthrough 后最终 commit

**用户手动 walkthrough 场景(Phase 6.7 blocker,8 条)**:

1. 打开 /kanban,卡片右上显示 `X/Y` progress 徽章(若 task 有 checklist)
2. 任意 task 双击 → TaskEditModal 打开,title/status/area/color/checklist 正确预填
3. ChecklistEditor 显示 body 中的 `- [ ]` / `- [x]` 项,勾选状态正确
4. 勾选一个 undone subtask → save → KanbanCard progress 从 `X/Y` 变 `X+1/Y`
5. 添加新 subtask "test item" → save → markdown 文件里新增 `- [ ] test item` 行
6. 删除一个 subtask → save → markdown 对应行消失,非 checklist 文本保留
7. 改 title → save → 文件 rename(走 task_update 的 rename 路径)
8. 找一个含多 block checklist 的 task(或临时造一个)→ 双击 → modal 打开 → 勾选 → save → 弹 "edit markdown file directly" 提示(MultiBlockChecklist typed error catch 验证)

**MulitBlock 准备方法**: 手动编辑 vault 里某个 task.md,让 body 变成:

```md
说明段 A
- [ ] item 1
说明段 B
- [ ] item 2
```

sync 或 reload,然后 kanban 双击该 task。

**Commit message**:

```
docs(backend): P3 Kanban V1.1 Subtask + 双击编辑完成收口 (V1.1 Phase 6.7)

V1.1 scope: Subtask 结构化建模 + 双击 modal 编辑 + 卡片 progress 徽章。
V1 Kanban 每个 task 从"单一 actionable 单位"升级为"area + checklist 子任务"。

commits 8 个:
  6.0: AppError struct variant refactor + MultiBlockChecklist variant
  6.1: parse_task_checklist + Subtask struct
  6.2: TaskEntity.subtasks + reader 填充
  6.3: render_subtasks_into_body + task_update_with_subtasks command
  6.4: KanbanCard progress 徽章
  6.5: TaskEditModal + ChecklistEditor(无 content textarea)
  6.6: 双击接线 + PointerSensor + dnd 冲突防护测试
  6.7: 本收口 commit

V1.1 scope 严格限定:读 + checklist-only 结构化写 + 单 block only。
自由文本 body 编辑、多 block 支持、嵌套 checklist 等全部留 V1.2+。

metrics: Rust 264 → ~290 (+~26) / TS 289 → ~305 (+~16) / test files 36 → ~38
```

---

## 8. 测试矩阵

| 层 | 文件 | 新增测试 | 类型 |
|---|---|---|---|
| Rust | `src-tauri/src/app_error.rs` 附近 | 1 (MultiBlockChecklist serialization shape) | unit |
| Rust | `src-tauri/src/modules/keysight/domain/task.rs::tests` | 12 parse_task_checklist edge cases | unit |
| Rust | 同上 | 3 TaskEntity.subtasks reader 填充(get / query_all / query_kanban)| unit |
| Rust | 同上 | 7 render_subtasks_into_body(空 / 单 block / 多 block error / 幂等 / 清空)| unit |
| Rust | 同上 | 5 task_update_with_subtasks(happy / title only / subtasks only / 清空 / multi-block error)| unit |
| **Rust 小计** | | **+~28** | |
| TS | `__tests__/components/kanban/KanbanCard.test.tsx` | 3 progress 徽章 | RTL |
| TS | `__tests__/components/kanban/TaskEditModal.test.tsx`(新) | ~10 modal 行为 | RTL |
| TS | `__tests__/components/kanban/KanbanBoard.test.tsx` | 2 双击 vs drag 冲突 | RTL |
| TS | `__tests__/components/kanban/KanbanView.test.tsx` | 1 双击端到端 | RTL |
| **TS 小计** | | **+~16** | |

**预期 V1.1 结束**: Rust **264 → ~292**(+~28),TS **289 → ~305**(+~16),test files **36 → ~38**(+~2 新文件)。

---

## 9. 风险与非目标

### 风险

1. **Multi-block 检测的 UX 影响** —— 用户可能在 modal 打开后才发现不能编辑(Rust 写路径 fail-closed)。**缓解**: 提示文案足够清晰("edit the markdown file directly"),并指向实际路径。V1.2 可以在 parser 层就给 read model 标 `editable: bool`,modal 打开时直接 disabled ChecklistEditor
2. **Parser 漏 edge case** —— GFM 实际语法比 V1 严格规则复杂。用户写的非严格格式被 silently 跳过,体感像 bug。**缓解**: Phase 6.4 progress 徽章直接反映 parser 识别结果,用户立即可见,未识别项不计入总数
3. **L0 违规温床 —— TS 侧解析 checklist** —— 未来为了即时 preview 可能有人加 TS 侧 parse。**守门**: Code Review 严查 TS 代码,禁止 `.match(/- \[/)` / `.includes("- [")` 之类 pattern,必须走 Rust command
4. **dnd activationConstraint 的"前 8px 无响应"感** —— 用户移动 pointer 前 8px 看上去像无响应。V1 接受,V2 可加"hover 超 8px 就 highlight 目标列"视觉 feedback
5. **AppError refactor 对未发现构造点的影响** —— grep 可能漏。**缓解**: `cargo build` 会在编译期捕获所有未迁移的 `AppError::Todo(...)` / `AppError::Keysight(...)`(因为 tuple 语法对 struct variant 不合法),回归测试全量通过才能算 Phase 6.0 完成

### 非目标(V1.2+)

- ❌ 嵌套 checklist(父项 / 缩进子项)
- ❌ Multi-block checklist 支持(body 内多个不连续 block)
- ❌ 自由文本 body 在 modal 内编辑(content textarea,半结构化 markdown 编辑器)
- ❌ Subtask 拖拽排序(只能在编辑器内通过增删位置改)
- ❌ Subtask 独立 entity_id / 跨 task 引用 / 独立 edge 目标
- ❌ Progress 徽章点击展开列表
- ❌ TS 侧 preview parse(即时 syntax highlight)
- ❌ 快捷键勾选(Space 切换当前行 checkbox)
- ❌ 跨 task subtask 聚合查询(例如 "project X 下所有 subtask 进度")
- ❌ Subtask modification via non-kanban 路径(canvas TaskNode 不加 subtask 编辑能力)

---

## 10. Executor 交接 Checklist

**开始前须知**:

- [ ] 确认 V1 收口 commit `80819dc` 已落地
- [ ] 确认基线 `cargo test --workspace --manifest-path src-tauri/Cargo.toml`: 264 passed / 0 failed / 1 ignored
- [ ] 确认基线 `pnpm test -- --run`: 36 files / 289 passed
- [ ] 确认基线 `pnpm build` clean + `cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings` clean
- [ ] 阅读本文档 §2 8 项架构决策,任何分歧先 raise 不要开写
- [ ] 理解 Phase 6.0 是其他 Phase 的硬前置(Phase 6.3 的 error variant 依赖它),不可跳过

**执行期间**:

- [ ] 每个 Phase 6.x 独立 commit,不 bundle
- [ ] TDD 流程: 先写失败测试 → 跑测试确认 Red(具体 error 符合预期)→ 写实现 → 跑测试确认 Green → Refactor 保持 Green。Red/Green 人工确认关卡已授权跳过
- [ ] 每个 Phase 结束跑全量 (cargo test + pnpm test + pnpm build + clippy),不跳过
- [ ] 遇到 LSP stale diagnostic → 不被误导,跑一次 clippy + build 验证(handoff 明确踩坑)
- [ ] 如发现本文档和仓库现状不一致(API 漂移、类型变化)→ 立即停下 raise,不要猜
- [ ] 每个 Phase 完成后更新 `docs/progress/backend.md` 的进度追踪(新增 active → done)

**Phase 6.7 收口前**:

- [ ] 跑 `/harness-check-tests` 自查测试覆盖
- [ ] 跑 `/harness-type-safety-check` 对照 L0 防火墙清单
- [ ] Code Review 6 项(测试 / 逻辑 / 回归 / IO / IPC / 建模)
- [ ] **用户手动 walkthrough**(§7 Phase 6.7 的 8 个场景)
- [ ] 通过后提交 docs collect commit

---

## 11. 术语表

| 术语 | 含义 |
|---|---|
| **Subtask** | Task body 中的一项 GFM checklist item,`{text, done}` IPC 类型 |
| **ParsedItem** | Rust parser 内部私有类型,`{subtask, line_index}`,不跨 IPC |
| **Body** | Task markdown 文件中 frontmatter 之后的正文部分 |
| **Checklist line** | Body 里符合 `- [ ] / - [x] / - [X]` 格式的行 |
| **Non-checklist line** | Body 里其他所有行(段落 / 空行 / 未识别格式)|
| **Single-block checklist** | Body 中所有 checklist 行的 line_index 形成连续整数序列(之间无非 checklist 行夹杂)|
| **Multi-block checklist** | 两个或更多 checklist block 之间夹杂非 checklist 行,V1.1 不支持结构化编辑 |
| **Progress 徽章** | KanbanCard 右上 `done/total` 进度 UI |
| **render_subtasks_into_body** | Rust 侧合并 subtasks 到 body 的 helper,保留非 checklist 行,多 block fail-closed |
| **Task area 粒度** | 用户语义:task 不是原子 TODO 而是工作方向 area,含多个 subtask |
| **activationConstraint** | `@dnd-kit` PointerSensor 的 drag 激活阈值,本项目用 `{distance: 8}` 防双击误触 |

---

**Document status**: `ready for execution`

**User approval status**: 等用户确认 V2 定稿(8 项架构决策 + 8 Phase 拆解 + 非目标清单) → 开始 Phase 6.0 TDD Red。

**Next action**: Executor 从 Phase 6.0 开始。第一步 Red: 写 `AppError::MultiBlockChecklist` 序列化形状测试,跑 `cargo test`,确认因 variant 不存在而失败。然后实现 enum refactor,跑测试绿。
