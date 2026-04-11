# Phase 5b: 实体渲染 + Toolbar

## 概述

在 Phase 5a 画布基础上，加载真实数据并渲染全部 6 种实体类型（card/task/question/note/section/alias），加入完整 Toolbar 和视口裁剪。视觉风格为 Clean Elevated（白底 + 精致投影 + 渐变图标 + pill 标签，Notion/Linear 风格）。

## 范围

### 做

- TanStack Query 数据加载层（`useWhiteboardData` hook）
- 6 种实体节点组件（CardNode/TaskNode/QuestionNode/NoteNode/SectionNode/AliasNode + EntityNode 分发）
- 视口裁剪（`useVisibleEntities` — 简单 bounds 检查 + 300px buffer）
- 完整 Toolbar（状态区 + 创建区 + 操作区 + 搜索区 + 白板切换占位）
- GraphView 集成（数据加载 → 合并位置 → 裁剪 → 渲染）
- GraphCanvas 改造（viewport 从外部注入，支持 GraphView + Toolbar 共享状态）
- `useContainerSize` hook（ResizeObserver 监听容器尺寸）
- 组件测试 + hook 测试

### 不做

- ❌ 拖拽定位（5c）
- ❌ Context menu（5c）
- ❌ Edge 渲染（5d）
- ❌ 多白板切换逻辑（5e，Toolbar 上 UI 占位先留好）
- ❌ Quadtree 空间索引（5f，当前用 O(N) bounds 检查）
- ❌ LOD 分级渲染（5f）
- ❌ 行内编辑（Phase 6）

## 设计

### 1. 数据加载层

```
src/components/keysight/hooks/useWhiteboardData.ts
```

用 TanStack Query 封装 Tauri commands。实现时须调用 TanStack Query 相关 skills 确保用法正确。

```typescript
function useWhiteboardData(whiteboardId: string) {
  const cards = useQuery({ queryKey: ["cards"], queryFn: () => commands.cardQueryAll(null, null) });
  const sections = useQuery({ queryKey: ["sections", whiteboardId], queryFn: () => commands.sectionQueryAll(whiteboardId) });
  const notes = useQuery({ queryKey: ["notes", whiteboardId], queryFn: () => commands.noteQueryAll(whiteboardId) });
  const aliases = useQuery({ queryKey: ["aliases", whiteboardId], queryFn: () => commands.aliasQueryAll(whiteboardId) });
  const positions = useQuery({ queryKey: ["positions", whiteboardId], queryFn: () => commands.layoutQueryPositions(whiteboardId) });

  const syncVault = useMutation({
    mutationFn: () => commands.syncVault(),
    onSuccess: () => queryClient.invalidateQueries(),
  });

  return { cards, sections, notes, aliases, positions, isLoading: ..., syncVault };
}
```

- 白板切换时 whiteboardId 变化 → 自动重新加载
- syncVault mutation 成功后 invalidateQueries 刷新全部
- Task/Question 在 cardQueryAll 结果中（kind 字段区分），不需要单独查询

### 2. 实体节点组件

```
src/components/keysight/nodes/
├── CardNode.tsx        # atomic-card
├── TaskNode.tsx        # project-task（+ status badge: next/active/done/blocked）
├── QuestionNode.tsx    # question（+ status badge: pending/doing/done/understood）
├── NoteNode.tsx        # 便签式笔记
├── SectionNode.tsx     # 分组容器（7 色半透明背景）
├── AliasNode.tsx       # 卡片别名（ghost 样式）
└── EntityNode.tsx      # 根据 kind 分发到对应节点组件
```

**视觉风格：Clean Elevated**
- 白底卡片 + `box-shadow: 0 1px 3px rgba(0,0,0,0.06), 0 4px 12px rgba(0,0,0,0.04)`
- 圆角 12px
- 渐变图标（kind 标识）
- pill 形标签（`border-radius: 10px, background: #eff6ff`）
- 暗色主题通过 CSS variables 适配

**EntityNode 分发**：

```tsx
function EntityNode({ entity, position }: Props) {
  // 绝对定位
  const style = { position: "absolute", left: position.x, top: position.y };
  switch (entity.kind) {
    case "card": return <CardNode card={entity} style={style} />;
    case "task": return <TaskNode task={entity} style={style} />;
    case "question": return <QuestionNode question={entity} style={style} />;
    // ...
  }
}
```

**组件尺寸**：
- Card/Task/Question: 320px 宽
- Note: 200px 宽
- Section: 由成员卡片位置动态计算 bounds（min/max x/y + padding）
- Alias: 280px 宽（比 card 窄，ghost 风格）

每个节点带 `data-entity-id` 属性，为 5c 拖拽和 context menu 做准备。

### 3. Toolbar

```
src/components/keysight/GraphToolbar.tsx
```

