# Card ⋯ 菜单的 "Related" 没反应 — 分析与修正方案

> 交接对象：下一个 agent / 人类，修复"Card 菜单里点 Related 看起来没反应"。
>
> 本文包含：bug 复现描述 + 完整的代码路径追踪 + 根因分析 + 4 个修正方案（按推荐度排）+
> 具体文件位置 + TDD 约束。

---

## Bug 报告（来自用户）

> Card 里的菜单功能：Related，目前没有反应。

用户在运行的 app 里打开一个 Card 的 ⋯ 菜单，点击 "Related" 菜单项，看起来什么都没发生。
没有报错、没有视觉变化、没有日志（前端 console 也可能看不到，Tauri webview devtools 需要
手动开）。

用户**没有明确说** Draw connection 菜单项是否工作。两者走同一套 drawing state 机制，
但渲染层差别巨大（见下文）。也没有说是否尝试过"点 Related 后再点另一个 card"的两阶段
操作 —— 这是下文根因分析的关键。

---

## 背景：⋯ 菜单的 Related 是怎么实现的

本 session 的 commit `0d21e45 feat(keysight): Card/Note/Alias 节点 ⋯ 三点上下文菜单`
新增了 ⋯ 菜单。Related 的处理链是：

1. **`src/components/keysight/nodes/NodeContextMenu.tsx`** — UI 层，card variant 下渲染
   ```tsx
   <DropdownMenuItem onClick={menu.onRelated}>Related</DropdownMenuItem>
   ```
2. **`src/components/keysight/nodes/EntityNode.tsx`** — 适配层，`cardMenu` useMemo 构造
   ```tsx
   onRelated: () => menuHandlers.onRelatedFrom(entity.id),
   ```
3. **`src/components/keysight/GraphView.tsx`** — 业务层，`menuHandlers` useMemo
   ```tsx
   // Card: Related → 进入 Related 模式
   onRelatedFrom: (id) => setDrawingState({ fromId: id, edgeType: "Related" }),
   ```
4. **`GraphView.handleSelectEntity`** — 两阶段点击机：第一次点菜单设置 `drawingState`，
   第二次点其他实体触发 `entity_connect`
   ```tsx
   if (drawingState && drawingState.fromId !== selection.id) {
     const { fromId, edgeType } = drawingState;
     setDrawingState(null);
     unwrapCommand(commands.entityConnect(fromId, selection.id, edgeType, null, null))
       .then(() => queryClient.invalidateQueries())
       .catch((err) => console.error(`建立 ${edgeType} 边失败:`, err));
     return;
   }
   ```
5. **Rust 侧 `entity_connect` 命令**（`src-tauri/src/modules/keysight/commands.rs:761`）
   直接调用 `SqliteEntityGraph::connect()` 写入 `edges` 表，edge_type 序列化为字符串
   `"related"`（`models.rs:115`）。
6. **Rust 侧 `card_query_all`**（`domain/card.rs:80-110`）的 `batch_load_edges` 读取
   ```sql
   SELECT ... FROM edges WHERE from_id IN (...) AND edge_type IN ('link_to', 'related', 'see_also')
   ```
   将 related 填到 `AtomicCard.related: Vec<String>`，返回给前端。

**全链路是正常的**。Related 边会成功写入 DB，并在下次 query 时出现在 `card.related` 字段里。
**没有后端 bug**。

---

## 根因分析

我本地没能复现（没启动 dev server），但**按代码推演，最可能的原因是**：

### ① Related 的操作是隐形的 — 没有任何视觉反馈

即使两阶段点击机正常工作，用户也**看不到**任何变化。三个原因叠加：

**1. Related 边不渲染在画布上**（by design）

`src/components/keysight/lib/buildEdges.ts:12-25` 的注释里白纸黑字：

```
 * 不包含：
 * - card.related — 旧 Obsidian 也只在卡片展开后的"Related"列表里显示，不画连线
 * - card.seeAlso — see-also 指向 vault 文件而非实体，画线没意义
```

`buildEdges()` 只返回 `link_to` / `note_link` / `alias_link` 三种 kind 的 edge，**显式
跳过 related**。所以不管你成功创建了多少 Related 边，画布上永远不会出现新的连线。

这和 **Draw connection**（`edgeType: "LinkTo"`）的行为恰好相反 —— 后者会被 buildEdges
包含，第二次点击后画布上会立即出现一条橙色实线。这就是为什么"Draw connection 工作，
Related 不工作"的直观印象。

**2. Card 的 Related 列表只在展开状态显示**

`src/components/keysight/nodes/CardNode.tsx` 大约 line 422 起：

