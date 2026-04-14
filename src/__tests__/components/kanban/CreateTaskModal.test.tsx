import type { ComponentProps } from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { CreateTaskModal } from "@/components/kanban/CreateTaskModal";

function renderModal(
  overrides: Partial<ComponentProps<typeof CreateTaskModal>> = {},
) {
  return render(
    <CreateTaskModal
      initialStatus="inbox"
      initialProject="alpha"
      availableProjects={["alpha", "beta"]}
      onSubmit={vi.fn().mockResolvedValue(undefined)}
      onCancel={vi.fn()}
      {...overrides}
    />,
  );
}

describe("CreateTaskModal", () => {
  it("渲染 Project / Title / Status 三个字段", () => {
    renderModal();
    expect(screen.getByText("Project")).toBeInTheDocument();
    expect(screen.getByText("Title")).toBeInTheDocument();
    expect(screen.getByText("Status")).toBeInTheDocument();
  });

  it("status 默认值来自 initialStatus prop", () => {
    renderModal({ initialStatus: "blocked" });
    expect(screen.getByDisplayValue("Blocked")).toBeInTheDocument();
  });

  it("stale-state 防护:第二次 mount 读新的 initialStatus", () => {
    // 模拟 parent 条件渲染:先 inbox unmount,再 blocked mount
    const first = renderModal({ initialStatus: "inbox" });
    expect(screen.getByDisplayValue("Inbox")).toBeInTheDocument();
    first.unmount();

    renderModal({ initialStatus: "blocked" });
    expect(screen.getByDisplayValue("Blocked")).toBeInTheDocument();
  });

  it("title 为空时 submit 按钮 disabled", () => {
    renderModal();
    expect(screen.getByText("Create")).toBeDisabled();
  });

  it("submit 带表单数据调 onSubmit", async () => {
    const handler = vi.fn().mockResolvedValue(undefined);
    renderModal({ onSubmit: handler });

    fireEvent.change(screen.getByPlaceholderText(/What needs doing/), {
      target: { value: "New task" },
    });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() => {
      expect(handler).toHaveBeenCalledWith({
        project: "alpha",
        title: "New task",
        status: "inbox",
      });
    });
  });

  it("点 Cancel 调 onCancel", () => {
    const handler = vi.fn();
    renderModal({ onCancel: handler });
    fireEvent.click(screen.getByText("Cancel"));
    expect(handler).toHaveBeenCalled();
  });

  it("onSubmit throw 时显示 error 文案", async () => {
    const handler = vi.fn().mockRejectedValue(new Error("Backend exploded"));
    renderModal({ onSubmit: handler });

    fireEvent.change(screen.getByPlaceholderText(/What needs doing/), {
      target: { value: "Boom" },
    });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() =>
      expect(screen.getByText("Backend exploded")).toBeInTheDocument(),
    );
  });
});
