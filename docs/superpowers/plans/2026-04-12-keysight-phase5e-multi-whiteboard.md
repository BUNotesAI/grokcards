# Phase 5e: 多白板 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 根白板上显示子白板预览卡，支持点击进入子白板、返回根白板，每个白板的画布视口持久化。

**Architecture:** 新增 `whiteboard_list` Rust command 返回子白板统计，前端用 `useState` 管理当前白板 ID，`WhiteboardNode` 新组件渲染预览卡，`useViewport` 扩展 localStorage 持久化。子白板卡位置存 positions 表（`"wb:{id}"` 约定），和普通实体统一管理。

**Tech Stack:** Rust/rusqlite, tauri-specta, React, TanStack Query, Tailwind CSS, Vitest/RTL

**Spec:** `docs/superpowers/specs/2026-04-12-keysight-phase5e-multi-whiteboard.md`

---

### Task 1: Rust — `WhiteboardSummary` 模型 + `list_whiteboards` domain 函数 + 测试

**Files:**
- Modify: `src-tauri/src/modules/keysight/models.rs:369` (在 `WhiteboardOverview` 后追加)
- Modify: `src-tauri/src/modules/keysight/domain/overview.rs:1-6` (追加 import + 新函数 + 测试)

- [ ] **Step 1: 在 `models.rs` 追加 `WhiteboardSummary` struct**

在 `GraphOverviewResponse` 之后（约 line 375）追加：

```rust
/// 子白板摘要 — 轻量级，不含卡片列表。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WhiteboardSummary {
    pub whiteboard_id: String,
    pub cards: i64,
    pub notes: i64,
    pub sections: i64,
    pub aliases: i64,
    pub tasks: i64,
    pub questions: i64,
}
```

- [ ] **Step 2: 在 `domain/overview.rs` 追加 `list_whiteboards` 函数**

在 `graph_overview` 函数之后、`#[cfg(test)]` 之前追加：

```rust
/// 查询所有子白板的轻量统计（排除 wb_root）。
pub(in crate::modules::keysight) fn list_whiteboards(
    conn: &Connection,
) -> Result<Vec<WhiteboardSummary>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT whiteboard_id,
                SUM(CASE WHEN kind = 'card' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'note' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'section' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'alias' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'task' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'question' THEN 1 ELSE 0 END)
         FROM entities
         WHERE whiteboard_id != 'wb_root'
         GROUP BY whiteboard_id
         ORDER BY whiteboard_id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WhiteboardSummary {
                whiteboard_id: r.get(0)?,
                cards: r.get(1)?,
                notes: r.get(2)?,
                sections: r.get(3)?,
                aliases: r.get(4)?,
                tasks: r.get(5)?,
                questions: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
```

在文件顶部 import 行追加 `WhiteboardSummary`：

```rust
use crate::modules::keysight::models::{CardSummary, GraphOverviewResponse, WhiteboardOverview, WhiteboardSummary};
```

- [ ] **Step 3: 写失败测试 — 3 个 `list_whiteboards` 测试**

在 `domain/overview.rs` 的 `#[cfg(test)] mod tests` 末尾追加：