布局四区：

```
┌──────────────────────────────────────────────────────────────────────┐
│ [3 cards · 2 notes · 100%]  [+ Section] [+ Note] [⚡Sync]  [🔍 Search...] │
│  状态区                       创建区              操作区      搜索区       │
└──────────────────────────────────────────────────────────────────────┘
```

| 区域 | 内容 | 行为 |
|------|------|------|
| 状态区 | entity 计数 + zoom 百分比 | 只读，从 data + viewport 读取 |
| 创建区 | Section 按钮、Note 按钮 | 调用 commands → invalidate → 新实体出现在画布中心 |
| 操作区 | Sync 按钮、zoom +/−/重置 | Sync → syncVault mutation；zoom → viewport actions |
| 搜索区 | 输入框 + 200ms debounce | 过滤可见卡片：匹配高亮，不匹配 dimming |
| 白板切换 | Boards 下拉 | UI 占位显示 "Root"，5e 阶段启用 |

用 shadcn Button/Input 组件，和 app 风格统一。

### 4. 视口裁剪

```
src/components/keysight/hooks/useVisibleEntities.ts
```

```typescript
function useVisibleEntities(
  entities: EntityWithPosition[],
  viewport: ViewportState,
  containerSize: { width: number; height: number }
): EntityWithPosition[]
```

算法：
1. 视口矩形从屏幕坐标转世界坐标：`left = -panX/zoom - buffer, top = -panY/zoom - buffer, right = (-panX + width)/zoom + buffer, bottom = (-panY + height)/zoom + buffer`
2. 每个实体 `(x, y, w, h)` 与世界视口做矩形相交检测
3. `useMemo` 缓存结果，依赖 `[entities, viewport, containerSize]`

Buffer = 300px（世界坐标），避免滚动时边缘闪烁。

面向 1000+ cards per whiteboard 设计，O(N) 遍历在此量级下足够（<1ms）。5f 阶段可升级为 Quadtree。

### 5. GraphView 集成

GraphView 成为数据和渲染的枢纽：

```tsx
function GraphView() {
  const viewport = useViewport();
  const containerRef = useRef();
  const containerSize = useContainerSize(containerRef);
  const data = useWhiteboardData("wb_root");

  const allEntities = useMemo(() => mergeEntitiesWithPositions(data), [data]);
  const visibleEntities = useVisibleEntities(allEntities, viewport.state, containerSize);

  if (data.isLoading) return <Loading />;

  return (
    <div ref={containerRef} className="h-full w-full flex flex-col">
      <GraphToolbar viewport={viewport} data={data} />
      <GraphCanvas viewport={viewport}>
        {visibleEntities.map(e => <EntityNode key={e.id} entity={e} position={e.position} />)}
      </GraphCanvas>
    </div>
  );
}
```

**GraphCanvas 改造**：5a 中 useViewport 在 GraphCanvas 内部创建。5b 改为从外部 props 注入，让 GraphView 可以同时给 Toolbar 和 Canvas 共享同一个 viewport 状态。

### 6. useContainerSize hook

```
src/components/keysight/hooks/useContainerSize.ts
```

用 ResizeObserver 监听容器尺寸变化，返回 `{ width: number, height: number }`。视口裁剪需要知道容器尺寸才能计算世界坐标范围。

## 测试策略

### Hook 测试

| Hook | 用例 |
|------|------|
| useWhiteboardData | mock commands → 正确数据结构；loading 状态；syncVault invalidate |
| useVisibleEntities | 视口内返回；视口外过滤；空列表；buffer 边界 |
| useContainerSize | 返回初始尺寸（mock ResizeObserver） |

### 组件测试

| 组件 | 用例 |
|------|------|
| CardNode | 渲染标题 + 内容摘要 + tags |
| TaskNode | 渲染标题 + status badge |
| QuestionNode | 渲染标题 + status badge |
| NoteNode | 渲染标题 + 内容 |
| SectionNode | 渲染标题 + 背景色 |
| AliasNode | 渲染 ghost 样式 + 原卡片标题 |
| EntityNode | 根据 kind 分发到正确组件 |
| GraphToolbar | 渲染计数 + zoom%；Sync 按钮触发回调；搜索输入 debounce |

### Mock 策略

- mock `commands` from `bindings.ts`
- TanStack Query 测试用 QueryClientProvider 包装
- 节点组件纯渲染测试（给 props，验证输出）

## 依赖

- 新增依赖：`@tanstack/react-query`
- 修改：GraphCanvas.tsx（viewport 从 props 注入）
- 修改：useViewport.ts（导出 return type 供 GraphCanvas props 使用）
- 修改：GraphView.tsx（从壳组件变为数据枢纽）
- 修改：main.tsx（包裹 QueryClientProvider）
- 无 Rust 后端变更
