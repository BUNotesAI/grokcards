---
area: keysight
last_updated: 2026-04-12T16:50:00+08:00
session_id: 8ec03324
status: ready-to-resume
stale_check: cd src-tauri && cargo test --lib modules::keysight 2>&1 | tail -1 && cd .. && pnpm test 2>&1 | tail -3
---

# Handoff: keysight

## 正在做的 Task

为 Card / Note / Alias 三种节点实现右上角的 `⋯` 三点上下文菜单 — 对应 `docs/progress/keysight.md` > Next > "Phase 6 行内菜单（Card/Note/Alias 三点菜单）"

**这个任务我自己做，不交给 codex**。参考旧 obsidian 插件的菜单实现，每种节点的菜单项见用户提供的三张截图（item 列表见下面"下一步"段）。CardNode/NoteNode/AliasNode 已有 `editing` 行内编辑能力，⋯ 菜单是它们的下一个 UI 增量。

## 已完成步骤（本 task 内部）

⋯ 菜单本身在本 session 还**没动**。本 session 时间花在以下三件事，结果都没 commit：

### 上一段：codex 完成 Phase 5f + 6 第一轮（uncommitted）

由 codex 在前一 session 落地，本 session 没碰：

- [x] 新增 `src/components/keysight/lib/quadtree.ts`（Phase 5f-1）
- [x] `src/components/keysight/hooks/useVisibleEntities.ts` 切到 Quadtree 查询 + 保留 force-visible id
- [x] `src/components/keysight/useViewport.ts` 新增 `lodLevel` 推导 + `centerOn(x, y, w, h, ew?, eh?)`（Phase 5f-2）
- [x] `src/components/keysight/nodes/{Card,Note,Alias,Task,Question}Node.tsx` 接入 lod0/lod1/lod2 三级渲染分支
- [x] 节点接入 selected / highlighted / dimmed 视觉态
- [x] `GraphView.tsx` 改受控：whiteboard / selection / focus target / highlighted ids 由外层驱动
- [x] 新建 `src/components/keysight/KeysightView.tsx`：Sidebar + GraphView 双栏布局，sidebar 宽度/折叠/tab 持久化
- [x] 新建 `src/components/keysight/sidebar/`：Sidebar / SidebarTabs / CardsList / ReviewView / FilterBar / ExportPanel / FollowView / InsightCardDetail / ContextPanel / NoteEditor 共 10 个文件
- [x] `src/App.tsx` 路由切换到 `KeysightView`
- [x] `pnpm build` 通过

### 本 session（8ec03324）追加的 5 个小修

- [x] **GraphToolbar 加 Sections / Boards 两个 dropdown**（`src/components/keysight/GraphToolbar.tsx:60+`）— 用 base-ui DropdownMenu + render prop（**不是 asChild**，base-ui Menu 不支持），右侧显示 member count / cards count，当前白板加粗
- [x] **`useWhiteboardList` 移除 wb_root 限制**（`src/components/keysight/hooks/useWhiteboardData.ts:166+`）— 所有白板都加载 + 套 `timedQuery` 加 perf 日志。原因：Boards dropdown 在子白板也要显示
- [x] **GraphView 新增 `handleJumpToSection / handleJumpToBoard`**（`src/components/keysight/GraphView.tsx:597+`）— Section 用 `allDimensions[id]` 算真实尺寸 + `viewport.actions.centerOn(...)` 居中；Board 调 `onWhiteboardChange`
- [x] **新建 Section/Note 改为视口中心**（`src/components/keysight/GraphView.tsx:31+, 569+`）— 之前用 `randomOffset()` 丢到 (200-600, 200-500)，rust 白板视口在 (-1500, -800) 看不到。新增 `viewportCenterWorld` 纯函数 + `newEntityPositionAtCenter(width, height)` callback，加 ±60/±40 抖动避免堆叠。删除了未使用的 `randomOffset`
- [x] **perf.ts 加 Long Animation Frame Observer + visibility tracking**（`src/lib/perf.ts:148+, 215+`）— LoAF 捕获 paint/GC/script 完整 frame；visibility 检测 macOS App Nap 暂停-恢复
- [x] **main.tsx 启用 LoAF + visibility 探针**（`src/main.tsx:6-15`）

### 测试 + 构建状态