```tsx
<div style={{ padding: "0 14px 12px 14px", display: isExpanded ? "block" : "none" }}>
  {linkedCards.length > 0 && (...)}
  {relatedCards.length > 0 && (
    <>
      <SectionHeading label="RELATED" count={relatedCards.length} />
      {relatedCards.map((c) => (<LinkedRow key={c.id} title={c.title} />))}
    </>
  )}
  ...
</div>
```

Related 列表在 `display: isExpanded ? "block" : "none"` 的块里 —— **默认折叠状态不显示**。
用户必须点击卡片左上角的 toggle 箭头（`▸` / `▾`）展开卡片才能看到新增的 related 条目。

**3. Drawing mode 本身没有视觉反馈**

上一次的 handoff 和 progress 都记过这点：

> ⋯ 菜单视觉反馈后续增强：Draw connection / Related 模式的视觉反馈（source 高亮 +
> 光标样式）

点击 Related 后 `drawingState` 切换为 `{fromId, edgeType: "Related"}`，但 UI 上**没有
任何提示**说明现在处于 drawing 模式。没有 source 节点高亮、没有光标样式变化、没有
status bar 提示、没有 toast。用户既不知道自己处于等待目标的中间状态，也不知道第一次
点击有效。

### 三者叠加的用户体验

```
用户点 ⋯ → 菜单打开                       ✓ 看到菜单
用户点 Related → 菜单关闭                  ✓ 看到菜单关闭
[drawingState 切换为 Related 模式]          ✗ UI 无任何反馈
用户看了一下画布                            ✗ 没变化
用户可能做三件事之一：
  (a) 以为"点 Related 就够了"             → 什么都没发生，结论"没反应"
  (b) 以为"需要再点一个目标"，试着点另一张卡 → 后端成功写边，但画布没画线
      → 如果原卡没展开，看不到 RELATED 列表 → 结论"没反应"
  (c) 点到空白画布或 section              → drawingState 仍然挂着，彻底没反应
```

**任何一条路径都看不到操作结果**。即使后端已经正确写入了 Related 边。

### ② 次要可能：不是操作成功的问题，而是根本没触发

这一节的可能性比①低，但不能排除。需要 verify：

- 用户可能根本不知道两阶段点击机制，点完 Related 就没再点别的了，drawingState 一直挂着。
- 用户可能把菜单当普通 action 按钮用（像 "Copy title" 那样点一下就完成）。

**判断依据**：问用户两个问题就能区分 ①/②：
1. "点 Related 后，你有没有再点另一张 card？"
2. "如果把 source card 展开（点 `▸`），RELATED 列表里有没有多出一条？"

如果 ②.1 答"没有" → 是 UX 问题，用户不知道流程。
如果 ②.1 答"有" + ②.2 答"有" → 是 ① 的视觉反馈问题，操作其实成功了。
如果 ②.1 答"有" + ②.2 答"没有" → **才是真 bug**，需要查 Rust 侧为什么没写入（可能是
edge_type 序列化问题 / invalidate 没覆盖到 / 其他）。

---

## 排除过的假设

以下假设查过代码后排除了：

1. ~~Rust 侧 `entity_connect` 对 "Related" edge type 有特殊验证/拒绝~~ —
   `commands.rs:761-774` 只调用 `graph.connect()`，没有 edge type 校验。`EdgeType::Related`
   在 `models.rs:115` 序列化为 `"related"`，和 `batch_load_edges` 的 SQL 匹配。
2. ~~`lastDidDragRef.current` 残留 true 导致 drawing 第二次点击被吞~~ — 追踪过序列：
   用户点 Card B 完成 drawing 时，Card B 的 mousedown → mouseup 会重置 `lastDidDragRef`
   为 `info.didDrag`（如果只是 click 没 drag，就是 `false`）。之后的 click handler 读到
   的是新值。**除非**用户之前在 Card B 上发生过 drag 且没有点击过别的东西清除 —— 极端
   边界情况，不认为是主因。
3. ~~`useCallback(handleSelectEntity)` 的 deps 漏了 `drawingState` 导致 stale closure~~ —
   实际代码 `[onSelectEntity, drawingState, queryClient]` 三项都在。
4. ~~`NodeContextMenu` 里 onClick 没 wire 上~~ — NodeContextMenu.test.tsx 里的 19 个测试
   已经验证了 `onRelated` click 会调 handler。

---

## 修正方案（按推荐度排）

### ⭐ Option A — 让 Related 边画线（推荐，直接对齐 Draw connection）

**改动**：让 `buildEdges()` 也输出 Related 边，用**不同样式**区分于 link_to：link_to 是
橙色实线（旧行为），Related 用**蓝色虚线**（或其他 distinct 的样式）。

**为什么推荐**：
- 和 Draw connection 对称，行为一致可预测
- 旧 Obsidian plugin 不画 Related 是因为那时用 picker popover 交互，不存在"选第二个"
  的直观反馈需求。新项目的两阶段点击机**需要**一个操作成功的信号
