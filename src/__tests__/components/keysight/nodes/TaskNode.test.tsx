import { render, screen, fireEvent } from "@testing-library/react";
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
  color: null,
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

  it("有 color 时使用该背景色", () => {
    const colored = { ...mockTask, color: "#caffbf" };
    const { container } = render(<TaskNode task={colored} style={{}} />);
    const node = container.firstElementChild as HTMLElement;
    expect(node.style.backgroundColor).toBe("rgb(202, 255, 191)");
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
        edit_title: vi.fn(),
        move_to_section: vi.fn(),
        remove_from_group: vi.fn(),
        set_color: vi.fn(),
        delete: vi.fn(),
      },
    };
    render(<TaskNode task={mockTask} style={{}} contextMenu={taskMenu} />);
    expect(screen.getByRole("button", { name: /open menu/i })).toBeInTheDocument();
  });

  it("双击 title -> onStartEdit('task-title')", () => {
    const onStartEdit = vi.fn();
    render(<TaskNode task={mockTask} style={{}} onStartEdit={onStartEdit} />);
    fireEvent.doubleClick(screen.getByText("【TASK】实现搜索功能"));
    expect(onStartEdit).toHaveBeenCalledWith("task_test0001", "task-title");
  });

  it("editingField='task-title' -> 渲染 input 显示 draft title", () => {
    render(
      <TaskNode task={mockTask} style={{}} editingField="task-title" />,
    );
    const input = screen.getByDisplayValue("【TASK】实现搜索功能");
    expect(input.tagName).toBe("INPUT");
  });

  // V1.1 Phase 6.6 follow-up: canvas 里结构化渲染 subtasks 而不是折叠的 raw body

  it("有 subtasks 时渲染结构化 checklist 列表,每项带 disabled checkbox + text", () => {
    const taskWithSubtasks: TaskEntity = {
      ...mockTask,
      content: "- [ ] item 02\n- [x] 蛤蛤",
      subtasks: [
        { text: "item 02", done: false },
        { text: "蛤蛤", done: true },
      ],
    };
    render(<TaskNode task={taskWithSubtasks} style={{}} />);
    const ul = screen.getByTestId(`task-node-subtasks-${taskWithSubtasks.id}`);
    expect(ul).toBeInTheDocument();
    // 两项都显示
    expect(screen.getByText("item 02")).toBeInTheDocument();
    expect(screen.getByText("蛤蛤")).toBeInTheDocument();
    // checkbox 状态正确且 disabled(canvas 只读)
    const checkboxes = ul.querySelectorAll(
      'input[type="checkbox"]',
    ) as NodeListOf<HTMLInputElement>;
    expect(checkboxes.length).toBe(2);
    expect(checkboxes[0].checked).toBe(false);
    expect(checkboxes[0].disabled).toBe(true);
    expect(checkboxes[1].checked).toBe(true);
    expect(checkboxes[1].disabled).toBe(true);
    // 原始 raw body 不被作为 <p> 渲染(避免 "- [ ] item 02" 重复显示为一行)
    expect(screen.queryByText(/- \[ \] item 02/)).not.toBeInTheDocument();
  });

  it("subtasks 超过 6 个时显示 '+N more' 溢出标记", () => {
    const manySubtasks: TaskEntity = {
      ...mockTask,
      subtasks: Array.from({ length: 10 }, (_, i) => ({
        text: `sub ${i}`,
        done: false,
      })),
    };
    render(<TaskNode task={manySubtasks} style={{}} />);
    // 前 6 个显示
    expect(screen.getByText("sub 0")).toBeInTheDocument();
    expect(screen.getByText("sub 5")).toBeInTheDocument();
    // 第 7 个不显示
    expect(screen.queryByText("sub 6")).not.toBeInTheDocument();
    // 溢出标记
    expect(screen.getByText("+ 4 more…")).toBeInTheDocument();
  });

  it("无 subtasks 时仍渲染 content 为保留空白的 <p>(fallback 路径)", () => {
    const noSubtasks: TaskEntity = {
      ...mockTask,
      content: "line 1\nline 2",
    };
    const { container } = render(<TaskNode task={noSubtasks} style={{}} />);
    // whitespace-pre-wrap 类生效(保留 \n,不折叠为空格)
    const p = container.querySelector("p.whitespace-pre-wrap");
    expect(p).not.toBeNull();
    expect(p?.textContent).toBe("line 1\nline 2");
  });
});
