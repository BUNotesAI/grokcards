# Phase 5a: 画布基础设施

## 概述

GraphView 画布核心 — 提供 CSS transform 容器 + pan/zoom 交互。这是 Phase 5 系列的基础，后续 sub-phase（实体渲染、拖拽、Edge、多白板）都构建在此之上。

本阶段只做画布容器和视口交互，不渲染任何实体。

## 范围

### 做

- `GraphCanvas` 组件 — CSS transform 容器（`translate + scale`）
- `useViewport` hook — 画布状态管理（zoom, panX, panY）
- Pan 交互 — 鼠标拖拽空白区域 + 普通 wheel 滚动
- Zoom 交互 — Cmd/Ctrl+wheel 鼠标中心缩放 + 键盘快捷键
- Zoom 范围 0.05x → 3.0x
- `GraphView` 壳组件
- 组件测试（useViewport hook + GraphCanvas 渲染）

### 不做

- ❌ 实体渲染（5b）
- ❌ 拖拽定位（5c）
- ❌ Edge 渲染（5d）
- ❌ 多白板切换（5e）
- ❌ Quadtree / LOD（5f）
- ❌ Toolbar（5b 阶段加入）
- ❌ 视口持久化 localStorage（5e 阶段）

## 设计

### 组件结构

```
src/components/keysight/
├── GraphView.tsx          # 顶层容器（组装 Canvas，未来加 Toolbar + 侧边栏）
├── GraphCanvas.tsx        # 画布核心 — pan/zoom + CSS transform 容器
└── useViewport.ts         # 画布状态 hook
```

### GraphCanvas 组件

```
┌─────────────────────────────────────────────┐
│ div.graph-viewport                          │  ← overflow: hidden
│   ┌─────────────────────────────────────┐   │     捕获 mouse/wheel 事件
│   │ div.graph-canvas                    │   │  ← transform: translate(panX, panY) scale(zoom)
│   │   {children}                        │   │     transform-origin: 0 0
│   │                                     │   │
│   └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

- viewport 占满父容器（`width: 100%, height: 100%`）
- canvas 是无限大的虚拟画布，通过 transform 映射到 viewport
- children 是未来的实体节点（5a 阶段为空或放调试用的占位块）

### useViewport hook

```typescript
interface ViewportState {
  zoom: number;    // 0.05 ~ 3.0，默认 1.0
  panX: number;    // 像素，默认 0
  panY: number;    // 像素，默认 0
}

function useViewport(): {
  state: ViewportState;
  handlers: {
    onMouseDown: (e: React.MouseEvent) => void;
    onMouseMove: (e: React.MouseEvent) => void;
    onMouseUp: (e: React.MouseEvent) => void;
    onWheel: (e: React.WheelEvent) => void;
  };
  actions: {
    zoomIn: () => void;
    zoomOut: () => void;
    resetView: () => void;
  };
}
```

### Pan 交互

| 触发方式 | 行为 |
|---------|------|
| 鼠标拖拽空白区域 | mousedown → mousemove 更新 panX/panY |
| 普通 wheel 滚动 | deltaY → panY，deltaX → panX |

拖拽判定：5a 阶段画布为空，所有 mousedown 都触发 pan。5c 阶段加入实体后，通过 event.target 判断是否点在实体上。

### Zoom 交互

| 触发方式 | 行为 |
|---------|------|
| Cmd/Ctrl + wheel | 以鼠标位置为中心缩放 |
| Cmd+= / Cmd+- | 以画布中心缩放，步进 ×1.2 / ÷1.2 |
| Cmd+0 | 重置为 zoom=1, pan=(0,0) |

**鼠标中心缩放算法**：

```
newZoom = clamp(oldZoom * (1 - deltaY * 0.001), 0.05, 3.0)
// 保持鼠标指向的世界坐标不变
panX = mouseX - (mouseX - panX) * (newZoom / oldZoom)
panY = mouseY - (mouseY - panY) * (newZoom / oldZoom)
```

### 键盘快捷键

在 GraphCanvas 挂载时注册 `keydown` listener，卸载时移除。只在 Cmd/Ctrl 按下时响应 `=`、`-`、`0`。

### GraphView 壳组件

```tsx
export function GraphView() {
  return (
    <div className="graph-view">
      <GraphCanvas />
    </div>
  );
}
```

5a 阶段只是包装，后续 sub-phase 加入 Toolbar、sidebar 等。

## 测试策略

### useViewport hook 测试（Vitest + renderHook）

| 用例 | 验证点 |
|------|--------|
| 初始状态 | zoom=1, panX=0, panY=0 |
| zoomIn action | zoom 增大（×1.2） |
| zoomOut action | zoom 减小（÷1.2） |
| zoom 上边界 | 不超过 3.0 |
| zoom 下边界 | 不低于 0.05 |
| resetView action | zoom=1, panX=0, panY=0 |

### GraphCanvas 组件测试（Vitest + RTL）

| 用例 | 验证点 |
|------|--------|
| 渲染 | viewport 和 canvas div 存在 |
| transform 样式 | canvas div style 包含 translate + scale |
| children 渲染 | 子元素出现在 canvas 内 |

## 依赖

- 无 Rust 后端变更（纯前端组件）
- 无新 Tauri command
- 需要 CSS 文件（`GraphCanvas.css` 或 Tailwind 类）