```rust
    // --- list_whiteboards ---

    #[test]
    fn test_list_whiteboards_empty() {
        let conn = test_conn();
        let result = list_whiteboards(&conn).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_whiteboards_excludes_wb_root() {
        let conn = test_conn();
        // wb_root 下的卡片（文件路径不在 whiteboard/ 子目录下）
        let card_md = "---\ntype: atomic-card\nid: card_wb_r001\n---\n\n# 【ATC】Root Card\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/root_card.md", card_md, 100.0).unwrap();

        let result = list_whiteboards(&conn).unwrap();
        assert!(result.is_empty(), "wb_root 的实体不应出现在子白板列表中");
    }

    #[test]
    fn test_list_whiteboards_counts_by_kind() {
        let conn = test_conn();
        // rust 白板：2 cards + 1 note
        let card1 = "---\ntype: atomic-card\nid: card_wbl_001\n---\n\n# 【ATC】Card 1\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/rust/c1.md", card1, 100.0).unwrap();
        let card2 = "---\ntype: atomic-card\nid: card_wbl_002\n---\n\n# 【ATC】Card 2\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/rust/c2.md", card2, 200.0).unwrap();

        // 手动插入 note（sync_file 只解析 card/task/question）
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('note_wbl_001', 'note', 'Note 1', 'rust')",
            [],
        ).unwrap();

        // chentian 白板：1 alias
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('alias_wbl_001', 'alias', 'Alias 1', 'chentian')",
            [],
        ).unwrap();

        let result = list_whiteboards(&conn).unwrap();
        assert_eq!(result.len(), 2);

        let rust_wb = result.iter().find(|w| w.whiteboard_id == "rust").unwrap();
        assert_eq!(rust_wb.cards, 2);
        assert_eq!(rust_wb.notes, 1);
        assert_eq!(rust_wb.sections, 0);

        let ct_wb = result.iter().find(|w| w.whiteboard_id == "chentian").unwrap();
        assert_eq!(ct_wb.aliases, 1);
        assert_eq!(ct_wb.cards, 0);
    }
```

- [ ] **Step 4: 跑测试确认全绿**

```bash
cd src-tauri && cargo test --lib modules::keysight::domain::overview -- -q
```

Expected: 所有 overview 测试通过（含新增 3 个）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/modules/keysight/models.rs src-tauri/src/modules/keysight/domain/overview.rs
git commit -m "feat(keysight): add WhiteboardSummary model + list_whiteboards domain fn"
```

---

### Task 2: Rust — `whiteboard_list` command + bindings 更新

**Files:**
- Modify: `src-tauri/src/modules/keysight/commands.rs:907` (在 `overview_graph` 之后追加)
- Modify: `src-tauri/src/lib.rs:73` (在 `overview_graph` 之后追加)

- [ ] **Step 1: 在 `commands.rs` 追加 command**

在 `overview_graph` 函数之后（约 line 907）追加：

```rust
/// 查询所有子白板的轻量统计。
#[tauri::command]
#[specta::specta]
pub fn whiteboard_list(
    state: State<'_, KeysightState>,
) -> Result<Vec<WhiteboardSummary>, AppError> {
    let conn = state.db.lock().unwrap();
    overview::list_whiteboards(&conn).map_err(Into::into)
}
```

在文件顶部 import 区域追加 `WhiteboardSummary`（如果 `use super::models::*` 已经覆盖则不需要，检查当前 import 方式）。

- [ ] **Step 2: 在 `lib.rs` 的 `collect_commands![]` 注册**

在 `modules::keysight::commands::overview_graph,` 之后追加：

```rust
        modules::keysight::commands::whiteboard_list,
```

- [ ] **Step 3: 重新生成 bindings.ts**

```bash
cd src-tauri && cargo test export_bindings
```

Expected: `src/bindings.ts` 中出现 `whiteboardList` 函数和 `WhiteboardSummary` 类型。

- [ ] **Step 4: 验证 bindings 内容**

```bash
grep -n "whiteboardList\|WhiteboardSummary" src/bindings.ts
```

Expected: 能找到两者。

- [ ] **Step 5: Clippy 检查**

```bash
cd src-tauri && cargo clippy --workspace -- -D warnings
```

Expected: 无 warning。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/commands.rs src-tauri/src/lib.rs src/bindings.ts
git commit -m "feat(keysight): add whiteboard_list command + update bindings"
```

---

### Task 3: 前端 — `useViewport` 添加 localStorage 持久化

**Files:**
- Modify: `src/components/keysight/useViewport.ts` (全文改造)

- [ ] **Step 1: 改造 `useViewport` 接口，添加 `whiteboardId` 参数**

将 `useViewport.ts` 改为：

