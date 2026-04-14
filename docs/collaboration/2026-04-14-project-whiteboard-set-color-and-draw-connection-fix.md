# Project 白板 set_color / draw_connection 修复执行文档

## 目标

修复 `projects/{name}` 白板上以下问题：

- `Note / Question / Task` 点击 `Set color` 后 UI 无变化
- `Note / Question` 点击 `Draw connection` 后无法形成可见连线

## 已确认事实

### 1. 颜色问题不是单一 Rust 路径 bug

静态检查确认：

- `NoteNode.tsx` 背景写死为 `#ecf7f2`
- `QuestionNode.tsx` 背景写死为 `bg-card`
- `TaskNode.tsx` 背景写死为 `bg-card`

结论：

- 即使后端把颜色写进 DB / markdown，当前 UI 也不会显示出来

### 2. Note 连线到 Question / Task 会被自己写回流程吞掉

`note::get()` 已经能从 `edges.note_link` 读出：

- `linked_question_ids`
- `linked_task_ids`

但 `note::sync_links_to_file()` / `render_note_markdown()` 只会把这些链接写回：

- card
- note
- section

不会写回：

- question
- task

因此 `entity_connect(Note -> Question/Task)` 的当前行为是：

1. 先往 `edges` 表插入 `note_link`
2. 再调用 `sync_links_to_file()`
3. 重写 markdown 时丢掉 question/task target
4. `sync_file()` 再把 DB 里的出边删回去

结论：

- 这能直接解释 project 白板里 `Note -> Question/Task`“点了没动静”

### 3. Note 的 hex 颜色当前也写不对

`render_note_markdown()` 直接输出：

```yaml
color: #fff8b3
```

YAML 会把 `#...` 当注释，正确写法应加引号。

结论：

- Note 的 `set_color` 不只是 UI 不渲染，后端 markdown 写回也有 bug

### 4. Question 连线当前缺少完整的数据闭环

现状：

- `entity_connect(Question -> X)` 会写入 `question_link`
- commands 层对 `QuestionLink` 还留着 TODO，不会 sync 回 question 文件
- `QuestionEntity` 也没有 linked ids 字段
- `buildEdges()` 不接受 questions 输入，前端无法画出 question 的 outgoing edge

结论：

- Question 连线问题不是 wb_id 派生问题，而是 question link read/write/render 链路没补完整

## 修复范围

### 前端

- 让 `NoteNode / QuestionNode / TaskNode` 消费实体 `color`
- 扩展 `buildEdges()` 支持 question outgoing edges
- `GraphView` 把 questions 纳入边渲染输入

### 后端

- Note markdown 写回时保留 `linked_question_ids / linked_task_ids`
- Note 写 color frontmatter 时给 hex 加引号
- Question model 增加 linked ids
- Question 读取 / 写回 / sync file 支持 `question_link`
- `entity_connect(QuestionLink)` 后同步 question markdown
- `sync_file()` 对 question 文件的 `linkTo` 解析成 `question_link`

## 测试计划

### TS

- 节点组件在有 `color` 时应改变背景
- `buildEdges()` 应输出 question outgoing edges

### Rust

- Note sync 到文件时保留 question/task targets
- Note hex color 写回后能 round-trip
- Question get/query 可读出 `question_link`
- Question sync 到文件后保留 outgoing links

## 执行记录

- 2026-04-14: 重建问题分析，放弃旧文档的 `wb_id` 单根因假设，改为按已确认代码事实修复
- 2026-04-14: 新增 TS/Rust 回归测试，先验证 Red：
  - 节点组件不消费 `color`
  - `buildEdges()` 不支持 question outgoing edges
  - note 写回会丢 question/task targets
- 2026-04-14: 完成实现：
  - `NoteNode / QuestionNode / TaskNode` 现在会消费实体 `color`
  - `buildEdges()` / `GraphView` 现在会渲染 question outgoing edges
  - note 文件写回现在保留 question/task link targets
  - note hex color frontmatter 现在会加引号
  - question 现在具备 link read/write/sync/render 闭环
- 2026-04-14: 验证通过：
  - `cargo test -q`
  - `pnpm test`
  - `pnpm build`
