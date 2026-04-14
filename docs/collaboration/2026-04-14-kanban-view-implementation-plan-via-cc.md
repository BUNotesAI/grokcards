# P3 Kanban View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **TDD 人工确认关卡**: 用户已**授权本次跳过 Red/Green 人工确认关卡**(2026-04-14)。测试本身仍然严格执行(Red 测失败 → 写代码 → Green 测通过 → Refactor),但不暂停等用户确认。

## ⚠️ Revision History

**2026-04-14 — post codex review**

本 plan 第一版经 codex agent review(见 [`2026-04-14-kanban-view-implementation-plan-via-codex.md`](./2026-04-14-kanban-view-implementation-plan-via-codex.md)),发现 7 项与当前仓库现实不一致的问题。下面条目都已在本版本修正/标注,执行时以本版本为准。

| # | Codex 发现 | 修正 |
|---|---|---|
| 1 | `entities` 表没有 `created_at` 列(db.rs:10-18) | Task 1.4 `query_kanban` 改为 `ORDER BY e.title`(与 `task::query_all` 对齐),V1 不做"最新优先"假设 |
| 2 | `KeysightView` 是纯本地 `useState`,不读 URL(KeysightView.tsx:42, 113) | 新增 **Task 0.2 KeysightView URL sync**,作为 Reveal Graph / Show Kanban 的硬前置 |
| 3 | `WhiteboardId` newtype 在本 scope 内无法真正 close 现有 String 流转,是半吊子 | **删除 Phase 0 WhiteboardId 任务**(原 Task 0.2 / 0.3),单独列入 Phase B3 P2 积压项,不在本次 P3 内做 |
| 4 | `CreateTaskModal` 内部 `useState(defaultStatus)` 有 stale default 问题 | Task 3.4 改为**受控组件**(parent 持有 project/title/status state,或 modal 用 `key={open}` 条件渲染强制 remount) |
| 5 | API 形状已漂移: `task::create` 用 `TaskCreateInput` struct;command 叫 `commands.whiteboardList`(不是 listWhiteboards);`WhiteboardSummary` 字段是 `whiteboardId`(camelCase)不是 `id`;项目已有 `unwrapCommand` helper 不要手写 result 拆包 | 所有 Rust test / TS 代码示例已对齐当前 API;执行时仍以实际 `bindings.ts` / `src/lib/commandResult.ts` 为准 |
| 6 | task 节点实际估计高度 `140` 不是 `80`(types.ts:47-50 `ENTITY_DIMENSIONS.task.height = 140`) | Task 1.2 `DEFAULT_NODE_HEIGHT: f64 = 140.0`;`NODE_SPACING` 同步调整,测试期望数值重算 |
| 7 | 默认创建状态 Inbox 只落到 kanban,GraphView(GraphView.tsx:952-959)仍 hardcode `"next"`,两个入口会默默分叉 | 本 plan 的产品决策: **kanban 入口 default = Inbox,GraphView 入口保持 default = Next**。这是有意的不同语义(canvas 上手动创建表示"我决定要做",kanban Inbox 表示"先收集未决定")。此条在 §11 明确文档化,不做 unify。如未来产品决策要 unify 到 Inbox,单独起 migration task |

**结论**: 沿用原方向,删掉一个 task,加一个 task,修正若干代码示例。核心架构(独立路由 / 纯视觉列 / SQLite SoT / auto-position / co-equal mode / @dnd-kit)均保留不变。

---

**Goal:** 实现 P3 Kanban view —— sidebar 顶级独立路由 `/kanban`,与 Canvas 是 co-equal 视图模式,用户可在 5 列(Inbox/Next/Active/Blocked/Done)按 status 管理 task,创建后自动同步到 canvas。

**Architecture:** 三层协作 —— Rust 数据层(TaskStatus 加 Inbox + 自动 position + query_kanban)、TS UI 层(5 个新组件 + 新路由 + @dnd-kit)、SQLite 单一真源(无 sync 代码,React Query cache key 命名约定驱动两边自动 invalidate)。Phase 0 只做 **TaskStatus IPC enum 迁移** 和 **KeysightView URL sync**(跨路由跳转硬前置),**WhiteboardId newtype 推迟到 Phase B3 P2 独立 task**。

**Tech Stack:** Rust + tauri-specta(IPC 类型) / React 19 + react-router-dom + @tanstack/react-query / @dnd-kit/core(新引入) / vitest + RTL(测试)。

**Spec:** [`docs/collaboration/2026-04-14-kanban-view-design-via-cc.md`](./2026-04-14-kanban-view-design-via-cc.md)

---

## 文件结构总览

### Rust 文件(`src-tauri/src/modules/keysight/`)

| 文件 | 操作 | 责任 |
|---|---|---|
| `models.rs` | Modify | TaskStatus enum 加 Inbox 变体;`TaskEntity.status: String → TaskStatus`(DTO 也走 enum,tauri-specta 生成 TS literal union) |
| `commands.rs` | Modify | task_create/update/etc 签名 `status: String → TaskStatus`;用 `TaskCreateInput`/`TaskUpdateInput` struct(当前已是 struct 形态);新增 `task_query_kanban` 命令;删 `parse_task_status_from_ipc` |
| `domain/task.rs` | Modify | parse_task_status 加 Inbox case;`compute_position_below_bottommost` 新 helper;`create` 集成 auto-position;`query_kanban` 新函数 |
| `domain/layout.rs` | Modify | 新增/复用 query 接口供 compute_position 用 |
| `db.rs` | Modify | 加 `CREATE INDEX IF NOT EXISTS idx_positions_wb_y ON positions(whiteboard_id, y)` |
| `lib.rs` | Modify | `collect_commands![]` 加 `task_query_kanban` |
| `mod.rs` (各处) | 不变 | 内部模块组织不变 |

### TS 文件(`src/`)

| 文件 | 操作 | 责任 |
|---|---|---|
| `App.tsx` | Modify | 加 `<Route path="/kanban" element={<KanbanView />} />` |
| `components/AppShell.tsx` | Modify | `modules` 数组加 Kanban entry + `pageTitles` 加 `/kanban` |
| `components/keysight/KeysightView.tsx` | Modify | 加 `useSearchParams` 读 `?wb=...`,与 `currentWhiteboardId` 本地 state 双向同步(Task 0.2 硬前置) |
| `components/kanban/KanbanView.tsx` | Create | 页面壳,URL state,queries,modal state |
| `components/kanban/KanbanToolbar.tsx` | Create | project dropdown + "+ New task" + "Reveal Graph" |
| `components/kanban/KanbanBoard.tsx` | Create | DndContext + 5 KanbanColumn + onDragEnd |
| `components/kanban/KanbanColumn.tsx` | Create | 单列 (header + count + "+" + cards list) |
| `components/kanban/KanbanCard.tsx` | Create | 单 task 卡片 (display + draggable) |
| `components/kanban/CreateTaskModal.tsx` | Create | project + title + status form |
| `components/kanban/columns.ts` | Create | `COLUMNS: Record<TaskStatus, ColumnConfig>` 强制穷尽 |
| `components/kanban/invalidateAllTaskCaches.ts` | Create | 集中 cache invalidate helper |
| `components/keysight/GraphToolbar.tsx` | Modify | 加 "Show Kanban" 按钮(reciprocal navigation) |
| `bindings.ts` | Auto-regenerate | tauri-specta 重新生成 (Phase 0/1 各一次) |

### 测试文件

| 文件 | 操作 | 责任 |
|---|---|---|
| `src-tauri/src/modules/keysight/domain/task.rs` (内部 `#[cfg(test)]`) | Modify | 加 compute_position / create-with-auto-position / query_kanban / Inbox 解析测试 |
| `src-tauri/src/modules/keysight/models.rs` (内部 `#[cfg(test)]`) | Modify | 加 WhiteboardId newtype 测试 |
| `src/__tests__/components/kanban/KanbanView.test.tsx` | Create | URL state / loading / error / empty 状态 |
| `src/__tests__/components/kanban/KanbanToolbar.test.tsx` | Create | dropdown 切换 / Reveal Graph 仅 specific project 显示 |
| `src/__tests__/components/kanban/KanbanBoard.test.tsx` | Create | 5 列固定顺序 / tasks 按 status 分组 |
| `src/__tests__/components/kanban/KanbanColumn.test.tsx` | Create | 计数 / 列头 + button / empty state |
| `src/__tests__/components/kanban/KanbanCard.test.tsx` | Create | render / project tag / status badge |
| `src/__tests__/components/kanban/CreateTaskModal.test.tsx` | Create | 表单校验 / submit / cancel / status 默认 Inbox |
| `src/__tests__/components/keysight/GraphToolbar.test.tsx` | Modify | 加 "Show Kanban" 按钮测试 |

### 文档

| 文件 | 操作 | 责任 |
|---|---|---|
| `CLAUDE.md` | Modify | 副作用矩阵 `task::create` 行加 `positions`;新增 `task_query_kanban` 行 |
| `docs/progress/backend.md` | Modify | P3 → Active → Done 流转 |
| `docs/handoff/backend.md` | Modify | status idle → in-progress → done |
| `docs/devlog/2026-04-14.md` 或新文件 | Modify | 追加 Session 实施记录 |

---

## Phase 0 — 类型基础(强类型迁移,共 3 task)

> **目的**: 把现有 `TaskStatus: String` IPC 边界升到 enum,新增 `WhiteboardId` newtype 收敛字符串拼接。kanban 实施前先把这层防火墙做好,让所有 status / wb_id 拼写错误编译期失败。

### Task 0.1: TaskStatus IPC 边界 String → enum

**Files:**
- Modify: `src-tauri/src/modules/keysight/commands.rs:461-540` (删 parse_task_status_from_ipc + 改签名)
- Modify: `src-tauri/src/modules/keysight/commands.rs` (其他 task_* 命令签名)
- Test: `src-tauri/src/modules/keysight/domain/task.rs` (现有测试不变,但 commands 要被检查)

- [ ] **Step 1: 阅读现有 IPC 调用,记录所有受影响的 command**

```bash
grep -rn "status: String\|status: &str" src-tauri/src/modules/keysight/commands.rs | grep -i task
```

预期看到 task_create / task_update 的 status 参数。记下行号。

- [ ] **Step 2: 修改 task_create 签名**

`src-tauri/src/modules/keysight/commands.rs:493-510` 大致改为:

```rust
#[tauri::command]
#[specta::specta]
pub fn task_create(
    state: State<'_, KeysightState>,
    project: String,
    title: String,
    content: Option<String>,
    status: TaskStatus,  // 原 String
    area: Option<String>,
    color: Option<String>,
) -> Result<TaskEntity, AppError> {
    let _t = ScopedTimer::new("cmd:task_create");
    let conn = lock_db(&state.db, "task_create");
    let vault_fs = RealVaultFs::new(state.vault_path.to_string_lossy().into_owned());
    let project_name = task::ProjectName::new(&project).map_err(Into::<AppError>::into)?;
    // 删除 parse_task_status_from_ipc,直接用 status
    task::create(
        &conn,
        &project_name,
        &title,
        content.as_deref(),
        status,  // 直接传 enum
        area.as_deref(),
        color.as_deref(),
        &vault_fs,
    )
    .map_err(Into::into)
}
```

- [ ] **Step 3: 修改 task_update 签名**

类似改 task_update 的 `status: Option<String>` → `status: Option<TaskStatus>`,删除内部 parse 调用。

- [ ] **Step 4: 删除 parse_task_status_from_ipc 函数**

`src-tauri/src/modules/keysight/commands.rs:460-464`:

```rust
// 删除整个函数 (4 行)
fn parse_task_status_from_ipc(status: &str) -> Result<TaskStatus, AppError> {
    serde_json::from_value::<TaskStatus>(serde_json::Value::String(status.to_string()))
        .map_err(|_| AppError::Keysight(format!("task 状态不合法: {status}")))
}
```

- [ ] **Step 5: 编译验证**

```bash
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings
```

预期: clean compile,没有 warning。如果 task_set_color 等其他命令也用了 status string,一并改完。

- [ ] **Step 6: 重新生成 bindings.ts**

```bash
cargo test --manifest-path src-tauri/Cargo.toml export_bindings
```

预期输出: `Total: ... | Exported: 1 | ...`。打开 `src/bindings.ts` 确认 task_create 的 status 参数是 `TaskStatus` literal union 而不是 `string`。

