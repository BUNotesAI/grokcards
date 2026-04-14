import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { KanbanBoard } from "@/components/kanban/KanbanBoard";
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
      <KanbanBoard tasks={[]} showProjectTags={false} onAddTask={vi.fn()} />,
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
      />,
    );
    const projectTags = screen.getAllByText("a");
    expect(projectTags.length).toBe(3);
  });
});
