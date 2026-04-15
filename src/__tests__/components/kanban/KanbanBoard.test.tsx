import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { KanbanBoard, parseDragEnd } from "@/components/kanban/KanbanBoard";
import type { TaskEntity } from "@/bindings";

const tasks: TaskEntity[] = [
  {
    id: "1",
    title: "T1",
    whiteboardId: "projects/a",
    content: "",
    status: "inbox",
    area: null,
    project: "a",
    color: null,
  },
  {
    id: "2",
    title: "T2",
    whiteboardId: "projects/a",
    content: "",
    status: "next",
    area: null,
    project: "a",
    color: null,
  },
  {
    id: "3",
    title: "T3",
    whiteboardId: "projects/a",
    content: "",
    status: "next",
    area: null,
    project: "a",
    color: null,
  },
];

describe("KanbanBoard", () => {
  it("按固定顺序渲染 5 列", () => {
    render(
      <KanbanBoard
        tasks={[]}
        showProjectTags={false}
        onAddTask={vi.fn()}
        onTaskMove={vi.fn()}
      />,
    );
    const board = screen.getByTestId("kanban-board");
    const columns = board.querySelectorAll(
      '[data-testid^="kanban-column-"]',
    );
    expect(columns.length).toBe(5);
    expect(columns[0].getAttribute("data-testid")).toBe(
      "kanban-column-inbox",
    );
    expect(columns[4].getAttribute("data-testid")).toBe("kanban-column-done");
  });

  it("按 status 对 tasks 分组渲染", () => {
    render(
      <KanbanBoard
        tasks={tasks}
        showProjectTags={false}
        onAddTask={vi.fn()}
        onTaskMove={vi.fn()}
      />,
    );
    expect(screen.getByText("T1")).toBeInTheDocument();
    expect(screen.getByText("T2")).toBeInTheDocument();
    expect(screen.getByText("T3")).toBeInTheDocument();
  });

  it("showProjectTags 传到 card 显示 project 标签", () => {
    render(
      <KanbanBoard
        tasks={tasks}
        showProjectTags={true}
        onAddTask={vi.fn()}
        onTaskMove={vi.fn()}
      />,
    );
    const projectTags = screen.getAllByText("a");
    expect(projectTags.length).toBe(3);
  });

  // V1.1 Phase 6.6: 双击 card 触发 onTaskDoubleClick

  it("双击卡片触发 onTaskDoubleClick with 对应 task", () => {
    const onTaskDoubleClick = vi.fn();
    render(
      <KanbanBoard
        tasks={tasks}
        showProjectTags={false}
        onAddTask={vi.fn()}
        onTaskMove={vi.fn()}
        onTaskDoubleClick={onTaskDoubleClick}
      />,
    );
    const card = screen.getByTestId("kanban-card-1");
    fireEvent.doubleClick(card);
    expect(onTaskDoubleClick).toHaveBeenCalledTimes(1);
    expect(onTaskDoubleClick).toHaveBeenCalledWith(tasks[0]);
  });

  it("drag 手势(pointer move > 8px)不触发 onTaskDoubleClick", () => {
    // codex review 要求的冲突防护测试:PointerSensor 的 activationConstraint
    // 应该让 drag 手势只走 dnd-kit 通道,不额外触发 React onDoubleClick。
    // 单一 pointer down/move/up 序列本身也不会产生 double click(需要两次 click
    // 事件才行),但这个测试是 V1.1 决策留的回归锚点 —— 未来移除 activationConstraint
    // 时必须有人意识到双击可能被破坏。
    const onTaskDoubleClick = vi.fn();
    render(
      <KanbanBoard
        tasks={tasks}
        showProjectTags={false}
        onAddTask={vi.fn()}
        onTaskMove={vi.fn()}
        onTaskDoubleClick={onTaskDoubleClick}
      />,
    );
    const card = screen.getByTestId("kanban-card-1");
    // 模拟 drag: pointerDown → pointerMove(10px) → pointerUp
    fireEvent.pointerDown(card, { clientX: 0, clientY: 0 });
    fireEvent.pointerMove(card, { clientX: 10, clientY: 0 });
    fireEvent.pointerUp(card, { clientX: 10, clientY: 0 });
    expect(onTaskDoubleClick).not.toHaveBeenCalled();
  });
});

describe("parseDragEnd", () => {
  function makeEvent(activeId: string, overId: string | null) {
    return {
      active: { id: activeId, data: { current: undefined }, rect: { current: {} } },
      over: overId
        ? { id: overId, data: { current: undefined }, rect: {}, disabled: false }
        : null,
      delta: { x: 0, y: 0 },
      collisions: null,
      activatorEvent: new Event("pointerdown"),
    } as unknown as import("@dnd-kit/core").DragEndEvent;
  }

  it("over=null 时返回 null", () => {
    expect(parseDragEnd(makeEvent("t1", null))).toBeNull();
  });

  it("over.id 是合法 TaskStatus 时返回 (taskId, newStatus)", () => {
    expect(parseDragEnd(makeEvent("t1", "next"))).toEqual({
      taskId: "t1",
      newStatus: "next",
    });
    expect(parseDragEnd(makeEvent("task_42", "blocked"))).toEqual({
      taskId: "task_42",
      newStatus: "blocked",
    });
  });

  it("over.id 不是 5 个合法 status 之一时返回 null", () => {
    expect(parseDragEnd(makeEvent("t1", "trash"))).toBeNull();
    expect(parseDragEnd(makeEvent("t1", "inbox_2"))).toBeNull();
    expect(parseDragEnd(makeEvent("t1", ""))).toBeNull();
  });

  it("覆盖全部 5 个合法 status", () => {
    const statuses = ["inbox", "next", "active", "blocked", "done"] as const;
    for (const s of statuses) {
      const parsed = parseDragEnd(makeEvent("t1", s));
      expect(parsed?.newStatus).toBe(s);
    }
  });
});
