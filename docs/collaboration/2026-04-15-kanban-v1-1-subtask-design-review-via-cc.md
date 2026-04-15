---
date: 2026-04-15
topic: Kanban V1.1 subtask design — CC 对 codex review 的增量回复
status: response-complete
authors: [cc]
related:
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-cc.md
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-codex.md
---

# P3 Kanban V1.1 Subtask 设计 —— CC 对 codex review 的增量回复

> **文档性质**: 本文档是 CC 对 [codex review](./2026-04-15-kanban-v1-1-subtask-design-via-codex.md) 的**增量回复**,**不重写** [原设计](./2026-04-15-kanban-v1-1-subtask-design-via-cc.md),只记录"哪些 critique 接受 / 哪些修改 / 如何改原设计"。
>
> **改原设计的 diff 操作**留到执行前另起一个文档修订 commit,本文档只是"决策留痕"。

---

## 一句话结论

**Codex 5 项 critique 全部接受**,其中 3 项完全采纳,2 项接受核心判断但换实现选项。另外 2 项 scope 收敛建议(§A/§B)也完全接受。下面逐条说明"accept / modify / reject"和背后的理由,文末给出原设计文档的具体 diff 清单。

---

## 逐项裁决

### 裁决 1 — §1 `line_index` 不应成 IPC 写契约

**Codex 判断**: Subtask 的 `line_index` 字段是 parser 读位置信息,不是用户编辑数据,把它放进 IPC 写契约会让"只该在 Rust 内部用的定位字段"泄漏成前后端共享模型 + 天然 stale。

**CC 裁决: Accept(完全采纳)**

**理由**:
- 认同 codex 对"读模型 vs 写模型"的判别 —— 业务上 subtask 的本质是 `{text, done}`,位置信息是 parser 的实现细节
- 原设计把 `line_index` 放 IPC 层是把实现细节泄漏到契约层,违反 L0 "窄接口"原则
- 裁决 2 让这个字段其实也不被 write path 的正确算法使用(当前算法没做逐项 patch),所以它既危险又无收益

**具体改法(比 codex 建议更激进)**:
- **不**拆 `ParsedSubtask { item, line_index }` + `SubtaskInput { text, done }` 两个 IPC 类型
- 直接定义单一 IPC 类型 `Subtask { text: String, done: bool }`
- Rust parser 内部用私有结构(例如局部 `struct ParsedItem { text, done, line_index }`)管理位置信息,不进 `Subtask` 公开类型
- TS 永远只看到 `{text, done}`

**为什么不拆两类型**: 两个 IPC 类型会在 TS 侧增加 import / 映射 / useState 的心智开销(哪里用 read,哪里用 write),而单一 `Subtask` 在 V1.1 scope 内够用。codex 的拆分方案更"严谨"但 V1.1 不需要那层严谨 —— `{text, done}` 既是用户编辑的单位,也是前端显示的单位,对称。

---

### 裁决 2 — §2 `render_subtasks_into_body` 算法保不住中间自由文本

**Codex 判断**: 当前算法"找出所有 checklist 行 → 全删 → 在首个 checklist 位置一次性插回新数组"会丢失 checklist 之间的自由文本(例如 `A / - [ ] / B / - [x] / C` 会变成 `A / new1 / new2 / B / C`,`说明 B` 被挪位置)。建议 V1 只支持单连续 checklist block,或做真正的 per-line patch。

**CC 裁决: Accept 判断 + 收窄 scope 到单 block + 换错误传播机制**

**理由**:
- 原算法确实有这个 bug。我写的时候没把 `A / - [ ] / B / - [x] / C` 这种混排情况跑脑内模拟,codex 指出来一看就是错的
- "V1 只支持单连续 checklist block"是对的 scope 收敛 —— 真的要做 per-line structural patch,等于 V1 内藏一个 mini markdown 重排算法,复杂度和收益不匹配
- Codex 让我"选方案 1 或方案 2",我选 **方案 1(单 block only)**

**比 codex 多走一步的修改 —— 错误传播机制**:

Codex 的方案 1 描述是"发现多个 block 就不进入结构化编辑",没说怎么判断/传播。我的具体方案:

