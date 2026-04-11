import { render, screen } from "@testing-library/react";
import { TaskNode } from "@/components/keysight/nodes/TaskNode";
import type { TaskEntity } from "@/bindings";

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
});
