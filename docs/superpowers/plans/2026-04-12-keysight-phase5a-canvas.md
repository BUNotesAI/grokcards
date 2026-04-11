# Phase 5a: 画布基础设施 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 GraphView 画布核心 — CSS transform 容器 + pan/zoom 交互，为后续实体渲染和交互打基础。

**Architecture:** `useViewport` hook 管理画布状态（zoom/panX/panY），`GraphCanvas` 组件用 CSS transform 渲染，`GraphView` 壳组件接入路由。纯前端，无 Rust 后端变更。

**Tech Stack:** React 19, TypeScript, Vitest + @testing-library/react, Tailwind CSS

**Spec:** `docs/superpowers/specs/2026-04-12-keysight-phase5a-canvas.md`

**五层质量机制：**
- **L0 TDD** — hook 和组件测试先写，🔴 Red → 实现 → 🟢 Green
- **L0 Anti-Test-Theater** — 测试调用真实 hook/组件，不复制逻辑
- **L0 TS/Rust 职责边界** — 画布状态是纯 UI 状态（zoom/pan），不涉及业务数据
- **L0 IPC 类型安全** — Phase 5a 无新 command，不涉及 IPC
- **L2 LESSONS.md** — keysight 模块无 TS 侧 LESSONS（首次写 TS 组件）

---

## 文件映射

| 操作 | 文件 | 职责 |
|------|------|------|
| 创建 | `vitest.config.ts` | Vitest 测试配置 |
| 创建 | `src/test/setup.ts` | 测试 setup（jest-dom matchers） |
| 修改 | `tsconfig.json` | 添加 vitest types |
| 修改 | `package.json` | 添加 test 脚本和依赖 |
| 创建 | `src/components/keysight/useViewport.ts` | 画布状态 hook |
| 创建 | `src/components/keysight/GraphCanvas.tsx` | CSS transform 画布容器 |
| 创建 | `src/components/keysight/GraphView.tsx` | 顶层壳组件 |
| 创建 | `src/__tests__/components/keysight/useViewport.test.ts` | hook 单元测试 |
| 创建 | `src/__tests__/components/keysight/GraphCanvas.test.tsx` | 组件渲染测试 |
| 修改 | `src/App.tsx` | 添加 `/keysight` 路由 |
| 修改 | `src/components/AppShell.tsx` | 侧边栏添加 KeySight 入口 |

---

### Task 1: TS 测试基础设施

**Files:**
- 创建: `vitest.config.ts`
- 创建: `src/test/setup.ts`
- 修改: `tsconfig.json`
- 修改: `package.json`

- [ ] **Step 1: 安装测试依赖**

```bash
pnpm add -D vitest @testing-library/react @testing-library/jest-dom @testing-library/user-event jsdom
```

- [ ] **Step 2: 创建 vitest.config.ts**

```typescript
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    css: true,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

- [ ] **Step 3: 创建 src/test/setup.ts**

```typescript
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 4: 修改 tsconfig.json 添加 vitest types**

在 `compilerOptions` 中追加：

```json
"types": ["vitest/globals"]
```

- [ ] **Step 5: 在 package.json 添加 test 脚本**

在 `scripts` 中追加：

```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 6: 写一个 smoke test 验证基础设施**

创建 `src/__tests__/smoke.test.ts`：

```typescript
describe("test infrastructure", () => {
  it("vitest works", () => {
    expect(1 + 1).toBe(2);
  });
});
```

- [ ] **Step 7: 运行测试验证**

运行: `pnpm test`
期望: 1 passed

- [ ] **Step 8: 提交**

```bash
git add vitest.config.ts src/test/setup.ts src/__tests__/smoke.test.ts package.json pnpm-lock.yaml tsconfig.json
git commit -m "chore: setup vitest + testing-library test infrastructure"
```

---

### Task 2: useViewport hook（TDD）

**Files:**
- 创建: `src/components/keysight/useViewport.ts`
- 创建: `src/__tests__/components/keysight/useViewport.test.ts`

- [ ] **Step 1: 写失败测试**

创建 `src/__tests__/components/keysight/useViewport.test.ts`：

```typescript
import { renderHook, act } from "@testing-library/react";
import { useViewport } from "@/components/keysight/useViewport";

