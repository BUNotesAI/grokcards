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
  });

  it("缺失 ?project= 时显示 All projects 标题", async () => {
    renderKanban("/kanban");
    await waitFor(() =>
      expect(screen.getByText(/All projects/)).toBeInTheDocument(),
    );
  });

  it("?project=super-tauri 时显示 project 名", async () => {
    renderKanban("/kanban?project=super-tauri");
    await waitFor(() =>
      expect(screen.getByText(/Kanban — super-tauri/)).toBeInTheDocument(),
    );
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
