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

  // V1.1 Phase 6.6 follow-up: 单击 card 触发 onTaskClick(用户反馈从双击改单击)

  it("单击卡片触发 onTaskClick with 对应 task", () => {
    const onTaskClick = vi.fn();
    render(
      <KanbanBoard
        tasks={tasks}
        showProjectTags={false}
        onAddTask={vi.fn()}
        onTaskMove={vi.fn()}
        onTaskClick={onTaskClick}
      />,
    );
    const card = screen.getByTestId("kanban-card-1");
    fireEvent.click(card);
    expect(onTaskClick).toHaveBeenCalledTimes(1);
    expect(onTaskClick).toHaveBeenCalledWith(tasks[0]);
  });

  it("drag 手势(pointer move > 8px)不触发 onTaskClick", () => {
    // codex review 要求的冲突防护测试:PointerSensor 的 activationConstraint
    // 应该让 drag 手势只走 dnd-kit 通道,不额外触发 React onClick。
    //
    // 浏览器默认行为:pointer down → move → up 如果 up 时 hit test 发现移动
    // 发生,不会 fire click 事件 —— 天然互斥。本测试是 V1.1 决策的回归锚点,
    // 未来移除 activationConstraint 或改 sensor 时必须有人意识到 click 可能被破坏。
    const onTaskClick = vi.fn();
    render(
      <KanbanBoard
        tasks={tasks}
        showProjectTags={false}
        onAddTask={vi.fn()}
        onTaskMove={vi.fn()}
        onTaskClick={onTaskClick}
      />,
    );
    const card = screen.getByTestId("kanban-card-1");
    // 模拟 drag: pointerDown → pointerMove(10px) → pointerUp
    // 注意:jsdom 不会自动 fire click(真实浏览器发现移动也不 fire click)
    fireEvent.pointerDown(card, { clientX: 0, clientY: 0 });
    fireEvent.pointerMove(card, { clientX: 10, clientY: 0 });
    fireEvent.pointerUp(card, { clientX: 10, clientY: 0 });
    expect(onTaskClick).not.toHaveBeenCalled();
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
