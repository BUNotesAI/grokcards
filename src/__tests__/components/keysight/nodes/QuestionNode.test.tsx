import { render, screen, fireEvent } from "@testing-library/react";
import { vi } from "vitest";
import { QuestionNode } from "@/components/keysight/nodes/QuestionNode";
import type { QuestionMenuConfig } from "@/components/keysight/nodes/NodeContextMenu";
import type { QuestionEntity } from "@/bindings";

const mockQuestion: QuestionEntity = {
  id: "q_test00001",
  title: "【QUE】为什么需要 FTS5",
  content: "问题详细描述",
  whiteboardId: "wb_root",
  status: "pending",
};

/** 构造 question 菜单 config — 各回调都是 vi.fn */
function makeQuestionMenu(): QuestionMenuConfig {
  return {
    kind: "question",
    onCopyUuidTitle: vi.fn(),
    onDrawConnection: vi.fn(),
    onEditTitle: vi.fn(),
    onMoveToSection: vi.fn(),
    onRemoveFromGroup: vi.fn(),
    onDelete: vi.fn(),
  };
}

describe("QuestionNode", () => {
  it("渲染问题标题", () => {
    render(<QuestionNode question={mockQuestion} style={{}} />);
    expect(screen.getByText("【QUE】为什么需要 FTS5")).toBeInTheDocument();
  });
  it("渲染 status badge", () => {
    render(<QuestionNode question={mockQuestion} style={{}} />);
    expect(screen.getByText("pending")).toBeInTheDocument();
  });
  it("带 data-entity-id 属性", () => {
    const { container } = render(<QuestionNode question={mockQuestion} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("q_test00001");
  });
  it("不同 status 显示不同样式", () => {
    const doing = { ...mockQuestion, status: "doing" };
    render(<QuestionNode question={doing} style={{}} />);
    expect(screen.getByText("doing")).toBeInTheDocument();
  });

  it("双击 title -> onStartEdit('question-title')", () => {
    const onStartEdit = vi.fn();
    render(<QuestionNode question={mockQuestion} style={{}} onStartEdit={onStartEdit} />);
    fireEvent.doubleClick(screen.getByText("【QUE】为什么需要 FTS5"));
    expect(onStartEdit).toHaveBeenCalledWith("q_test00001", "question-title");
  });

  it("editingField='question-body' -> 渲染 textarea", () => {
    render(<QuestionNode question={mockQuestion} style={{}} editingField="question-body" />);
    expect(screen.getByDisplayValue("问题详细描述").tagName).toBe("TEXTAREA");
  });

  it("传入 contextMenu prop → 渲染 ⋯ 按钮（aria-label='Open menu'）", () => {
    const menu = makeQuestionMenu();
    render(<QuestionNode question={mockQuestion} style={{}} contextMenu={menu} />);
    expect(screen.getByRole("button", { name: /open menu/i })).toBeInTheDocument();
  });

  it("不传 contextMenu 时不渲染 ⋯ 按钮", () => {
    render(<QuestionNode question={mockQuestion} style={{}} />);
    expect(screen.queryByRole("button", { name: /open menu/i })).not.toBeInTheDocument();
  });
});
