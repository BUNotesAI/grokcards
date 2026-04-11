# Phase 5b: 实体渲染 + Toolbar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Phase 5a 画布基础上，加载真实数据并渲染全部 6 种实体类型（card/task/question/note/section/alias），加入完整 Toolbar 和视口裁剪。视觉风格为 Clean Elevated（白底 + 精致投影 + 渐变图标 + pill 标签，Notion/Linear 风格）。

**Architecture:** GraphView 成为数据枢纽 — TanStack Query 加载数据 → 合并位置 → 视口裁剪 → EntityNode 分发渲染。GraphCanvas 从内部创建 viewport 改为接受外部注入，使 Toolbar 和 Canvas 共享同一 viewport 状态。

**Tech Stack:** React 19, TypeScript, TanStack Query, Vitest + @testing-library/react, Tailwind CSS, shadcn/ui, Rust (rusqlite)

**Spec:** `docs/superpowers/specs/2026-04-12-keysight-phase5b-entity-rendering.md`

**五层质量机制：**
- **L0 TDD** — hook 和组件测试先写，Red → 实现 → Green
- **L0 Anti-Test-Theater** — 测试调用真实 hook/组件，mock 只限 Tauri commands
- **L0 TS/Rust 职责边界** — 实体数据来自 Rust，画布/搜索/选中是纯 UI 状态
- **L0 IPC 类型安全** — 新增 Task/Question query commands + specta，bindings.ts 重新生成
- **L2 LESSONS.md** — 修改模块前先读 LESSONS

**关键发现：**
- `cardQueryAll` 的 SQL 过滤 `WHERE e.kind = 'card'`，不包含 task/question。Task 和 Question 各自在 `task_fields` / `question_fields` 表有扩展字段（status, area, project）。必须先新增 Rust query commands 才能在前端渲染。
- `typedError` wrapper 使所有 commands 返回 `{ status: "ok", data: T } | { status: "error", error: E }`。TanStack Query 的 queryFn 需要 unwrap 这个结构。

---

## 文件映射

| 操作 | 文件 | 职责 |
|------|------|------|
| 创建 | `src-tauri/src/modules/keysight/models.rs` (+2 structs) | TaskEntity, QuestionEntity 模型 |
| 修改 | `src-tauri/src/modules/keysight/domain/task.rs` | 新增 `query_all` 函数 |
| 修改 | `src-tauri/src/modules/keysight/domain/question.rs` | 新增 `query_all` 函数 |
| 修改 | `src-tauri/src/modules/keysight/commands.rs` | 新增 2 个 query commands |
| 修改 | `src-tauri/src/lib.rs` | collect_commands 注册 |
| 修改 | `src/main.tsx` | QueryClientProvider 包裹 |
| 创建 | `src/lib/commandResult.ts` | typedError unwrap 工具函数 |
| 修改 | `src/components/keysight/useViewport.ts` | 导出 UseViewportReturn 类型 |
| 修改 | `src/components/keysight/GraphCanvas.tsx` | viewport 从外部 props 注入 |
| 创建 | `src/components/keysight/hooks/useContainerSize.ts` | ResizeObserver hook |
| 创建 | `src/components/keysight/hooks/useWhiteboardData.ts` | TanStack Query 数据加载 |
| 创建 | `src/components/keysight/hooks/useVisibleEntities.ts` | 视口裁剪 |
| 创建 | `src/components/keysight/nodes/CardNode.tsx` | 原子卡片节点 |
| 创建 | `src/components/keysight/nodes/TaskNode.tsx` | 任务节点 |
| 创建 | `src/components/keysight/nodes/QuestionNode.tsx` | 问题节点 |
| 创建 | `src/components/keysight/nodes/NoteNode.tsx` | 笔记节点 |
| 创建 | `src/components/keysight/nodes/SectionNode.tsx` | 分组容器节点 |
| 创建 | `src/components/keysight/nodes/AliasNode.tsx` | 别名节点 |
| 创建 | `src/components/keysight/nodes/EntityNode.tsx` | 类型分发 |
| 创建 | `src/components/keysight/GraphToolbar.tsx` | 工具栏 |
| 修改 | `src/components/keysight/GraphView.tsx` | 从壳变为数据枢纽 |
| 创建 | `src/components/keysight/types.ts` | 前端统一实体类型定义 |
| 修改 | `src/__tests__/components/keysight/GraphCanvas.test.tsx` | 适配新 props |
| 创建 | `src/__tests__/components/keysight/hooks/useContainerSize.test.ts` | hook 测试 |
| 创建 | `src/__tests__/components/keysight/hooks/useWhiteboardData.test.ts` | hook 测试 |
| 创建 | `src/__tests__/components/keysight/hooks/useVisibleEntities.test.ts` | hook 测试 |
| 创建 | `src/__tests__/components/keysight/nodes/CardNode.test.tsx` | 组件测试 |
| 创建 | `src/__tests__/components/keysight/nodes/TaskNode.test.tsx` | 组件测试 |
| 创建 | `src/__tests__/components/keysight/nodes/QuestionNode.test.tsx` | 组件测试 |
| 创建 | `src/__tests__/components/keysight/nodes/NoteNode.test.tsx` | 组件测试 |
| 创建 | `src/__tests__/components/keysight/nodes/SectionNode.test.tsx` | 组件测试 |
| 创建 | `src/__tests__/components/keysight/nodes/AliasNode.test.tsx` | 组件测试 |
| 创建 | `src/__tests__/components/keysight/nodes/EntityNode.test.tsx` | 组件测试 |
| 创建 | `src/__tests__/components/keysight/GraphToolbar.test.tsx` | 组件测试 |

---

### Task 1: Rust — Task/Question query commands（TDD）

**Files:**
- 修改: `src-tauri/src/modules/keysight/models.rs`
- 修改: `src-tauri/src/modules/keysight/domain/task.rs`
- 修改: `src-tauri/src/modules/keysight/domain/question.rs`
- 修改: `src-tauri/src/modules/keysight/commands.rs`
- 修改: `src-tauri/src/lib.rs`

**背景：** `cardQueryAll` 的 SQL 固定过滤 `WHERE e.kind = 'card'`，不返回 task/question。Task 在 `task_fields` 表有 status/area/project 字段，Question 在 `question_fields` 表有 status 字段。需要新增 typed query commands。

- [ ] **Step 1: 在 models.rs 添加 TaskEntity 和 QuestionEntity 类型**

在 `src-tauri/src/modules/keysight/models.rs` 的 `CardAlias` struct 之后添加：

```rust
/// 任务实体（entities + task_fields 的联合查询结果）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskEntity {
    pub id: String,
    pub title: String,
    pub content: String,
    pub whiteboard_id: String,
    pub status: String,
    #[serde(default)]
    pub area: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

/// 问题实体（entities + question_fields 的联合查询结果）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct QuestionEntity {
    pub id: String,
    pub title: String,
    pub content: String,
    pub whiteboard_id: String,
    pub status: String,
}
```

- [ ] **Step 2: 在 task.rs 写 query_all 测试（Red）**

在 `src-tauri/src/modules/keysight/domain/task.rs` 的 `mod tests` 内追加：

```rust
    #[test]
    fn test_query_all_returns_typed_tasks() {
        let conn = test_conn();
        seed_task(&conn);
        let tasks = query_all(&conn, "wb_root").unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task_test0001");
        assert_eq!(tasks[0].title, "【TASK】Test Task");
        assert_eq!(tasks[0].status, "next");
        assert_eq!(tasks[0].area, Some("backend".to_string()));
        assert_eq!(tasks[0].project, Some("keysight".to_string()));
    }

    #[test]
    fn test_query_all_empty_whiteboard() {
        let conn = test_conn();
        seed_task(&conn);
        // task 在 wb_root 白板，查 other 应为空
        let tasks = query_all(&conn, "other").unwrap();
        assert!(tasks.is_empty());
    }
```

运行: `cargo test --workspace -- task::tests::test_query_all 2>&1`
期望: 编译失败 — `query_all` 函数不存在

- [ ] **Step 3: 在 task.rs 实现 query_all（Green）**

在 `src-tauri/src/modules/keysight/domain/task.rs` 中，在文件顶部添加 import：

```rust
use crate::modules::keysight::models::TaskEntity;
```

在 `by_status` 函数之后添加：

```rust
/// 查询指定白板的所有任务。
pub(super) fn query_all(conn: &Connection, whiteboard_id: &str) -> Result<Vec<TaskEntity>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, \
         t.status, t.area, t.project \
         FROM entities e JOIN task_fields t ON e.id = t.entity_id \
         WHERE e.whiteboard_id = ?1 ORDER BY e.title"
    )?;
    let rows = stmt.query_map([whiteboard_id], |r| {
        Ok(TaskEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            content: r.get(2)?,
            whiteboard_id: r.get(3)?,
            status: r.get(4)?,
            area: r.get(5)?,
            project: r.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(KeysightError::from)
}
```

运行: `cargo test --workspace -- task::tests::test_query_all 2>&1`
期望: 2 passed

- [ ] **Step 4: 在 question.rs 写 query_all 测试（Red）**

在 `src-tauri/src/modules/keysight/domain/question.rs` 的 `mod tests` 内追加：

```rust
    use crate::modules::keysight::models::QuestionEntity;

    #[test]
    fn test_query_all_returns_typed_questions() {
        let conn = test_conn();
        seed_question(&conn);
        let questions = query_all(&conn, "wb_root").unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].id, "q_test00001");
        assert_eq!(questions[0].title, "【QUE】Test Question");
        assert_eq!(questions[0].status, "pending");
    }

    #[test]
    fn test_query_all_empty_whiteboard() {
        let conn = test_conn();
        seed_question(&conn);
        let questions = query_all(&conn, "other").unwrap();
        assert!(questions.is_empty());
    }
```

运行: `cargo test --workspace -- question::tests::test_query_all 2>&1`
期望: 编译失败 — `query_all` 函数不存在

- [ ] **Step 5: 在 question.rs 实现 query_all（Green）**

在 `src-tauri/src/modules/keysight/domain/question.rs` 中，在文件顶部添加 import：