- 改动小且局部（edge kind 新增一种）

**具体文件**：

1. **`src/components/keysight/lib/buildEdges.ts`** — 把 `EdgeKind` 加 `"related"` 成员，
   在 `for card` 循环里加一段处理 `card.related`：
   ```typescript
   export type EdgeKind = "link_to" | "note_link" | "alias_link" | "related";
   ...
   for (const target of card.related ?? []) {
     if (target === card.id || !entitySet.has(target)) continue;
     edges.push({ from: card.id, to: target, kind: "related" });
   }
   ```
   顺手更新文件顶部"不包含：card.related"的注释 —— 改成"card.related → related edge
   （蓝色虚线）"。

2. **`src/components/keysight/GraphEdges.tsx`**（或 `lib/edgePath.ts`，具体看渲染逻辑在哪）
   加 `kind === "related"` 的 case：蓝色（比如 `#3b82f6`）+ `stroke-dasharray="6 4"` +
   细一点的 `stroke-width`。旧 plugin 的 CSS 查 `ks-graph-edge--related` 或类似 selector
   能找到参考样式。

3. **`src/__tests__/components/keysight/lib/buildEdges.test.ts`**（如果存在）— 加测试：
   `card.related = ["card_b"]` 时返回 `[{from: card.id, to: "card_b", kind: "related"}]`。

**代价**：需要想一下颜色/样式区分——不要和 link_to 的橙色、note_link 的青色冲突。

### ⭐ Option D — 给 drawing mode 加视觉反馈（并行推荐，和 A 一起做）

**改动**：`drawingState` 非 null 时，source 节点加**外发光边框** + 画布光标改为 `crosshair`。

**为什么和 A 一起做**：A 解决"操作成功后看不见"，D 解决"中间态看不见"。两者互补。
单做 A 而不做 D，用户还是不知道自己处于"等待目标"的状态；单做 D 而不做 A，Related
还是没有最终结果反馈。

**具体文件**：

1. **`src/components/keysight/GraphView.tsx`** — `drawingState` 非 null 时给 canvas 容器
   加 `cursor: "crosshair"`，把 `drawingState.fromId` 作为 prop 传给 EntityNode。

2. **`src/components/keysight/nodes/EntityNode.tsx`** — 新增 `isDrawingSource?: boolean`
   prop，为 true 时在渲染时传 `selected`（或新增一个 `drawing`）状态，让 CardNode/NoteNode
   的 emphasisRing 显示一个区别于 `selected` 的颜色（比如紫色）。memo 比较器加
   `prev.isDrawingSource !== next.isDrawingSource`。

3. **可选**: 在 toolbar 或 canvas 顶部加一个 banner — "Click another node to create
   {Related/Link} edge (Esc to cancel)"。这个 Esc 取消需要在 GraphView 全局监听 keydown，
   看到 Escape 就 `setDrawingState(null)`。

4. **测试**：EntityNode.test.tsx 加一条 — `isDrawingSource={true}` 时渲染出紫色 ring。
   GraphView.test.tsx 比较难写，可以只加 unit 测试 drawingState state 机的行为。

### Option B — 保留 Related 不画线，但加 toast/高亮反馈

**改动**：entity_connect 成功后，用短暂的节点高亮（比如 500ms 的绿色闪烁）或者一个
right-bottom toast "Connected X → Y (Related)" 作为反馈。

**为什么不如 A**：
- 用户仍然看不到持久的关系结构 —— 要点展开才能看到
- 需要引入一个 toast 组件（项目目前没有）或实现临时高亮动画 —— 额外复杂度
- 和 Draw connection 行为不对称 —— 一个画线一个 flash，用户要记两套

只在用户**明确反对 A**（比如"Related 太多会让画布太乱"）的情况下考虑 B。

### Option C — 恢复旧 plugin 的 picker popover 交互

**改动**：Related 不进入 drawing mode，而是点菜单后立即弹出一个 card picker popover
（类似旧 plugin 的 `relatedPickerCardId` + 搜索框 + 候选列表），用户从列表里选目标。

**为什么不推荐作为默认方案**：
- Draw connection 也有同样问题但不适合用 picker（它会接 section/note/alias，不只 card）
- Draw connection 和 Related 再次出现行为不对称
- 工程量比 A+D 大一倍，需要新建 SectionPicker 类似的 CardPicker 组件
- 但某些场景（比如 1000+ cards 的白板，两阶段点击需要 viewport 平移）picker 交互明确
  更好

可以作为**后续增强**，但不是现在的主修路径。

---

## 推荐执行顺序

1. **先问用户（如果可能）**：是否已经尝试过点第二张卡？是否知道原卡展开后能看到 related
   列表？—— 区分是 UX 问题还是真 bug
