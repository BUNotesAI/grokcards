import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { KanbanView } from "@/components/kanban/KanbanView";

describe("KanbanView", () => {
  it("缺失 ?project= 时显示 All projects 标题", () => {
    render(
      <MemoryRouter initialEntries={["/kanban"]}>
        <KanbanView />
      </MemoryRouter>,
    );
    expect(screen.getByText(/Kanban.*All projects/)).toBeInTheDocument();
  });

  it("?project=super-tauri 时显示 project 名", () => {
    render(
      <MemoryRouter initialEntries={["/kanban?project=super-tauri"]}>
        <KanbanView />
      </MemoryRouter>,
    );
    expect(screen.getByText(/Kanban — super-tauri/)).toBeInTheDocument();
  });
});