describe("useViewport", () => {
  it("初始状态: zoom=1, panX=0, panY=0", () => {
    const { result } = renderHook(() => useViewport());
    expect(result.current.state.zoom).toBe(1);
    expect(result.current.state.panX).toBe(0);
    expect(result.current.state.panY).toBe(0);
  });

  it("zoomIn 放大（×1.2）", () => {
    const { result } = renderHook(() => useViewport());
    act(() => result.current.actions.zoomIn());
    expect(result.current.state.zoom).toBeCloseTo(1.2);
  });

  it("zoomOut 缩小（÷1.2）", () => {
    const { result } = renderHook(() => useViewport());
    act(() => result.current.actions.zoomOut());
    expect(result.current.state.zoom).toBeCloseTo(1 / 1.2);
  });

  it("zoom 上边界不超过 3.0", () => {
    const { result } = renderHook(() => useViewport());
    // 连续放大 20 次
    for (let i = 0; i < 20; i++) {
      act(() => result.current.actions.zoomIn());
    }
    expect(result.current.state.zoom).toBeLessThanOrEqual(3.0);
  });

  it("zoom 下边界不低于 0.05", () => {
    const { result } = renderHook(() => useViewport());
    // 连续缩小 50 次
    for (let i = 0; i < 50; i++) {
      act(() => result.current.actions.zoomOut());
    }
    expect(result.current.state.zoom).toBeGreaterThanOrEqual(0.05);
  });

  it("resetView 重置到初始状态", () => {
    const { result } = renderHook(() => useViewport());
    act(() => result.current.actions.zoomIn());
    act(() => result.current.actions.resetView());
    expect(result.current.state.zoom).toBe(1);
    expect(result.current.state.panX).toBe(0);
    expect(result.current.state.panY).toBe(0);
  });
});
```

- [ ] **Step 2: 运行测试确认 Red**

运行: `pnpm test -- src/__tests__/components/keysight/useViewport.test.ts 2>&1`
期望: 编译失败 — 模块 `@/components/keysight/useViewport` 不存在

- [ ] **Step 3: 写最小实现**

创建 `src/components/keysight/useViewport.ts`：

```typescript
import { useCallback, useRef, useState } from "react";

const MIN_ZOOM = 0.05;
const MAX_ZOOM = 3.0;
const ZOOM_STEP = 1.2;

export interface ViewportState {
  zoom: number;
  panX: number;
  panY: number;
}

function clampZoom(z: number): number {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));
}

export function useViewport() {
  const [state, setState] = useState<ViewportState>({
    zoom: 1,
    panX: 0,
    panY: 0,
  });

  // 拖拽状态用 ref，不触发渲染
  const dragRef = useRef<{ dragging: boolean; startX: number; startY: number; startPanX: number; startPanY: number }>({
    dragging: false,
    startX: 0,
    startY: 0,
    startPanX: 0,
    startPanY: 0,
  });

  const zoomIn = useCallback(() => {
    setState((s) => ({ ...s, zoom: clampZoom(s.zoom * ZOOM_STEP) }));
  }, []);

  const zoomOut = useCallback(() => {
    setState((s) => ({ ...s, zoom: clampZoom(s.zoom / ZOOM_STEP) }));
  }, []);

  const resetView = useCallback(() => {
    setState({ zoom: 1, panX: 0, panY: 0 });
  }, []);

  // Pan: 鼠标拖拽
  const onMouseDown = useCallback((e: React.MouseEvent) => {
    // 只响应左键（button === 0）
    if (e.button !== 0) return;
    dragRef.current = {
      dragging: true,
      startX: e.clientX,
      startY: e.clientY,
      startPanX: state.panX,
      startPanY: state.panY,
    };
  }, [state.panX, state.panY]);

  const onMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragRef.current.dragging) return;
    const dx = e.clientX - dragRef.current.startX;
    const dy = e.clientY - dragRef.current.startY;
    setState((s) => ({
      ...s,
      panX: dragRef.current.startPanX + dx,
      panY: dragRef.current.startPanY + dy,
    }));
  }, []);

  const onMouseUp = useCallback(() => {
    dragRef.current.dragging = false;
  }, []);

  // Zoom: Cmd/Ctrl + wheel = zoom（鼠标中心）；普通 wheel = pan
  const onWheel = useCallback((e: React.WheelEvent) => {
    if (e.metaKey || e.ctrlKey) {
      // 以鼠标位置为中心缩放
      e.preventDefault();
      setState((s) => {
        const newZoom = clampZoom(s.zoom * (1 - e.deltaY * 0.001));
        const ratio = newZoom / s.zoom;
        // 获取鼠标相对于 viewport 的位置
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;
        return {
          zoom: newZoom,
          panX: mouseX - (mouseX - s.panX) * ratio,
          panY: mouseY - (mouseY - s.panY) * ratio,
        };
      });
    } else {
      // 普通滚动 = pan
      setState((s) => ({
        ...s,
        panX: s.panX - e.deltaX,
        panY: s.panY - e.deltaY,
      }));
    }
  }, []);

  return {
    state,
    handlers: { onMouseDown, onMouseMove, onMouseUp, onWheel },
    actions: { zoomIn, zoomOut, resetView },
  };
}
```

- [ ] **Step 4: 运行测试确认 Green**

运行: `pnpm test -- src/__tests__/components/keysight/useViewport.test.ts 2>&1`
期望: 6 passed

- [ ] **Step 5: 提交**

```bash
git add src/components/keysight/useViewport.ts src/__tests__/components/keysight/useViewport.test.ts
git commit -m "feat(keysight): add useViewport hook with TDD — zoom/pan/reset"
```

---

### Task 3: GraphCanvas 组件（TDD）

**Files:**
- 创建: `src/components/keysight/GraphCanvas.tsx`
- 创建: `src/__tests__/components/keysight/GraphCanvas.test.tsx`

- [ ] **Step 1: 写失败测试**

创建 `src/__tests__/components/keysight/GraphCanvas.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";