- [ ] **Step 7: 修复 TS 端调用点**

```bash
grep -rn "taskCreate\|taskUpdate" src/components/ src/__tests__/
```

每处 `commands.taskCreate(..., "next", ...)` 改成 `commands.taskCreate(..., "next" as const, ...)` 或直接传 literal,因为 TS literal union 接受字符串字面量。如有 `string` 变量传入,需要 narrow 成 TaskStatus。

- [ ] **Step 8: pnpm build 验证 TS 编译**

```bash
pnpm build
```

预期: clean,无 type error。

- [ ] **Step 9: pnpm test 验证 TS 测试**

```bash
pnpm test -- --run
```

预期: 246/246 全绿(P0 修复后基线)。

- [ ] **Step 10: cargo test 验证 Rust 测试**

```bash
cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

预期: 252/252 全绿。

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/modules/keysight/commands.rs src/bindings.ts
git commit -m "$(cat <<'EOF'
refactor(keysight): TaskStatus IPC 边界 String → enum (Phase 0)

删除 parse_task_status_from_ipc helper,task_create/update 直接接 TaskStatus enum。
tauri-specta 自动生成 TS literal union,TS 端拼错 status 编译期失败。

Kanban view 实施前置 — Phase 0/3 类型迁移。
EOF
)"
```

---

### Task 0.2: KeysightView URL sync(`?wb=<whiteboardId>` ↔ 本地 state)

> **为什么需要这个 task**: 当前 `KeysightView` 用纯本地 `useState` 管理 `currentWhiteboardId`(KeysightView.tsx:42),**不读 URL**。Kanban 的 "Reveal Graph" 按钮要跳到 `/keysight?wb=projects/{name}` 才能让 canvas 显示该 project 白板 — 但现状下跳过去 canvas 仍然显示 `wb_root`,按钮等于无效。这个 task 必须在 Task 3.6(Show Kanban / Reveal Graph 双向按钮)之前完成。

**Files:**
- Modify: `src/components/keysight/KeysightView.tsx` (加 `useSearchParams` + `useEffect` 同步)
- Test: `src/__tests__/components/keysight/KeysightView.test.tsx`(如已有)或新增

- [ ] **Step 1: 写失败的测试**

`src/__tests__/components/keysight/KeysightView.test.tsx` 加(如文件不存在则新建):

```typescript
import { describe, it, expect, vi } from "vitest";
import { render, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { KeysightView } from "@/components/keysight/KeysightView";

// 需要 mock 所有 useWhiteboardData 相关 commands - 复用现有 KeysightView test 的 mock setup
// 这里假设 mock 已存在。如不存在,参照 GraphView.test.tsx 或 KanbanView.test.tsx 的 setup。

const renderWithRoute = (initialEntry: string) => {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <KeysightView />
      </MemoryRouter>
    </QueryClientProvider>
  );
};

describe("KeysightView URL sync", () => {
  it("reads ?wb=projects/super-tauri from URL on initial render", async () => {
    renderWithRoute("/keysight?wb=projects/super-tauri");
    // 预期 GraphView 被初始化为 projects/super-tauri 白板,不是 ROOT_WHITEBOARD
    // 具体断言方式取决于 GraphView 如何把 currentWhiteboardId 渲染出来(例如 breadcrumb、data-testid 等)
    await waitFor(() => {
      // 示例: 检查某个显示当前 wb 的元素
      expect(document.body.textContent).toContain("projects/super-tauri");
    });
  });

  it("falls back to wb_root when ?wb= absent", async () => {
    renderWithRoute("/keysight");
    await waitFor(() => {
      expect(document.body.textContent).toContain("wb_root");
    });
  });

  // 注: "本地切白板同步回 URL" 的断言留到 integration test 做,
  // 因为 Boards 下拉的交互涉及 useWhiteboardData / toolbar / 下拉 UI 多层组合,
  // 在 KeysightView 单元测试里 mock 成本过高。这里只验证读 URL 路径。
});
```

- [ ] **Step 2: 跑测试确认失败**

```bash
pnpm test -- --run KeysightView
```

预期: 失败(因为 KeysightView 还不读 URL)。

- [ ] **Step 3: 修改 KeysightView.tsx 读 URL**

`src/components/keysight/KeysightView.tsx:41-43` 附近改为:

```typescript
import { useCallback, useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";
import { useSearchParams } from "react-router-dom";
// ...其他 import 不变

export function KeysightView() {
  const [searchParams, setSearchParams] = useSearchParams();
  const urlWbId = searchParams.get("wb");

  // 初始 state 从 URL 读;若 URL 没有则 fallback 到 ROOT_WHITEBOARD
  const [currentWhiteboardId, setCurrentWhiteboardId] = useState(
    urlWbId ?? ROOT_WHITEBOARD,
  );
  // ...其他 state 不变

  // URL → state: 外部 navigate(/keysight?wb=xxx) 时同步本地 state
  useEffect(() => {
    if (urlWbId && urlWbId !== currentWhiteboardId) {
      setCurrentWhiteboardId(urlWbId);
      setSelected(null);
    }
  }, [urlWbId, currentWhiteboardId]);

  // state → URL: 本地切白板时同步 URL(通过 handleWhiteboardChange 统一入口)
  const handleWhiteboardChange = useCallback(
    (whiteboardId: string) => {
      setCurrentWhiteboardId(whiteboardId);
      setSelected(null);
      // 同步 URL (replace 模式,不堆历史栈)
      if (whiteboardId === ROOT_WHITEBOARD) {
        setSearchParams({}, { replace: true });
      } else {
        setSearchParams({ wb: whiteboardId }, { replace: true });
      }
    },
    [setSearchParams],
  );
```

注意要点:
- `useEffect(() => {...}, [urlWbId])` 处理外部跳转(reveal graph 按钮 / 刷新页面 / 直接贴 URL)
- `handleWhiteboardChange` 处理内部切换(Boards 下拉)并同步回 URL
- 用 `replace: true` 避免每次切白板都堆进浏览器历史栈
- `ROOT_WHITEBOARD` 不写入 URL(保持路径简洁)

- [ ] **Step 4: 跑测试通过**

```bash
pnpm test -- --run KeysightView
```

预期: 相关 test passed。现有 KeysightView 其他测试可能因 `useSearchParams` 需要 router context 失败,需补 `MemoryRouter` wrapper — 修完为止。

- [ ] **Step 5: pnpm build 验证类型**

```bash
pnpm build
```

预期: green。

- [ ] **Step 6: 回归 — 现有 canvas Task 创建路径(GraphView.tsx)不受影响**

手动核对 GraphView.tsx 里对 `currentWhiteboardId` 的所有读取点是否仍然正确(因为 state 源没变,只是加了 URL 双向同步)。

- [ ] **Step 7: Commit**

```bash
git add src/components/keysight/KeysightView.tsx src/__tests__/components/keysight/KeysightView.test.tsx
git commit -m "$(cat <<'EOF'
feat(keysight): KeysightView URL state sync via ?wb= (Phase 0)

加 useSearchParams 双向同步 currentWhiteboardId 与 URL:
- 外部 navigate(/keysight?wb=projects/xxx) 可直接切到该白板
- Boards 下拉切换时同步写回 URL (replace 模式)
- ROOT_WHITEBOARD 不写入 URL,保持路径简洁

为 Kanban view 的 Reveal Graph / Show Kanban 双向跳转提供硬前置。
EOF
)"
```

---

## Phase 1 — Rust 数据层(共 6 task)

### Task 1.1: TaskStatus 加 Inbox 变体

**Files:**
- Modify: `src-tauri/src/modules/keysight/models.rs:171-176`
- Modify: `src-tauri/src/modules/keysight/domain/task.rs:128-138` (parse_task_status)
- Modify: 编译器报错的所有 `match status` 处

- [ ] **Step 1: 写失败的测试**

`domain/task.rs` 现有测试块内加:

```rust
#[test]
fn parse_task_status_inbox() {
    assert_eq!(parse_task_status("inbox").unwrap(), TaskStatus::Inbox);
}

#[test]
fn task_status_inbox_serializes_lowercase() {
    let json = serde_json::to_string(&TaskStatus::Inbox).unwrap();
    assert_eq!(json, "\"inbox\"");
}
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace parse_task_status_inbox task_status_inbox_serializes
```

预期: 编译失败 "no variant `Inbox`"。

- [ ] **Step 3: TaskStatus enum 加 Inbox**

`models.rs:171-176`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Inbox,    // 新增 — 收集需求阶段暂存
    Next,
    Active,
    Done,
    Blocked,
}
```

- [ ] **Step 4: parse_task_status 加 Inbox case**

`domain/task.rs:128`:

```rust
fn parse_task_status(s: &str) -> Result<TaskStatus, KeysightError> {
    match s {
        "inbox" => Ok(TaskStatus::Inbox),  // 新增
        "next" => Ok(TaskStatus::Next),
        "active" => Ok(TaskStatus::Active),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        _ => Err(KeysightError::InvalidTaskStatus(s.to_string())),
    }
}
```

- [ ] **Step 5: 跑 cargo build 看编译错误**

```bash
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings
```

预期: 多处 match arm 编译失败(防火墙生效!)。逐个补 `TaskStatus::Inbox => ...` 分支。常见点:

- `task_status_to_str` (反向函数)
- 任何 `match status { Next => ... }` 的 UI label / display 函数
- frontmatter 渲染的 status 字符串映射

每个补 case,Inbox 的 string repr 是 `"inbox"`。

- [ ] **Step 6: 跑测试通过**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace parse_task_status_inbox task_status_inbox_serializes
```

预期: 2 passed。

- [ ] **Step 7: 全量 test + clippy**

```bash
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

预期: 258 passed (256 + 2 new)。

- [ ] **Step 8: 重新生成 bindings.ts**

```bash
cargo test --manifest-path src-tauri/Cargo.toml export_bindings
```

确认 `src/bindings.ts` 中 TaskStatus literal 是 5 值:`"inbox" | "next" | "active" | "blocked" | "done"`。

- [ ] **Step 9: pnpm build (TS 应被强制更新)**

```bash
pnpm build
```

预期: 如果有 TS 代码用 `Record<TaskStatus, ...>` 或 `switch (status)` 全穷尽,编译失败提示加 inbox case。修完。当前代码大概率没有,build 直接通过。

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/modules/keysight/models.rs src-tauri/src/modules/keysight/domain/task.rs src/bindings.ts
git commit -m "$(cat <<'EOF'
feat(keysight): TaskStatus 加 Inbox 变体 (Phase 1)

5 值 TaskStatus = Inbox / Next / Active / Blocked / Done。Inbox 用于收集需求阶段。
所有 match arm 编译期强制补齐(防火墙生效)。bindings.ts 同步更新。
EOF
)"
```

---

### Task 1.2: compute_position_below_bottommost helper

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/task.rs` (新增 helper + tests)

- [ ] **Step 1: 写失败的测试**

`domain/task.rs` 测试块内加:

```rust
#[test]
fn compute_position_empty_whiteboard_returns_origin() {
    let conn = test_conn();
    let pos = compute_position_below_bottommost(&conn, "projects/test").unwrap();
    assert_eq!(pos.x, 0.0);
    assert_eq!(pos.y, 0.0);
}

#[test]
fn compute_position_below_single_row() {
    let conn = test_conn();
    conn.execute(
        "INSERT INTO positions (entity_id, whiteboard_id, x, y) VALUES ('e1', 'projects/test', 100.0, 200.0)",
        [],
    ).unwrap();
    let pos = compute_position_below_bottommost(&conn, "projects/test").unwrap();
    assert_eq!(pos.x, 0.0); // 默认 x = 0
    // y = max_y + DEFAULT_NODE_HEIGHT + NODE_SPACING = 200 + 140 + 40 = 380
    assert_eq!(pos.y, 380.0);
}

#[test]
fn compute_position_isolates_whiteboards() {
    let conn = test_conn();
    conn.execute(
        "INSERT INTO positions (entity_id, whiteboard_id, x, y) VALUES ('e1', 'projects/A', 0.0, 500.0)",
        [],
    ).unwrap();
    let pos = compute_position_below_bottommost(&conn, "projects/B").unwrap();
    assert_eq!(pos.y, 0.0); // B 是空的
}

