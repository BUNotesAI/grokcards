import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { KanbanColumn } from "@/components/kanban/KanbanColumn";
import type { TaskEntity } from "@/bindings";

function mockTask(over: Partial<TaskEntity> = {}): TaskEntity {
  return {
    id: "t1",
    title: "Test Task",
    whiteboardId: "projects/test",
    content: "",
    status: "inbox",
    area: null,
    project: "test",
    color: null,
    ...over,
  };
}

describe("KanbanColumn", () => {
  it("显示 COLUMNS config 里的 label", () => {
    render(<KanbanColumn status="inbox" tasks={[]} onAddTask={vi.fn()} />);
    expect(screen.getByText("Inbox")).toBeInTheDocument();
  });

  it("显示 task 数量", () => {
    render(
      <KanbanColumn
        status="next"
        tasks={[mockTask(), mockTask({ id: "t2" })]}
        onAddTask={vi.fn()}
      />,
    );
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("空列显示占位文案", () => {
    render(<KanbanColumn status="active" tasks={[]} onAddTask={vi.fn()} />);
    expect(screen.getByText(/还没有 task/)).toBeInTheDocument();
  });

  it("点 + 按钮调 onAddTask 并传入当前列的 status", () => {
    const handler = vi.fn();
    render(
      <KanbanColumn status="blocked" tasks={[]} onAddTask={handler} />,
    );
    fireEvent.click(screen.getByLabelText(/新建 Blocked task/));
    expect(handler).toHaveBeenCalledWith("blocked");
  });
});