describe("GraphCanvas", () => {
  it("渲染 viewport 和 canvas 容器", () => {
    render(<GraphCanvas />);
    expect(screen.getByTestId("graph-viewport")).toBeInTheDocument();
    expect(screen.getByTestId("graph-canvas")).toBeInTheDocument();
  });

  it("canvas 的 transform 包含 translate 和 scale", () => {
    render(<GraphCanvas />);
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas.style.transform).toContain("translate");
    expect(canvas.style.transform).toContain("scale");
  });

  it("渲染传入的 children", () => {
    render(
      <GraphCanvas>
        <div data-testid="child-node">Hello</div>
      </GraphCanvas>
    );
    expect(screen.getByTestId("child-node")).toBeInTheDocument();
    // child 应在 canvas 内部
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas).toContainElement(screen.getByTestId("child-node"));
  });
});
```

- [ ] **Step 2: 运行测试确认 Red**

运行: `pnpm test -- src/__tests__/components/keysight/GraphCanvas.test.tsx 2>&1`
期望: 编译失败 — 模块不存在

- [ ] **Step 3: 写最小实现**

创建 `src/components/keysight/GraphCanvas.tsx`：

```tsx
import { useViewport } from "@/components/keysight/useViewport";
import { useEffect } from "react";

interface GraphCanvasProps {
  children?: React.ReactNode;
}

