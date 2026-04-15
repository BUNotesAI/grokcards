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
    taskUpdate: vi.fn(),
    taskUpdateWithSubtasks: vi.fn(),
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

  it("query 挂起时渲染 5 列 skeleton", () => {
    // 用不 resolve 的 Promise 模拟 query 永远 pending
    vi.mocked(commands.taskQueryKanban).mockReturnValue(
      new Promise(() => {}),
    );
    vi.mocked(commands.whiteboardList).mockReturnValue(
      new Promise(() => {}),
    );
    renderKanban("/kanban");
    expect(
      screen.getByTestId("kanban-column-inbox-skeleton"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("kanban-column-next-skeleton"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("kanban-column-active-skeleton"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("kanban-column-blocked-skeleton"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("kanban-column-done-skeleton"),
    ).toBeInTheDocument();
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

  // V1.1 Phase 6.6 follow-up: 单击端到端 → TaskEditModal → submit → taskUpdateWithSubtasks

  it("单击 kanban card 打开 TaskEditModal,submit 调 taskUpdateWithSubtasks", async () => {
    const existingTask = {
      id: "task_abc",
      title: "Existing task",
      whiteboardId: "projects/alpha",
      content: "- [ ] a\n- [x] b",
      status: "next" as const,
      area: "backend",
      project: "alpha",
      color: null,
      subtasks: [
        { text: "a", done: false },
        { text: "b", done: true },
      ],
    };
    vi.mocked(commands.taskQueryKanban).mockResolvedValue({
      status: "ok",
      data: [existingTask],
    });
    vi.mocked(commands.whiteboardList).mockResolvedValue({
      status: "ok",
      data: [
        {
          whiteboardId: "projects/alpha",
          cards: 0,
          notes: 0,
          sections: 0,
          aliases: 0,
          tasks: 1,
          questions: 0,
        },
      ],
    });
    vi.mocked(commands.taskUpdateWithSubtasks).mockResolvedValue({
      status: "ok",
      data: { ...existingTask, title: "New title" },
    });

    renderKanban("/kanban?project=alpha");

    // 等 query 完成 → card 渲染
    await waitFor(() => screen.getByTestId("kanban-card-task_abc"));

    // 单击打开 modal
    fireEvent.click(screen.getByTestId("kanban-card-task_abc"));
    expect(screen.getByTestId("task-edit-modal")).toBeInTheDocument();

    // 改 title 并 submit
    const titleInput = screen.getByDisplayValue("Existing task");
    fireEvent.change(titleInput, { target: { value: "New title" } });
    const form = screen.getByTestId("task-edit-modal").querySelector("form")!;
    fireEvent.submit(form);

    await waitFor(() => {
      expect(commands.taskUpdateWithSubtasks).toHaveBeenCalledTimes(1);
    });
    const call = vi.mocked(commands.taskUpdateWithSubtasks).mock.calls[0];
    expect(call[0]).toBe("task_abc");
    expect(call[1]).toBe("New title");
    // subtasks 无 uiKey
    expect(call[2]).toEqual([
      { text: "a", done: false },
      { text: "b", done: true },
    ]);
    expect(call[3]).toBe("next");
    expect(call[4]).toBe("backend");
    expect(call[5]).toBe(null);

    // submit 成功后 modal 应关闭
    await waitFor(() => {
      expect(screen.queryByTestId("task-edit-modal")).not.toBeInTheDocument();
    });
  });
});