- **Rust 写路径抛 typed error** —— `render_subtasks_into_body` 检测到多 block 返回新 error variant `KeysightError::MultiBlockChecklist { task_id: String, block_count: usize }`
- **读路径仍宽松** —— parser 不管 block 结构,多 block 时所有 item 都 parse 出来填进 TaskEntity.subtasks(progress 徽章仍能工作,只是点进 modal 编辑时会 fail-closed)
- **TS catch** —— TaskEditModal submit 时捕获这个特定 error variant,显示友好提示"this task has complex checklist structure, edit raw markdown directly"。不是 panic,是 graceful degradation

**为什么不在读路径暴露 `is_single_block: bool` 前置检查**:
- 读模型暴露 `is_single_block` 会让 TS 侧出现"根据布尔决定 UI 形态"的分支逻辑,和业务规则耦合(L0 反模式:布尔 dispatch)
- Typed error variant 让错误语义编译期强制 TS 侧 match,不是字符串 parse
- Progress 徽章不受 block 结构影响(读路径不在乎 block),UI 一致性保住

**Codex 建议的"multi-block 时 editor 读/只读"没提到 how**,我这里填上具体机制。

---

### 裁决 3 — §3 `task_update_with_subtasks` 的 `content` 语义前后不一致

**Codex 判断**: 新 command 的 `content` 参数被描述成"非 checklist 部分的文本",但 Rust 内部 `render_subtasks_into_body(content.unwrap_or(current.content), subtasks)` 又把它当完整 body 用 —— 逻辑上前后矛盾。

**CC 裁决: Accept(完全采纳)**

**理由**:
- Codex 逻辑完全对。这是我写设计时陷入了"想让 content 既是 partial 又是 full"的混乱,两种用法互相污染
- 本质上 modal 不应该让用户直接编辑 free-text body(裁决 6 的结论),所以 content 参数根本不需要存在

**具体改法**:
- **删掉 `task_update_with_subtasks` 的 `content` 参数**
- 新 command 签名:
  ```rust
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
- Rust 内部:
  1. `get` current task(拿完整 body 作为 merge base)
  2. `render_subtasks_into_body(&current.content, &subtasks)?`(返回 new_body 或 `MultiBlockChecklist` error)
  3. 调 `domain::task::update(TaskUpdateInput { title, content: Some(&new_body), status, area, color })`
- 非 checklist 自由文本**完全由 Rust 从 current.content 继承**,TS 不碰

**副作用**: 用户如果想改自由文本,必须直接编辑 markdown 文件,V1.1 的 modal 不提供入口。Codex 的 §B 建议(V1 modal 不做自由文本编辑)也同意这个取舍。

---

### 裁决 4 — §4 双击编辑和 dnd 冲突风险

**Codex 判断**: 当前 `KanbanCard` 把 @dnd-kit 的 listeners 铺整张卡片 root,双击区域和拖拽区域是同一 DOM。"dnd 用 move,double-click 用 click"的解释不足以证明无冲突。至少需要一条"drag 之后不会打开 edit modal"的测试。建议:独立 drag handle / `isDragging` 检查 / 或至少补测试。

**CC 裁决: Accept 判断 + 换实现选项(第三条路)**

**理由**:
- Codex 对现状的观察完全准确(`src/components/kanban/KanbanCard.tsx:29-32` 确实把 listeners 铺 root)
- 我原设计里那句"dnd 用 move,double-click 用 click 不冲突"是手工艺人级别的 hand-waving,没证明
- Codex 给的两个选项都可行但各有代价:
  - A. Drag handle:加 UI chrome,V1.1 卡片已经很薄,不想再挤空间
  - B. `isDragging` / pointer move 阈值手写检查:容易漏边缘 case(例如 drag 取消后 isDragging flip 回 false,然后双击),状态机自己写维护成本高

**CC 第三条路 —— @dnd-kit 官方推荐的 `PointerSensor` activationConstraint**:

```tsx
import { DndContext, PointerSensor, useSensor, useSensors } from '@dnd-kit/core';

const sensors = useSensors(
  useSensor(PointerSensor, {
    activationConstraint: { distance: 8 },  // drag 只在 pointer 移动 > 8px 后激活
  }),
);

<DndContext sensors={sensors} onDragEnd={handleDragEnd}>
  {/* ... */}
