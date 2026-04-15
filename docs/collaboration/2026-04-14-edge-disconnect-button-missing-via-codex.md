# Edge Disconnect Button 缺失 — 分析与修复方案

## 问题

用户反馈：

- 之前点击画布中的连线，会出现一个 `×` 按钮
- 点击 `×` 可以 disconnect 这条边
- 现在这个功能消失了

本次需要回答两件事：

1. 这个功能是最近哪类修改“淹没”的
2. 应该如何在当前架构下修复

---

## 已确认结论

### 1. 这不是后端坏了，而是前端 UI 层根本没再渲染删除控件

当前 [src/components/keysight/GraphEdges.tsx](/Users/alexwang/codes/vibe-coding/super-tauri/src/components/keysight/GraphEdges.tsx) 只做了两件事：

- 计算路径
- 渲染 SVG path + marker

它**没有**：

- edge 选中状态
- `×` 按钮
- `onDisconnect` 回调
- 对 `commands.entityDisconnect(...)` 的任何调用

所以“没有 ×”不是点击没响应，而是组件层已经完全不再提供这个 UI。

---

### 2. 在当前 keysight 代码历史里，disconnect 按钮并不是最近 Question/Task 改坏的

检查历史发现：

- 当前 keysight 代码树里，`entityDisconnect` 从未在 TS 侧真正接通过
- `git log -S "entityDisconnect"` 在 `src/components/keysight` 基本查不到有效接入历史
- `GraphEdges.tsx` 在较早的 edges 落地版本里就已经是“只画线，不可点击”

结论：

- 这个功能**不是**最近增加 `Question / Task` 或这轮“防火墙建模”改坏的
- 更准确地说，是旧系统里用户习惯过的能力，在新 keysight 画布重构后**没有被迁移回来**

因此它不是“近期 regression from green to red”，而是“历史遗留缺口直到现在才被重新暴露”。

---

### 3. 直接把 × 补回去前，现有 RenderEdge 建模还不够强，会导致删错边

当前 [src/components/keysight/lib/buildEdges.ts](/Users/alexwang/codes/vibe-coding/super-tauri/src/components/keysight/lib/buildEdges.ts) 的 `RenderEdge` 只有：

- `from`
- `to`
- `kind`（视觉种类）

问题在于 `kind` 不是 DB 真正的 `edge_type`。

典型例子：

- alias 的 `incomingCardIds -> alias` 在视觉上被画成 `alias_link`
- 但数据库里真实边类型其实是 `card_to_alias`

如果 UI 只根据 `kind: "alias_link"` 去调用：

```ts
commands.entityDisconnect(from, to, "AliasLink")
```

就会删错，或者删不掉。

结论：

- 要恢复 disconnect，不能只靠当前 `RenderEdge.kind`
- 必须把“视觉样式”和“真实 DB edge_type”分开建模

---

### 4. `QuestionLink` 也还没进入通用 disconnect 类型系统

当前 Rust/TS `EdgeType` 里只有：

- `LinkTo`
- `Related`
- `SeeAlso`
- `SectionLink`
- `NoteLink`
- `AliasLink`
- `CardToAlias`

没有 `QuestionLink`。

但我们最近已经在后端引入了 `question_link` 真实 DB 行，并在前端渲染了 question outgoing edges。

这意味着：

- 现在即使补了 UI
- 对 `question_link` 也还不能走 typed `entity_disconnect`

结论：

- 修复 disconnect 按钮时，必须顺手补 `EdgeType::QuestionLink`

---

## 根因归类

这个 bug 不是单点问题，而是 3 层组合缺口：

1. **UI 缺口**
   - `GraphEdges` 没有 edge selection / delete affordance
2. **前端建模缺口**
   - `RenderEdge` 只有视觉 `kind`，没有真实 `edge_type`
3. **后端类型缺口**
   - `EdgeType` 尚未覆盖 `QuestionLink`

---

## 修复方案

### A. 收紧前端 edge 建模

扩展 `RenderEdge`：

```ts
type EdgeVisualKind = "link_to" | "note_link" | "alias_link" | "question_link";
type EdgeDbType = "link_to" | "note_link" | "alias_link" | "card_to_alias" | "question_link";

interface RenderEdge {
  from: string;
  to: string;
  kind: EdgeVisualKind;
  edgeType: EdgeDbType;
}
```

规则：