export function GraphCanvas({ children }: GraphCanvasProps) {
  const { state, handlers, actions } = useViewport();

  // 键盘快捷键：Cmd+= 放大，Cmd+- 缩小，Cmd+0 重置
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      switch (e.key) {
        case "=":
        case "+":
          e.preventDefault();
          actions.zoomIn();
          break;
        case "-":
          e.preventDefault();
          actions.zoomOut();
          break;
        case "0":
          e.preventDefault();
          actions.resetView();
          break;
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [actions]);

  return (
    <div
      data-testid="graph-viewport"
      className="relative h-full w-full overflow-hidden bg-muted/30"
      onMouseDown={handlers.onMouseDown}
      onMouseMove={handlers.onMouseMove}
      onMouseUp={handlers.onMouseUp}
      onMouseLeave={handlers.onMouseUp}
      onWheel={handlers.onWheel}
      style={{ cursor: "grab" }}
    >
      <div
        data-testid="graph-canvas"
        style={{
          transform: `translate(${state.panX}px, ${state.panY}px) scale(${state.zoom})`,
          transformOrigin: "0 0",
          position: "absolute",
          top: 0,
          left: 0,
        }}
      >
        {children}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 运行测试确认 Green**

运行: `pnpm test -- src/__tests__/components/keysight/GraphCanvas.test.tsx 2>&1`
期望: 3 passed

- [ ] **Step 5: 运行全部测试确认无回归**

运行: `pnpm test 2>&1`
期望: 全部通过（smoke + useViewport + GraphCanvas）

- [ ] **Step 6: 提交**

```bash
git add src/components/keysight/GraphCanvas.tsx src/__tests__/components/keysight/GraphCanvas.test.tsx
git commit -m "feat(keysight): add GraphCanvas component with TDD — CSS transform container"
```

---

### Task 4: GraphView 壳组件 + 路由接入

**Files:**
- 创建: `src/components/keysight/GraphView.tsx`
- 修改: `src/App.tsx`
- 修改: `src/components/AppShell.tsx`

- [ ] **Step 1: 创建 GraphView 壳组件**

创建 `src/components/keysight/GraphView.tsx`：

```tsx
import { GraphCanvas } from "@/components/keysight/GraphCanvas";

export function GraphView() {
  return (
    <div className="h-full w-full">
      <GraphCanvas>
        {/* 5a 阶段：调试用占位块，验证 pan/zoom 工作 */}
        <div
          style={{
            position: "absolute",
            left: 100,
            top: 100,
            width: 200,
            height: 100,
            backgroundColor: "hsl(var(--primary) / 0.15)",
            border: "1px solid hsl(var(--border))",
            borderRadius: 8,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 14,
            color: "hsl(var(--foreground))",
          }}
        >
          占位卡片 (100, 100)
        </div>
        <div
          style={{
            position: "absolute",
            left: 400,
            top: 250,
            width: 200,
            height: 100,
            backgroundColor: "hsl(var(--accent) / 0.15)",
            border: "1px solid hsl(var(--border))",
            borderRadius: 8,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 14,
            color: "hsl(var(--foreground))",
          }}
        >
          占位卡片 (400, 250)
        </div>
      </GraphCanvas>
    </div>
  );
}
```

- [ ] **Step 2: 在 App.tsx 添加 /keysight 路由**

在 `App.tsx` 中，添加 import 和路由：

```tsx
import { GraphView } from "@/components/keysight/GraphView";
```

在 `<Route element={<AppShell />}>` 内部，`<Route path="/todo" ...>` 之前添加：

```tsx
          <Route
            path="/keysight"
            element={
              <div className="absolute inset-0">
                <GraphView />
              </div>
            }
          />
```

注意：GraphView 需要全屏渲染（脱离 AppShell 的 `p-6` padding），所以用 `absolute inset-0` 覆盖。

同时修改默认路由从 `/todo` 改为 `/keysight`：

```tsx
          <Route path="*" element={<Navigate to="/keysight" replace />} />
```

- [ ] **Step 3: 在 AppShell.tsx 侧边栏添加 KeySight 入口**

在 `AppShell.tsx` 中，import `Eye` 图标（或用 `Grid3x3`）：

```tsx
import { ..., Grid3x3 } from "lucide-react";
```

在 `modules` 数组开头添加：

```typescript
  { label: "KeySight", icon: Grid3x3, path: "/keysight" },
```

在 `pageTitles` 中添加：

```typescript
  "/keysight": "KeySight",
```

- [ ] **Step 4: 修改 AppShell 让 GraphView 路由不带 padding**

AppShell 的 `<main>` 有 `p-6` padding，但 GraphView 需要全屏。修改 main 标签，根据路径动态去掉 padding：

将 `AppShell.tsx` 中的 Topbar + main 部分改为：

```tsx
function ContentArea() {
  const location = useLocation();
  // GraphView 需要全屏，不带 padding 和滚动
  const isFullscreen = location.pathname === "/keysight";

  return (
    <>
      <Topbar />
      <main className={isFullscreen ? "relative flex-1 overflow-hidden" : "flex-1 overflow-y-auto p-6"}>
        <Outlet />
      </main>
    </>
  );
}
```

在 `AppShell` 函数中，将 `<Topbar />` 和 `<main>` 替换为 `<ContentArea />`：

```tsx
export default function AppShell() {
  return (
    <TooltipProvider>
      <SidebarProvider>
        <AppSidebar />
        <SidebarInset>
          <ContentArea />
        </SidebarInset>
      </SidebarProvider>
    </TooltipProvider>
  );
}
```

- [ ] **Step 5: TS 类型检查**

运行: `pnpm build 2>&1 | tail -5`
期望: 构建成功

- [ ] **Step 6: 提交**

```bash
git add src/components/keysight/GraphView.tsx src/App.tsx src/components/AppShell.tsx
git commit -m "feat(keysight): add GraphView shell + route + sidebar entry"
```

---

### Task 5: 手动验证 + 清理

**Files:**
- 可能修改: 上述任何文件（修复问题）

- [ ] **Step 1: 启动 dev server**

运行: `pnpm tauri dev`

- [ ] **Step 2: 手动验证清单**

在浏览器中逐项验证：

| 检查项 | 操作 | 期望 |
|--------|------|------|
| 侧边栏入口 | 点击 KeySight | 导航到 /keysight |
| 画布渲染 | 查看页面 | 看到两个占位卡片 |
| 鼠标拖拽 pan | 在空白区域拖拽 | 画布跟随移动 |
| wheel pan | 滚动鼠标滚轮 | 画布上下移动 |
| Cmd+wheel zoom | Cmd+滚轮 | 以鼠标位置为中心缩放 |
| Cmd+= | 按键 | 放大 |
| Cmd+- | 按键 | 缩小 |
| Cmd+0 | 按键 | 重置视口 |
| zoom 边界 | 持续缩小/放大 | 不低于 0.05，不超过 3.0 |
| 其他页面 | 切换到 Todo | 正常显示，有 padding |

- [ ] **Step 3: 修复发现的问题**

如有问题，修复后重新运行 `pnpm test` 确认测试仍通过。

- [ ] **Step 4: 删除 smoke test**

删除 `src/__tests__/smoke.test.ts`（已完成使命）。

- [ ] **Step 5: 最终全量验证**

```bash
pnpm test           # TS 组件测试
pnpm build          # TS 类型检查 + 构建
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(keysight): Phase 5a canvas infrastructure — pan/zoom verified"
```
