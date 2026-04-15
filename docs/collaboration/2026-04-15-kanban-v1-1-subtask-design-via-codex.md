---
date: 2026-04-15
topic: Kanban V1.1 subtask design review
status: review-complete
authors: [codex]
related:
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-cc.md
---

# P3 Kanban V1.1 Subtask 设计 Review

## 总结

我同意这份设计的主方向：

- checklist parser 放 Rust 侧，而不是 TS 侧
- `TaskEntity` 读路径补 `subtasks`，让 Kanban 卡片直接显示进度
- 不改现有 `task_update`，另开 kanban 专用写命令是合理的
- V1 不做嵌套 checklist / subtask 独立 entity / 跨 task 聚合，这个 scope 控制是对的

但我不同意它按当前文本直接执行。当前版本最主要的问题不是“实现细节还没定”，而是有几处核心语义互相打架：`line_index` 的职责混杂、`render_subtasks_into_body` 实际保不住文档结构、`content` 在最终 command 里的含义前后不一致、以及 double-click 和当前 dnd 接法并没有被证明安全。

下面只讲我认为值得改的增量和分歧，尤其是我不同意的部分。

## 主要问题

### 1. `line_index` 不应该成为跨 IPC 的公开写入契约

这是我最不同意的一点。

- 文档在 `...via-cc.md:42-55`、`91-109` 把 `Subtask` 定义成 `{ text, done, line_index }`
- 后面在 `...via-cc.md:447-468` 又让 TS 把这个 `Subtask` 回传给 `task_update_with_subtasks`

问题在于：`line_index` 本质上是 **parser 读出来的位置信息**，不是用户真正编辑的业务数据。把它放进 IPC 写入契约，会让一个“只该在 Rust 内部使用的定位辅助字段”泄漏成前后端共享模型。

这会带来两个后果：

1. 它天然会 stale。文档自己也承认了这个风险（`...via-cc.md:747`），但现在不是“边缘风险”，而是主路径设计就依赖它。
2. 它让模型变脏。UI 真正关心的是 `text/done`，不是“这行原来在第几行”。

更关键的是，文档后面的 `render_subtasks_into_body` 算法其实并没有按每个 subtask 的 `line_index` 做逐项 patch（见下一条），所以现在这个字段既危险，又没有换来对应收益。

我的建议：

- 读模型和写模型拆开
- 读模型可以是 `ParsedSubtask { text, done, line_index }`
- 写模型应该是 `SubtaskInput { text, done }`
- 如果 Rust 写路径确实需要原位信息，就在 Rust 内部从“当前 body + parser”重新计算，不要信任 TS 回传的 `line_index`

### 2. 当前 `render_subtasks_into_body` 算法保不住“中间自由文本”

- 文档在 `...via-cc.md:52-55` 里把 `line_index` 的核心价值说成“保留 checklist 之间的自由文本”
- 但在 `...via-cc.md:213-228`，实际算法是：
  1. 找出所有 checklist 行
  2. 全删
  3. 在首个 checklist 位置一次性插回新数组

这两者是冲突的。

如果 body 是这样：

```md
说明 A
- [ ] one
说明 B
- [x] two
总结 C
```

按当前算法，结果会变成：

```md
说明 A
- [ ] ...
- [x] ...
说明 B
总结 C
```

也就是说，原来夹在两个 checklist item 中间的 `说明 B` 会被挪位置。文档一开始强调“不可接受的 data loss”，但当前算法虽然不一定丢字，**会丢文档结构语义**。

我的建议二选一：

1. 增量做法：V1 明确只支持“一个连续 checklist block”，发现多个 block 就不进入结构化编辑
2. 真要支持任意穿插文本，就不要“删全部再插一次”，而是按 block 或按位置做更细的 patch

我倾向于选 1。它更符合 V1 的 scope，也更诚实。

### 3. `task_update_with_subtasks` 里的 `content` 语义前后不一致

- 文档在 `...via-cc.md:403-445` 先发现“两次 command 非原子”有问题
- 然后在 `...via-cc.md:447-468` 收敛到 `task_update_with_subtasks`
- 但新 command 的 `content` 被描述成“非 checklist 部分的文本”
- 同时又在 Rust 内部直接做 `render_subtasks_into_body(content.unwrap_or(current.content), subtasks)`

这里的问题是：`render_subtasks_into_body` 的输入应该是 **完整 body**，而不是“只含非 checklist 的文本”。

当前仓库里的 `task::update` 也已经明确是拿完整 `content` 重渲染 markdown：

- `src-tauri/src/modules/keysight/domain/task.rs:629-655`