```rust
use crate::modules::keysight::models::QuestionEntity;
```

在 `by_status` 函数之后添加：

```rust
/// 查询指定白板的所有问题。
pub(super) fn query_all(conn: &Connection, whiteboard_id: &str) -> Result<Vec<QuestionEntity>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, \
         q.status \
         FROM entities e JOIN question_fields q ON e.id = q.entity_id \
         WHERE e.whiteboard_id = ?1 ORDER BY e.title"
    )?;
    let rows = stmt.query_map([whiteboard_id], |r| {
        Ok(QuestionEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            content: r.get(2)?,
            whiteboard_id: r.get(3)?,
            status: r.get(4)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(KeysightError::from)
}
```

运行: `cargo test --workspace -- question::tests::test_query_all 2>&1`
期望: 2 passed

- [ ] **Step 6: 在 commands.rs 添加 task_query_all 和 question_query_all commands**

在 `src-tauri/src/modules/keysight/commands.rs` 中：

1. 在文件顶部 `use super::domain::` 行附近添加：

```rust
use super::domain::{task, question};
use super::models::{TaskEntity, QuestionEntity};
```

注意：如果 `TaskEntity` 和 `QuestionEntity` 已被其他 import 覆盖，只需在现有 models import 行追加。

2. 在 `// ============================================================ // Note` 注释块之前添加：

```rust
// ============================================================
// Task
// ============================================================

/// 查询指定白板的所有 task。
#[tauri::command]
#[specta::specta]
pub fn task_query_all(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
) -> Result<Vec<TaskEntity>, AppError> {
    let conn = state.db.lock().unwrap();
    task::query_all(&conn, &whiteboard_id).map_err(Into::into)
}

// ============================================================
// Question
// ============================================================

/// 查询指定白板的所有 question。
#[tauri::command]
#[specta::specta]
pub fn question_query_all(
    state: State<'_, KeysightState>,
    whiteboard_id: String,
) -> Result<Vec<QuestionEntity>, AppError> {
    let conn = state.db.lock().unwrap();
    question::query_all(&conn, &whiteboard_id).map_err(Into::into)
}
```

- [ ] **Step 7: 在 lib.rs 注册新 commands**

在 `src-tauri/src/lib.rs` 的 `collect_commands![]` 中，在 `// keysight: note` 注释之前添加：

```rust
        // keysight: task
        modules::keysight::commands::task_query_all,
        // keysight: question
        modules::keysight::commands::question_query_all,
```

- [ ] **Step 8: 编译 + 重新生成 bindings.ts**

运行: `cargo clippy --workspace -- -D warnings 2>&1`
期望: 无 error、无 warning

运行: `cargo test export_bindings 2>&1`
期望: bindings.ts 更新，包含 `taskQueryAll` 和 `questionQueryAll` 以及 `TaskEntity` 和 `QuestionEntity` 类型

- [ ] **Step 9: 全量 Rust 测试**

运行: `cargo test --workspace 2>&1`
期望: 全部通过（包括新增的 4 个 query_all 测试）

- [ ] **Step 10: 提交**

```bash
git add src-tauri/src/modules/keysight/models.rs src-tauri/src/modules/keysight/domain/task.rs src-tauri/src/modules/keysight/domain/question.rs src-tauri/src/modules/keysight/commands.rs src-tauri/src/lib.rs src/bindings.ts
git commit -m "feat(keysight): add task/question query commands with TDD"
```

---

### Task 2: TanStack Query 基础设施 + typedError unwrap

**Files:**
- 修改: `src/main.tsx`
- 创建: `src/lib/commandResult.ts`
- 创建: `src/components/keysight/types.ts`

- [ ] **Step 1: 安装 TanStack Query**

```bash
pnpm add @tanstack/react-query
```

- [ ] **Step 2: 在 main.tsx 包裹 QueryClientProvider**

修改 `src/main.tsx`：

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import "./index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // 画布场景：数据变化少，长缓存 + 手动 invalidate
      staleTime: 5 * 60 * 1000,
      retry: 1,
    },
  },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 3: 创建 commandResult.ts — typedError unwrap 工具函数**

bindings.ts 中所有 commands 返回 `{ status: "ok", data: T } | { status: "error", error: E }`。TanStack Query 的 queryFn 需要 unwrap 为 T 或 throw。

创建 `src/lib/commandResult.ts`：

```typescript
/**
 * 解包 tauri-specta typedError 的返回值。
 *
 * bindings.ts 中的 commands 返回 { status: "ok", data: T } | { status: "error", error: E }。
 * 此函数提取 data 或将 error 转为 throw，适配 TanStack Query 的 queryFn 期望。
 */
export async function unwrapCommand<T, E>(
  promise: Promise<{ status: "ok"; data: T } | { status: "error"; error: E }>,
): Promise<T> {
  const result = await promise;
  if (result.status === "ok") {
    return result.data;
  }
  throw result.error;
}
```

- [ ] **Step 4: 创建 types.ts — 前端统一实体类型**

创建 `src/components/keysight/types.ts`：

```typescript
import type {
  AtomicCard,
  GraphSection,
  GraphNote,
  CardAlias,
  TaskEntity,
  QuestionEntity,
  Position,
} from "@/bindings";

/** 实体类型标识 */
export type EntityKind = "card" | "task" | "question" | "note" | "section" | "alias";

/** 各类型实体的联合 — 带位置信息 */
export type EntityWithPosition =
  | { kind: "card"; entity: AtomicCard; position: Position; id: string }
  | { kind: "task"; entity: TaskEntity; position: Position; id: string }
  | { kind: "question"; entity: QuestionEntity; position: Position; id: string }
  | { kind: "note"; entity: GraphNote; position: Position; id: string }
  | { kind: "section"; entity: GraphSection; position: Position; id: string }
  | { kind: "alias"; entity: CardAlias; position: Position; id: string };

/** 各类型实体的预设宽高（世界坐标像素），视口裁剪和 section bounds 计算用 */
export const ENTITY_DIMENSIONS: Record<EntityKind, { width: number; height: number }> = {
  card: { width: 320, height: 160 },
  task: { width: 320, height: 140 },
  question: { width: 320, height: 140 },
  note: { width: 200, height: 120 },
  section: { width: 400, height: 300 },   // section 的实际尺寸由成员位置动态计算
  alias: { width: 280, height: 100 },
};
```

- [ ] **Step 5: TS 类型检查**

运行: `pnpm build 2>&1 | tail -5`
期望: 构建成功

- [ ] **Step 6: 提交**

```bash
git add src/main.tsx src/lib/commandResult.ts src/components/keysight/types.ts package.json pnpm-lock.yaml
git commit -m "feat(keysight): add TanStack Query setup + typedError unwrap + entity types"
```

---

### Task 3: GraphCanvas refactor + useContainerSize（TDD）

**Files:**
- 修改: `src/components/keysight/useViewport.ts`
- 修改: `src/components/keysight/GraphCanvas.tsx`
- 创建: `src/components/keysight/hooks/useContainerSize.ts`
- 修改: `src/__tests__/components/keysight/GraphCanvas.test.tsx`
- 创建: `src/__tests__/components/keysight/hooks/useContainerSize.test.ts`

- [ ] **Step 1: 在 useViewport.ts 导出 return type**

在 `src/components/keysight/useViewport.ts` 末尾追加类型导出：

```typescript
/** useViewport hook 的返回值类型，供 GraphCanvas props 使用 */
export type UseViewportReturn = ReturnType<typeof useViewport>;
```

- [ ] **Step 2: 写 useContainerSize 测试（Red）**

创建 `src/__tests__/components/keysight/hooks/useContainerSize.test.ts`：

```typescript
import { renderHook } from "@testing-library/react";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useRef } from "react";

// mock ResizeObserver — jsdom 不支持
const mockObserve = vi.fn();
const mockDisconnect = vi.fn();

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", class {
    constructor(private cb: ResizeObserverCallback) {}
    observe = mockObserve;
    unobserve = vi.fn();
    disconnect = mockDisconnect;
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("useContainerSize", () => {
  it("返回初始尺寸 { width: 0, height: 0 }", () => {
    const { result } = renderHook(() => {
      const ref = useRef<HTMLDivElement>(null);
      return useContainerSize(ref);
    });
    expect(result.current.width).toBe(0);
    expect(result.current.height).toBe(0);
  });

  it("挂载后调用 ResizeObserver.observe", () => {
    const div = document.createElement("div");
    const { result } = renderHook(() => {
      const ref = useRef<HTMLDivElement>(div);
      // 手动设置 current 模拟 ref 已挂载
      ref.current = div;
      return useContainerSize(ref);
    });
    expect(mockObserve).toHaveBeenCalled();
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/hooks/useContainerSize.test.ts 2>&1`
期望: 编译失败 — 模块不存在

- [ ] **Step 3: 实现 useContainerSize（Green）**

创建 `src/components/keysight/hooks/useContainerSize.ts`：

```typescript
import { useEffect, useState, type RefObject } from "react";

/** 容器尺寸 */
export interface ContainerSize {
  width: number;
  height: number;
}

/**
 * 监听容器尺寸变化的 hook。
 *
 * 基于 ResizeObserver，返回容器当前 width/height。
 * 视口裁剪需要知道容器尺寸才能计算世界坐标范围。
 */
export function useContainerSize(ref: RefObject<HTMLElement | null>): ContainerSize {
  const [size, setSize] = useState<ContainerSize>({ width: 0, height: 0 });

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        setSize({ width, height });
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);

  return size;
}
```

运行: `pnpm test -- src/__tests__/components/keysight/hooks/useContainerSize.test.ts 2>&1`
期望: 2 passed

- [ ] **Step 4: 重构 GraphCanvas — viewport 从外部 props 注入**

修改 `src/components/keysight/GraphCanvas.tsx`：

