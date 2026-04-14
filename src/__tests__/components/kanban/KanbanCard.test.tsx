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
});