```typescript
import { useCallback, useEffect, useRef, useState } from "react";

const MIN_ZOOM = 0.05;
const MAX_ZOOM = 3.0;
const ZOOM_STEP = 1.2;

export interface ViewportState {
  zoom: number;
  panX: number;
  panY: number;
}

/** 将 zoom 值限制在 [MIN_ZOOM, MAX_ZOOM] 范围内 */
function clampZoom(z: number): number {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));
}

const DEFAULT_VIEWPORT: ViewportState = { zoom: 1, panX: 0, panY: 0 };
const STORAGE_PREFIX = "keysight:viewport:";
const SAVE_DEBOUNCE_MS = 500;

/** 从 localStorage 读取白板视口 */
function loadViewport(whiteboardId: string): ViewportState {
  try {
    const raw = localStorage.getItem(STORAGE_PREFIX + whiteboardId);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (
        typeof parsed.zoom === "number" &&
        typeof parsed.panX === "number" &&
        typeof parsed.panY === "number"
      ) {
        return { zoom: clampZoom(parsed.zoom), panX: parsed.panX, panY: parsed.panY };
      }
    }
  } catch {
    // 损坏的 JSON，忽略
  }
  return { ...DEFAULT_VIEWPORT };
}

/** 保存白板视口到 localStorage */
function saveViewport(whiteboardId: string, state: ViewportState): void {
  localStorage.setItem(STORAGE_PREFIX + whiteboardId, JSON.stringify(state));
}

/**
 * 画布视口控制 hook
 *
 * 提供 zoom（缩放）和 pan（平移）能力：
 * - zoomIn / zoomOut：按 ZOOM_STEP 倍率缩放，受 [0.05, 3.0] 边界约束
 * - resetView：重置到初始状态（zoom=1, panX=0, panY=0）
 * - 鼠标拖拽平移
 * - Cmd/Ctrl + 滚轮缩放（以鼠标位置为中心）；普通滚轮平移
 * - 视口状态按白板 ID 持久化到 localStorage（500ms debounce）
 */
export function useViewport(whiteboardId: string) {
  const [state, setState] = useState<ViewportState>(() => loadViewport(whiteboardId));

  // 白板切换时加载对应视口
  const prevWhiteboardIdRef = useRef(whiteboardId);
  useEffect(() => {
    if (prevWhiteboardIdRef.current !== whiteboardId) {
      // 保存旧白板视口（立即写入，不等 debounce）
      saveViewport(prevWhiteboardIdRef.current, state);
      prevWhiteboardIdRef.current = whiteboardId;
      // 加载新白板视口
      setState(loadViewport(whiteboardId));
    }
  }, [whiteboardId]); // eslint-disable-line react-hooks/exhaustive-deps

  // Debounced 持久化
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    debounceRef.current = setTimeout(() => {
      saveViewport(whiteboardId, state);
    }, SAVE_DEBOUNCE_MS);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [state, whiteboardId]);

  // 拖拽状态用 ref，不触发渲染
  const dragRef = useRef<{
    dragging: boolean;
    startX: number;
    startY: number;
    startPanX: number;
    startPanY: number;
  }>({
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
    setState({ ...DEFAULT_VIEWPORT });
  }, []);

  // Pan: 鼠标拖拽
  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      dragRef.current = {
        dragging: true,
        startX: e.clientX,
        startY: e.clientY,
        startPanX: state.panX,
        startPanY: state.panY,
      };
    },
    [state.panX, state.panY],
  );

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
      e.preventDefault();
      setState((s) => {
        const newZoom = clampZoom(s.zoom * (1 - e.deltaY * 0.001));
        const ratio = newZoom / s.zoom;
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

/** useViewport hook 的返回值类型，供 GraphCanvas props 使用 */
export type UseViewportReturn = ReturnType<typeof useViewport>;
```

- [ ] **Step 2: 更新 `GraphView.tsx` 调用处**

将 `const viewport = useViewport();` 改为：

```typescript
const viewport = useViewport(ROOT_WHITEBOARD);
```

（后续 Task 5 会改为 `useViewport(currentWhiteboardId)`，这里先用常量保持编译通过）