- Rust: `153 passed; 0 failed; 1 ignored`
- TS: `122 passed`
- pnpm build: clean
- cargo clippy: clean

### 已捕获的诊断数据点

- [x] **冻结日志捕获 #1**：用户提供了 `16:31:20.676 ⚠️ HEARTBEAT GAP 950ms`，但**无 LONGTASK** + **无 LoAF**（当时 LoAF 还没装）+ 用户描述"啥也没做就是第一次启动"。结论暂定：可能是 V8 GC pause / WKWebView 合成器抖动，**不是用户报的"几分钟卡死"**。装了 LoAF + visibility 之后等下次复现

### 当前 working tree 状态

```
Modified (17): docs/devlog/2026-04-12.md docs/handoff/keysight.md docs/progress/keysight.md
               src/App.tsx src/components/keysight/GraphToolbar.tsx
               src/components/keysight/GraphView.tsx
               src/components/keysight/hooks/useVisibleEntities.ts
               src/components/keysight/hooks/useWhiteboardData.ts
               src/components/keysight/nodes/{AliasNode,CardNode,EntityNode,NoteNode,QuestionNode,TaskNode}.tsx
               src/components/keysight/types.ts
               src/components/keysight/useViewport.ts
               src/lib/perf.ts src/main.tsx

Untracked: Agents.md (codex auto-gen, 822 行 = CLAUDE.md 副本)
           docs/collaboration/2026-04-12-phase-5f-and-6.md (本 session 写的 codex brief)
           src/components/keysight/KeysightView.tsx
           src/components/keysight/lib/quadtree.ts
           src/components/keysight/sidebar/{Sidebar,SidebarTabs,CardsList,ReviewView,FilterBar,ExportPanel,FollowView,InsightCardDetail,ContextPanel,NoteEditor}.tsx
```

- [ ] **卡在这里**：本 session 没动 ⋯ 菜单。下一 session 第一件事是把 working tree 拆 commit 清干净，然后从零实现三个节点的 ⋯ 菜单

## 下一步具体动作

### 第一步：commit 拆分（必须先做，不要直接动新功能）

1. **commit A — `feat(keysight): Phase 5f Quadtree + LOD`**
   - `git add src/components/keysight/lib/quadtree.ts`
   - `git add src/components/keysight/hooks/useVisibleEntities.ts`
   - `git add src/components/keysight/useViewport.ts`
   - `git add src/components/keysight/types.ts`
   - `git add src/components/keysight/nodes/{Card,Note,Alias,Task,Question}Node.tsx`
   - `git add src/components/keysight/nodes/EntityNode.tsx` *(注意：本 commit 只取 lodLevel 部分；本 session 没动 EntityNode，所以这里直接 add 整个文件即可)*

2. **commit B — `feat(keysight): Phase 6 Sidebar + KeysightView 双栏布局`**
   - `git add src/components/keysight/KeysightView.tsx`
   - `git add src/components/keysight/sidebar/`
   - `git add src/components/keysight/GraphView.tsx` *(包含 GraphView 受控化 + 本 session 的 toolbar 跳转 handlers — 一起 commit 比 -p 拆分省事)*
   - `git add src/App.tsx`

3. **commit C — `feat(keysight): Toolbar Sections/Boards 跳转下拉`**
   - `git add src/components/keysight/GraphToolbar.tsx`
   - `git add src/components/keysight/hooks/useWhiteboardData.ts` *(移除 wb_root 限制 + timedQuery 包装)*

4. **commit D — `chore(perf): LoAF observer + visibility tracking + 视口中心创建`**
   - `git add src/lib/perf.ts`
   - `git add src/main.tsx`
   - 注：viewportCenterWorld + newEntityPositionAtCenter 已经在 commit B 的 GraphView 里一起 commit 掉了

5. **commit E — `docs(keysight): progress + devlog + handoff + collaboration brief`**
   - `git add docs/`

6. **决定 `Agents.md` 的命运**：codex 自动生成的 822 行副本（= CLAUDE.md 内容）。倾向 gitignore（追加到 `.gitignore`），但需要快速确认一下用户意图。

### 第二步：实现 ⋯ 三点菜单

