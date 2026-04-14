import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { KanbanView } from "@/components/kanban/KanbanView";
import { commands } from "@/bindings";

vi.mock("@/bindings", () => ({
  commands: {
    taskQueryKanban: vi.fn(),
    whiteboardList: vi.fn(),
    taskCreate: vi.fn(),
  },
}));

function renderKanban(initialEntry: string) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <KanbanView />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("KanbanView", () => {
  beforeEach(() => {
    vi.mocked(commands.taskQueryKanban).mockResolvedValue({
      status: "ok",
      data: [],
    });
    vi.mocked(commands.whiteboardList).mockResolvedValue({
      status: "ok",
      data: [],
    });
    vi.mocked(commands.taskCreate).mockResolvedValue({
      status: "ok",
      data: {
        id: "task_test",
        title: "Created task",
        whiteboardId: "projects/alpha",
        content: "",
        status: "inbox",
        area: null,
        project: "alpha",
        color: null,
      },
    });
  });

  it("缺失 ?project= 时 subtitle 显示 All projects", async () => {
    renderKanban("/kanban");
    await waitFor(() =>
      expect(
        screen.getByText(/Kanban \(All projects\)/),
      ).toBeInTheDocument(),
    );
  });

  it("?project=super-tauri 时 subtitle 显示 project 名", async () => {
    renderKanban("/kanban?project=super-tauri");
    await waitFor(() =>
      expect(screen.getByText(/Kanban — super-tauri/)).toBeInTheDocument(),
    );
  });

  it("点击 New task 打开 modal 默认 Inbox status", async () => {
    vi.mocked(commands.whiteboardList).mockResolvedValue({
      status: "ok",
      data: [
        {
          whiteboardId: "projects/alpha",
          cards: 0,
          notes: 0,
          sections: 0,
          aliases: 0,
          tasks: 0,
          questions: 0,
        },
      ],
    });
    renderKanban("/kanban?project=alpha");

    await waitFor(() => screen.getByText("New task"));
    fireEvent.click(screen.getByText("New task"));

    expect(screen.getByTestId("create-task-modal")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Inbox")).toBeInTheDocument();
  });

  it("端到端 happy path: query → board → modal → create → taskCreate called", async () => {
    vi.mocked(commands.whiteboardList).mockResolvedValue({
      status: "ok",
      data: [
        {
          whiteboardId: "projects/alpha",
          cards: 0,
          notes: 0,
          sections: 0,
          aliases: 0,
          tasks: 0,
          questions: 0,
        },
      ],
    });

    renderKanban("/kanban?project=alpha");

    await waitFor(() => screen.getByText("New task"));
    fireEvent.click(screen.getByText("New task"));
    fireEvent.change(screen.getByPlaceholderText(/What needs doing/), {
      target: { value: "Build kanban" },
    });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() => {
      expect(commands.taskCreate).toHaveBeenCalledWith(
        "alpha",
        "Build kanban",
        null,
        "inbox",
        null,
        null,
      );
    });
  });

  it("query 失败时显示加载失败 + Retry 按钮", async () => {
    vi.mocked(commands.taskQueryKanban).mockResolvedValue({
      status: "error",
      error: { kind: "Keysight", message: "DB locked" },
    });
    renderKanban("/kanban");
    await waitFor(() =>
      expect(screen.getByText(/加载失败/)).toBeInTheDocument(),
    );
    expect(screen.getByText(/Retry/)).toBeInTheDocument();
  });
});