- [ ] **Step 3: 跑 TS 测试 + 编译**

```bash
pnpm test && pnpm build
```

Expected: 全部通过。GraphToolbar.test.tsx 的 `createTestViewport` 需要更新参数——`useViewport()` 改为 `useViewport("test-wb")`。

- [ ] **Step 4: Commit**

```bash
git add src/components/keysight/useViewport.ts src/components/keysight/GraphView.tsx src/__tests__/components/keysight/GraphToolbar.test.tsx
git commit -m "feat(keysight): add viewport persistence to localStorage per whiteboard"
```

---

### Task 4: 前端 — `WhiteboardNode` 组件 + 测试

**Files:**
- Create: `src/components/keysight/nodes/WhiteboardNode.tsx`
- Create: `src/__tests__/components/keysight/nodes/WhiteboardNode.test.tsx`

- [ ] **Step 1: 写失败测试 — 3 个 WhiteboardNode 测试**

创建 `src/__tests__/components/keysight/nodes/WhiteboardNode.test.tsx`：

```tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { WhiteboardNode } from "@/components/keysight/nodes/WhiteboardNode";
import type { WhiteboardSummary } from "@/bindings";
import { vi } from "vitest";

const mockSummary: WhiteboardSummary = {
  whiteboardId: "rust",
  cards: 145,
  notes: 72,
  sections: 3,
  aliases: 6,
  tasks: 0,
  questions: 0,
};

describe("WhiteboardNode", () => {
  it("渲染白板名称和非零统计", () => {
    render(<WhiteboardNode summary={mockSummary} onNavigate={vi.fn()} style={{}} />);
    expect(screen.getByText("rust")).toBeInTheDocument();
    expect(screen.getByText(/145 cards/)).toBeInTheDocument();
    expect(screen.getByText(/72 notes/)).toBeInTheDocument();
    expect(screen.getByText(/6 aliases/)).toBeInTheDocument();
    // 零值不显示
    expect(screen.queryByText(/task/)).not.toBeInTheDocument();
    expect(screen.queryByText(/question/)).not.toBeInTheDocument();
  });

  it("点击触发 onNavigate", () => {
    const onNavigate = vi.fn();
    render(<WhiteboardNode summary={mockSummary} onNavigate={onNavigate} style={{}} />);
    fireEvent.click(screen.getByText("rust"));
    expect(onNavigate).toHaveBeenCalledWith("rust");
  });

  it("空白板显示 empty", () => {
    const emptySummary: WhiteboardSummary = {
      whiteboardId: "agent",
      cards: 0,
      notes: 0,
      sections: 0,
      aliases: 0,
      tasks: 0,
      questions: 0,
    };
    render(<WhiteboardNode summary={emptySummary} onNavigate={vi.fn()} style={{}} />);
    expect(screen.getByText("agent")).toBeInTheDocument();
    expect(screen.getByText("empty")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: 跑测试确认红色**

```bash
pnpm test -- --reporter=verbose 2>&1 | tail -10
```

Expected: FAIL — `WhiteboardNode` 模块不存在。

- [ ] **Step 3: 实现 `WhiteboardNode` 组件**

创建 `src/components/keysight/nodes/WhiteboardNode.tsx`：

```tsx
import type { CSSProperties } from "react";
import type { WhiteboardSummary } from "@/bindings";

interface WhiteboardNodeProps {
  summary: WhiteboardSummary;
  onNavigate: (whiteboardId: string) => void;
  style: CSSProperties;
}

/** 非零统计项格式化 */
function formatStats(s: WhiteboardSummary): string {
  const parts: string[] = [];
  if (s.cards > 0) parts.push(`${s.cards} card${s.cards !== 1 ? "s" : ""}`);
  if (s.notes > 0) parts.push(`${s.notes} note${s.notes !== 1 ? "s" : ""}`);
  if (s.sections > 0) parts.push(`${s.sections} section${s.sections !== 1 ? "s" : ""}`);
  if (s.aliases > 0) parts.push(`${s.aliases} alias${s.aliases !== 1 ? "es" : ""}`);
  if (s.tasks > 0) parts.push(`${s.tasks} task${s.tasks !== 1 ? "s" : ""}`);
  if (s.questions > 0) parts.push(`${s.questions} question${s.questions !== 1 ? "s" : ""}`);
  return parts.join(" · ");
}