- `kind` 只决定颜色/虚线样式
- `edgeType` 才用于 disconnect

特别是：

- alias 自己发出的边 → `kind: "alias_link"`, `edgeType: "alias_link"`
- card -> alias 的 incoming 视觉边 → `kind: "alias_link"`, `edgeType: "card_to_alias"`

---

### B. 在 GraphEdges 增加“点击边 -> 显示 × -> disconnect”

推荐做法：

1. path 增加 click hit area
2. `GraphEdges` 接收：
   - `selectedEdgeId`
   - `onSelectEdge(edge)`
   - `onDisconnectEdge(edge)`
3. 点击 path 后，把该 edge 设为选中
4. 选中时，在 `midX / midY` 位置渲染 `×` 按钮
5. 点击 `×` 调 `onDisconnectEdge`

实现约束：

- 按钮和 path 的 `mousedown/click` 必须 `stopPropagation()`
- 否则会冒泡到 `GraphCanvas`，触发平移/其他点击逻辑

---

### C. 在 GraphView 接上 entityDisconnect

`GraphView` 负责：

1. 保存 `selectedEdge`
2. 把 `RenderEdge.edgeType` 映射为 typed command 的 `EdgeType`
3. 调用：

```ts
commands.entityDisconnect(fromId, toId, edgeType)
```

4. 成功后：
   - 清除 `selectedEdge`
   - `queryClient.invalidateQueries()`

建议在以下场景清空 `selectedEdge`：

- 白板切换
- 成功 disconnect 后
- 进入 draw connection 模式时

---

### D. Rust 侧补 QuestionLink 到 EdgeType

需要改：

- [src-tauri/src/modules/keysight/models.rs](/Users/alexwang/codes/vibe-coding/super-tauri/src-tauri/src/modules/keysight/models.rs)
  - `EdgeType` 新增 `QuestionLink`
  - `as_db_str()` / `from_db_str()` 对应补 `"question_link"`

然后重新导出 bindings。

---

### E. commands::entity_disconnect 增加 question sync

当前 `entity_disconnect` 只对：

- card
- note

做了断边后的文件同步。

修复后要补：

- `from_id.starts_with("q_") && edge_type == EdgeType::QuestionLink`

走 question 的同步路径。

否则 question outgoing edge 在 DB 删了，但文件侧仍可能残留。

---

## 建议测试

### 1. GraphView 级交互测试

新增一个真实链路测试：

- 渲染一条 note -> question 边
- 点击这条边
- 断言出现 `×`
- 点击 `×`
- 断言 `mockEntityDisconnect("note_x", "q_y", "NoteLink")` 被调用

再补一条：

- 渲染 card -> alias incoming visual edge
- 点击 `×`
- 断言调用的是 `"CardToAlias"`，不是 `"AliasLink"`

---

### 2. buildEdges 单测

断言不同来源生成正确 `edgeType`：

- card.linkTo -> `edgeType: "link_to"`
- note.* -> `edgeType: "note_link"`
- alias outgoing -> `edgeType: "alias_link"`
- alias incomingCardIds -> `edgeType: "card_to_alias"`
- question.* -> `edgeType: "question_link"`

---

### 3. GraphEdges 集成测试

断言：

- 选中 edge 前不显示 `×`
- 点击 edge 后显示 `×`
- 近距离 edge 也能正确显示 `×`

---

### 4. Rust 类型回归测试

至少补：

- `EdgeType::QuestionLink.as_db_str() == "question_link"`
- `EdgeType::from_db_str("question_link") == Some(QuestionLink)`

---

## 实施顺序

1. 先改 `RenderEdge` 建模
2. 再补 Rust `EdgeType::QuestionLink`
3. 再接 `GraphEdges` 的选中 + `×`
4. 再接 `GraphView -> entityDisconnect`
5. 最后补测试并跑：
   - `pnpm test`
   - `pnpm build`
   - `cargo test -q`
   - `cargo clippy --all-targets --all-features -- -D warnings`

---

## 一句话结论

这次“没有 × disconnect”**不是**近期 `Question / Task` 重构直接引入的 regression；更接近于 keysight 新画布重构后，旧交互能力没有被迁移回来。要修，必须一起补：

- edge 真实类型建模
- `GraphEdges` 选中/删除 UI
- `GraphView` disconnect 调用
- `QuestionLink` 类型系统收口