2. **Option A + Option D 一起做**，作为一个功能完整的修正
3. 提交前必须跑 `pnpm test && pnpm build && cd src-tauri && cargo test --workspace &&
   cargo clippy --workspace -- -D warnings`（全绿）
4. 手动验证：跑 `pnpm tauri dev`，实测 Card ⋯ → Related → 点另一张卡 → 应看到蓝色虚线
   和 source 节点紫色高亮

---

## 不要做的事

1. **不要只做 Option A 而不做 Option D**，反之亦然 —— drawing 中间态的反馈缺失本身就是
   独立 bug，单做一边不够
2. **不要把 Related 的行为降级为 "点一下就完成"**（不需要第二次点击），这会让语义崩塌
   —— Related 必须指向一个目标实体，不能只对 source 自己操作
3. **不要顺手改 Draw connection 的边样式** —— 它的橙色实线是旧 plugin 延续下来的，有
   用户记忆。只给 Related 加新样式，不动 link_to
4. **不要一次 commit A+D 打包** —— CLAUDE.md L0 规则要求每个子 task 独立 commit，便于
   code review 和回滚
5. **不要跳过"手动验证"** —— 这是 UI 反馈 bug，测试通过不等于 UX 通过。必须实际在运行
   的 app 里点一次

---

## TDD 约束

按 CLAUDE.md L0 规则：

- **buildEdges** 新增 related case → `buildEdges.test.ts` 先加失败测试（Red），再改实现
  （Green）。
- **GraphEdges 渲染 related kind** → 渲染层测试（如果有）或组件测试验证 SVG 输出含
  `stroke-dasharray`。
- **EntityNode 的 `isDrawingSource` prop** → `EntityNode.test.tsx` 加断言 "`isDrawingSource=true`
  时节点渲染紫色 ring"。
- **GraphView 的 drawing cursor 和 Escape 取消** → 较难单测，用集成测试或手动验证。

⚠️ **关于 Red/Green 人工确认关卡**：本项目 CLAUDE.md 默认要求每个 Red/Green 切换人工跑
测试确认。**之前 session（⋯ 菜单落地）用户授权了"自主模式"，但那个授权是针对那一次
task 的，不继承到这次**。下一个 agent 开始改之前，**先和用户确认是否沿用自主模式**。

---

## 延伸：其他菜单项可能也有类似隐形问题

既然 Related 有这个问题，审查一遍其他菜单项是否也存在"后端成功但前端无感知"的情况：

- **Draw connection**：会画线 → OK
- **Create alias**：新建的 alias 在原卡右侧 540px 位置，会在画布上立刻出现 → OK
- **Copy title / Copy UUID+title**：写剪贴板，用户能粘贴验证 → OK（但也可以加 toast）
- **Move to Section**：实体的所属 section 变了，但 section 的视觉边界是从成员位置算的，
  所以会立刻看到 section 框子扩张/收缩 → OK
- **Remove from group**：section 缩小 → OK
- **Delete note / Delete alias**：节点从画布消失 → OK
- **Edit title**：进入 inline 编辑 → OK
- **Set color (Note)**：节点背景色变了 → OK
- **Jump to source card (Alias)**：viewport 平移 → OK

只有 **Related 是唯一没有自然 UI 反馈的菜单项**。修好它之后菜单功能就全覆盖了。

---

## 完成标准

1. Option A + D 两个修正完成并独立 commit
2. 全量测试 + clippy + build 全绿
3. 手动验证：
   - 点 Card A 的 ⋯ → Related → Card A 出现紫色外发光 + 光标变 crosshair
   - 点 Card B → 画布出现从 A 到 B 的蓝色虚线 → Card A 紫色外发光消失 → 光标恢复
   - 按 Esc 也能取消 drawing 模式
4. 更新 `docs/progress/keysight.md`：把"⋯ 菜单视觉反馈增强"条从 `Next` 移到 `Done`
5. 追加 `docs/devlog/2026-04-13.md` 一段修复总结

## Review 完成后

如果 reviewer 或用户发现 Option A + D 仍然没解决 "Related 没反应" 的问题（比如用户报告
"我确实看到线画出来了，但 xxx 还是没生效"），下一步排查方向：

- 检查 `commands.entityConnect(from, to, "Related")` 的实际调用是否发生 —— 在浏览器
  devtools Network tab 或 Tauri invoke trace 里看
- 检查 DB 是否真的写入：`sqlite3 ~/Library/Application\ Support/co.bunotes.super-tauri/keysight.db
  "SELECT * FROM edges WHERE edge_type='related'"`
- 检查 `queryClient.invalidateQueries()` 是否触发了 refetch（TanStack Query devtools 看）
- 如果 DB 写入了但前端没 refetch，查 useWhiteboardData 的 query key 设计