/**
 * 子白板预览卡 — Clean Elevated 风格
 *
 * 白底圆角卡片 + 紫色渐变竖条 + 统计行 + 进入提示。
 * 宽度 320px，点击整卡导航进入子白板。
 */
export function WhiteboardNode({ summary, onNavigate, style }: WhiteboardNodeProps) {
  const stats = formatStats(summary);
  const isEmpty = stats.length === 0;

  return (
    <div
      data-whiteboard-id={summary.whiteboardId}
      className="flex cursor-pointer select-none overflow-hidden rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)] transition-shadow hover:shadow-[0_2px_8px_rgba(0,0,0,0.1),0_8px_24px_rgba(0,0,0,0.08)]"
      style={{ ...style, width: 320 }}
      onClick={() => onNavigate(summary.whiteboardId)}
    >
      {/* 左侧紫色渐变竖条 */}
      <div className="w-1.5 shrink-0 bg-gradient-to-b from-violet-400 to-purple-600" />

      <div className="flex flex-1 flex-col px-4 py-3">
        {/* 标题行 */}
        <div className="flex items-center gap-2">
          <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-violet-400 to-purple-600 text-[10px] font-bold text-white">
            W
          </span>
          <h3 className="flex-1 truncate text-sm font-semibold text-foreground">
            {summary.whiteboardId}
          </h3>
        </div>

        {/* 统计行 */}
        <p className="mt-1.5 text-xs text-muted-foreground">
          {isEmpty ? "empty" : stats}
        </p>

        {/* 进入提示 */}
        <p className="mt-2 text-[10px] text-muted-foreground/50">
          Click to enter →
        </p>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 跑测试确认绿色**

```bash
pnpm test -- --reporter=verbose 2>&1 | tail -15
```

Expected: WhiteboardNode 3 个测试全绿。

- [ ] **Step 5: Commit**

```bash
git add src/components/keysight/nodes/WhiteboardNode.tsx src/__tests__/components/keysight/nodes/WhiteboardNode.test.tsx
git commit -m "feat(keysight): add WhiteboardNode component with tests"
```

---

### Task 5: 前端 — GraphToolbar 导航按钮 + 测试

**Files:**
- Modify: `src/components/keysight/GraphToolbar.tsx`
- Modify: `src/__tests__/components/keysight/GraphToolbar.test.tsx`

- [ ] **Step 1: 写失败测试 — 2 个导航测试**

在 `GraphToolbar.test.tsx` 末尾 `describe` 块内追加：

```tsx
  it("根白板时不显示返回按钮", () => {
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
        currentWhiteboardId="wb_root"
      />,
    );
    expect(screen.queryByRole("button", { name: /root/i })).not.toBeInTheDocument();
  });

  it("子白板时显示返回按钮和白板名，点击触发 onNavigateBack", () => {
    const viewport = createTestViewport();
    const onNavigateBack = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 50, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
        currentWhiteboardId="rust"
        onNavigateBack={onNavigateBack}
      />,
    );
    expect(screen.getByText("rust")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(onNavigateBack).toHaveBeenCalledTimes(1);
  });
```

- [ ] **Step 2: 跑测试确认红色**

```bash
pnpm test -- --reporter=verbose 2>&1 | tail -10
```

Expected: FAIL — 新 props 类型不匹配。

- [ ] **Step 3: 修改 `GraphToolbar` — 添加导航 props + 动态渲染**

修改 `GraphToolbar.tsx`：

在 import 区域追加：

```tsx
import { ArrowLeft } from "lucide-react";
```

修改 `GraphToolbarProps` interface：

```typescript
interface GraphToolbarProps {
  viewport: UseViewportReturn;
  entityCounts: EntityCounts;
  onSync: () => void;
  onCreateSection: () => void;
  onCreateNote: () => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  currentWhiteboardId?: string;
  onNavigateBack?: () => void;
}
```

修改函数签名，解构新 props：

```typescript
export function GraphToolbar({
  viewport,
  entityCounts,
  onSync,
  onCreateSection,
  onCreateNote,
  searchQuery,
  onSearchChange,
  currentWhiteboardId = "wb_root",
  onNavigateBack,
}: GraphToolbarProps) {
```

将底部「白板切换占位」区域替换为：

```tsx
      {/* 白板导航 */}
      {currentWhiteboardId !== "wb_root" && (
        <div className="flex items-center gap-2 border-l border-border pl-3">
          <Button variant="ghost" size="sm" onClick={onNavigateBack} aria-label="Back to root">
            <ArrowLeft className="mr-1 h-4 w-4" />
            Root
          </Button>
          <span className="text-xs font-medium text-foreground">{currentWhiteboardId}</span>
        </div>
      )}
```

- [ ] **Step 4: 更新现有测试中旧的 disabled Root 断言**（如有）

检查现有测试是否断言了 disabled "Root" 按钮。如果有，删除该断言。

- [ ] **Step 5: 跑测试确认全绿**

```bash
pnpm test -- --reporter=verbose 2>&1 | tail -15
```

Expected: GraphToolbar 所有测试通过。

- [ ] **Step 6: Commit**

```bash
git add src/components/keysight/GraphToolbar.tsx src/__tests__/components/keysight/GraphToolbar.test.tsx
git commit -m "feat(keysight): add whiteboard navigation to GraphToolbar"
```

---

### Task 6: 前端 — `useWhiteboardList` hook + GraphView 集成

**Files:**
- Modify: `src/components/keysight/hooks/useWhiteboardData.ts` (追加 `useWhiteboardList`)
- Modify: `src/components/keysight/types.ts` (追加 whiteboard 的 ENTITY_DIMENSIONS)
- Modify: `src/components/keysight/GraphView.tsx` (白板切换 + 子白板卡渲染)

- [ ] **Step 1: 在 `types.ts` 追加 whiteboard dimensions**

在 `ENTITY_DIMENSIONS` 对象中追加：

```typescript
export type EntityKind = "card" | "task" | "question" | "note" | "section" | "alias" | "whiteboard";
```

```typescript
export const ENTITY_DIMENSIONS: Record<EntityKind, { width: number; height: number }> = {
  card: { width: 320, height: 160 },
  task: { width: 320, height: 140 },
  question: { width: 320, height: 140 },
  note: { width: 200, height: 120 },
  section: { width: 400, height: 300 },
  alias: { width: 280, height: 100 },
  whiteboard: { width: 320, height: 130 },
};
```

- [ ] **Step 2: 在 `useWhiteboardData.ts` 追加 `useWhiteboardList` hook**

在文件末尾追加：

```typescript
import type { WhiteboardSummary } from "@/bindings";

/**
 * 加载子白板列表（仅在 wb_root 时启用）。
 */
export function useWhiteboardList(whiteboardId: string) {
  return useQuery({
    queryKey: ["whiteboards"],
    queryFn: () => unwrapCommand(commands.whiteboardList()),
    enabled: whiteboardId === "wb_root",
  });
}
```

- [ ] **Step 3: 改造 `GraphView.tsx` — 白板切换 + 子白板卡渲染**

完整修改 `GraphView.tsx`：

1) import 区域追加：

```typescript
import { useWhiteboardList } from "@/components/keysight/hooks/useWhiteboardData";
import { WhiteboardNode } from "@/components/keysight/nodes/WhiteboardNode";
import type { WhiteboardSummary, Position } from "@/bindings";
```

2) 将 `ROOT_WHITEBOARD` 常量保留，组件内部新增状态：