#[test]
fn compute_position_n_rows_picks_max() {
    let conn = test_conn();
    for (i, y) in [50.0, 200.0, 100.0].iter().enumerate() {
        conn.execute(
            &format!("INSERT INTO positions (entity_id, whiteboard_id, x, y) VALUES ('e{}', 'projects/test', 0.0, {})", i, y),
            [],
        ).unwrap();
    }
    let pos = compute_position_below_bottommost(&conn, "projects/test").unwrap();
    // max(50, 200, 100) + 140 + 40 = 380
    assert_eq!(pos.y, 380.0);
}
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace compute_position
```

预期: 编译失败 "function `compute_position_below_bottommost` not found"。

- [ ] **Step 3: 实现 helper**

`domain/task.rs` 找一个合适位置(在 `create` 函数附近)加:

```rust
/// 节点默认高度,用于 compute_position 计算下方坐标。
/// 值 = 140 是 task 节点的折叠状态典型高度,与前端 `src/components/keysight/types.ts`
/// 的 `ENTITY_DIMENSIONS.task.height` 保持对齐。以 task 为基准(kanban 创建的
/// 主要是 task),其他 entity 高度可能更大但不是 kanban 关心的场景。
const DEFAULT_NODE_HEIGHT: f64 = 140.0;

/// 节点之间垂直间距。
const NODE_SPACING: f64 = 40.0;

/// 计算给定 whiteboard 上"最底元素下方"的坐标,用于 kanban 创建 task 时
/// 自动定位到 canvas 上不重叠位置。
///
/// 算法: SELECT MAX(y) FROM positions WHERE whiteboard_id = ?
/// → new_y = max_y + DEFAULT_NODE_HEIGHT + NODE_SPACING
/// → new_x = 0.0 (默认)
///
/// 空白板返回 Position { x: 0, y: 0 }。
pub(in crate::modules::keysight) fn compute_position_below_bottommost(
    conn: &Connection,
    whiteboard_id: &str,
) -> Result<Position, KeysightError> {
    let max_y: Option<f64> = conn.query_row(
        "SELECT MAX(y) FROM positions WHERE whiteboard_id = ?1",
        [whiteboard_id],
        |row| row.get(0),
    ).optional()?;
    let new_y = match max_y {
        Some(y) => y + DEFAULT_NODE_HEIGHT + NODE_SPACING,
        None => 0.0,
    };
    Ok(Position { x: 0.0, y: new_y })
}
```

注意: 需要 import `rusqlite::OptionalExtension` (`use rusqlite::OptionalExtension;`) 顶部加上,或 fully qualify `optional()`。

- [ ] **Step 4: 跑测试通过**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace compute_position
```

预期: 4 passed。

- [ ] **Step 5: clippy + 全测**

```bash
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

预期: 262 passed (258 + 4 new)。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/task.rs
git commit -m "$(cat <<'EOF'
feat(keysight): compute_position_below_bottommost helper (Phase 1)

新 helper 计算 kanban 创建 task 时 canvas 上的自动坐标,
落在最底元素下方一个 node 高度 + spacing 的位置。空白板返回 (0, 0)。

为 task::create 集成 auto-position 做准备。
EOF
)"
```

---

### Task 1.3: 集成 auto-position 到 task::create

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/task.rs` (`create` 函数)

- [ ] **Step 1: 写失败的测试**

`domain/task.rs` 测试块内加:

```rust
// 注意: task::create 当前真实签名是 `create(conn, vault_fs, project, TaskCreateInput)` struct 输入,
// 不是位置参数。TaskCreateInput 见 models/task.rs 定义,字段为 title/content/status/area/color。