```tsx
import { useEffect } from "react";
import type { UseViewportReturn } from "@/components/keysight/useViewport";

interface GraphCanvasProps {
  /** 从外部注入的 viewport 状态（GraphView 创建，Toolbar 和 Canvas 共享） */
  viewport: UseViewportReturn;
  children?: React.ReactNode;
}

/**
 * 知识图谱画布容器
 *
 * 提供可缩放、可平移的无限画布：
 * - 鼠标拖拽平移
 * - Cmd/Ctrl + 滚轮缩放（以鼠标位置为中心）
 * - Cmd/Ctrl + =/-/0 键盘快捷键（放大/缩小/重置）
 * - 普通滚轮平移
 *
 * viewport 由外部通过 props 注入，使 GraphView 可以同时给 Toolbar 和 Canvas 共享同一个 viewport 状态。
 * children 渲染在 transform 容器内，跟随视口变换。
 */
export function GraphCanvas({ viewport, children }: GraphCanvasProps) {
  const { state, handlers, actions } = viewport;

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

- [ ] **Step 5: 更新 GraphCanvas 测试**

修改 `src/__tests__/components/keysight/GraphCanvas.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { renderHook } from "@testing-library/react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { useViewport } from "@/components/keysight/useViewport";

/** 创建一个真实的 viewport 用于测试 */
function createTestViewport() {
  const { result } = renderHook(() => useViewport());
  return result.current;
}

describe("GraphCanvas", () => {
  it("渲染 viewport 和 canvas 容器", () => {
    const viewport = createTestViewport();
    render(<GraphCanvas viewport={viewport} />);
    expect(screen.getByTestId("graph-viewport")).toBeInTheDocument();
    expect(screen.getByTestId("graph-canvas")).toBeInTheDocument();
  });

  it("canvas 的 transform 包含 translate 和 scale", () => {
    const viewport = createTestViewport();
    render(<GraphCanvas viewport={viewport} />);
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas.style.transform).toContain("translate");
    expect(canvas.style.transform).toContain("scale");
  });

  it("渲染传入的 children", () => {
    const viewport = createTestViewport();
    render(
      <GraphCanvas viewport={viewport}>
        <div data-testid="child-node">Hello</div>
      </GraphCanvas>,
    );
    expect(screen.getByTestId("child-node")).toBeInTheDocument();
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas).toContainElement(screen.getByTestId("child-node"));
  });
});
```

- [ ] **Step 6: 临时更新 GraphView 以适配新 props**

修改 `src/components/keysight/GraphView.tsx`，让它暂时能编译通过（Task 8 会完整重写）：

```tsx
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { useViewport } from "@/components/keysight/useViewport";

/**
 * KeySight 知识图谱主视图
 *
 * 5b 阶段临时状态 — Task 8 会完整重写为数据枢纽。
 */
export function GraphView() {
  const viewport = useViewport();

  return (
    <div className="h-full w-full">
      <GraphCanvas viewport={viewport}>
        {/* 占位 — Task 8 替换为真实实体 */}
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
      </GraphCanvas>
    </div>
  );
}
```

- [ ] **Step 7: 运行全部测试**

运行: `pnpm test 2>&1`
期望: 全部通过（useViewport + GraphCanvas 新 props + useContainerSize）

运行: `pnpm build 2>&1 | tail -5`
期望: 构建成功

- [ ] **Step 8: 提交**

```bash
git add src/components/keysight/useViewport.ts src/components/keysight/GraphCanvas.tsx src/components/keysight/GraphView.tsx src/components/keysight/hooks/useContainerSize.ts src/__tests__/components/keysight/GraphCanvas.test.tsx src/__tests__/components/keysight/hooks/useContainerSize.test.ts
git commit -m "refactor(keysight): inject viewport into GraphCanvas + add useContainerSize hook"
```

---

### Task 4: useWhiteboardData + useVisibleEntities（TDD）

**Files:**
- 创建: `src/components/keysight/hooks/useWhiteboardData.ts`
- 创建: `src/components/keysight/hooks/useVisibleEntities.ts`
- 创建: `src/__tests__/components/keysight/hooks/useWhiteboardData.test.ts`
- 创建: `src/__tests__/components/keysight/hooks/useVisibleEntities.test.ts`

- [ ] **Step 1: 写 useWhiteboardData 测试（Red）**

创建 `src/__tests__/components/keysight/hooks/useWhiteboardData.test.ts`：

```typescript
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useWhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import { commands } from "@/bindings";
import { vi } from "vitest";
import React from "react";

// mock Tauri commands
vi.mock("@/bindings", () => ({
  commands: {
    cardQueryAll: vi.fn(),
    sectionQueryAll: vi.fn(),
    noteQueryAll: vi.fn(),
    aliasQueryAll: vi.fn(),
    taskQueryAll: vi.fn(),
    questionQueryAll: vi.fn(),
    layoutQueryPositions: vi.fn(),
    syncVault: vi.fn(),
  },
}));

/** 创建测试用 QueryClient wrapper */
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