```typescript
const [currentWhiteboardId, setCurrentWhiteboardId] = useState(ROOT_WHITEBOARD);
```

3) 将所有 `ROOT_WHITEBOARD` 使用替换为 `currentWhiteboardId`：

- `useWhiteboardData(currentWhiteboardId)`
- `useViewport(currentWhiteboardId)`
- `commands.sectionCreate(currentWhiteboardId, ...)`
- `commands.noteCreate(currentWhiteboardId, ...)`
- `commands.layoutSetPosition(currentWhiteboardId, ...)`

4) 加载子白板列表：

```typescript
const whiteboardListQuery = useWhiteboardList(currentWhiteboardId);
const whiteboards = whiteboardListQuery.data ?? [];
```

5) 构建子白板卡的 EntityWithPosition 数据（与实体合并渲染）：

在 `mergeEntitiesWithPositions` 之后，追加子白板卡合并逻辑：

```typescript
// 子白板卡合并到实体列表（使用 positions 表中 "wb:{id}" 的位置）
const whiteboardEntities: Array<{ whiteboardId: string; position: Position }> = useMemo(() => {
  if (currentWhiteboardId !== ROOT_WHITEBOARD) return [];
  return whiteboards
    .map((wb) => {
      const pos = data.positions[`wb:${wb.whiteboardId}`];
      return pos ? { whiteboardId: wb.whiteboardId, position: pos } : null;
    })
    .filter(Boolean) as Array<{ whiteboardId: string; position: Position }>;
}, [whiteboards, data.positions, currentWhiteboardId]);
```

