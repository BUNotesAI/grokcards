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
  it("渲染 task title", () => {
    render(<KanbanCard task={baseTask} />);
    expect(screen.getByText("Build feature")).toBeInTheDocument();
  });

  it("默认不显示 project tag", () => {
    render(<KanbanCard task={baseTask} />);
    expect(screen.queryByText("super-tauri")).not.toBeInTheDocument();
  });

  it("showProjectTag=true 时显示 project 标签", () => {
    render(<KanbanCard task={baseTask} showProjectTag />);
    expect(screen.getByText("super-tauri")).toBeInTheDocument();
  });

  it("task.color 驱动左侧边框色", () => {
    render(<KanbanCard task={{ ...baseTask, color: "#ff0000" }} />);
    const card = screen.getByTestId("kanban-card-t1");
    expect(card.style.borderLeftColor).toBe("rgb(255, 0, 0)");
  });

  // V1.1 Phase 6.4 progress 徽章

  it("subtasks 为空时不显示 progress 徽章", () => {
    render(<KanbanCard task={baseTask} />);
    expect(screen.queryByTestId("kanban-card-progress-t1")).not.toBeInTheDocument();
  });

  it("subtasks 非空时显示 done/total 进度", () => {
    const task = {
      ...baseTask,
      subtasks: [
        { text: "a", done: true },
        { text: "b", done: false },
        { text: "c", done: false },
      ],
    };
    render(<KanbanCard task={task} />);
    const badge = screen.getByTestId("kanban-card-progress-t1");
    expect(badge).toHaveTextContent("1/3");
    expect(badge).toHaveAttribute("aria-label", "1 of 3 subtasks done");
  });

  it("所有 subtask 都 done 时显示全满进度", () => {
    const task = {
      ...baseTask,
      subtasks: [
        { text: "a", done: true },
        { text: "b", done: true },
      ],
    };
    render(<KanbanCard task={task} />);
    expect(screen.getByTestId("kanban-card-progress-t1")).toHaveTextContent("2/2");
  });
});