所以如果 `content` 只是“去掉 checklist 后的自由文本”，那它根本不具备给 Rust merge 的锚点信息；如果 `content` 实际上又必须是完整 body，那 TS 端就还是得知道 checklist 在哪里，设计又回到了它自己想规避的复杂度。

我的建议：

- 不要在 V1.1 同时解决“自由文本编辑 + checklist 结构化编辑”
- 更增量的做法是把 V1.1 切成两步：
  1. 先做 parser + progress badge + checklist-only 编辑
  2. 自由文本编辑继续走现有 `task_update`，或者干脆延到 V1.2

如果坚持单 command，我建议把 `content` 明确定义成“完整 body”，而不是“非 checklist 部分”；然后由 Rust 决定是否替换其中 checklist 行。现在这版注释会误导实现者。

### 4. 双击编辑和当前 dnd 绑定方式存在真实冲突风险

- 文档在 `...via-cc.md:478-489` 里默认认为 double-click 和 `@dnd-kit` 不冲突
- 但当前 `KanbanCard` 是把 `useDraggable` 的 `listeners/attributes` 直接铺到整张卡片 root 上
- 见 `src/components/kanban/KanbanCard.tsx:29-32`

这意味着：

- 双击区域和拖拽区域是同一个 DOM
- 任何轻微 pointer move 都可能进入 drag 手势
- “拖了一下但 browser 仍触发 double-click” 这种误触没有任何 guard

我不认为“dnd 用 move，double-click 用 click”这句解释足够。当前设计至少缺一条测试：

- drag 之后不会打开 edit modal

我的建议：

- 要么给 card 一个明确的 drag handle，把双击绑在非 handle 区域
- 要么在 `onDoubleClick` 前检查 `isDragging` / 最近 pointer move 阈值
- 至少补一条 `drag != edit` 的 RTL 行为测试，不要只测 `doubleClick(card)` happy path

### 5. parser 规格现在是自相矛盾的，需要先钉死一版

文档里这几处说法互相不一致：

- `...via-cc.md:57` 说“允许前置空白但不支持嵌套”
- `...via-cc.md:157-164` 先说前置空白不影响顶层判定，后面又说实现是“拒绝前置空白超过 0”
- 同一段还在犹豫 `* [ ]` 到底认不认
- `...via-cc.md:166-177` 又收成严格的 `^- \[( |x|X)\] (.+)$`

这个问题本身不复杂，但如果文档不先钉死，测试和实现一定漂。

我的建议：

- 直接采用最严格版本：`^- \[( |x|X)\] .+$`
- 不认 `*`
- 不认前置空白
- 不认空文本

然后把前面的模糊表述全部删掉。V1 最怕的不是“支持少”，而是“文档说支持，实际代码没支持”。

## 我同意保留的部分

- `TaskEntity` 在 `get/query_all/query_kanban` 统一填 `subtasks`，这和当前 `TaskEntity` 读路径很契合，改动面也清晰。参考现状 `src-tauri/src/modules/keysight/domain/task.rs:351-461`
- 单独加 kanban 写命令，而不是改坏现有 `task_update` 契约，这点我同意。参考现状 `src-tauri/src/modules/keysight/commands.rs:530-559`
- `KanbanCard` 先只显示 `done/total`，不在卡片上展开 checklist，UI scope 控制是对的。当前卡片本来就很薄：`src/components/kanban/KanbanCard.tsx:28-43`

## 我建议的增量收口

### A. 先把 V1.1 缩成“读 + checklist-only 写”

1. Phase 6.1 parser
2. Phase 6.2 `TaskEntity.subtasks`
3. Phase 6.4 progress badge
4. 新 command 只处理 checklist 更新，不碰自由文本 body

这样先把最有价值、风险最低的部分落地。

### B. `TaskEditModal` 第一版不要同时编辑自由文本

第一版 modal 只做：

- title
- status
- area
- color
- checklist editor
- project 只读

把 free-text body 编辑延后。否则这个 feature 的真正难点会从“subtask”滑向“半结构化 markdown 编辑器”。

### C. 如果一定要带 body 编辑，就把模型拆清楚

至少要有下面三者之一，不然实现会乱：

1. `content` = 完整 body，Rust 负责 strip/replace checklist
2. `content_without_checklist` + `subtasks`，但 Rust 必须自己持有旧 body 作为 merge base
3. 干脆引入专门的 `TaskBodyDraft` / `TaskChecklistDraft`，不要复用 `TaskEntity.content`

现在文档把这三种思路混在一起了。

## 结论

一句话结论：

**方向对，但当前版本还不够“ready for execution”。我建议保留 Rust parser + `TaskEntity.subtasks` + progress badge，把 `line_index` 从 IPC 写契约里拿掉，把 V1 scope 收敛成单连续 checklist block，并把自由文本编辑从这次设计里降级或延后。**