6) 在渲染区域，EntityNode 之后追加子白板卡渲染：

```tsx
{whiteboardEntities.map((wb) => {
  const summary = whiteboards.find((s) => s.whiteboardId === wb.whiteboardId);
  if (!summary) return null;
  return (
    <WhiteboardNode
      key={`wb:${wb.whiteboardId}`}
      summary={summary}
      onNavigate={setCurrentWhiteboardId}
      style={{
        position: "absolute",
        left: wb.position.x,
        top: wb.position.y,
      }}
    />
  );
})}
```

7) 导航回调传给 Toolbar：

```tsx
<GraphToolbar
  {...existingProps}
  currentWhiteboardId={currentWhiteboardId}
  onNavigateBack={() => setCurrentWhiteboardId(ROOT_WHITEBOARD)}
/>
```

- [ ] **Step 4: 跑编译 + 测试**

```bash
pnpm build && pnpm test
```

Expected: 编译和测试通过。

- [ ] **Step 5: Commit**

```bash
git add src/components/keysight/types.ts src/components/keysight/hooks/useWhiteboardData.ts src/components/keysight/GraphView.tsx
git commit -m "feat(keysight): integrate whiteboard switching + WhiteboardNode rendering"
```

---

### Task 7: 前端 — 无位置子白板卡的初始布局

**Files:**
- Modify: `src/components/keysight/GraphView.tsx` (追加初始布局逻辑)

- [ ] **Step 1: 追加初始布局 effect**

在 GraphView 组件内，`whiteboardEntities` memo 之后追加：

```typescript
// 无位置的子白板卡：自动计算初始坐标并写入 DB
const initializedRef = useRef<Set<string>>(new Set());
useEffect(() => {
  if (currentWhiteboardId !== ROOT_WHITEBOARD) return;
  const unpositioned = whiteboards.filter(
    (wb) =>
      !data.positions[`wb:${wb.whiteboardId}`] &&
      !initializedRef.current.has(wb.whiteboardId),
  );
  if (unpositioned.length === 0) return;

  // 找现有实体的 bounding box 最大 y 值
  const maxY = allEntities.reduce((max, e) => Math.max(max, e.position.y + 200), 0);
  const startY = Math.max(maxY + 60, 100);
  const colWidth = 360; // 320px 卡 + 40px 间距
  const rowHeight = 170; // 130px 卡 + 40px 间距
  const cols = 2;

  const writes = unpositioned.map(async (wb, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    const x = 100 + col * colWidth;
    const y = startY + row * rowHeight;
    initializedRef.current.add(wb.whiteboardId);
    await unwrapCommand(
      commands.layoutSetPosition(ROOT_WHITEBOARD, `wb:${wb.whiteboardId}`, x, y),
    );
  });

  Promise.all(writes).then(() => {
    queryClient.invalidateQueries({ queryKey: ["positions", ROOT_WHITEBOARD] });
  });
}, [whiteboards, data.positions, currentWhiteboardId, allEntities, queryClient]);
```