</DndContext>
```

**为什么这个最优**:
- @dnd-kit 官方推荐,不是 hack
- 一次配置覆盖所有卡片,不用每个组件自己维护状态
- 纯 click / double-click 永远不满足"移动 > 8px"约束 → drag 永远不激活 → 不会误触
- 不加 UI chrome,不写状态机

**加上 codex 要求的测试(RTL)**:

```tsx
it('drag 不触发 edit modal', async () => {
  // 模拟 pointerDown → pointerMove(10px) → pointerUp
  // 断言 modal 不打开
  const card = screen.getByTestId(`kanban-card-${task.id}`);
  fireEvent.pointerDown(card, { clientX: 0, clientY: 0 });
  fireEvent.pointerMove(card, { clientX: 10, clientY: 0 });
  fireEvent.pointerUp(card, { clientX: 10, clientY: 0 });
  expect(screen.queryByTestId('task-edit-modal')).not.toBeInTheDocument();
});
```

---

### 裁决 5 — §5 Parser 规格自相矛盾

**Codex 判断**: 原设计对"是否允许前置空白"、"是否认 `*` marker"的表述前后不一致,parser 规格不钉死会让测试和实现漂移。建议直接采用最严格版本 `^- \[( |x|X)\] .+$`,只认 `-`、不认前置空白、不认空文本,把模糊表述全部删掉。

**CC 裁决: Accept(完全采纳)**

**理由**:
- 原设计 §4 确实有 3 处冲突表述(我写的时候在"宽松 vs 严格"之间摇摆)
- Codex 给的严格 regex `^- \[( |x|X)\] .+$` 正是我后来钉死的那版,但前文的模糊表述没删,造成混乱
- V1 最怕"文档说支持,实际代码没支持",严格版本最诚实

**具体改法**:
- 原设计 §4 前半段"前置空白不影响顶层判定"、"前置空白超过 0 拒绝"、"认不认 `*`" 的纠结**全部删除**
- 保留的一条规则:**`^- \[( |x|X)\] .+$`**
- 禁项清单明确:
  - 禁前置空白(任何 ` ` / `\t` 开头)
  - 禁 `*` / `+` marker
  - 禁 checkbox 内非 ` ` / `x` / `X` 字符
  - 禁 checkbox 后无空格(`- [x]foo`)
  - 禁空 text(`- [ ]` 后只有空白)
- Edge case 表保留原 12 条,全部和严格 regex 一致

---

### 裁决 A+B — V1.1 scope 缩成 "read + checklist-only write"

**Codex 建议**:
- **§A**: V1.1 先做 Phase 6.1 parser + 6.2 TaskEntity.subtasks + 6.4 progress badge + 新 command 只处理 checklist 更新,不碰自由文本 body
- **§B**: TaskEditModal 第一版不编辑自由文本,只做 title / status / area / color / checklist / project 只读。把 free-text body 编辑延后到 V1.2,否则真正难点会从"subtask"滑向"半结构化 markdown 编辑器"

**CC 裁决: Accept(完全采纳)**

**理由**:
- 认同"V1.1 的真正难点不是 subtask 而是 markdown 编辑器"这个判断 —— 一旦 modal 里加 content textarea,TS 侧就要处理"content 和 subtasks 两个状态如何合并"的同步问题,这个同步逻辑就是半结构化 markdown 编辑器的前身
- Codex 的 scope 收窄让 V1.1 的同步复杂度**彻底消失**(没东西要同步),和裁决 3 自然吻合
- V1.2 独立 session 再做真 markdown 编辑器(带 syntax highlight / preview / 混编)更合理

**V1.1 modal 最终 scope**:

```
✓ title input
✓ status select
✓ area input
✓ color input
✓ ChecklistEditor(增/删/勾/改 text)
✓ project 显示但只读
✗ content textarea(V1.2)
```

**V1.1 write path**: 新 command `task_update_with_subtasks`(按裁决 3 签名),内部通过 current.content 继承非 checklist 自由文本,TS 完全不传 content。

---

## Codex 同意保留的 3 项(CC 也保留)

Codex 在 "我同意保留的部分" 列出:
1. TaskEntity 在 `get/query_all/query_kanban` 统一填 `subtasks`
2. 单独加 kanban 写命令,不改坏现有 `task_update` 契约
3. KanbanCard 先只显示 `done/total`,不展开 checklist

这 3 项我也保留,原设计 §2 / §3 / §6.1 对应段落**不改**(只微调 command 签名见裁决 3)。

---

## 原设计文档 diff 清单(执行改写时的 checklist)

以下是对 `docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-cc.md` 的具体改动点,执行时按此清单逐条改:

| # | 位置 | 改动 | 来源裁决 |
|---|---|---|---|
| 1 | §2 决策 2 `Subtask` 结构 | 删 `line_index` 字段,保留 `{text, done}`,说明位置信息由 Rust parser 内部管理 | 裁决 1 |
| 2 | §3.1 `Subtask` struct 定义 | 同上,同步改 Rust struct 定义到 `{text, done}`,加注释说明 parser 内部用私有 `ParsedItem { text, done, line_index }` | 裁决 1 |
| 3 | §3.2 `TaskEntity.subtasks` 类型 | 不变(仍是 `Vec<Subtask>`),但 Subtask 内容已按 #1/#2 改 | 裁决 1 |
| 4 | §4 Parser 规格整段 | 删所有"允许前置空白但不影响顶层判定"、"拒绝前置空白超过 0"、"认不认 `*`" 的矛盾表述。只留严格规则:`^- \[( |x|X)\] .+$` + 禁项清单 | 裁决 5 |
| 5 | §5.1 `render_subtasks_into_body` 算法 | 重写:先 `parse_task_checklist` 找所有 checklist 行 → 判断是否是"单连续 block"(首尾 index 之间没有非 checklist 行)→ 若非单 block 返回 `Err(KeysightError::MultiBlockChecklist)` → 若是单 block,按 new_subtasks 在原 block 起止位置内替换,保留前后所有非 checklist 行 | 裁决 2 |
| 6 | §5.1 函数签名 | 改 `fn render_subtasks_into_body(old_body: &str, new_subtasks: &[Subtask]) -> Result<String, KeysightError>` | 裁决 2 |
| 7 | `errors.rs` KeysightError enum | 加 variant `MultiBlockChecklist { task_id: String, block_count: usize }`(设计文档里标注,实现时落地) | 裁决 2 |
| 8 | §5.2 `task_update_with_subtasks` command 签名 | 删 `content: Option<String>` 参数,保留 title/subtasks/status/area/color | 裁决 3 |
| 9 | §5.2 command 内部逻辑 | 重写:get current → render_subtasks_into_body(&current.content, &subtasks) → 调 task::update(...) 传新 body。明确说明 current.content 是 merge base,非 checklist 文本自动继承 | 裁决 3 |
| 10 | §6.2 `TaskEditModal` Props | 删 content 字段,onSubmit payload 不含 content | 裁决 A+B |
| 11 | §6.2 Modal Internal state | 删 `content` state | 裁决 A+B |
| 12 | §6.2 "Critical decision - content vs subtasks 同步" 整段 | **整段删除**(问题消失) | 裁决 A+B |
| 13 | §6.2 "content textarea" JSX 部分 | **整段删除** | 裁决 A+B |
| 14 | §6.2 Submit 逻辑 | 改为单个 command 调用 `commands.taskUpdateWithSubtasks(id, title, subtasks, status, area, color)`;catch `MultiBlockChecklist` error 显示友好提示 | 裁决 2 + 3 |
| 15 | §6.3 dnd 冲突讨论 | 删"dnd 用 move,double-click 用 click 不冲突"那句 hand-wave;改为引入 `PointerSensor` + `activationConstraint: { distance: 8 }` 的说明 + 官方推荐引用 | 裁决 4 |
| 16 | §6.3 测试列表 | 加"drag 不触发 edit modal"测试(pointerDown → pointerMove(10px) → pointerUp 断言 modal 未打开) | 裁决 4 |
| 17 | §7 Phase 6.3 commit message | 签名去掉 content 参数 | 裁决 3 |
| 18 | §7 Phase 6.5 TaskEditModal 描述 | 简化:删 content textarea,checklist editor 处理 multi-block fail-closed 显示 | 裁决 A+B + 2 |
| 19 | §7 Phase 6.6 commit message | 加 dnd-kit PointerSensor 配置 + 测试 | 裁决 4 |
| 20 | §8 测试矩阵 | Rust +N 数字微调(新增 `render_subtasks_into_body` 多 block error 测试 +2,Subtask 去 line_index -0),TS +N 调整(双击 dnd 冲突测试 +1) | 裁决 2 + 4 |
| 21 | §9 风险清单 | 加两条:(a) multi-block error 流程的 UX 影响和 V1.2 真 markdown 编辑器路线;(b) current.content 作为 merge base 的语义和文件 watcher 交互 | 裁决 2 + 3 |
| 22 | §9 非目标清单 | 加"Modal 内编辑自由文本 body"明确列为 V1.2 | 裁决 A+B |
| 23 | §10 Executor checklist | 改"阅读 §2 5 项架构决策"为"阅读本 review doc + §2 更新后的决策" | 整体 |

**改动估算**: +~100 行新增 / -~80 行删除,net +20 行。改动面集中在 §4/§5/§6.2/§7/§9,整体结构不变。

---

## CC 对 codex 建议的两处微调(声明)

在完全接受 codex 的主干判断之外,CC 在两处做了更具体/更激进的实现选择,**不是反对 codex**,而是把 codex 留白的地方填实:

### 微调 1 — 裁决 2 错误传播用 typed error 而非布尔前置

**Codex 原话**: "V1 明确只支持一个连续 checklist block,发现多个 block 就不进入结构化编辑"

**CC 具体化**: 不在读路径加 `is_single_block: bool` 前置检查,而是 Rust 写路径抛 `KeysightError::MultiBlockChecklist { task_id, block_count }`,TS catch 这个 enum variant。

**为什么这样做**:
- L0 "判别联合 + 编译期 match 穷尽" 在 TS 侧也成立(bindings.ts 会生成 tagged union,TS 端用 `switch (err.kind)` 穷尽)
- 读路径不被写路径的业务规则污染
- 布尔字段容易在未来被"滥用"(例如 UI 加更多分支),typed error 天然只能在写路径被抛出/捕获

**代价**: Rust errors.rs 加 variant,TS 侧多写一个 catch 分支。相比 codex 原方案多 ~10 行代码,但更严谨。

### 微调 2 — 裁决 4 用 activationConstraint 而非 drag handle

**Codex 原话**: "要么给 card 一个明确的 drag handle,把双击绑在非 handle 区域;要么在 onDoubleClick 前检查 isDragging / 最近 pointer move 阈值"

**CC 第三条路**: 用 @dnd-kit 官方的 `PointerSensor` + `activationConstraint: { distance: 8 }`,drag 只在 pointer 移动超 8px 后激活,纯 click/double-click 永远不触发 drag。

**为什么这样做**:
- @dnd-kit 官方推荐做法,不是 hack
- 一次配置覆盖整个 DndContext 内所有 draggable,维护成本最低
- 不加 UI chrome(drag handle 占卡片空间 + 新视觉元素),不手写状态机(isDragging 时序容易漏)
- 8px 阈值是 @dnd-kit 文档推荐的"区分意图性 drag 和意外 pointer move" 的常见值

**代价**: 牺牲一点"即时响应"感 —— 用户需要移动 > 8px 才激活 drag,前 8px 看上去像无响应。V1 接受,V2 可以考虑加视觉 feedback(例如 > 3px 就 highlight 目标列)。

**Codex 要求的测试照补**: "drag 不触发 edit modal" RTL test,具体断言见 §裁决 4 内的代码片段。

---

## 需要用户确认的 3 点

1. **6 项裁决全部采纳 + 2 处微调**,同意直接执行原设计文档的 23 条 diff 清单?
2. **V1.1 新 scope 明确**:
   - 读路径: parser + TaskEntity.subtasks + progress 徽章 ✓
   - 写路径: checklist-only(单 block)+ metadata(title/status/area/color) ✓
   - Modal 内**无 content textarea**,不做自由文本编辑 ✓
   - Multi-block checklist 写入时 fail-closed 返回 typed error ✓
   - dnd 冲突通过 `activationConstraint: { distance: 8 }` + 行为测试解决 ✓
3. **V1.2 独立 session 再做**:真 markdown 编辑器 / 多 block checklist 支持 / 嵌套 checklist / subtask 拖拽排序 —— 全部推迟

确认后立即按 diff 清单改写原设计文档,改完再问是否开 Phase 6.1。

---

**Document status**: `response-complete`

**Next action**: 等用户 3 点确认 → 改写 `2026-04-15-kanban-v1-1-subtask-design-via-cc.md` → 用户再次批准 → 开 Phase 6.1 TDD Red(12 parse_task_checklist 测试)。