7. **加 menu state 到 GraphView 或 KeysightView**：
   ```ts
   const [openMenuId, setOpenMenuId] = useState<string | null>(null);
   const handleOpenMenu = useCallback((id: string) => setOpenMenuId(id), []);
   const handleCloseMenu = useCallback(() => setOpenMenuId(null), []);
   ```
   传给 EntityNode → CardNode/NoteNode/AliasNode

8. **CardNode / NoteNode / AliasNode 各加一个 `⋯` 按钮** 在右上角（旧 plugin 的 `ks-graph-card-menu`）。
   - 按钮 onMouseDown stopPropagation 防止触发拖拽
   - 按钮 onClick 触发 `onOpenMenu(entity.id)`
   - 当 `openMenuId === entity.id` 时渲染菜单 popover

9. **菜单 UI**：用 base-ui DropdownMenu（参考 `GraphToolbar.tsx` 已有的 render prop 写法）。或者用 absolute positioned div + click outside detection。倾向前者，已有现成模式。

10. **菜单项**（**默认不要 "原文 / Jump to source"**）：

| Menu | Items |
|---|---|
| **Card** | Copy title · Draw connection · Related · Create alias · Move to Section · Remove from group |
| **Note** | Copy UUID + title · Draw connection · Edit title · Move to Section · Remove from group · Delete · 7 色块（背景色） |
| **Alias** | Remove from group · → 原卡（jump to source card）· Draw connection · Move to Section · Delete alias |

11. **Handler 对应 Rust commands**（绝大多数已存在，bindings.ts 都生成好了）：
    - **Copy title** / **Copy UUID + title** → `navigator.clipboard.writeText(...)` (无 Rust)
    - **Draw connection** → 进入 "draw mode" state（旧 plugin `drawingFrom` ref）+ 下次点击 target → `commands.entityConnect(from, to, "linkTo", null, null)`
    - **Related** → `commands.entityConnect(from, to, "related", null, null)`
    - **Create alias** → `commands.aliasCreate(currentWhiteboardId, cardId)` + `commands.layoutSetPosition(wb, aliasId, cardX + 540, cardY)` 放原卡右侧
    - **Move to Section** → 弹 Section picker → `commands.sectionAddMember(sectionId, entityId)`
    - **Remove from group** → 找到 entity 所在 section（遍历 `data.sections.find(s => s.cardIds.includes(id))`） → `commands.sectionRemoveMember(sectionId, entityId)`
    - **Edit title** (Note) → `setEditing({ id, field: 'note-title' })`（已有的 inline edit）
    - **Delete** (Note) → `commands.noteDelete(id)` + `queryClient.invalidateQueries({ queryKey: ['notes', wb] })`
    - **Delete alias** → `commands.aliasDelete(id)` + invalidate
    - **Note 颜色块** → `commands.noteUpdate(id, null, null, color)` + invalidate
    - **→ 原卡** → 用 `viewport.actions.centerOn(cardPos.x, cardPos.y, containerSize.width, containerSize.height, allDimensions[cardId].width, allDimensions[cardId].height)` 跳过去（cardPos 来自 `data.positions[cardId]`）

12. **参考旧 plugin 实现**（⚠️ **必读**）：`~/codes/vibe-coding/obsidian-plugin-keysight/src/components/GraphView.tsx` 搜：
    - `menuCardId` / `setMenuCardId` (~line 2470+) — Card 菜单的 state + JSX
    - `menuAliasId` (~line 2750+) — Alias 菜单
    - 编辑 note 那段 — Note 菜单
    - `cardToAliasConnect` / `relatedPickerCardId` / `drawingFrom` 相关 ref state — Draw connection / Related 的状态机
    - `ks-graph-card-menu-dropdown` CSS class

13. **新增组件**：
    - `src/components/keysight/SectionPicker.tsx`（Move to Section 弹出选择器） — 或者复用 toolbar 的 Sections dropdown 样式新建一个
    - 颜色块用 7 个 button 横排，色板：`["#fff8b3","#ffd6a5","#ffadad","#caffbf","#a0c4ff","#bdb2ff","#ffc6ff"]`（旧 plugin 色板）

14. **测试**（每个节点至少 3 个 RTL 测试）：
    - 点 ⋯ 按钮 → 菜单出现 (`screen.getByText("Copy title")`)
    - 点菜单 item → mock 的 command handler 被调
    - 点 outside / Escape → 菜单关闭