#[test]
fn create_task_writes_position_row_first_time() {
    let conn = test_conn();
    let fs = MockVaultFs::new();
    let project = ProjectName::new("test").unwrap();
    let task = create(
        &conn,
        &fs,
        &project,
        TaskCreateInput {
            title: "first task",
            content: None,
            status: TaskStatus::Inbox,
            area: None,
            color: None,
        },
    ).unwrap();

    let pos: (f64, f64) = conn.query_row(
        "SELECT x, y FROM positions WHERE entity_id = ?1 AND whiteboard_id = ?2",
        rusqlite::params![&task.id, "projects/test"],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(pos, (0.0, 0.0)); // 首个 task,默认坐标
}

#[test]
fn create_task_writes_position_below_bottommost() {
    let conn = test_conn();
    let fs = MockVaultFs::new();
    let project = ProjectName::new("test").unwrap();
    let _first = create(
        &conn, &fs, &project,
        TaskCreateInput { title: "first", content: None, status: TaskStatus::Inbox, area: None, color: None },
    ).unwrap();
    let second = create(
        &conn, &fs, &project,
        TaskCreateInput { title: "second", content: None, status: TaskStatus::Inbox, area: None, color: None },
    ).unwrap();

    let pos: (f64, f64) = conn.query_row(
        "SELECT x, y FROM positions WHERE entity_id = ?1",
        rusqlite::params![&second.id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    // 第一个 task 落在 (0, 0) → 第二个落在 0 + 140 + 40 = 180
    assert_eq!(pos.1, 180.0);
}
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace create_task_writes_position
```

预期: 测试编译通过(相关函数存在),但 assert 失败(positions 表没行)。

- [ ] **Step 3: 修改 task::create 加 position 写入**

找到 `create` 函数(大概 `domain/task.rs:200-280` 范围),在最后 `sync::sync_file` 调用之后(或 task entity 已经 insert 之后)加:

```rust
// 自动定位: 在 canvas 最底元素下方
let position = compute_position_below_bottommost(conn, &whiteboard_id_str)?;
conn.execute(
    "INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, ?2, ?3, ?4)",
    params![&task_id, &whiteboard_id_str, position.x, position.y],
)?;
```

注意: 需要确认 `task::create` 函数当前是怎么算 wb_id 的(从 `project.whiteboard_id()` 推导)。把那个 wb_id 字符串复用即可。

- [ ] **Step 4: 跑测试通过**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace create_task_writes_position
```

预期: 2 passed。

- [ ] **Step 5: 全量 test + clippy**

```bash
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

预期: 264 passed。如有 task::create 相关现有测试被打破(因为它们没预期 positions row),逐个修正断言。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/task.rs
git commit -m "$(cat <<'EOF'
feat(keysight): task::create 集成 auto-position (Phase 1)

新建 task 自动写 positions 行,坐标 = 当前 wb 最底元素下方。
让 kanban 创建的 task 立即在 canvas 上可见,无需手动 layout_set_position。

- 首个 task → (0, 0)
- 后续 task → (0, max_y + 80 + 40)
EOF
)"
```

---

### Task 1.4: task::query_kanban 函数 + IPC command

**Files:**
- Modify: `src-tauri/src/modules/keysight/domain/task.rs` (新增 `query_kanban` 函数)
- Modify: `src-tauri/src/modules/keysight/commands.rs` (新增 `task_query_kanban` 命令)
- Modify: `src-tauri/src/lib.rs` (collect_commands! 注册)

- [ ] **Step 1: 写失败的测试**

`domain/task.rs` 测试块内加:

```rust
fn make_input<'a>(title: &'a str, status: TaskStatus) -> TaskCreateInput<'a> {
    TaskCreateInput { title, content: None, status, area: None, color: None }
}

#[test]
fn query_kanban_with_project_filter() {
    let conn = test_conn();
    let fs = MockVaultFs::new();
    let project_a = ProjectName::new("alpha").unwrap();
    let project_b = ProjectName::new("beta").unwrap();
    create(&conn, &fs, &project_a, make_input("task in alpha", TaskStatus::Inbox)).unwrap();
    create(&conn, &fs, &project_a, make_input("another in alpha", TaskStatus::Next)).unwrap();
    create(&conn, &fs, &project_b, make_input("task in beta", TaskStatus::Active)).unwrap();

    let alpha_tasks = query_kanban(&conn, Some(&project_a)).unwrap();
    assert_eq!(alpha_tasks.len(), 2);
    // TaskEntity.project 是 Option<String>, 这里都是 Some("alpha")
    assert!(alpha_tasks.iter().all(|t| t.project.as_deref() == Some("alpha")));
}

#[test]
fn query_kanban_all_projects() {
    let conn = test_conn();
    let fs = MockVaultFs::new();
    let project_a = ProjectName::new("alpha").unwrap();
    let project_b = ProjectName::new("beta").unwrap();
    create(&conn, &fs, &project_a, make_input("a1", TaskStatus::Inbox)).unwrap();
    create(&conn, &fs, &project_b, make_input("b1", TaskStatus::Active)).unwrap();

    let all = query_kanban(&conn, None).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn query_kanban_empty_returns_empty_vec() {
    let conn = test_conn();
    let result = query_kanban(&conn, None).unwrap();
    assert_eq!(result.len(), 0);
}
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace query_kanban
```

预期: 编译失败 "function `query_kanban` not found"。

- [ ] **Step 3: 实现 query_kanban**

`domain/task.rs` 在现有 `query_all` 函数附近加:

```rust
/// 查询 task list 用于 kanban view。
///
/// - `project = None` → 跨项目查所有 task entity
/// - `project = Some(name)` → 仅该 project 的 task
///
/// 不按 status 分组(留给前端按 task.status 渲染)。**V1 排序按 `e.title` ASC**
/// (对齐 `task::query_all` 的现有行为),因为 entities 表当前没有 `created_at` 列。
/// V2 如需"最新优先"再单独加 migration + 排序字段。
pub(in crate::modules::keysight) fn query_kanban(
    conn: &Connection,
    project: Option<&ProjectName>,
) -> Result<Vec<TaskEntity>, KeysightError> {
    // 两条 SQL 只差一个 WHERE 子句,分开写避免动态 params 借用问题。
    // SELECT 列顺序和 mapper 索引必须对照 task::query_all 保持一致。
    let mapper = |r: &rusqlite::Row<'_>| -> Result<TaskEntity, rusqlite::Error> {
        Ok(TaskEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            whiteboard_id: r.get(2)?,
            content: r.get(3)?,
            status: r.get(4)?,
            area: r.get(5)?,
            project: r.get(6)?,
            color: r.get(7)?,
        })
    };

    match project {
        Some(p) => {
            let wb = p.whiteboard_id();
            let mut stmt = conn.prepare(
                "SELECT e.id, e.title, e.whiteboard_id, COALESCE(e.content, '') AS content, t.status, t.area, t.project, e.color \
                 FROM entities e \
                 JOIN task_fields t ON t.entity_id = e.id \
                 WHERE e.kind = 'task' AND e.whiteboard_id = ?1 \
                 ORDER BY e.title",
            )?;
            Ok(stmt.query_map([wb], mapper)?.collect::<Result<Vec<_>, _>>()?)
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.title, e.whiteboard_id, COALESCE(e.content, '') AS content, t.status, t.area, t.project, e.color \
                 FROM entities e \
                 JOIN task_fields t ON t.entity_id = e.id \
                 WHERE e.kind = 'task' \
                 ORDER BY e.title",
            )?;
            Ok(stmt.query_map([], mapper)?.collect::<Result<Vec<_>, _>>()?)
        }
    }
}
```

注意:
- **entities 表 `kind` 字段名是 `kind` 不是 `entity_kind`**(见 db.rs:12)。上面 SQL 已对齐。
- **entities 表没有 `created_at` 列**,所以不能 ORDER BY 它。V1 用 `e.title`。
- column 顺序 / TaskEntity 字段顺序对照 `task::query_all` 现有实现(`domain/task.rs` 的 `query_all`),包括 `COALESCE(e.content, '')` 处理 NULL content。

- [ ] **Step 4: 跑测试通过**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace query_kanban
```

预期: 3 passed。

- [ ] **Step 5: 加 IPC command 包装**

`commands.rs` 在 task 段(line 466 附近)加:

```rust
/// 查询 kanban view 数据 — 跨项目或单项目 task list。
#[tauri::command]
#[specta::specta]
pub fn task_query_kanban(
    state: State<'_, KeysightState>,
    project: Option<String>,
) -> Result<Vec<TaskEntity>, AppError> {
    let _t = ScopedTimer::new("cmd:task_query_kanban");
    let conn = lock_db(&state.db, "task_query_kanban");
    let project_name = match project {
        Some(p) => Some(task::ProjectName::new(&p).map_err(Into::<AppError>::into)?),
        None => None,
    };
    task::query_kanban(&conn, project_name.as_ref()).map_err(Into::into)
}
```

- [ ] **Step 6: 注册 command 到 collect_commands!**

`src-tauri/src/lib.rs` 在 task_create / task_update 旁边加:

```rust
        modules::keysight::commands::task_query_kanban,
```

- [ ] **Step 7: 重新生成 bindings.ts**

```bash
cargo test --manifest-path src-tauri/Cargo.toml export_bindings
```

预期: bindings.ts 多 `taskQueryKanban` 函数。

- [ ] **Step 8: 全量 test + clippy + ts build**

```bash
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings && \
cargo test --workspace --manifest-path src-tauri/Cargo.toml && \
pnpm build
```

预期: Rust 267 passed (264 + 3 new),TS build green。

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/modules/keysight/domain/task.rs src-tauri/src/modules/keysight/commands.rs src-tauri/src/lib.rs src/bindings.ts
git commit -m "$(cat <<'EOF'
feat(keysight): task_query_kanban 命令 (Phase 1)

新 query 函数 + IPC command 支持 kanban view 数据需求:
- project = None → 跨项目查所有 task
- project = Some(name) → 仅该 project 的 task
V1 按 e.title ASC 排序(entities 表当前无 created_at 列),
前端按 task.status 分列渲染。
EOF
)"
```

---

### Task 1.5: positions 表加 index (whiteboard_id, y)

**Files:**
- Modify: `src-tauri/src/modules/keysight/db.rs`

- [ ] **Step 1: 找现有 positions index 定义**

```bash
grep -n "CREATE INDEX.*positions\|positions" src-tauri/src/modules/keysight/db.rs
```

记下当前 schema 区的位置。

- [ ] **Step 2: 加 index DDL**

`db.rs` 在 positions 表 CREATE 之后加:

```rust
    "CREATE INDEX IF NOT EXISTS idx_positions_wb_y ON positions(whiteboard_id, y)",
```

(具体语法取决于 db.rs 是用 array of statements 还是 batch SQL,对照现有风格写。)

- [ ] **Step 3: 跑 test 验证 schema 没坏**

```bash
cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

预期: 267 passed,无新失败。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/modules/keysight/db.rs
git commit -m "perf(keysight): positions 表加 (whiteboard_id, y) index 加速 compute_position (Phase 1)"
```

---

### Task 1.6: 副作用矩阵更新

**Files:**
- Modify: `CLAUDE.md` (副作用矩阵章节)

- [ ] **Step 1: 找现有 task::create 行**

```bash
grep -n "task_create\|task::create" CLAUDE.md
```

- [ ] **Step 2: 修改 task_create 行加 positions**

把 `task_create` (B2) 行的"影响的表/资源"列从原值改为 + `positions`。具体改 line ~430 附近(参考 `grep` 结果)。

示例:

```markdown
| `task_create` (B2) | `entities`, `task_fields`, `file_mtimes`, `entities_fts`, **`positions`** + `whiteboard/projects/{project}/{id} 【TASK】{title}.md` | DB 写 + 文件写(含 color/area/project) + auto-position 写 positions 行;路径由 `ProjectName` 决定 | domain unit test |
```

- [ ] **Step 3: 加 task_query_kanban 行**

在 task 段加(纯查询不入矩阵,但**矩阵旁边的"质询"段或新增 query 段如有**):

实际上 query 不入副作用矩阵(矩阵是写操作专用),所以这步可能跳过。检查 CLAUDE.md 副作用矩阵的范围,纯 query 不入。

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: 副作用矩阵 task_create 加 positions (Phase 1)"
```

---

## Phase 2 — Frontend 基础组件壳(共 5 task)

### Task 2.1: AppShell sidebar 加 Kanban nav item

**Files:**
- Modify: `src/components/AppShell.tsx:30-49` (modules + pageTitles)

- [ ] **Step 1: 写失败的测试**

`src/__tests__/components/AppShell.test.tsx` (如不存在则创建,但可能本项目没单独测 AppShell;如不存在,跳到 step 3 直接改代码并人眼验证)

如果存在测试,加:

```typescript
test("sidebar shows Kanban nav item", () => {
  render(<AppShell />, { wrapper: BrowserRouter });
  expect(screen.getByText("Kanban")).toBeInTheDocument();
});
```

- [ ] **Step 2: 跑测试确认失败(如有测试)**

```bash
pnpm test -- --run AppShell
```

预期: fail (no element with text "Kanban")。

- [ ] **Step 3: 修改 AppShell.tsx**

`src/components/AppShell.tsx`:

引入 icon:

```typescript
import {
  CheckSquare,
  FileText,
  Bookmark,
  Code2,
  Grid3x3,
  LayoutGrid,  // 新增
  Settings,
  Zap,
} from "lucide-react";
```

`modules` 数组(line 30):

```typescript
const modules = [
  { label: "KeySight", icon: Grid3x3, path: "/keysight" },
  { label: "Kanban", icon: LayoutGrid, path: "/kanban" },  // 新增,放在 KeySight 下方
  { label: "Todo", icon: CheckSquare, path: "/todo" },
  { label: "Notes", icon: FileText, path: "/notes" },
  { label: "Bookmarks", icon: Bookmark, path: "/bookmarks" },
  { label: "Snippets", icon: Code2, path: "/snippets" },
];
```

`pageTitles` 对象(line 42):

```typescript
const pageTitles: Record<string, string> = {
  "/keysight": "KeySight",
  "/kanban": "Kanban",  // 新增
  "/todo": "Todo",
  "/notes": "Notes",
  "/bookmarks": "Bookmarks",
  "/snippets": "Snippets",
  "/settings": "Settings",
};
```

- [ ] **Step 4: pnpm test + build 验证**

```bash
pnpm test -- --run && pnpm build
```

预期: 现有测试不破(/kanban 路由还不存在所以 navigate 会 fall through 到 /keysight,但 sidebar item 渲染没问题)。

- [ ] **Step 5: Commit**

```bash
git add src/components/AppShell.tsx
git commit -m "feat(kanban): AppShell sidebar 加 Kanban 导航项 (Phase 2)"
```

---

### Task 2.2: /kanban 路由 + KanbanView 空壳

**Files:**
- Create: `src/components/kanban/KanbanView.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: 创建 KanbanView 空壳组件**

```typescript
// src/components/kanban/KanbanView.tsx
import { useSearchParams } from "react-router-dom";

/**
 * Kanban 主页面壳。URL state: `?project={name}` 单项目,缺失 = "All projects"。
 * V1 阶段先放空白 placeholder,后续 task 接入数据。
 */
export function KanbanView() {
  const [searchParams] = useSearchParams();
  const project = searchParams.get("project");

  return (
    <div className="flex h-full flex-col p-6" data-testid="kanban-view">
      <h2 className="mb-4 text-xl font-semibold">
        Kanban {project ? `— ${project}` : "(All projects)"}
      </h2>
      <p className="text-muted-foreground">Kanban view 还在搭建中...</p>
    </div>
  );
}
```

- [ ] **Step 2: 在 App.tsx 注册路由**

`src/App.tsx`:

```typescript
import { KanbanView } from "@/components/kanban/KanbanView";
```

在 `<Routes>` 内加:

```typescript
<Route path="/kanban" element={<KanbanView />} />
```

放在 `/keysight` 和 `/todo` 之间。

- [ ] **Step 3: 写测试**

```typescript
// src/__tests__/components/kanban/KanbanView.test.tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { KanbanView } from "@/components/kanban/KanbanView";

describe("KanbanView", () => {
  it("shows All projects heading when no query param", () => {
    render(
      <MemoryRouter initialEntries={["/kanban"]}>
        <KanbanView />
      </MemoryRouter>
    );
    expect(screen.getByText(/Kanban.*All projects/)).toBeInTheDocument();
  });

  it("shows project name when ?project= present", () => {
    render(
      <MemoryRouter initialEntries={["/kanban?project=super-tauri"]}>
        <KanbanView />
      </MemoryRouter>
    );
    expect(screen.getByText(/Kanban — super-tauri/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 4: 跑测试通过**

```bash
pnpm test -- --run KanbanView
```

预期: 2 passed。

- [ ] **Step 5: pnpm build 验证类型**

```bash
pnpm build
```

预期: green。

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/components/kanban/KanbanView.tsx src/__tests__/components/kanban/KanbanView.test.tsx
git commit -m "feat(kanban): /kanban 路由 + KanbanView 空壳 (Phase 2)"
```

---

### Task 2.3: KanbanColumn 静态组件

**Files:**
- Create: `src/components/kanban/KanbanColumn.tsx`
- Create: `src/components/kanban/columns.ts`
- Test: `src/__tests__/components/kanban/KanbanColumn.test.tsx`

- [ ] **Step 1: 创建 columns.ts (强类型 column 配置)**

```typescript
// src/components/kanban/columns.ts
import type { TaskStatus } from "@/bindings";

export interface ColumnConfig {
  status: TaskStatus;
  label: string;
  description: string;
}

/**
 * 5 个 status 列的配置,用 Record 强制穷尽 — 加新 TaskStatus variant 时
 * 这里编译期失败,所有引用点必须同步更新。
 */
export const COLUMNS: Record<TaskStatus, ColumnConfig> = {
  inbox: { status: "inbox", label: "Inbox", description: "收集需求" },
  next: { status: "next", label: "Next", description: "下一步" },
  active: { status: "active", label: "Active", description: "进行中" },
  blocked: { status: "blocked", label: "Blocked", description: "阻塞" },
  done: { status: "done", label: "Done", description: "完成" },
};

/**
 * 列在 board 上的固定显示顺序。
 */
export const COLUMN_ORDER: TaskStatus[] = ["inbox", "next", "active", "blocked", "done"];
```

- [ ] **Step 2: 创建 KanbanColumn.tsx**

```typescript
// src/components/kanban/KanbanColumn.tsx
import { Plus } from "lucide-react";
import type { TaskEntity, TaskStatus } from "@/bindings";
import { COLUMNS } from "./columns";

interface KanbanColumnProps {
  status: TaskStatus;
  tasks: TaskEntity[];
  onAddTask: (status: TaskStatus) => void;
  children?: React.ReactNode;
}

export function KanbanColumn({ status, tasks, onAddTask, children }: KanbanColumnProps) {
  const config = COLUMNS[status];

  return (
    <div className="flex w-72 flex-col rounded-lg bg-muted/30 p-3" data-testid={`kanban-column-${status}`}>
      <div className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-sm font-semibold">{config.label}</div>
          <div className="text-xs text-muted-foreground">{tasks.length}</div>
        </div>
        <button
          aria-label={`新建 ${config.label} task`}
          onClick={() => onAddTask(status)}
          className="rounded p-1 hover:bg-muted"
        >
          <Plus className="size-4" />
        </button>
      </div>
      <div className="flex flex-1 flex-col gap-2">
        {tasks.length === 0 ? (
          <div className="rounded border border-dashed p-4 text-center text-xs text-muted-foreground">
            还没有 task。点 + 创建第一个
          </div>
        ) : (
          children
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: 写测试**

```typescript
// src/__tests__/components/kanban/KanbanColumn.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { KanbanColumn } from "@/components/kanban/KanbanColumn";
import type { TaskEntity } from "@/bindings";

const mockTask = (over: Partial<TaskEntity> = {}): TaskEntity => ({
  id: "t1",
  title: "Test Task",
  whiteboardId: "projects/test",
  content: "",
  status: "inbox",
  area: null,
  project: "test",
  color: null,
  ...over,
});

describe("KanbanColumn", () => {
  it("displays label from COLUMNS config", () => {
    render(<KanbanColumn status="inbox" tasks={[]} onAddTask={vi.fn()} />);
    expect(screen.getByText("Inbox")).toBeInTheDocument();
  });

  it("shows count of tasks", () => {
    render(<KanbanColumn status="next" tasks={[mockTask(), mockTask({ id: "t2" })]} onAddTask={vi.fn()} />);
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("shows empty state when no tasks", () => {
    render(<KanbanColumn status="active" tasks={[]} onAddTask={vi.fn()} />);
    expect(screen.getByText(/还没有 task/)).toBeInTheDocument();
  });

  it("calls onAddTask with column status when + clicked", () => {
    const handler = vi.fn();
    render(<KanbanColumn status="blocked" tasks={[]} onAddTask={handler} />);
    fireEvent.click(screen.getByLabelText(/新建 Blocked task/));
    expect(handler).toHaveBeenCalledWith("blocked");
  });
});
```

- [ ] **Step 4: 跑测试通过**

```bash
pnpm test -- --run KanbanColumn
```

预期: 4 passed。

- [ ] **Step 5: Commit**

```bash
git add src/components/kanban/KanbanColumn.tsx src/components/kanban/columns.ts src/__tests__/components/kanban/KanbanColumn.test.tsx
git commit -m "feat(kanban): KanbanColumn + COLUMNS Record 强类型配置 (Phase 2)"
```

---

### Task 2.4: KanbanCard 静态组件

**Files:**
- Create: `src/components/kanban/KanbanCard.tsx`
- Test: `src/__tests__/components/kanban/KanbanCard.test.tsx`

- [ ] **Step 1: 创建 KanbanCard.tsx**

```typescript
// src/components/kanban/KanbanCard.tsx
import type { TaskEntity } from "@/bindings";

interface KanbanCardProps {
  task: TaskEntity;
  showProjectTag?: boolean;
}

export function KanbanCard({ task, showProjectTag = false }: KanbanCardProps) {
  const bgColor = task.color ?? undefined;
  return (
    <div
      className="rounded-md border bg-card p-3 shadow-sm"
      style={{ borderLeftColor: bgColor, borderLeftWidth: bgColor ? 4 : 1 }}
      data-testid={`kanban-card-${task.id}`}
    >
      <div className="text-sm font-medium">{task.title}</div>
      {showProjectTag && (
        <div className="mt-1 inline-block rounded bg-secondary px-1.5 py-0.5 text-xs text-secondary-foreground">
          {task.project}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: 写测试**

```typescript
// src/__tests__/components/kanban/KanbanCard.test.tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { KanbanCard } from "@/components/kanban/KanbanCard";
import type { TaskEntity } from "@/bindings";

const baseTask: TaskEntity = {
  id: "t1",
  title: "Build feature",
  whiteboardId: "projects/super-tauri",
  content: "",
  status: "next",
  area: null,
  project: "super-tauri",
  color: null,
};

describe("KanbanCard", () => {
  it("renders task title", () => {
    render(<KanbanCard task={baseTask} />);
    expect(screen.getByText("Build feature")).toBeInTheDocument();
  });

  it("does not show project tag by default", () => {
    render(<KanbanCard task={baseTask} />);
    expect(screen.queryByText("super-tauri")).not.toBeInTheDocument();
  });

  it("shows project tag when showProjectTag=true", () => {
    render(<KanbanCard task={baseTask} showProjectTag />);
    expect(screen.getByText("super-tauri")).toBeInTheDocument();
  });

  it("uses task color for left border", () => {
    render(<KanbanCard task={{ ...baseTask, color: "#ff0000" }} />);
    const card = screen.getByTestId("kanban-card-t1");
    expect(card.style.borderLeftColor).toBe("rgb(255, 0, 0)");
  });
});
```

- [ ] **Step 3: 跑测试通过**

```bash
pnpm test -- --run KanbanCard
```

预期: 4 passed。

- [ ] **Step 4: Commit**

```bash
git add src/components/kanban/KanbanCard.tsx src/__tests__/components/kanban/KanbanCard.test.tsx
git commit -m "feat(kanban): KanbanCard 卡片组件 (Phase 2)"
```

---

### Task 2.5: KanbanBoard 组合 5 列(无 dnd 无 query)

**Files:**
- Create: `src/components/kanban/KanbanBoard.tsx`
- Test: `src/__tests__/components/kanban/KanbanBoard.test.tsx`

- [ ] **Step 1: 创建 KanbanBoard.tsx**

```typescript
// src/components/kanban/KanbanBoard.tsx
import type { TaskEntity, TaskStatus } from "@/bindings";
import { KanbanColumn } from "./KanbanColumn";
import { KanbanCard } from "./KanbanCard";
import { COLUMN_ORDER } from "./columns";

interface KanbanBoardProps {
  tasks: TaskEntity[];
  showProjectTags: boolean;
  onAddTask: (status: TaskStatus) => void;
}

export function KanbanBoard({ tasks, showProjectTags, onAddTask }: KanbanBoardProps) {
  // 按 status 分组
  const grouped: Record<TaskStatus, TaskEntity[]> = {
    inbox: [],
    next: [],
    active: [],
    blocked: [],
    done: [],
  };
  for (const task of tasks) {
    grouped[task.status].push(task);
  }

  return (
    <div className="flex gap-4 overflow-x-auto p-4" data-testid="kanban-board">
      {COLUMN_ORDER.map((status) => (
        <KanbanColumn key={status} status={status} tasks={grouped[status]} onAddTask={onAddTask}>
          {grouped[status].map((task) => (
            <KanbanCard key={task.id} task={task} showProjectTag={showProjectTags} />
          ))}
        </KanbanColumn>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: 写测试**

```typescript
// src/__tests__/components/kanban/KanbanBoard.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { KanbanBoard } from "@/components/kanban/KanbanBoard";
import type { TaskEntity } from "@/bindings";

const tasks: TaskEntity[] = [
  { id: "1", title: "T1", whiteboardId: "projects/a", content: "", status: "inbox", area: null, project: "a", color: null },
  { id: "2", title: "T2", whiteboardId: "projects/a", content: "", status: "next", area: null, project: "a", color: null },
  { id: "3", title: "T3", whiteboardId: "projects/a", content: "", status: "next", area: null, project: "a", color: null },
];

describe("KanbanBoard", () => {
  it("renders all 5 columns in fixed order", () => {
    render(<KanbanBoard tasks={[]} showProjectTags={false} onAddTask={vi.fn()} />);
    const board = screen.getByTestId("kanban-board");
    const columns = board.querySelectorAll('[data-testid^="kanban-column-"]');
    expect(columns.length).toBe(5);
    expect(columns[0].getAttribute("data-testid")).toBe("kanban-column-inbox");
    expect(columns[4].getAttribute("data-testid")).toBe("kanban-column-done");
  });

  it("groups tasks by status", () => {
    render(<KanbanBoard tasks={tasks} showProjectTags={false} onAddTask={vi.fn()} />);
    expect(screen.getByText("T1")).toBeInTheDocument();
    expect(screen.getByText("T2")).toBeInTheDocument();
    expect(screen.getByText("T3")).toBeInTheDocument();
  });

  it("passes showProjectTags to cards", () => {
    render(<KanbanBoard tasks={tasks} showProjectTags={true} onAddTask={vi.fn()} />);
    const projectTags = screen.getAllByText("a");
    expect(projectTags.length).toBe(3);
  });
});
```

- [ ] **Step 3: 跑测试通过**

```bash
pnpm test -- --run KanbanBoard
```

预期: 3 passed。

- [ ] **Step 4: Commit**

```bash
git add src/components/kanban/KanbanBoard.tsx src/__tests__/components/kanban/KanbanBoard.test.tsx
git commit -m "feat(kanban): KanbanBoard 组合 5 列固定顺序 (Phase 2)"
```

---

## Phase 3 — Frontend 数据接线(共 8 task)

### Task 3.1: invalidateAllTaskCaches helper

**Files:**
- Create: `src/components/kanban/invalidateAllTaskCaches.ts`

- [ ] **Step 1: 创建 helper**

```typescript
// src/components/kanban/invalidateAllTaskCaches.ts
import type { QueryClient } from "@tanstack/react-query";

/**
 * 集中 invalidate 所有 task 相关 cache key,确保 kanban 和 canvas 双向同步。
 *
 * 任何 task 写操作(create / update / delete / set_color)成功后必须调用此 helper,
 * 否则会出现两边数据漂移(L0 数据真源约束)。
 */
export function invalidateAllTaskCaches(queryClient: QueryClient, whiteboardId?: string) {
  // Kanban view 的所有 query (跨项目 + 单项目)
  queryClient.invalidateQueries({ queryKey: ["tasks-kanban"] });
  // Canvas view 的 task list,如果知道具体 wb 优先精确 invalidate
  if (whiteboardId) {
    queryClient.invalidateQueries({ queryKey: ["tasks", whiteboardId] });
    queryClient.invalidateQueries({ queryKey: ["positions", whiteboardId] });
  } else {
    // fallback: 全 invalidate,正确但更多 refetch
    queryClient.invalidateQueries({ queryKey: ["tasks"] });
    queryClient.invalidateQueries({ queryKey: ["positions"] });
  }
}
```

- [ ] **Step 2: 写测试**

```typescript
// src/__tests__/components/kanban/invalidateAllTaskCaches.test.ts
import { describe, it, expect, vi } from "vitest";
import { QueryClient } from "@tanstack/react-query";
import { invalidateAllTaskCaches } from "@/components/kanban/invalidateAllTaskCaches";

describe("invalidateAllTaskCaches", () => {
  it("invalidates tasks-kanban + specific whiteboard caches when wb provided", () => {
    const qc = new QueryClient();
    const spy = vi.spyOn(qc, "invalidateQueries");

    invalidateAllTaskCaches(qc, "projects/test");

    expect(spy).toHaveBeenCalledWith({ queryKey: ["tasks-kanban"] });
    expect(spy).toHaveBeenCalledWith({ queryKey: ["tasks", "projects/test"] });
    expect(spy).toHaveBeenCalledWith({ queryKey: ["positions", "projects/test"] });
  });

  it("invalidates broad keys when no wb provided", () => {
    const qc = new QueryClient();
    const spy = vi.spyOn(qc, "invalidateQueries");

    invalidateAllTaskCaches(qc);

    expect(spy).toHaveBeenCalledWith({ queryKey: ["tasks-kanban"] });
    expect(spy).toHaveBeenCalledWith({ queryKey: ["tasks"] });
    expect(spy).toHaveBeenCalledWith({ queryKey: ["positions"] });
  });
});
```

- [ ] **Step 3: 跑测试通过**

```bash
pnpm test -- --run invalidateAllTaskCaches
```

预期: 2 passed。

- [ ] **Step 4: Commit**

```bash
git add src/components/kanban/invalidateAllTaskCaches.ts src/__tests__/components/kanban/invalidateAllTaskCaches.test.ts
git commit -m "feat(kanban): invalidateAllTaskCaches helper (Phase 3)"
```

---

### Task 3.2: KanbanView 接 query 渲染 KanbanBoard

**Files:**
- Modify: `src/components/kanban/KanbanView.tsx`
- Modify: `src/__tests__/components/kanban/KanbanView.test.tsx`

- [ ] **Step 1: 改 KanbanView.tsx 接 query**

```typescript
// src/components/kanban/KanbanView.tsx
import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { KanbanBoard } from "./KanbanBoard";
import type { TaskStatus } from "@/bindings";

export function KanbanView() {
  const [searchParams] = useSearchParams();
  const project = searchParams.get("project");

  // 关键: 用 unwrapCommand helper,不要手写 result.status 判断(L0 不解析 error 字符串 + 一致性)
  const tasksQuery = useQuery({
    queryKey: ["tasks-kanban", project],
    queryFn: () => unwrapCommand(commands.taskQueryKanban(project)),
  });

  const [, setCreatingStatus] = useState<TaskStatus | null>(null);

  if (tasksQuery.isLoading) {
    return <div className="p-6 text-muted-foreground">Loading kanban...</div>;
  }

  if (tasksQuery.isError) {
    return (
      <div className="p-6">
        <div className="text-destructive">
          加载失败: {String(tasksQuery.error)}
        </div>
        <button onClick={() => tasksQuery.refetch()} className="mt-2 rounded border px-3 py-1">
          Retry
        </button>
      </div>
    );
  }

  const tasks = tasksQuery.data ?? [];
  const showProjectTags = !project; // "All projects" 模式才显示 project tag

  return (
    <div className="flex h-full flex-col" data-testid="kanban-view">
      <div className="border-b p-4">
        <h2 className="text-xl font-semibold">
          Kanban {project ? `— ${project}` : "(All projects)"}
        </h2>
      </div>
      <div className="flex-1 overflow-hidden">
        <KanbanBoard tasks={tasks} showProjectTags={showProjectTags} onAddTask={setCreatingStatus} />
      </div>
    </div>
  );
}
```

注意:
- 统一用 `unwrapCommand` helper(见 `src/lib/commandResult.ts`),不要手写 `if (result.status === "error")`
- `unwrapCommand` 把 Rust 侧 error `throw` 出来,React Query 自动进 `isError` 状态
- `tasksQuery.error` 是 `unknown`,直接 `String(...)` 展示即可(不解析内部结构)

- [ ] **Step 2: 更新测试 mock commands**

```typescript
// src/__tests__/components/kanban/KanbanView.test.tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { KanbanView } from "@/components/kanban/KanbanView";
import { commands } from "@/bindings";

vi.mock("@/bindings", () => ({
  commands: {
    taskQueryKanban: vi.fn(),
  },
}));

const renderKanban = (initialEntry: string) => {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <KanbanView />
      </MemoryRouter>
    </QueryClientProvider>
  );
};

describe("KanbanView", () => {
  beforeEach(() => {
    vi.mocked(commands.taskQueryKanban).mockResolvedValue({ status: "ok", data: [] });
  });

  it("shows All projects heading when no query param", async () => {
    renderKanban("/kanban");
    await waitFor(() => expect(screen.getByText(/All projects/)).toBeInTheDocument());
  });

  it("shows project name when ?project= present", async () => {
    renderKanban("/kanban?project=super-tauri");
    await waitFor(() => expect(screen.getByText(/super-tauri/)).toBeInTheDocument());
  });

  it("shows error and retry on query failure", async () => {
    vi.mocked(commands.taskQueryKanban).mockResolvedValue({ status: "error", error: "DB locked" });
    renderKanban("/kanban");
    await waitFor(() => expect(screen.getByText(/加载失败/)).toBeInTheDocument());
    expect(screen.getByText(/Retry/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: 跑测试通过**

```bash
pnpm test -- --run KanbanView
```

预期: 3 passed。

- [ ] **Step 4: pnpm build 验证类型**

```bash
pnpm build
```

预期: green。

- [ ] **Step 5: Commit**

```bash
git add src/components/kanban/KanbanView.tsx src/__tests__/components/kanban/KanbanView.test.tsx
git commit -m "feat(kanban): KanbanView 接 taskQueryKanban + loading/error/empty 状态 (Phase 3)"
```

---

### Task 3.3: KanbanToolbar 组件 + project dropdown

**Files:**
- Create: `src/components/kanban/KanbanToolbar.tsx`
- Test: `src/__tests__/components/kanban/KanbanToolbar.test.tsx`
- Modify: `src/components/kanban/KanbanView.tsx` (集成 toolbar)

- [ ] **Step 1: 创建 KanbanToolbar.tsx**

```typescript
// src/components/kanban/KanbanToolbar.tsx
import { useNavigate } from "react-router-dom";
import { Plus, ArrowRight } from "lucide-react";

interface KanbanToolbarProps {
  currentProject: string | null;
  projects: string[];
  onCreateTask: () => void;
}

export function KanbanToolbar({ currentProject, projects, onCreateTask }: KanbanToolbarProps) {
  const navigate = useNavigate();

  const handleProjectChange = (value: string) => {
    if (value === "__all__") {
      navigate("/kanban");
    } else {
      navigate(`/kanban?project=${encodeURIComponent(value)}`);
    }
  };

  const handleRevealGraph = () => {
    if (currentProject) {
      navigate(`/keysight?wb=projects/${encodeURIComponent(currentProject)}`);
    }
  };

  return (
    <div className="flex items-center gap-3 border-b p-4" data-testid="kanban-toolbar">
      <select
        value={currentProject ?? "__all__"}
        onChange={(e) => handleProjectChange(e.target.value)}
        className="rounded border px-2 py-1 text-sm"
        aria-label="切换 project"
      >
        <option value="__all__">All projects</option>
        {projects.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </select>

      <button
        onClick={onCreateTask}
        className="flex items-center gap-1 rounded bg-primary px-3 py-1 text-sm text-primary-foreground"
      >
        <Plus className="size-3" />
        New task
      </button>

      {currentProject && (
        <button
          onClick={handleRevealGraph}
          className="ml-auto flex items-center gap-1 rounded border px-3 py-1 text-sm"
          data-testid="reveal-graph-button"
        >
          Reveal Graph
          <ArrowRight className="size-3" />
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: 写测试**

```typescript
// src/__tests__/components/kanban/KanbanToolbar.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MemoryRouter, Routes, Route } from "react-router-dom";
import { KanbanToolbar } from "@/components/kanban/KanbanToolbar";

const renderToolbar = (props: Partial<React.ComponentProps<typeof KanbanToolbar>> = {}) => {
  return render(
    <MemoryRouter initialEntries={["/kanban"]}>
      <Routes>
        <Route path="*" element={
          <KanbanToolbar
            currentProject={null}
            projects={["alpha", "beta"]}
            onCreateTask={vi.fn()}
            {...props}
          />
        } />
      </Routes>
    </MemoryRouter>
  );
};

describe("KanbanToolbar", () => {
  it("renders project dropdown with All projects + project list", () => {
    renderToolbar();
    expect(screen.getByLabelText(/切换 project/)).toBeInTheDocument();
    expect(screen.getByText("All projects")).toBeInTheDocument();
    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("beta")).toBeInTheDocument();
  });

  it("hides Reveal Graph when no project selected", () => {
    renderToolbar({ currentProject: null });
    expect(screen.queryByTestId("reveal-graph-button")).not.toBeInTheDocument();
  });

  it("shows Reveal Graph when project selected", () => {
    renderToolbar({ currentProject: "alpha" });
    expect(screen.getByTestId("reveal-graph-button")).toBeInTheDocument();
  });

  it("calls onCreateTask when New task clicked", () => {
    const handler = vi.fn();
    renderToolbar({ onCreateTask: handler });
    fireEvent.click(screen.getByText("New task"));
    expect(handler).toHaveBeenCalled();
  });
});
```

- [ ] **Step 3: 跑测试通过**

```bash
pnpm test -- --run KanbanToolbar
```

预期: 4 passed。

- [ ] **Step 4: 集成 toolbar 到 KanbanView**

`src/components/kanban/KanbanView.tsx` 在 query 区下方加 whiteboard list query + Toolbar 渲染。**注意**:当前 API 命名是 `commands.whiteboardList()`(不是 `listWhiteboards`),`WhiteboardSummary` 字段是 `whiteboardId`(camelCase,不是 `id`),全部走 `unwrapCommand`:

```typescript
// 在 KanbanView.tsx 顶部已 import { unwrapCommand } from "@/lib/commandResult";
// 现在加 whiteboards query:

const whiteboardsQuery = useQuery({
  queryKey: ["whiteboards"],
  queryFn: () => unwrapCommand(commands.whiteboardList()),
});

const projects = (whiteboardsQuery.data ?? [])
  .filter((wb) => wb.whiteboardId.startsWith("projects/"))
  .map((wb) => wb.whiteboardId.slice("projects/".length));
```

在 JSX return 中,把原 `<div className="border-b p-4">...</div>` 换成:

```tsx
<KanbanToolbar
  currentProject={project}
  projects={projects}
  onCreateTask={() => setCreatingStatus("inbox")}
/>
```

- [ ] **Step 5: 全测验证**

```bash
pnpm test -- --run kanban && pnpm build
```

预期: 全绿。

- [ ] **Step 6: Commit**

```bash
git add src/components/kanban/KanbanToolbar.tsx src/components/kanban/KanbanView.tsx src/__tests__/components/kanban/KanbanToolbar.test.tsx
git commit -m "feat(kanban): KanbanToolbar + project dropdown + Reveal Graph (Phase 3)"
```

---

### Task 3.4: CreateTaskModal 组件(受控 state,无 stale default)

**Files:**
- Create: `src/components/kanban/CreateTaskModal.tsx`
- Test: `src/__tests__/components/kanban/CreateTaskModal.test.tsx`

> **Stale state 防护**: 这个 modal 的 `project / status` 字段**不在 modal 内部持有 state**,改为**受控组件** — 由 parent 每次打开时传入正确的 initial 值,并通过 `onFieldChange` 回调更新。这样从 Inbox 列打开和从 Blocked 列打开不会出现"第二次打开还是上次的 status"的 stale default 问题(codex review §4)。
>
> **另一种等价做法**: 在 KanbanView 里**条件渲染** modal(`{modalState.open && <CreateTaskModal ...>}`),这样 modal 每次开启是全新实例,内部 state 自动 fresh。两种方式任选其一,plan 里用后者因为对 parent 改动最小。

- [ ] **Step 1: 创建 CreateTaskModal.tsx(内部 title state,但 project/status 是 props)**

```typescript
// src/components/kanban/CreateTaskModal.tsx
import { useState } from "react";
import type { TaskStatus } from "@/bindings";
import { COLUMN_ORDER, COLUMNS } from "./columns";

interface CreateTaskModalProps {
  // 受控字段 — parent 每次打开时传入最新值
  initialStatus: TaskStatus;
  initialProject: string | null; // 当前 view 的 project (single project mode)
  availableProjects: string[];
  onSubmit: (data: { project: string; title: string; status: TaskStatus }) => Promise<void>;
  onCancel: () => void;
}

/**
 * Modal 内部只对 title / submitting / error 持有 state。project 和 status 通过 initial* props
 * 传入,本地可修改(因为 parent 不需要同步每一个按键),但由于本组件由 parent 用
 * `{open && <CreateTaskModal />}` 条件渲染,每次打开是新实例,不会出现 stale state。
 */
export function CreateTaskModal({
  initialStatus,
  initialProject,
  availableProjects,
  onSubmit,
  onCancel,
}: CreateTaskModalProps) {
  const [project, setProject] = useState(initialProject ?? availableProjects[0] ?? "");
  const [title, setTitle] = useState("");
  const [status, setStatus] = useState<TaskStatus>(initialStatus);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 注意: 没有 `open` prop, 没有 `if (!open) return null`。
  // Parent 用 `{modalState.open && <CreateTaskModal ... />}` 条件渲染。

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !project) return;
    setSubmitting(true);
    setError(null);
    try {
      await onSubmit({ project, title: title.trim(), status });
      setTitle("");
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
      data-testid="create-task-modal"
      role="dialog"
    >
      <form
        onSubmit={handleSubmit}
        className="w-96 rounded-lg bg-card p-6 shadow-xl"
      >
        <h3 className="mb-4 text-lg font-semibold">New task</h3>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Project</div>
          <select
            value={project}
            onChange={(e) => setProject(e.target.value)}
            className="w-full rounded border px-2 py-1.5"
            required
          >
            {availableProjects.map((p) => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        </label>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Title</div>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="w-full rounded border px-2 py-1.5"
            placeholder="What needs doing?"
            autoFocus
            required
          />
        </label>

        <label className="mb-4 block">
          <div className="mb-1 text-sm font-medium">Status</div>
          <select
            value={status}
            onChange={(e) => setStatus(e.target.value as TaskStatus)}
            className="w-full rounded border px-2 py-1.5"
          >
            {COLUMN_ORDER.map((s) => (
              <option key={s} value={s}>{COLUMNS[s].label}</option>
            ))}
          </select>
        </label>

        {error && <div className="mb-3 text-sm text-destructive">{error}</div>}

        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded border px-3 py-1 text-sm"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting || !title.trim() || !project}
            className="rounded bg-primary px-3 py-1 text-sm text-primary-foreground disabled:opacity-50"
          >
            {submitting ? "Creating..." : "Create"}
          </button>
        </div>
      </form>
    </div>
  );
}
```

- [ ] **Step 2: 写测试**

```typescript
// src/__tests__/components/kanban/CreateTaskModal.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { CreateTaskModal } from "@/components/kanban/CreateTaskModal";

const renderModal = (props: Partial<React.ComponentProps<typeof CreateTaskModal>> = {}) => {
  return render(
    <CreateTaskModal
      initialStatus="inbox"
      initialProject="alpha"
      availableProjects={["alpha", "beta"]}
      onSubmit={vi.fn().mockResolvedValue(undefined)}
      onCancel={vi.fn()}
      {...props}
    />
  );
};

describe("CreateTaskModal", () => {
  it("renders form fields", () => {
    renderModal();
    expect(screen.getByText("Project")).toBeInTheDocument();
    expect(screen.getByText("Title")).toBeInTheDocument();
    expect(screen.getByText("Status")).toBeInTheDocument();
  });

  it("defaults status from initialStatus prop", () => {
    renderModal({ initialStatus: "blocked" });
    expect(screen.getByDisplayValue("Blocked")).toBeInTheDocument();
  });

  it("stale-state guard: second mount picks up new initialStatus", () => {
    // 模拟 parent 条件渲染:先用 inbox unmount,再用 blocked mount
    const { unmount } = renderModal({ initialStatus: "inbox" });
    expect(screen.getByDisplayValue("Inbox")).toBeInTheDocument();
    unmount();
    renderModal({ initialStatus: "blocked" });
    expect(screen.getByDisplayValue("Blocked")).toBeInTheDocument();
  });

  it("disables submit when title empty", () => {
    renderModal();
    expect(screen.getByText("Create")).toBeDisabled();
  });

  it("calls onSubmit with form data when submit clicked", async () => {
    const handler = vi.fn().mockResolvedValue(undefined);
    renderModal({ onSubmit: handler });

    fireEvent.change(screen.getByPlaceholderText(/What needs doing/), { target: { value: "New task" } });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith({
        project: "alpha",
        title: "New task",
        status: "inbox",
      });
    });
  });

  it("calls onCancel when Cancel clicked", () => {
    const handler = vi.fn();
    renderModal({ onCancel: handler });
    fireEvent.click(screen.getByText("Cancel"));
    expect(handler).toHaveBeenCalled();
  });

  it("shows error when onSubmit throws", async () => {
    const handler = vi.fn().mockRejectedValue(new Error("Backend exploded"));
    renderModal({ onSubmit: handler });

    fireEvent.change(screen.getByPlaceholderText(/What needs doing/), { target: { value: "Boom" } });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() => expect(screen.getByText("Backend exploded")).toBeInTheDocument());
  });
});
```

- [ ] **Step 3: 跑测试通过**

```bash
pnpm test -- --run CreateTaskModal
```

预期: 7 passed。

- [ ] **Step 4: Commit**

```bash
git add src/components/kanban/CreateTaskModal.tsx src/__tests__/components/kanban/CreateTaskModal.test.tsx
git commit -m "feat(kanban): CreateTaskModal 表单 + 校验 + 错误反馈 (Phase 3)"
```

---

### Task 3.5: 集成 CreateTaskModal 到 KanbanView (创建 task 数据流)

**Files:**
- Modify: `src/components/kanban/KanbanView.tsx`

- [ ] **Step 1: 改 KanbanView 加 modal 状态 + onSubmit handler(条件渲染防 stale state)**

```typescript
// 在 KanbanView 内部新加 state
const [modalState, setModalState] = useState<{ open: boolean; status: TaskStatus }>({
  open: false,
  status: "inbox",
});
const queryClient = useQueryClient();

const handleCreateTask = async (data: { project: string; title: string; status: TaskStatus }) => {
  // 用 unwrapCommand 统一 error 路径
  await unwrapCommand(
    commands.taskCreate(data.project, data.title, null, data.status, null, null),
  );
  invalidateAllTaskCaches(queryClient, `projects/${data.project}`);
  setModalState({ open: false, status: "inbox" });
};

// 把 setCreatingStatus 改成 setModalState
const openModal = (status: TaskStatus) => setModalState({ open: true, status });

// JSX render 用**条件渲染**创建 modal —— 每次 open 是新实例,无 stale state:
{modalState.open && (
  <CreateTaskModal
    initialStatus={modalState.status}
    initialProject={project}
    availableProjects={projects}
    onSubmit={handleCreateTask}
    onCancel={() => setModalState((s) => ({ ...s, open: false }))}
  />
)}
```

注意:
- `{modalState.open && <CreateTaskModal ... />}` 条件渲染是 stale state 防护的**关键** — 每次关了再开,组件被完全 unmount 然后重新 mount,`useState(initialStatus)` 重新初始化,不会继承上次的值
- 错误路径靠 `unwrapCommand` throw,`handleCreateTask` 让它往上传播到 `CreateTaskModal` 的 `try/catch`,由 modal 展示错误 + 保持打开状态
- Task update 失败路径也同样走 `unwrapCommand`(drag-and-drop 改 status 的 mutation 在 Task 4.3)

需要 import:

```typescript
import { useQueryClient } from "@tanstack/react-query";
import { CreateTaskModal } from "./CreateTaskModal";
import { invalidateAllTaskCaches } from "./invalidateAllTaskCaches";
```

把 `<KanbanBoard onAddTask={setCreatingStatus}>` 改成 `onAddTask={openModal}`,同样 toolbar 的 `onCreateTask={() => openModal("inbox")}`.

- [ ] **Step 2: 更新 KanbanView 测试 mock + 加 create flow 测试**

`src/__tests__/components/kanban/KanbanView.test.tsx` 加:

```typescript
it("opens modal with inbox status when New task clicked", async () => {
  vi.mocked(commands.taskQueryKanban).mockResolvedValue({ status: "ok", data: [] });
  vi.mocked(commands.whiteboardList).mockResolvedValue({ status: "ok", data: [
    { whiteboardId: "projects/alpha", cards: 0, notes: 0, sections: 0, aliases: 0, tasks: 0, questions: 0 } as any,
  ]});
  renderKanban("/kanban?project=alpha");

  await waitFor(() => screen.getByText("New task"));
  fireEvent.click(screen.getByText("New task"));

  expect(screen.getByTestId("create-task-modal")).toBeInTheDocument();
});
```

注意需要 mock `commands.whiteboardList` 和 `commands.taskCreate` 也(添加到 vi.mock 块)。

- [ ] **Step 3: 跑测试通过**

```bash
pnpm test -- --run KanbanView
```

预期: passed (新 test + 旧 tests)。

- [ ] **Step 4: Commit**

```bash
git add src/components/kanban/KanbanView.tsx src/__tests__/components/kanban/KanbanView.test.tsx
git commit -m "feat(kanban): KanbanView 集成 CreateTaskModal + create 数据流 (Phase 3)"
```

---

### Task 3.6: 反向 — Canvas GraphToolbar 加 "Show Kanban" 按钮

**Files:**
- Modify: `src/components/keysight/GraphToolbar.tsx`
- Modify: `src/__tests__/components/keysight/GraphToolbar.test.tsx`

- [ ] **Step 1: 找现有 GraphToolbar Reveal Graph / Boards 按钮代码**

```bash
grep -n "Boards\|Show\|navigate" src/components/keysight/GraphToolbar.tsx
```

- [ ] **Step 2: 在 toolbar 加 "Show Kanban" 按钮**

仅在 `currentWhiteboardId.startsWith("projects/")` 时显示:

```tsx
{currentWhiteboardId.startsWith("projects/") && (
  <button
    onClick={() => {
      const project = currentWhiteboardId.slice("projects/".length);
      navigate(`/kanban?project=${encodeURIComponent(project)}`);
    }}
    className="flex items-center gap-1 rounded border px-2 py-1 text-sm"
    data-testid="show-kanban-button"
  >
    Show Kanban
  </button>
)}
```

- [ ] **Step 3: 加测试**

```typescript
// 在现有 GraphToolbar.test.tsx 内加
it("shows 'Show Kanban' button on project whiteboard", () => {
  // render with currentWhiteboardId="projects/super-tauri"
  // expect getByTestId("show-kanban-button") to exist
});

it("hides 'Show Kanban' button on non-project whiteboard", () => {
  // render with currentWhiteboardId="wb_root"
  // expect queryByTestId("show-kanban-button") to be null
});
```

- [ ] **Step 4: 跑测试 + build**

```bash
pnpm test -- --run GraphToolbar && pnpm build
```

预期: green。

- [ ] **Step 5: Commit**

```bash
git add src/components/keysight/GraphToolbar.tsx src/__tests__/components/keysight/GraphToolbar.test.tsx
git commit -m "feat(keysight): GraphToolbar 加 Show Kanban 按钮 (project 白板专属) (Phase 3)"
```

---

### Task 3.7: 端到端 happy-path 测试 (KanbanView full flow)

**Files:**
- Modify: `src/__tests__/components/kanban/KanbanView.test.tsx`

- [ ] **Step 1: 加 full happy path 测试**

```typescript
it("end-to-end: query → render board → open modal → create task → invalidate", async () => {
  vi.mocked(commands.taskQueryKanban).mockResolvedValue({ status: "ok", data: [] });
  vi.mocked(commands.whiteboardList).mockResolvedValue({ status: "ok", data: [
    { whiteboardId: "projects/alpha", cards: 0, notes: 0, sections: 0, aliases: 0, tasks: 0, questions: 0 } as any,
  ]});
  vi.mocked(commands.taskCreate).mockResolvedValue({ status: "ok", data: {
    id: "new-1", title: "New", whiteboardId: "projects/alpha", content: "", status: "inbox", area: null, project: "alpha", color: null,
  }});

  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={["/kanban?project=alpha"]}>
        <KanbanView />
      </MemoryRouter>
    </QueryClientProvider>
  );

  await waitFor(() => screen.getByText("New task"));
  fireEvent.click(screen.getByText("New task"));
  fireEvent.change(screen.getByPlaceholderText(/What needs doing/), { target: { value: "Build kanban" } });
  fireEvent.click(screen.getByText("Create"));

  await waitFor(() => {
    expect(commands.taskCreate).toHaveBeenCalledWith("alpha", "Build kanban", null, "inbox", null, null);
  });
});
```

- [ ] **Step 2: 跑测试通过**

```bash
pnpm test -- --run KanbanView
```

- [ ] **Step 3: Commit**

```bash
git add src/__tests__/components/kanban/KanbanView.test.tsx
git commit -m "test(kanban): KanbanView 端到端 happy path 测试 (Phase 3)"
```

---

### Task 3.8: Cross-route 集成测试 (Reveal Graph 双向跳转)

**Files:**
- Modify: 现有 KanbanToolbar 测试
- Modify: 现有 GraphToolbar 测试

- [ ] **Step 1: 在 KanbanToolbar 测试加跳转断言**

```typescript
it("Reveal Graph navigates to /keysight?wb=projects/{name}", () => {
  // 用 useNavigate mock 或 MemoryRouter 检查 location
});
```

- [ ] **Step 2: 在 GraphToolbar 测试加 Show Kanban 跳转断言**

类似上面。

- [ ] **Step 3: 跑测试 + commit**

```bash
pnpm test -- --run kanban
git add src/__tests__/components/kanban/ src/__tests__/components/keysight/
git commit -m "test(kanban): cross-route 双向跳转测试 (Phase 3)"
```

---

## Phase 4 — Drag-and-Drop(共 4 task)

### Task 4.1: 引入 @dnd-kit/core

**Files:**
- Modify: `package.json`

- [ ] **Step 1: 安装 dependency**

```bash
pnpm add @dnd-kit/core
```

- [ ] **Step 2: 验证 install**

```bash
grep "@dnd-kit" package.json
```

- [ ] **Step 3: pnpm build 验证**

```bash
pnpm build
```

预期: green。

- [ ] **Step 4: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "deps: add @dnd-kit/core for kanban drag-and-drop (Phase 4)"
```

---

### Task 4.2: KanbanBoard 加 DndContext + onDragEnd handler

**Files:**
- Modify: `src/components/kanban/KanbanBoard.tsx`

- [ ] **Step 1: 写失败的测试 (mock dnd event 流程)**

```typescript
// 在 KanbanBoard.test.tsx 加
it("calls onDragEnd handler when drag ends with valid drop", () => {
  const onTaskMove = vi.fn();
  render(<KanbanBoard tasks={[]} showProjectTags={false} onAddTask={vi.fn()} onTaskMove={onTaskMove} />);
  // 触发 mock dnd-kit dragEnd event
  // 断言 onTaskMove 被调用 with (taskId, newStatus)
});
```

注: dnd-kit 测试在 RTL 下需要 mock 或用 @dnd-kit/core 提供的 test utilities。视情况简化或跳过 dnd 内部测试,只测 handler 接线。

- [ ] **Step 2: 修改 KanbanBoard 包 DndContext**

```typescript
// src/components/kanban/KanbanBoard.tsx
import { DndContext, DragEndEvent } from "@dnd-kit/core";
import type { TaskStatus } from "@/bindings";

interface KanbanBoardProps {
  tasks: TaskEntity[];
  showProjectTags: boolean;
  onAddTask: (status: TaskStatus) => void;
  onTaskMove: (taskId: string, newStatus: TaskStatus) => void;
}

export function KanbanBoard({ tasks, showProjectTags, onAddTask, onTaskMove }: KanbanBoardProps) {
  // ... 现有 grouped 逻辑 ...

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over) return;
    const taskId = active.id as string;
    const newStatus = over.id as string;
    // parse don't validate: 必须在 5 个合法 status 中
    if (!["inbox", "next", "active", "blocked", "done"].includes(newStatus)) return;
    onTaskMove(taskId, newStatus as TaskStatus);
  };

  return (
    <DndContext onDragEnd={handleDragEnd}>
      <div className="flex gap-4 overflow-x-auto p-4" data-testid="kanban-board">
        {/* 现有 columns 渲染 */}
      </div>
    </DndContext>
  );
}
```

- [ ] **Step 3: KanbanColumn 加 droppable**

```typescript
// src/components/kanban/KanbanColumn.tsx
import { useDroppable } from "@dnd-kit/core";

// 在组件内:
const { setNodeRef, isOver } = useDroppable({ id: status });

// 在外层 div 加 ref + 视觉态:
<div
  ref={setNodeRef}
  className={`flex w-72 flex-col rounded-lg p-3 ${isOver ? "bg-muted/60" : "bg-muted/30"}`}
  data-testid={`kanban-column-${status}`}
>
```

- [ ] **Step 4: KanbanCard 加 draggable**

```typescript
// src/components/kanban/KanbanCard.tsx
import { useDraggable } from "@dnd-kit/core";

// 在组件内:
const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({ id: task.id });
const style = transform ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)`, opacity: isDragging ? 0.5 : 1 } : {};

// 在外层 div:
<div
  ref={setNodeRef}
  {...listeners}
  {...attributes}
  className="..."
  style={{ ...style, /* 现有 style */ }}
>
```

- [ ] **Step 5: 全测验证**

```bash
pnpm test -- --run kanban && pnpm build
```

- [ ] **Step 6: Commit**

```bash
git add src/components/kanban/KanbanBoard.tsx src/components/kanban/KanbanColumn.tsx src/components/kanban/KanbanCard.tsx src/__tests__/components/kanban/KanbanBoard.test.tsx
git commit -m "feat(kanban): @dnd-kit DndContext + draggable card + droppable column (Phase 4)"
```

---

### Task 4.3: KanbanView 接 onTaskMove → taskUpdate mutation

**Files:**
- Modify: `src/components/kanban/KanbanView.tsx`

- [ ] **Step 1: 加 handleTaskMove handler**

```typescript
const handleTaskMove = async (taskId: string, newStatus: TaskStatus) => {
  const result = await commands.taskUpdate(taskId, null, null, newStatus, null, null);
  if (result.status === "error") {
    console.error("Task update failed:", result.error);
    // 可选: 显示 toast
    return;
  }
  invalidateAllTaskCaches(queryClient);
};

// 传给 KanbanBoard:
<KanbanBoard tasks={tasks} showProjectTags={showProjectTags} onAddTask={openModal} onTaskMove={handleTaskMove} />
```

注意: `commands.taskUpdate` 的实际签名以当前 bindings.ts 为准。

- [ ] **Step 2: 加 KanbanView 测试: drag task → mutation 调用**

```typescript
it("calls taskUpdate when onTaskMove triggered from board", async () => {
  // 设置 mock + render + 触发 onTaskMove + 断言 taskUpdate 被调用
});
```

- [ ] **Step 3: 跑测试 + build**

```bash
pnpm test -- --run KanbanView && pnpm build
```

- [ ] **Step 4: Commit**

```bash
git add src/components/kanban/KanbanView.tsx src/__tests__/components/kanban/KanbanView.test.tsx
git commit -m "feat(kanban): KanbanView 接 onTaskMove → taskUpdate 数据流 (Phase 4)"
```

---

### Task 4.4: Drag-and-drop 端到端测试 (mock dnd events)

**Files:**
- Modify: `src/__tests__/components/kanban/KanbanBoard.test.tsx`

- [ ] **Step 1: 加 mock dnd dragEnd 测试**

参考 @dnd-kit 测试文档,用 fireEvent 模拟拖拽流程,或者直接用 `DndContext` 暴露的 onDragEnd 直接调用。

```typescript
it("end-to-end drag: card from inbox → next triggers onTaskMove", () => {
  const onTaskMove = vi.fn();
  const tasks = [{ ...mockTask(), id: "t1", status: "inbox" }];
  const { container } = render(
    <KanbanBoard tasks={tasks} showProjectTags={false} onAddTask={vi.fn()} onTaskMove={onTaskMove} />
  );

  // 直接构造 dragEnd event 调 handler — 比 mock dnd-kit 完整流程简单
  const dndContext = container.querySelector("[data-testid='kanban-board']");
  // 不太好直接触发 — 实际可以拆 handler 出来单独 export 测试
});
```

如果 dnd-kit 测试复杂,把 `handleDragEnd` 从 KanbanBoard 内提取为一个独立的 `parseDragEndEvent(event, onMove)` 函数,export 后单独测试。

- [ ] **Step 2: Commit**

```bash
git add src/__tests__/components/kanban/KanbanBoard.test.tsx src/components/kanban/KanbanBoard.tsx
git commit -m "test(kanban): drag-end handler 单元测试 (Phase 4)"
```

---

## Phase 5 — Polish + 验证(共 6 task)

### Task 5.1: Loading skeleton 优化

**Files:**
- Modify: `src/components/kanban/KanbanView.tsx`

- [ ] **Step 1: 用 5 列 placeholder 替换"Loading kanban..."文本**

```typescript
if (tasksQuery.isLoading) {
  return (
    <div className="flex h-full flex-col">
      <div className="border-b p-4">Kanban (loading)</div>
      <div className="flex gap-4 p-4">
        {COLUMN_ORDER.map((status) => (
          <div key={status} className="w-72 animate-pulse rounded-lg bg-muted/30 p-3">
            <div className="mb-3 h-6 w-20 rounded bg-muted" />
            {[1, 2].map((i) => (
              <div key={i} className="mb-2 h-12 rounded bg-muted" />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: pnpm test + build**

```bash
pnpm test -- --run KanbanView && pnpm build
```

- [ ] **Step 3: Commit**

```bash
git add src/components/kanban/KanbanView.tsx
git commit -m "feat(kanban): loading skeleton 优化 (Phase 5)"
```

---

### Task 5.2: harness-check-tests 自查

- [ ] **Step 1: 跑 harness 命令**

```
/harness-check-tests
```

让 agent 检查测试覆盖缺口并补测试。

- [ ] **Step 2: 修复 agent 报告的缺口**

如有遗漏的 edge case / 错误路径未测,补上。

- [ ] **Step 3: 跑全测**

```bash
cargo test --workspace --manifest-path src-tauri/Cargo.toml && pnpm test -- --run
```

- [ ] **Step 4: Commit (如有改动)**

```bash
git add .
git commit -m "test(kanban): harness-check-tests 补缺口 (Phase 5)"
```

---

### Task 5.3: harness-type-safety-check 自查

- [ ] **Step 1: 跑 harness 命令**

```
/harness-type-safety-check
```

agent 对照 L0 防火墙清单检查类型强度。

- [ ] **Step 2: 修复 agent 报告的弱点**

如发现 stringly typed 漂移点 / 缺 newtype / match `_` 通配等,逐条修复。

- [ ] **Step 3: 全测 + clippy**

```bash
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "refactor(kanban): harness-type-safety-check 防火墙加固 (Phase 5)"
```

---

### Task 5.4: Code Review — 6 项检查

按 CLAUDE.md L0 Code Review 6 项检查通读所有 kanban 相关 commit:

- [ ] **测试覆盖**: 每个 domain 纯函数 happy + error path / 边界条件
- [ ] **逻辑正确性**: 移植/新增逻辑逐行核对
- [ ] **回归风险**: 静默破坏的场景是否有测试锁住
- [ ] **I/O 正确性**: SQLite 读写完整性
- [ ] **IPC 类型安全**: 所有 command 有 `#[specta::specta]`,bindings.ts 最新,TS 从 bindings import
- [ ] **建模强度**: 检查 §8 防火墙清单,String/bool/&str 当业务参数?invariant 是构造器还是注释?match 是否穷尽?

发现问题 inline 修复。

---

### Task 5.5: dev server 手动 walkthrough

- [ ] **Step 1: 起 dev server**

```bash
pnpm tauri dev
```

(用户在终端执行,因为 dev 是交互式)

- [ ] **Step 2: 验证 walkthrough 项**

| # | 测试场景 | 预期 |
|---|---|---|
| 1 | sidebar 看到 Kanban 项 | ✓ |
| 2 | 点 Kanban → 进入 "All projects" 视图 | ✓ |
| 3 | dropdown 切换到 super-tauri | URL 变 `?project=super-tauri`,5 列展示 |
| 4 | 创建 task title="test" status=Inbox | task 出现在 Inbox 列 |
| 5 | 再创建 task | 出现在 Inbox 第二个 |
| 6 | 拖拽 task 从 Inbox → Next | task 移列,canvas (打开时) status badge 同步 |
| 7 | kanban 创建的 task 在 canvas 出现 | 在 canvas 最底位置 |
| 8 | "Reveal Graph" 跳到 canvas | URL 变 /keysight?wb=projects/super-tauri |
| 9 | canvas "Show Kanban" 跳回 | URL 变 /kanban?project=super-tauri |
| 10 | "All projects" 模式跨项目展示 | tasks 显示 project tag |
| 11 | 列内多 task 时 dropdown 切换 | 数据正确 |

如有问题,记录 + 修复 + 重新走一遍。

- [ ] **Step 3: Commit (如有 polish 改动)**

```bash
git add .
git commit -m "polish(kanban): dev walkthrough 修复 (Phase 5)"
```

---

### Task 5.6: 收口 — progress / handoff / devlog / changelog

- [ ] **Step 1: 更新 docs/progress/backend.md**

P3 Kanban view 从 Next → Done。

```markdown
- [x] **P3 — Kanban view (V1) 完成** — commits ca18317...{最后一个 commit}
  - Phase 0: TaskStatus IPC enum + WhiteboardId newtype
  - Phase 1: TaskStatus 加 Inbox + compute_position helper + auto-position + query_kanban
  - Phase 2: 5 个新 React 组件 + sidebar nav + /kanban 路由
  - Phase 3: 数据接线 + create modal + invalidate helper + Reveal Graph 双向
  - Phase 4: @dnd-kit drag 改 status
  - Phase 5: skeleton + harness 自查 + Code Review + dev 验证
```

- [ ] **Step 2: 更新 docs/handoff/backend.md** 为 status=idle

```markdown
P3 Kanban view 已完成 + dev 验证通过。本 area 当前无 Active 任务。
下次方向: Phase B3 P2 推迟项 / 其他 backlog
```

- [ ] **Step 3: 追加 devlog**

`docs/devlog/2026-04-{date}.md` 加 Session 段记录 P3 实施全程。

- [ ] **Step 4: 追加 changelog ✅**

```bash
SLIPBOX=~/Documents/obsidian_workspace/agent-slipbox-v3 && \
echo "$(date +%H:%M) | ✅ | super-tauri P3 Kanban view V1 完成 + dev 验证 | <session_id> | <commit hashes>;5 phase 38 task TDD;Rust +20 测试 TS +30 测试;@dnd-kit drag;Co-equal Reveal Graph 双向" >> "$SLIPBOX/logs/changelog/$(date +%Y-%m-%d).md"
```

- [ ] **Step 5: Commit**

```bash
git add docs/progress/backend.md docs/handoff/backend.md docs/devlog/
git commit -m "docs(backend): P3 Kanban V1 完成 + progress/handoff/devlog 收口 (Phase 5)"
```

---

## 完成标准核对清单

整个 plan 实施完后,逐项核对:

- [ ] Phase 0~5 全部 task 完成
- [ ] Rust cargo test 全绿 (252 → 280+ passed)
- [ ] TS pnpm test 全绿 (246 → 290+ passed)
- [ ] cargo clippy --all-targets --all-features -- -D warnings green
- [ ] pnpm build green
- [ ] dev server walkthrough 11 项全通过
- [ ] CLAUDE.md 副作用矩阵更新 (task::create 加 positions)
- [ ] docs/progress/backend.md P3 Done
- [ ] docs/handoff/backend.md status=idle
- [ ] devlog + changelog ✅ 追加
- [ ] 本 plan 文件被 commit 到 git (一并 review)
- [ ] L0 防火墙检查通过 (零 stringly typed 逃生 / 零 _ 通配 / 所有新 type 用 newtype)
