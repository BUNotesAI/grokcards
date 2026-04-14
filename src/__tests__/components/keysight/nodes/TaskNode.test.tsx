import { render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { TaskNode } from "@/components/keysight/nodes/TaskNode";
import type { TaskEntity } from "@/bindings";
import type { TaskMenuConfig } from "@/components/keysight/nodes/NodeContextMenu";

const mockTask: TaskEntity = {
  id: "task_test0001",
  title: "【TASK】实现搜索功能",
  content: "任务描述正文",
  whiteboardId: "wb_root",
  status: "active",
  area: "backend",
  project: "keysight",
};

describe("TaskNode", () => {
  it("渲染任务标题", () => {
    render(<TaskNode task={mockTask} style={{}} />);
    expect(screen.getByText("【TASK】实现搜索功能")).toBeInTheDocument();
  });
  it("渲染 status badge", () => {
    render(<TaskNode task={mockTask} style={{}} />);
    expect(screen.getByText("active")).toBeInTheDocument();
  });
  it("渲染 area 和 project", () => {
    render(<TaskNode task={mockTask} style={{}} />);
    expect(screen.getByText("backend")).toBeInTheDocument();
    expect(screen.getByText("keysight")).toBeInTheDocument();
  });
  it("带 data-entity-id 属性", () => {
    const { container } = render(<TaskNode task={mockTask} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("task_test0001");
  });
  it("无 area/project 时不崩溃", () => {
    const noMeta = { ...mockTask, area: null, project: null };
    render(<TaskNode task={noMeta} style={{}} />);
    expect(screen.getByText("【TASK】实现搜索功能")).toBeInTheDocument();
  });

  it("未传 contextMenu 时不渲染 ⋯ 按钮", () => {
    render(<TaskNode task={mockTask} style={{}} />);
    expect(screen.queryByRole("button", { name: /open menu/i })).not.toBeInTheDocument();
  });

  it("传入 contextMenu 时渲染 ⋯ 按钮", () => {
    const taskMenu: TaskMenuConfig = {
      kind: "task",
      handlers: {
        copy_uuid_title: vi.fn(),
        move_to_section: vi.fn(),
        remove_from_group: vi.fn(),
      },
    };
    render(<TaskNode task={mockTask} style={{}} contextMenu={taskMenu} />);
    expect(screen.getByRole("button", { name: /open menu/i })).toBeInTheDocument();
  });
});