追加 `useRef` import（如果还没有）和确保 `useEffect` 已 imported。

- [ ] **Step 2: 跑编译**

```bash
pnpm build
```

Expected: 编译通过。

- [ ] **Step 3: Commit**

```bash
git add src/components/keysight/GraphView.tsx
git commit -m "feat(keysight): auto-layout unpositioned whiteboard cards on first load"
```

---

### Task 8: 手动验证 + 修复

**Files:** 无预定修改（根据验证结果可能需要微调）

- [ ] **Step 1: 启动 dev server**

```bash
KEYSIGHT_VAULT_PATH=~/Documents/obsidian_workspace/agent-slipbox-v3 pnpm tauri dev
```

- [ ] **Step 2: 验证清单逐项检查**

1. 根白板同时显示实体（cards/notes/sections）+ 子白板卡（rust/chentian/rust-examples/agent）
2. 子白板卡显示正确统计数字（对比 `overview_stats` 返回值）
3. 点击 rust 卡 → 画布切换为 rust 白板，显示 rust 的卡片
4. Toolbar 显示 `← Root` + `rust`，点击返回根白板
5. 返回后视口位置不变（画布没有跳到原点）
6. 进入 rust → 拖动画布 → 返回 → 再进入 rust → 画布位置保持

- [ ] **Step 3: 修复验证中发现的问题**

根据验证结果修复，每个修复单独 commit。

- [ ] **Step 4: 全量测试确认**

```bash
cd src-tauri && cargo clippy --workspace -- -D warnings && cargo test --workspace && cd .. && pnpm test && pnpm build
```

Expected: 全部通过。

- [ ] **Step 5: 最终 commit（如有修复）**

```bash
git add -A && git commit -m "fix(keysight): Phase 5e manual verification fixes"
```

---

### Task 9: 文档更新

**Files:**
- Modify: `docs/progress/keysight.md`
- Modify: `CLAUDE.md` (副作用矩阵 — 无新增写操作，确认无需更新)

- [ ] **Step 1: 更新 progress**

将 `docs/progress/keysight.md` 中 Phase 5e 从 Next 移到 Active（或 Done，取决于验证结果）。

- [ ] **Step 2: 写 devlog**

追加 `docs/devlog/{YYYY-MM-DD}.md`。

- [ ] **Step 3: 写 changelog**

```bash
bash ~/.claude/scripts/write_session_id.sh 2>/dev/null; SLIPBOX=~/Documents/obsidian_workspace/agent-slipbox-v3 && FILE="$SLIPBOX/logs/changelog/$(date +%Y-%m-%d).md" && SID=$(p=$PPID; n=0; while [ -n "$p" ] && [ "$p" != "1" ] && [ $n -lt 8 ]; do [ -f "/tmp/cc-session-$p" ] && cut -c1-8 "/tmp/cc-session-$p" && exit; p=$(ps -o ppid= -p "$p" 2>/dev/null | tr -d ' '); n=$((n+1)); done; echo unknown) && [ ! -f "$FILE" ] && printf -- '---\ntags: [changelog]\ndate: %s\ntype: changelog\n---\n' "$(date +%Y-%m-%d)" > "$FILE"; echo "$(date +%H:%M) | ✅ | Phase 5e 多白板 | ${SID} | whiteboard_list command + WhiteboardNode + viewport持久化 + 白板切换导航" >> "$FILE"
```

- [ ] **Step 4: Commit docs**

```bash
git add docs/ && git commit -m "docs(keysight): mark Phase 5e complete, append devlog + changelog"
```