describe("useWhiteboardData", () => {
  beforeEach(() => {
    vi.mocked(commands.cardQueryAll).mockResolvedValue({
      status: "ok",
      data: [{ id: "card_001", title: "Test Card", content: "body", filePath: "", tags: [], linkTo: [], related: [], understanding: "", source: "", seeAlso: [] }],
    } as any);
    vi.mocked(commands.sectionQueryAll).mockResolvedValue({ status: "ok", data: [] } as any);
    vi.mocked(commands.noteQueryAll).mockResolvedValue({ status: "ok", data: [] } as any);
    vi.mocked(commands.aliasQueryAll).mockResolvedValue({ status: "ok", data: [] } as any);
    vi.mocked(commands.taskQueryAll).mockResolvedValue({ status: "ok", data: [] } as any);
    vi.mocked(commands.questionQueryAll).mockResolvedValue({ status: "ok", data: [] } as any);
    vi.mocked(commands.layoutQueryPositions).mockResolvedValue({ status: "ok", data: {} } as any);
  });

  it("加载完成后返回 cards 数据", async () => {
    const { result } = renderHook(() => useWhiteboardData("wb_root"), { wrapper: createWrapper() });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.cards).toHaveLength(1);
    expect(result.current.cards[0].id).toBe("card_001");
  });

  it("isLoading 初始为 true", () => {
    const { result } = renderHook(() => useWhiteboardData("wb_root"), { wrapper: createWrapper() });
    expect(result.current.isLoading).toBe(true);
  });

  it("所有查询使用 whiteboardId 作 queryKey", async () => {
    renderHook(() => useWhiteboardData("wb_root"), { wrapper: createWrapper() });

    await waitFor(() => {
      expect(commands.sectionQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.noteQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.aliasQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.taskQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.questionQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.layoutQueryPositions).toHaveBeenCalledWith("wb_root");
    });
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/hooks/useWhiteboardData.test.ts 2>&1`
期望: 编译失败 — 模块不存在

- [ ] **Step 2: 实现 useWhiteboardData（Green）**

创建 `src/components/keysight/hooks/useWhiteboardData.ts`：

```typescript
import { useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type {
  AtomicCard,
  GraphSection,
  GraphNote,
  CardAlias,
  TaskEntity,
  QuestionEntity,
  Position,
} from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/** useWhiteboardData 返回的数据结构 */
export interface WhiteboardData {
  cards: AtomicCard[];
  sections: GraphSection[];
  notes: GraphNote[];
  aliases: CardAlias[];
  tasks: TaskEntity[];
  questions: QuestionEntity[];
  positions: Record<string, Position>;
  isLoading: boolean;
  syncVault: ReturnType<typeof useMutation>;
}

/**
 * 加载指定白板的所有实体数据。
 *
 * 用 TanStack Query 封装 Tauri commands，白板切换时自动重新加载。
 * syncVault mutation 成功后 invalidateQueries 刷新全部。
 */
export function useWhiteboardData(whiteboardId: string): WhiteboardData {
  const queryClient = useQueryClient();

  const cardsQuery = useQuery({
    queryKey: ["cards"],
    queryFn: () => unwrapCommand(commands.cardQueryAll(null, null)),
  });

  const sectionsQuery = useQuery({
    queryKey: ["sections", whiteboardId],
    queryFn: () => unwrapCommand(commands.sectionQueryAll(whiteboardId)),
  });

  const notesQuery = useQuery({
    queryKey: ["notes", whiteboardId],
    queryFn: () => unwrapCommand(commands.noteQueryAll(whiteboardId)),
  });

  const aliasesQuery = useQuery({
    queryKey: ["aliases", whiteboardId],
    queryFn: () => unwrapCommand(commands.aliasQueryAll(whiteboardId)),
  });

  const tasksQuery = useQuery({
    queryKey: ["tasks", whiteboardId],
    queryFn: () => unwrapCommand(commands.taskQueryAll(whiteboardId)),
  });

  const questionsQuery = useQuery({
    queryKey: ["questions", whiteboardId],
    queryFn: () => unwrapCommand(commands.questionQueryAll(whiteboardId)),
  });

  const positionsQuery = useQuery({
    queryKey: ["positions", whiteboardId],
    queryFn: () => unwrapCommand(commands.layoutQueryPositions(whiteboardId)),
  });

  const syncVaultMutation = useMutation({
    mutationFn: () => unwrapCommand(commands.syncVault()),
    onSuccess: () => {
      queryClient.invalidateQueries();
    },
  });

  const isLoading = useMemo(
    () =>
      cardsQuery.isLoading ||
      sectionsQuery.isLoading ||
      notesQuery.isLoading ||
      aliasesQuery.isLoading ||
      tasksQuery.isLoading ||
      questionsQuery.isLoading ||
      positionsQuery.isLoading,
    [
      cardsQuery.isLoading,
      sectionsQuery.isLoading,
      notesQuery.isLoading,
      aliasesQuery.isLoading,
      tasksQuery.isLoading,
      questionsQuery.isLoading,
      positionsQuery.isLoading,
    ],
  );

  return {
    cards: cardsQuery.data ?? [],
    sections: sectionsQuery.data ?? [],
    notes: notesQuery.data ?? [],
    aliases: aliasesQuery.data ?? [],
    tasks: tasksQuery.data ?? [],
    questions: questionsQuery.data ?? [],
    positions: positionsQuery.data ?? {},
    isLoading,
    syncVault: syncVaultMutation,
  };
}
```

运行: `pnpm test -- src/__tests__/components/keysight/hooks/useWhiteboardData.test.ts 2>&1`
期望: 3 passed

- [ ] **Step 3: 写 useVisibleEntities 测试（Red）**

创建 `src/__tests__/components/keysight/hooks/useVisibleEntities.test.ts`：

```typescript
import { renderHook } from "@testing-library/react";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { ViewportState } from "@/components/keysight/useViewport";

/** 创建测试用实体 */
function makeCard(id: string, x: number, y: number): EntityWithPosition {
  return {
    kind: "card",
    id,
    entity: {
      id,
      title: `Card ${id}`,
      content: "",
      filePath: "",
      tags: [],
      linkTo: [],
      related: [],
      understanding: "",
      source: "",
      seeAlso: [],
    },
    position: { x, y },
  };
}

const defaultViewport: ViewportState = { zoom: 1, panX: 0, panY: 0 };
const defaultSize = { width: 1000, height: 800 };

describe("useVisibleEntities", () => {
  it("视口内的实体被返回", () => {
    const entities = [makeCard("c1", 100, 100)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
    expect(result.current[0].id).toBe("c1");
  });

  it("视口外的实体被过滤", () => {
    // 实体在 (5000, 5000)，视口 1000x800，远超范围
    const entities = [makeCard("far", 5000, 5000)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(0);
  });

  it("300px buffer 内的边缘实体被保留", () => {
    // 实体在 (1200, 400)，视口宽 1000 + buffer 300 = 1300，卡片宽 320
    // 实体右边界 = 1200 + 320 = 1520，视口右边界 = 1300 → 实体左边界 1200 < 1300
    const entities = [makeCard("edge", 1200, 400)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
  });

  it("空实体列表返回空数组", () => {
    const { result } = renderHook(() =>
      useVisibleEntities([], defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(0);
  });

  it("zoom 缩小时可见范围扩大", () => {
    // zoom=0.5 → 视口覆盖 2000x1600 世界坐标
    // 实体在 (1500, 1200) 应可见
    const viewport: ViewportState = { zoom: 0.5, panX: 0, panY: 0 };
    const entities = [makeCard("zoomed", 1500, 1200)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, viewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
  });

  it("pan 偏移后视口跟随", () => {
    // panX=-500 → 视口向右移 500px，左边界 = -(-500)/1 - 300 = 200
    // 实体在 (0, 0)，右边界 = 320 < 200? 不，左边界 200 > 0 但 entity 右边界 = 320 > 200 → 可见
    const viewport: ViewportState = { zoom: 1, panX: -500, panY: 0 };
    const entities = [makeCard("panned", 300, 100)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, viewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/hooks/useVisibleEntities.test.ts 2>&1`
期望: 编译失败 — 模块不存在

- [ ] **Step 4: 实现 useVisibleEntities（Green）**

创建 `src/components/keysight/hooks/useVisibleEntities.ts`：

```typescript
import { useMemo } from "react";
import type { ViewportState } from "@/components/keysight/useViewport";
import type { EntityWithPosition } from "@/components/keysight/types";
import { ENTITY_DIMENSIONS } from "@/components/keysight/types";

/** 视口裁剪 buffer（世界坐标像素），避免滚动时边缘闪烁 */
const BUFFER = 300;

/**
 * 视口裁剪 hook — 只返回视口内可见的实体。
 *
 * 算法：
 * 1. 视口矩形从屏幕坐标转世界坐标
 * 2. 每个实体 (x, y, w, h) 与世界视口做矩形相交检测
 * 3. useMemo 缓存结果
 *
 * O(N) 遍历，面向 1000+ cards，<1ms。5f 阶段可升级为 Quadtree。
 */
export function useVisibleEntities(
  entities: EntityWithPosition[],
  viewport: ViewportState,
  containerSize: { width: number; height: number },
): EntityWithPosition[] {
  return useMemo(() => {
    const { zoom, panX, panY } = viewport;
    const { width, height } = containerSize;

    // 世界坐标下的视口边界
    const viewLeft = -panX / zoom - BUFFER;
    const viewTop = -panY / zoom - BUFFER;
    const viewRight = (-panX + width) / zoom + BUFFER;
    const viewBottom = (-panY + height) / zoom + BUFFER;

    return entities.filter((e) => {
      const dim = ENTITY_DIMENSIONS[e.kind];
      const ex = e.position.x;
      const ey = e.position.y;
      const ew = dim.width;
      const eh = dim.height;

      // 矩形相交检测：两个矩形不相交的逆命题
      return !(ex + ew < viewLeft || ex > viewRight || ey + eh < viewTop || ey > viewBottom);
    });
  }, [entities, viewport, containerSize]);
}
```

运行: `pnpm test -- src/__tests__/components/keysight/hooks/useVisibleEntities.test.ts 2>&1`
期望: 6 passed

- [ ] **Step 5: 运行全部测试**

运行: `pnpm test 2>&1`
期望: 全部通过

- [ ] **Step 6: 提交**

```bash
git add src/components/keysight/hooks/useWhiteboardData.ts src/components/keysight/hooks/useVisibleEntities.ts src/__tests__/components/keysight/hooks/useWhiteboardData.test.ts src/__tests__/components/keysight/hooks/useVisibleEntities.test.ts
git commit -m "feat(keysight): add useWhiteboardData + useVisibleEntities hooks with TDD"
```

---

### Task 5: CardNode + NoteNode + SectionNode + EntityNode（TDD）

**Files:**
- 创建: `src/components/keysight/nodes/CardNode.tsx`
- 创建: `src/components/keysight/nodes/NoteNode.tsx`
- 创建: `src/components/keysight/nodes/SectionNode.tsx`
- 创建: `src/components/keysight/nodes/EntityNode.tsx`
- 创建: `src/__tests__/components/keysight/nodes/CardNode.test.tsx`
- 创建: `src/__tests__/components/keysight/nodes/NoteNode.test.tsx`
- 创建: `src/__tests__/components/keysight/nodes/SectionNode.test.tsx`
- 创建: `src/__tests__/components/keysight/nodes/EntityNode.test.tsx`

- [ ] **Step 1: 写 CardNode 测试（Red）**

创建 `src/__tests__/components/keysight/nodes/CardNode.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { CardNode } from "@/components/keysight/nodes/CardNode";
import type { AtomicCard } from "@/bindings";

const mockCard: AtomicCard = {
  id: "card_test0001",
  filePath: "whiteboard/test.md",
  title: "测试卡片标题",
  content: "这是卡片的正文内容，可能会很长很长需要截断显示。",
  tags: ["rust", "testing"],
  linkTo: [],
  related: [],
  understanding: "",
  source: "",
  seeAlso: [],
};

describe("CardNode", () => {
  it("渲染卡片标题", () => {
    render(<CardNode card={mockCard} style={{}} />);
    expect(screen.getByText("测试卡片标题")).toBeInTheDocument();
  });

  it("渲染内容摘要", () => {
    render(<CardNode card={mockCard} style={{}} />);
    expect(screen.getByText(/这是卡片的正文内容/)).toBeInTheDocument();
  });

  it("渲染 tags 为 pill 样式", () => {
    render(<CardNode card={mockCard} style={{}} />);
    expect(screen.getByText("rust")).toBeInTheDocument();
    expect(screen.getByText("testing")).toBeInTheDocument();
  });

  it("带 data-entity-id 属性", () => {
    const { container } = render(<CardNode card={mockCard} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("card_test0001");
  });

  it("无 tags 时不渲染 tag 区域", () => {
    const noTagCard = { ...mockCard, tags: [] };
    render(<CardNode card={noTagCard} style={{}} />);
    expect(screen.queryByText("rust")).not.toBeInTheDocument();
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/CardNode.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 2: 实现 CardNode（Green）**

创建 `src/components/keysight/nodes/CardNode.tsx`：

```tsx
import type { CSSProperties } from "react";
import type { AtomicCard } from "@/bindings";

interface CardNodeProps {
  card: AtomicCard;
  style: CSSProperties;
}

/** 内容截断到指定长度 */
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + "...";
}

/**
 * 原子卡片节点 — Clean Elevated 风格
 *
 * 白底、精致投影、渐变图标、pill 标签。
 * 宽度 320px，显示标题 + 内容摘要 + tags。
 */
export function CardNode({ card, style }: CardNodeProps) {
  return (
    <div
      data-entity-id={card.id}
      className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
      style={{ ...style, width: 320 }}
    >
      {/* 标题行 */}
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-blue-400 to-indigo-500 text-[10px] font-bold text-white">
          C
        </span>
        <h3 className="flex-1 truncate text-sm font-semibold text-foreground">
          {card.title}
        </h3>
      </div>

      {/* 内容摘要 */}
      {card.content && (
        <p className="px-4 pb-2 text-xs leading-relaxed text-muted-foreground">
          {truncate(card.content, 120)}
        </p>
      )}

      {/* Tags */}
      {card.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 px-4 pb-3">
          {card.tags.slice(0, 5).map((tag) => (
            <span
              key={tag}
              className="rounded-full bg-blue-50 px-2 py-0.5 text-[10px] font-medium text-blue-600 dark:bg-blue-950 dark:text-blue-300"
            >
              {tag}
            </span>
          ))}
          {card.tags.length > 5 && (
            <span className="rounded-full bg-muted px-2 py-0.5 text-[10px] text-muted-foreground">
              +{card.tags.length - 5}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/CardNode.test.tsx 2>&1`
期望: 5 passed

- [ ] **Step 3: 写 NoteNode 测试（Red）**

创建 `src/__tests__/components/keysight/nodes/NoteNode.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { NoteNode } from "@/components/keysight/nodes/NoteNode";
import type { GraphNote } from "@/bindings";

const mockNote: GraphNote = {
  id: "note_test0001",
  title: "测试笔记",
  content: "笔记内容详情",
  color: "yellow",
};

describe("NoteNode", () => {
  it("渲染笔记标题", () => {
    render(<NoteNode note={mockNote} style={{}} />);
    expect(screen.getByText("测试笔记")).toBeInTheDocument();
  });

  it("渲染笔记内容", () => {
    render(<NoteNode note={mockNote} style={{}} />);
    expect(screen.getByText("笔记内容详情")).toBeInTheDocument();
  });

  it("带 data-entity-id 属性", () => {
    const { container } = render(<NoteNode note={mockNote} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("note_test0001");
  });

  it("无 color 时使用默认黄色", () => {
    const noColor = { ...mockNote, color: undefined };
    render(<NoteNode note={noColor} style={{}} />);
    // 不崩溃即可 — 具体样式由 CSS 验证
    expect(screen.getByText("测试笔记")).toBeInTheDocument();
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/NoteNode.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 4: 实现 NoteNode（Green）**

创建 `src/components/keysight/nodes/NoteNode.tsx`：

```tsx
import type { CSSProperties } from "react";
import type { GraphNote } from "@/bindings";

interface NoteNodeProps {
  note: GraphNote;
  style: CSSProperties;
}

/** 笔记颜色映射 — 便签纸配色 */
const NOTE_COLORS: Record<string, { bg: string; border: string }> = {
  yellow: { bg: "bg-amber-50 dark:bg-amber-950/40", border: "border-amber-200 dark:border-amber-800" },
  blue: { bg: "bg-blue-50 dark:bg-blue-950/40", border: "border-blue-200 dark:border-blue-800" },
  green: { bg: "bg-emerald-50 dark:bg-emerald-950/40", border: "border-emerald-200 dark:border-emerald-800" },
  pink: { bg: "bg-pink-50 dark:bg-pink-950/40", border: "border-pink-200 dark:border-pink-800" },
};

/**
 * 便签式笔记节点 — 暖色背景
 *
 * 宽度 200px，显示标题 + 内容。
 */
export function NoteNode({ note, style }: NoteNodeProps) {
  const colors = NOTE_COLORS[note.color ?? "yellow"] ?? NOTE_COLORS.yellow;

  return (
    <div
      data-entity-id={note.id}
      className={`select-none rounded-lg border ${colors.border} ${colors.bg} shadow-[0_1px_2px_rgba(0,0,0,0.04)]`}
      style={{ ...style, width: 200 }}
    >
      <div className="px-3 pt-2 pb-1">
        <h3 className="truncate text-sm font-semibold text-foreground">{note.title}</h3>
      </div>
      {note.content && (
        <p className="px-3 pb-2 text-xs leading-relaxed text-muted-foreground">
          {note.content.length > 80 ? note.content.slice(0, 80) + "..." : note.content}
        </p>
      )}
    </div>
  );
}
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/NoteNode.test.tsx 2>&1`
期望: 4 passed

- [ ] **Step 5: 写 SectionNode 测试（Red）**

创建 `src/__tests__/components/keysight/nodes/SectionNode.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { SectionNode } from "@/components/keysight/nodes/SectionNode";
import type { GraphSection, Position } from "@/bindings";

const mockSection: GraphSection = {
  id: "sec_test0001",
  title: "核心概念",
  cardIds: ["card_001", "card_002"],
  color: "blue",
};

/** section 成员卡片的位置 */
const memberPositions: Record<string, Position> = {
  card_001: { x: 100, y: 100 },
  card_002: { x: 400, y: 300 },
};

describe("SectionNode", () => {
  it("渲染 section 标题", () => {
    render(<SectionNode section={mockSection} memberPositions={memberPositions} style={{}} />);
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });

  it("带 data-entity-id 属性", () => {
    const { container } = render(
      <SectionNode section={mockSection} memberPositions={memberPositions} style={{}} />,
    );
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("sec_test0001");
  });

  it("无成员位置时仍能渲染（使用 section 自身位置）", () => {
    render(<SectionNode section={mockSection} memberPositions={{}} style={{}} />);
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });

  it("无 color 时使用默认色", () => {
    const noColor = { ...mockSection, color: undefined };
    render(<SectionNode section={noColor} memberPositions={memberPositions} style={{}} />);
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/SectionNode.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 6: 实现 SectionNode（Green）**

创建 `src/components/keysight/nodes/SectionNode.tsx`：

```tsx
import { useMemo, type CSSProperties } from "react";
import type { GraphSection, Position } from "@/bindings";
import { ENTITY_DIMENSIONS } from "@/components/keysight/types";

interface SectionNodeProps {
  section: GraphSection;
  /** 成员卡片的位置映射 — 用于计算 section 边界 */
  memberPositions: Record<string, Position>;
  style: CSSProperties;
}

/** Section 7 色配色方案 — 半透明背景 */
const SECTION_COLORS: Record<string, { bg: string; border: string; text: string }> = {
  default: { bg: "bg-gray-100/50 dark:bg-gray-800/30", border: "border-gray-300/50 dark:border-gray-600/50", text: "text-gray-600 dark:text-gray-300" },
  purple: { bg: "bg-purple-100/50 dark:bg-purple-900/30", border: "border-purple-300/50 dark:border-purple-600/50", text: "text-purple-600 dark:text-purple-300" },
  blue: { bg: "bg-blue-100/50 dark:bg-blue-900/30", border: "border-blue-300/50 dark:border-blue-600/50", text: "text-blue-600 dark:text-blue-300" },
  green: { bg: "bg-emerald-100/50 dark:bg-emerald-900/30", border: "border-emerald-300/50 dark:border-emerald-600/50", text: "text-emerald-600 dark:text-emerald-300" },
  orange: { bg: "bg-orange-100/50 dark:bg-orange-900/30", border: "border-orange-300/50 dark:border-orange-600/50", text: "text-orange-600 dark:text-orange-300" },
  yellow: { bg: "bg-yellow-100/50 dark:bg-yellow-900/30", border: "border-yellow-300/50 dark:border-yellow-600/50", text: "text-yellow-600 dark:text-yellow-300" },
  pink: { bg: "bg-pink-100/50 dark:bg-pink-900/30", border: "border-pink-300/50 dark:border-pink-600/50", text: "text-pink-600 dark:text-pink-300" },
};

const PADDING = 40;

/**
 * 分组容器节点 — 半透明背景矩形
 *
 * 大小由成员卡片位置动态计算（min/max + padding）。
 * 无成员时使用最小尺寸。7 色配色。
 */
export function SectionNode({ section, memberPositions, style }: SectionNodeProps) {
  const colors = SECTION_COLORS[section.color ?? "default"] ?? SECTION_COLORS.default;

  const bounds = useMemo(() => {
    const memberPos = section.cardIds
      .map((id) => memberPositions[id])
      .filter((p): p is Position => p != null);

    if (memberPos.length === 0) {
      // 无成员：最小尺寸
      return { width: 200, height: 100 };
    }

    const cardDim = ENTITY_DIMENSIONS.card;
    const xs = memberPos.map((p) => p.x);
    const ys = memberPos.map((p) => p.y);
    const minX = Math.min(...xs);
    const minY = Math.min(...ys);
    const maxX = Math.max(...xs) + cardDim.width;
    const maxY = Math.max(...ys) + cardDim.height;

    return {
      width: maxX - minX + PADDING * 2,
      height: maxY - minY + PADDING * 2,
    };
  }, [section.cardIds, memberPositions]);

  return (
    <div
      data-entity-id={section.id}
      className={`select-none rounded-2xl border-2 border-dashed ${colors.border} ${colors.bg}`}
      style={{ ...style, width: bounds.width, height: bounds.height, minWidth: 200, minHeight: 100 }}
    >
      <div className="px-3 pt-2">
        <h3 className={`text-xs font-bold uppercase tracking-wide ${colors.text}`}>
          {section.title}
        </h3>
      </div>
    </div>
  );
}
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/SectionNode.test.tsx 2>&1`
期望: 4 passed

- [ ] **Step 7: 写 EntityNode 测试（Red）**

创建 `src/__tests__/components/keysight/nodes/EntityNode.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import type { EntityWithPosition } from "@/components/keysight/types";

describe("EntityNode", () => {
  it("kind=card 时渲染 CardNode", () => {
    const entity: EntityWithPosition = {
      kind: "card",
      id: "card_001",
      entity: {
        id: "card_001",
        title: "Test Card",
        content: "body",
        filePath: "",
        tags: [],
        linkTo: [],
        related: [],
        understanding: "",
        source: "",
        seeAlso: [],
      },
      position: { x: 0, y: 0 },
    };
    render(<EntityNode entity={entity} allPositions={{}} />);
    expect(screen.getByText("Test Card")).toBeInTheDocument();
  });

  it("kind=note 时渲染 NoteNode", () => {
    const entity: EntityWithPosition = {
      kind: "note",
      id: "note_001",
      entity: {
        id: "note_001",
        title: "Test Note",
        content: "note body",
      },
      position: { x: 0, y: 0 },
    };
    render(<EntityNode entity={entity} allPositions={{}} />);
    expect(screen.getByText("Test Note")).toBeInTheDocument();
  });

  it("绝对定位到 position 坐标", () => {
    const entity: EntityWithPosition = {
      kind: "card",
      id: "card_002",
      entity: {
        id: "card_002",
        title: "Positioned",
        content: "",
        filePath: "",
        tags: [],
        linkTo: [],
        related: [],
        understanding: "",
        source: "",
        seeAlso: [],
      },
      position: { x: 150, y: 250 },
    };
    const { container } = render(<EntityNode entity={entity} allPositions={{}} />);
    const wrapper = container.firstElementChild as HTMLElement;
    expect(wrapper.style.left).toBe("150px");
    expect(wrapper.style.top).toBe("250px");
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/EntityNode.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 8: 实现 EntityNode（Green）**

创建 `src/components/keysight/nodes/EntityNode.tsx`：

```tsx
import type { CSSProperties } from "react";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { Position } from "@/bindings";
import { CardNode } from "./CardNode";
import { NoteNode } from "./NoteNode";
import { SectionNode } from "./SectionNode";

interface EntityNodeProps {
  entity: EntityWithPosition;
  /** 所有实体位置 — SectionNode 计算 bounds 用 */
  allPositions: Record<string, Position>;
}

/**
 * 实体节点分发器 — 根据 kind 路由到对应节点组件。
 *
 * 负责绝对定位和分发，不含业务逻辑。
 * TaskNode / QuestionNode / AliasNode 在 Task 6 补充。
 */
export function EntityNode({ entity, allPositions }: EntityNodeProps) {
  const posStyle: CSSProperties = {
    position: "absolute",
    left: entity.position.x,
    top: entity.position.y,
  };

  switch (entity.kind) {
    case "card":
      return <CardNode card={entity.entity} style={posStyle} />;
    case "note":
      return <NoteNode note={entity.entity} style={posStyle} />;
    case "section":
      return (
        <SectionNode
          section={entity.entity}
          memberPositions={allPositions}
          style={posStyle}
        />
      );
    case "task":
      // Task 6 实现，临时 fallback
      return (
        <div data-entity-id={entity.id} style={{ ...posStyle, width: 320 }} className="rounded-xl border bg-card p-3 text-sm">
          {entity.entity.title}
        </div>
      );
    case "question":
      // Task 6 实现，临时 fallback
      return (
        <div data-entity-id={entity.id} style={{ ...posStyle, width: 320 }} className="rounded-xl border bg-card p-3 text-sm">
          {entity.entity.title}
        </div>
      );
    case "alias":
      // Task 6 实现，临时 fallback
      return (
        <div data-entity-id={entity.id} style={{ ...posStyle, width: 280, opacity: 0.6 }} className="rounded-xl border bg-card p-3 text-sm">
          Alias: {entity.entity.cardId}
        </div>
      );
  }
}
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/EntityNode.test.tsx 2>&1`
期望: 3 passed

- [ ] **Step 9: 运行全部测试**

运行: `pnpm test 2>&1`
期望: 全部通过

- [ ] **Step 10: 提交**

```bash
git add src/components/keysight/nodes/ src/__tests__/components/keysight/nodes/
git commit -m "feat(keysight): add CardNode + NoteNode + SectionNode + EntityNode with TDD"
```

---

### Task 6: TaskNode + QuestionNode + AliasNode（TDD）

**Files:**
- 创建: `src/components/keysight/nodes/TaskNode.tsx`
- 创建: `src/components/keysight/nodes/QuestionNode.tsx`
- 创建: `src/components/keysight/nodes/AliasNode.tsx`
- 修改: `src/components/keysight/nodes/EntityNode.tsx`
- 创建: `src/__tests__/components/keysight/nodes/TaskNode.test.tsx`
- 创建: `src/__tests__/components/keysight/nodes/QuestionNode.test.tsx`
- 创建: `src/__tests__/components/keysight/nodes/AliasNode.test.tsx`

- [ ] **Step 1: 写 TaskNode 测试（Red）**

创建 `src/__tests__/components/keysight/nodes/TaskNode.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { TaskNode } from "@/components/keysight/nodes/TaskNode";
import type { TaskEntity } from "@/bindings";

const mockTask: TaskEntity = {
  id: "task_test0001",
  title: "【TASK】实现搜索功能",
  content: "任务描述正文",
  whiteboardId: "wb_root",
  status: "active",
  area: "backend",
  project: "keysight",
};

describe("TaskNode", () => {
  it("渲染任务标题", () => {
    render(<TaskNode task={mockTask} style={{}} />);
    expect(screen.getByText("【TASK】实现搜索功能")).toBeInTheDocument();
  });

  it("渲染 status badge", () => {
    render(<TaskNode task={mockTask} style={{}} />);
    expect(screen.getByText("active")).toBeInTheDocument();
  });

  it("渲染 area 和 project", () => {
    render(<TaskNode task={mockTask} style={{}} />);
    expect(screen.getByText("backend")).toBeInTheDocument();
    expect(screen.getByText("keysight")).toBeInTheDocument();
  });

  it("带 data-entity-id 属性", () => {
    const { container } = render(<TaskNode task={mockTask} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("task_test0001");
  });

  it("无 area/project 时不崩溃", () => {
    const noMeta = { ...mockTask, area: undefined, project: undefined };
    render(<TaskNode task={noMeta} style={{}} />);
    expect(screen.getByText("【TASK】实现搜索功能")).toBeInTheDocument();
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/TaskNode.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 2: 实现 TaskNode（Green）**

创建 `src/components/keysight/nodes/TaskNode.tsx`：

```tsx
import type { CSSProperties } from "react";
import type { TaskEntity } from "@/bindings";

interface TaskNodeProps {
  task: TaskEntity;
  style: CSSProperties;
}

/** 任务状态 badge 样式 */
const STATUS_STYLES: Record<string, string> = {
  next: "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300",
  active: "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300",
  done: "bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400",
  blocked: "bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300",
};

/**
 * 任务节点 — 卡片变体 + status badge + area/project
 *
 * 宽度 320px，和 CardNode 同族但带状态标记。
 */
export function TaskNode({ task, style }: TaskNodeProps) {
  const badgeClass = STATUS_STYLES[task.status] ?? STATUS_STYLES.next;

  return (
    <div
      data-entity-id={task.id}
      className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
      style={{ ...style, width: 320 }}
    >
      {/* 标题行 + status badge */}
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-emerald-400 to-green-600 text-[10px] font-bold text-white">
          T
        </span>
        <h3 className="flex-1 truncate text-sm font-semibold text-foreground">
          {task.title}
        </h3>
        <span className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${badgeClass}`}>
          {task.status}
        </span>
      </div>

      {/* 内容摘要 */}
      {task.content && (
        <p className="px-4 pb-2 text-xs leading-relaxed text-muted-foreground">
          {task.content.length > 100 ? task.content.slice(0, 100) + "..." : task.content}
        </p>
      )}

      {/* Area / Project 标签 */}
      {(task.area || task.project) && (
        <div className="flex gap-1 px-4 pb-3">
          {task.area && (
            <span className="rounded-full bg-gray-100 px-2 py-0.5 text-[10px] font-medium text-gray-600 dark:bg-gray-800 dark:text-gray-300">
              {task.area}
            </span>
          )}
          {task.project && (
            <span className="rounded-full bg-violet-50 px-2 py-0.5 text-[10px] font-medium text-violet-600 dark:bg-violet-950 dark:text-violet-300">
              {task.project}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/TaskNode.test.tsx 2>&1`
期望: 5 passed

- [ ] **Step 3: 写 QuestionNode 测试（Red）**

创建 `src/__tests__/components/keysight/nodes/QuestionNode.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { QuestionNode } from "@/components/keysight/nodes/QuestionNode";
import type { QuestionEntity } from "@/bindings";

const mockQuestion: QuestionEntity = {
  id: "q_test00001",
  title: "【QUE】为什么需要 FTS5",
  content: "问题详细描述",
  whiteboardId: "wb_root",
  status: "pending",
};

describe("QuestionNode", () => {
  it("渲染问题标题", () => {
    render(<QuestionNode question={mockQuestion} style={{}} />);
    expect(screen.getByText("【QUE】为什么需要 FTS5")).toBeInTheDocument();
  });

  it("渲染 status badge", () => {
    render(<QuestionNode question={mockQuestion} style={{}} />);
    expect(screen.getByText("pending")).toBeInTheDocument();
  });

  it("带 data-entity-id 属性", () => {
    const { container } = render(<QuestionNode question={mockQuestion} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("q_test00001");
  });

  it("不同 status 显示不同样式", () => {
    const doing = { ...mockQuestion, status: "doing" };
    render(<QuestionNode question={doing} style={{}} />);
    expect(screen.getByText("doing")).toBeInTheDocument();
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/QuestionNode.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 4: 实现 QuestionNode（Green）**

创建 `src/components/keysight/nodes/QuestionNode.tsx`：

```tsx
import type { CSSProperties } from "react";
import type { QuestionEntity } from "@/bindings";

interface QuestionNodeProps {
  question: QuestionEntity;
  style: CSSProperties;
}

/** 问题状态 badge 样式 */
const STATUS_STYLES: Record<string, string> = {
  pending: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300",
  doing: "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300",
  done: "bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400",
  understood: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300",
};

/**
 * 问题节点 — 卡片变体 + status badge
 *
 * 宽度 320px，和 CardNode 同族但带问题状态标记。
 */
export function QuestionNode({ question, style }: QuestionNodeProps) {
  const badgeClass = STATUS_STYLES[question.status] ?? STATUS_STYLES.pending;

  return (
    <div
      data-entity-id={question.id}
      className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
      style={{ ...style, width: 320 }}
    >
      {/* 标题行 + status badge */}
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-amber-400 to-orange-500 text-[10px] font-bold text-white">
          Q
        </span>
        <h3 className="flex-1 truncate text-sm font-semibold text-foreground">
          {question.title}
        </h3>
        <span className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${badgeClass}`}>
          {question.status}
        </span>
      </div>

      {/* 内容摘要 */}
      {question.content && (
        <p className="px-4 pb-3 text-xs leading-relaxed text-muted-foreground">
          {question.content.length > 100 ? question.content.slice(0, 100) + "..." : question.content}
        </p>
      )}
    </div>
  );
}
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/QuestionNode.test.tsx 2>&1`
期望: 4 passed

- [ ] **Step 5: 写 AliasNode 测试（Red）**

创建 `src/__tests__/components/keysight/nodes/AliasNode.test.tsx`：

```tsx
import { render, screen } from "@testing-library/react";
import { AliasNode } from "@/components/keysight/nodes/AliasNode";
import type { CardAlias } from "@/bindings";

const mockAlias: CardAlias = {
  aliasId: "alias_test001",
  cardId: "card_001",
};

describe("AliasNode", () => {
  it("渲染原卡片标题", () => {
    render(<AliasNode alias={mockAlias} originalTitle="原始卡片标题" style={{}} />);
    expect(screen.getByText("原始卡片标题")).toBeInTheDocument();
  });

  it("带 ghost/translucent 样式标记", () => {
    const { container } = render(
      <AliasNode alias={mockAlias} originalTitle="标题" style={{}} />,
    );
    const node = container.firstElementChild as HTMLElement;
    // 检查 opacity 类或 style
    expect(node.className).toContain("opacity");
  });

  it("带 data-entity-id 属性（使用 aliasId）", () => {
    const { container } = render(
      <AliasNode alias={mockAlias} originalTitle="标题" style={{}} />,
    );
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("alias_test001");
  });

  it("显示 alias 标识", () => {
    render(<AliasNode alias={mockAlias} originalTitle="标题" style={{}} />);
    // 应该有某种 alias 标识（图标或文字）
    expect(screen.getByText(/alias/i)).toBeInTheDocument();
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/AliasNode.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 6: 实现 AliasNode（Green）**

创建 `src/components/keysight/nodes/AliasNode.tsx`：

```tsx
import type { CSSProperties } from "react";
import type { CardAlias } from "@/bindings";

interface AliasNodeProps {
  alias: CardAlias;
  /** 原始卡片的标题（从 cards 数据中查找） */
  originalTitle: string;
  style: CSSProperties;
}

/**
 * 别名节点 — ghost/translucent 风格
 *
 * 宽度 280px，半透明显示原卡片标题。表示这张卡片在别处的引用。
 */
export function AliasNode({ alias, originalTitle, style }: AliasNodeProps) {
  return (
    <div
      data-entity-id={alias.aliasId}
      className="select-none rounded-xl border border-dashed border-border/50 bg-card/60 opacity-70 shadow-[0_1px_2px_rgba(0,0,0,0.03)]"
      style={{ ...style, width: 280 }}
    >
      <div className="flex items-center gap-2 px-3 pt-2 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-gray-300 to-gray-400 text-[10px] font-bold text-white dark:from-gray-600 dark:to-gray-700">
          A
        </span>
        <h3 className="flex-1 truncate text-sm font-medium text-foreground/80">
          {originalTitle}
        </h3>
      </div>
      <p className="px-3 pb-2 text-[10px] text-muted-foreground/60">
        Alias of {alias.cardId}
      </p>
    </div>
  );
}
```

运行: `pnpm test -- src/__tests__/components/keysight/nodes/AliasNode.test.tsx 2>&1`
期望: 4 passed

- [ ] **Step 7: 更新 EntityNode — 替换临时 fallback 为真实组件**

修改 `src/components/keysight/nodes/EntityNode.tsx`，替换 task/question/alias 分支：

```tsx
import type { CSSProperties } from "react";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { Position } from "@/bindings";
import { CardNode } from "./CardNode";
import { TaskNode } from "./TaskNode";
import { QuestionNode } from "./QuestionNode";
import { NoteNode } from "./NoteNode";
import { SectionNode } from "./SectionNode";
import { AliasNode } from "./AliasNode";

interface EntityNodeProps {
  entity: EntityWithPosition;
  /** 所有实体位置 — SectionNode 计算 bounds 用 */
  allPositions: Record<string, Position>;
  /** 所有卡片标题映射 — AliasNode 显示原卡片标题用 */
  cardTitles?: Record<string, string>;
}

/**
 * 实体节点分发器 — 根据 kind 路由到对应节点组件。
 *
 * 负责绝对定位和分发，不含业务逻辑。
 */
export function EntityNode({ entity, allPositions, cardTitles = {} }: EntityNodeProps) {
  const posStyle: CSSProperties = {
    position: "absolute",
    left: entity.position.x,
    top: entity.position.y,
  };

  switch (entity.kind) {
    case "card":
      return <CardNode card={entity.entity} style={posStyle} />;
    case "task":
      return <TaskNode task={entity.entity} style={posStyle} />;
    case "question":
      return <QuestionNode question={entity.entity} style={posStyle} />;
    case "note":
      return <NoteNode note={entity.entity} style={posStyle} />;
    case "section":
      return (
        <SectionNode
          section={entity.entity}
          memberPositions={allPositions}
          style={posStyle}
        />
      );
    case "alias":
      return (
        <AliasNode
          alias={entity.entity}
          originalTitle={cardTitles[entity.entity.cardId] ?? entity.entity.cardId}
          style={posStyle}
        />
      );
  }
}
```

- [ ] **Step 8: 运行全部测试**

运行: `pnpm test 2>&1`
期望: 全部通过

- [ ] **Step 9: 提交**

```bash
git add src/components/keysight/nodes/ src/__tests__/components/keysight/nodes/
git commit -m "feat(keysight): add TaskNode + QuestionNode + AliasNode with TDD"
```

---

### Task 7: GraphToolbar（TDD）

**Files:**
- 创建: `src/components/keysight/GraphToolbar.tsx`
- 创建: `src/__tests__/components/keysight/GraphToolbar.test.tsx`

- [ ] **Step 1: 写 GraphToolbar 测试（Red）**

创建 `src/__tests__/components/keysight/GraphToolbar.test.tsx`：

```tsx
import { render, screen, fireEvent, act } from "@testing-library/react";
import { renderHook } from "@testing-library/react";
import { GraphToolbar } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import { vi } from "vitest";

/** 创建 mock 数据 */
function createMockData() {
  return {
    cards: [{ id: "c1" }, { id: "c2" }],
    notes: [{ id: "n1" }],
    sections: [{ id: "s1" }],
    tasks: [],
    questions: [],
    aliases: [],
  };
}

/** 创建真实 viewport */
function createTestViewport() {
  const { result } = renderHook(() => useViewport());
  return result.current;
}

describe("GraphToolbar", () => {
  it("显示实体计数", () => {
    const viewport = createTestViewport();
    const data = createMockData();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 2, notes: 1, sections: 1, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    expect(screen.getByText(/2 cards/)).toBeInTheDocument();
    expect(screen.getByText(/1 note/)).toBeInTheDocument();
  });

  it("显示 zoom 百分比", () => {
    const viewport = createTestViewport();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("Sync 按钮调用 onSync", () => {
    const viewport = createTestViewport();
    const onSync = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={onSync}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /sync/i }));
    expect(onSync).toHaveBeenCalledTimes(1);
  });

  it("搜索输入触发 onSearchChange", () => {
    const viewport = createTestViewport();
    const onSearchChange = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={onSearchChange}
      />,
    );
    fireEvent.change(screen.getByPlaceholderText(/search/i), {
      target: { value: "test" },
    });
    expect(onSearchChange).toHaveBeenCalledWith("test");
  });

  it("+ Section 按钮调用 onCreateSection", () => {
    const viewport = createTestViewport();
    const onCreateSection = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={onCreateSection}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /section/i }));
    expect(onCreateSection).toHaveBeenCalledTimes(1);
  });
});
```

运行: `pnpm test -- src/__tests__/components/keysight/GraphToolbar.test.tsx 2>&1`
期望: 编译失败

- [ ] **Step 2: 实现 GraphToolbar（Green）**

创建 `src/components/keysight/GraphToolbar.tsx`：

```tsx
import type { UseViewportReturn } from "@/components/keysight/useViewport";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Minus, Plus, RotateCcw, RefreshCw, FolderPlus, StickyNote, Search } from "lucide-react";

/** 实体计数 */
export interface EntityCounts {
  cards: number;
  notes: number;
  sections: number;
  tasks: number;
  questions: number;
  aliases: number;
}

interface GraphToolbarProps {
  viewport: UseViewportReturn;
  entityCounts: EntityCounts;
  onSync: () => void;
  onCreateSection: () => void;
  onCreateNote: () => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
}

/**
 * 画布工具栏 — 4 个区域
 *
 * 状态区：实体计数 + zoom 百分比
 * 创建区：+ Section / + Note
 * 操作区：Sync + zoom +/-/reset
 * 搜索区：输入框（debounce 由父组件管理）
 */
export function GraphToolbar({
  viewport,
  entityCounts,
  onSync,
  onCreateSection,
  onCreateNote,
  searchQuery,
  onSearchChange,
}: GraphToolbarProps) {
  const { state, actions } = viewport;
  const zoomPercent = Math.round(state.zoom * 100);

  const totalEntities =
    entityCounts.cards +
    entityCounts.notes +
    entityCounts.sections +
    entityCounts.tasks +
    entityCounts.questions +
    entityCounts.aliases;

  return (
    <div className="flex items-center gap-3 border-b border-border bg-background/80 px-4 py-2 backdrop-blur-sm">
      {/* 状态区 */}
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        {entityCounts.cards > 0 && (
          <span>{entityCounts.cards} card{entityCounts.cards !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.tasks > 0 && (
          <span>{entityCounts.tasks} task{entityCounts.tasks !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.questions > 0 && (
          <span>{entityCounts.questions} question{entityCounts.questions !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.notes > 0 && (
          <span>{entityCounts.notes} note{entityCounts.notes !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.sections > 0 && (
          <span>{entityCounts.sections} section{entityCounts.sections !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.aliases > 0 && (
          <span>{entityCounts.aliases} alias{entityCounts.aliases !== 1 ? "es" : ""}</span>
        )}
        {totalEntities === 0 && <span>Empty whiteboard</span>}
        <span className="text-muted-foreground/50">|</span>
        <span>{zoomPercent}%</span>
      </div>

      {/* 分隔 */}
      <div className="flex-1" />

      {/* 创建区 */}
      <Button variant="ghost" size="sm" onClick={onCreateSection} aria-label="Create Section">
        <FolderPlus className="mr-1 h-4 w-4" />
        Section
      </Button>
      <Button variant="ghost" size="sm" onClick={onCreateNote} aria-label="Create Note">
        <StickyNote className="mr-1 h-4 w-4" />
        Note
      </Button>

      {/* 操作区 */}
      <div className="flex items-center gap-1 border-l border-border pl-3">
        <Button variant="ghost" size="icon" onClick={onSync} aria-label="Sync">
          <RefreshCw className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="icon" onClick={actions.zoomOut} aria-label="Zoom out">
          <Minus className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="icon" onClick={actions.zoomIn} aria-label="Zoom in">
          <Plus className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="icon" onClick={actions.resetView} aria-label="Reset zoom">
          <RotateCcw className="h-4 w-4" />
        </Button>
      </div>

      {/* 搜索区 */}
      <div className="relative w-48">
        <Search className="absolute left-2 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Search..."
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          className="h-8 pl-8 text-sm"
        />
      </div>

      {/* 白板切换占位 */}
      <div className="border-l border-border pl-3">
        <Button variant="outline" size="sm" disabled>
          Root
        </Button>
      </div>
    </div>
  );
}
```

运行: `pnpm test -- src/__tests__/components/keysight/GraphToolbar.test.tsx 2>&1`
期望: 5 passed

- [ ] **Step 3: 运行全部测试**

运行: `pnpm test 2>&1`
期望: 全部通过

- [ ] **Step 4: 提交**

```bash
git add src/components/keysight/GraphToolbar.tsx src/__tests__/components/keysight/GraphToolbar.test.tsx
git commit -m "feat(keysight): add GraphToolbar with TDD — status/create/action/search zones"
```

---

### Task 8: GraphView 集成 + 手动验证

**Files:**
- 修改: `src/components/keysight/GraphView.tsx`

- [ ] **Step 1: 重写 GraphView 为数据枢纽**

修改 `src/components/keysight/GraphView.tsx`：

```tsx
import { useMemo, useRef, useState, useCallback } from "react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { GraphToolbar, type EntityCounts } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useWhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { Position } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { commands } from "@/bindings";
import { useQueryClient } from "@tanstack/react-query";

/** 默认白板 ID */
const ROOT_WHITEBOARD = "wb_root";

/** 默认新实体位置（画布中心附近，带随机偏移防重叠） */
function randomOffset(): Position {
  return {
    x: 200 + Math.random() * 400,
    y: 200 + Math.random() * 300,
  };
}

/**
 * 合并所有实体和位置数据为 EntityWithPosition[]。
 *
 * 只有有位置数据的实体才会被渲染（没位置的卡片不在画布上显示）。
 */
function mergeEntitiesWithPositions(
  data: ReturnType<typeof useWhiteboardData>,
): EntityWithPosition[] {
  const result: EntityWithPosition[] = [];
  const positions = data.positions;

  // Sections — 即使没有 position 也渲染（使用第一个成员位置或默认位置）
  for (const section of data.sections) {
    const pos = positions[section.id];
    if (pos) {
      result.push({ kind: "section", id: section.id, entity: section, position: pos });
    }
  }

  // Cards
  for (const card of data.cards) {
    const pos = positions[card.id];
    if (pos) {
      result.push({ kind: "card", id: card.id, entity: card, position: pos });
    }
  }

  // Tasks
  for (const task of data.tasks) {
    const pos = positions[task.id];
    if (pos) {
      result.push({ kind: "task", id: task.id, entity: task, position: pos });
    }
  }

  // Questions
  for (const question of data.questions) {
    const pos = positions[question.id];
    if (pos) {
      result.push({ kind: "question", id: question.id, entity: question, position: pos });
    }
  }

  // Notes
  for (const note of data.notes) {
    const pos = positions[note.id];
    if (pos) {
      result.push({ kind: "note", id: note.id, entity: note, position: pos });
    }
  }

  // Aliases
  for (const alias of data.aliases) {
    const pos = positions[alias.aliasId];
    if (pos) {
      result.push({ kind: "alias", id: alias.aliasId, entity: alias, position: pos });
    }
  }

  return result;
}

/**
 * KeySight 知识图谱主视图 — 数据枢纽
 *
 * 数据流: useWhiteboardData → mergeEntitiesWithPositions → useVisibleEntities → EntityNode 渲染
 * viewport 由 GraphView 创建，GraphToolbar 和 GraphCanvas 共享。
 */
export function GraphView() {
  const viewport = useViewport();
  const containerRef = useRef<HTMLDivElement>(null);
  const containerSize = useContainerSize(containerRef);
  const data = useWhiteboardData(ROOT_WHITEBOARD);
  const queryClient = useQueryClient();

  // 搜索状态（200ms debounce 在这里管理）
  const [searchQuery, setSearchQuery] = useState("");

  // 合并实体和位置
  const allEntities = useMemo(() => mergeEntitiesWithPositions(data), [data]);

  // 视口裁剪
  const visibleEntities = useVisibleEntities(allEntities, viewport.state, containerSize);

  // 搜索过滤（简单 title 匹配）
  const filteredEntities = useMemo(() => {
    if (!searchQuery.trim()) return visibleEntities;
    const q = searchQuery.toLowerCase();
    return visibleEntities.filter((e) => {
      const title = "entity" in e && "title" in e.entity ? (e.entity as any).title : "";
      return title.toLowerCase().includes(q);
    });
  }, [visibleEntities, searchQuery]);

  // 卡片标题映射（AliasNode 需要）
  const cardTitles = useMemo(() => {
    const map: Record<string, string> = {};
    for (const card of data.cards) {
      map[card.id] = card.title;
    }
    return map;
  }, [data.cards]);

  // 所有位置映射（SectionNode bounds 计算用）
  const allPositions = data.positions;

  // 实体计数
  const entityCounts: EntityCounts = useMemo(
    () => ({
      cards: data.cards.length,
      notes: data.notes.length,
      sections: data.sections.length,
      tasks: data.tasks.length,
      questions: data.questions.length,
      aliases: data.aliases.length,
    }),
    [data],
  );

  // 创建 Section 回调
  const handleCreateSection = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.sectionCreate(ROOT_WHITEBOARD, "New Section", null),
      );
      // 设置位置到画布中心
      const pos = randomOffset();
      await unwrapCommand(
        commands.layoutSetPosition(ROOT_WHITEBOARD, result.id, pos.x, pos.y),
      );
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 section 失败:", e);
    }
  }, [queryClient]);

  // 创建 Note 回调
  const handleCreateNote = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.noteCreate(ROOT_WHITEBOARD, "New Note", null, null),
      );
      const pos = randomOffset();
      await unwrapCommand(
        commands.layoutSetPosition(ROOT_WHITEBOARD, result.id, pos.x, pos.y),
      );
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 note 失败:", e);
    }
  }, [queryClient]);

  // Sync 回调
  const handleSync = useCallback(() => {
    data.syncVault.mutate();
  }, [data.syncVault]);

  // Loading 状态
  if (data.isLoading) {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <div className="text-sm text-muted-foreground">Loading whiteboard...</div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="flex h-full w-full flex-col">
      <GraphToolbar
        viewport={viewport}
        entityCounts={entityCounts}
        onSync={handleSync}
        onCreateSection={handleCreateSection}
        onCreateNote={handleCreateNote}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
      />
      <div className="relative flex-1 overflow-hidden">
        <GraphCanvas viewport={viewport}>
          {filteredEntities.map((e) => (
            <EntityNode
              key={e.id}
              entity={e}
              allPositions={allPositions}
              cardTitles={cardTitles}
            />
          ))}
        </GraphCanvas>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 运行全部 TS 测试**

运行: `pnpm test 2>&1`
期望: 全部通过

- [ ] **Step 3: TS 类型检查 + 构建**

运行: `pnpm build 2>&1 | tail -10`
期望: 构建成功

- [ ] **Step 4: Rust 全量检查**

运行: `cargo clippy --workspace -- -D warnings 2>&1`
期望: 无 error

运行: `cargo test --workspace 2>&1`
期望: 全部通过

- [ ] **Step 5: 启动 dev server 手动验证**

运行: `pnpm tauri dev`

手动验证清单：

| 检查项 | 操作 | 期望 |
|--------|------|------|
| 页面加载 | 导航到 /keysight | 看到 Toolbar + 画布 |
| 实体渲染 | 查看画布 | 有位置的卡片/任务/笔记等正确渲染 |
| Toolbar 计数 | 查看状态区 | 显示正确的实体数量 |
| Zoom 控制 | 点击 +/-/reset | zoom 变化，百分比更新 |
| Sync 按钮 | 点击 Sync | 触发 vault 同步，数据刷新 |
| 创建 Section | 点击 + Section | 画布出现新 section |
| 创建 Note | 点击 + Note | 画布出现新 note |
| 搜索 | 输入关键词 | 不匹配的实体消失 |
| Pan/Zoom | 鼠标拖拽 + Cmd+滚轮 | 画布正常平移缩放 |
| 视口裁剪 | 缩放到很小后平移 | 远处实体不渲染（DevTools 确认 DOM 节点数合理） |
| CardNode | 检查卡片样式 | 白底、圆角、投影、渐变图标、pill tags |
| TaskNode | 检查任务样式 | 带 status badge（next/active/done/blocked） |
| QuestionNode | 检查问题样式 | 带 status badge（pending/doing/done/understood） |
| NoteNode | 检查笔记样式 | 暖色便签纸背景 |
| SectionNode | 检查分组样式 | 虚线边框、半透明背景、标题 |
| AliasNode | 检查别名样式 | ghost 半透明、虚线边框 |
| 暗色主题 | 切换暗色 | 所有节点颜色适配 |

- [ ] **Step 6: 修复发现的问题**

如有问题，修复后重新运行 `pnpm test` + `pnpm build` 确认。

- [ ] **Step 7: 最终全量验证**

```bash
cargo clippy --workspace -- -D warnings    # Rust lint
cargo test --workspace                     # Rust 测试
pnpm test                                  # TS 组件测试
pnpm build                                 # TS 类型检查 + 构建
```

- [ ] **Step 8: 提交**

```bash
git add src/components/keysight/GraphView.tsx
git commit -m "feat(keysight): Phase 5b — entity rendering + toolbar + viewport culling"
```

---

## 副作用矩阵更新

Task 1 新增的是**纯读操作**（`task_query_all`、`question_query_all`），不写入数据，不需要在副作用矩阵中登记。

---

## 质量检查清单

完成所有 8 个 Task 后，执行功能域完成 Code Review：

1. **测试覆盖** — 每个 hook（useWhiteboardData/useVisibleEntities/useContainerSize）和每个节点组件都有 TDD 测试
2. **逻辑正确性** — 视口裁剪公式从 spec 移植，buffer=300px，矩形相交检测
3. **回归风险** — GraphCanvas 接口变更（viewport 从 props 注入）有测试覆盖
4. **I/O 正确性** — TanStack Query 正确 unwrap typedError，query key 包含 whiteboardId
5. **IPC 类型安全** — 新增 task_query_all/question_query_all 都有 `#[specta::specta]`，bindings.ts 重新生成