## 关键上下文（/new 之后会丢的东西）

### 本次会话的假设与决策

- **base-ui Menu 不支持 `asChild`**：`DropdownMenuTrigger` 用 `render={(props) => <Button {...props} />}` 而不是 `<DropdownMenuTrigger asChild><Button>...</Button></DropdownMenuTrigger>`。codex 的 dropdown-menu.tsx 用的是 `@base-ui/react/menu` 不是 Radix。这个写法在 GraphToolbar.tsx 里有现成例子可以抄
- **新建实体位置改用视口中心**：之前 randomOffset (200-600) 在 rust 白板（视口偏移到负坐标）下不可见。viewportCenterWorld 是新增的 helper 纯函数
- **useWhiteboardList 不再限定 wb_root**：所有白板都加载（数据小，summary 表 < 10 行）。否则 Boards 跳转 dropdown 在子白板里没数据
- **Sections dropdown 的 onJumpToSection 用 allDimensions[id]**：因为 section 真实尺寸是 computeSectionBounds 算出来的，不是 ENTITY_DIMENSIONS.section placeholder。viewport.actions.centerOn 接受 width/height 参数计算屏幕中心
- **冻结调查方向**：950ms heartbeat gap + 0 longtask + 0 LoAF（当时未装）= 强烈怀疑 GC pause / 合成器抖动 / macOS App Nap。LoAF observer 装好后等下次复现给出 script breakdown
- **Note 色块面板**：旧 plugin 的颜色面板是 7 个固定色 + click → noteUpdate(id, null, null, color)。Rust 侧 noteUpdate 已支持 color 字段
- **Alias 的 → 原卡**：alias.cardId 字段就是父卡片 id。viewport.centerOn 跳过去，需要 allDimensions[cardId]
- **Codex Phase 5f+6 全部没 commit**：虽然 codex 跑了 pnpm build clean，但没 commit。本 session 又在上面叠了 5 个小改动。**下一 session 第一件事是拆 commit 而不是动新功能**

### 试过但不行的方案

- **DropdownMenuTrigger asChild** — TypeScript 报错，base-ui 的 Trigger 不是 Radix，没有 asChild prop。改成 `render={(props) => <Button {...props} />}` 后通过
- **冻结诊断尝试 #1**：靠 longtask observer。结果：950ms 阻塞但 longtask 没报。说明阻塞不是单个 JS task。改加 LoAF + visibility tracking（更全面的探针）

### 开放问题

- **`Agents.md` 怎么处理**：codex 自动生成的副本（822 行 = CLAUDE.md 内容）。要 commit 还是 gitignore？倾向 gitignore，需要快速确认
- **Move to Section 的交互**：是弹 Section picker dropdown 还是直接进入"点击 section 选择"模式？旧 plugin 用 picker（看 `relatedPickerCardId` 那一段），但新项目可以更简单 — 用 base-ui DropdownMenu 嵌套或用一个独立 popover 都行
- **Note 颜色块 UI**：放在菜单里横排还是单独一个面板？旧 plugin 是菜单底部横排 7 个圆点。倾向同样
- **冻结的"几分钟级"复现**：用户报告过几次几分钟卡死，但本 session 只捕到一次 950ms。LoAF + visibility 探针装好了，等下次复现

## Resume 检查清单

- [ ] 读 `docs/progress/keysight.md` 确认 Active task 没变
- [ ] 读 `docs/collaboration/2026-04-12-phase-5f-and-6.md` 了解 codex 完成的 Phase 5f + 6 上下文
- [ ] 跑 `stale_check` — 期望：Rust `153 passed`, TS `122 passed`
- [ ] `git status` — 期望：17 modified + 5 untracked，**未 commit**
- [ ] `git log --oneline -3` — HEAD 应是 `bbbefca fix(keysight): strip backslash escapes from card titles (codex rescue)`
- [ ] **先决定 commit 拆分策略**（见上面"第一步" 5 个 commit），**动 ⋯ 菜单之前必须先把 working tree 清干净**
- [ ] 读旧 plugin GraphView.tsx 中 `menuCardId` / `menuAliasId` / `drawingFrom` / `relatedPickerCardId` 附近的代码作为菜单实现参考
