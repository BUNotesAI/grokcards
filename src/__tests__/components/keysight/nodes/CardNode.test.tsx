import { render, screen, fireEvent } from "@testing-library/react";
import { vi } from "vitest";
import { CardNode } from "@/components/keysight/nodes/CardNode";
import type { AtomicCard } from "@/bindings";

const mockCard: AtomicCard = {
  id: "card_test0001",
  filePath: "whiteboard/test.md",
  title: "测试卡片标题",
  content: "这是卡片的正文内容，可能会很长很长需要截断显示。",
  tags: ["rust", "testing"],
  linkTo: [],
  related: [],
  understanding: "",
  source: "",
  seeAlso: [],
};

describe("CardNode", () => {
  it("渲染卡片标题", () => {
    render(<CardNode card={mockCard} style={{}} />);
    expect(screen.getByText("测试卡片标题")).toBeInTheDocument();
  });

  it("展开时渲染内容摘要", () => {
    render(<CardNode card={mockCard} style={{}} isExpanded />);
    expect(screen.getByText(/这是卡片的正文内容/)).toBeInTheDocument();
  });

  it("展开时渲染 tags 为 pill 样式", () => {
    render(<CardNode card={mockCard} style={{}} isExpanded />);
    expect(screen.getByText("rust")).toBeInTheDocument();
    expect(screen.getByText("testing")).toBeInTheDocument();
  });

  it("折叠状态下不显示 content 和 tags", () => {
    render(<CardNode card={mockCard} style={{}} />);
    expect(screen.queryByText(/这是卡片的正文内容/)).not.toBeInTheDocument();
    expect(screen.queryByText("rust")).not.toBeInTheDocument();
  });

  it("带 data-entity-id 属性", () => {
    const { container } = render(<CardNode card={mockCard} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("card_test0001");
  });

  describe("inline editing", () => {
    it("双击 title → onStartEdit('card-title') 被调用", () => {
      const onStartEdit = vi.fn();
      render(<CardNode card={mockCard} style={{}} onStartEdit={onStartEdit} />);
      fireEvent.doubleClick(screen.getByText("测试卡片标题"));
      expect(onStartEdit).toHaveBeenCalledWith("card_test0001", "card-title");
    });

    it("editingField='card-title' → 渲染 input + 默认值", () => {
      render(
        <CardNode card={mockCard} style={{}} editingField="card-title" />,
      );
      const input = screen.getByDisplayValue("测试卡片标题") as HTMLInputElement;
      expect(input.tagName).toBe("INPUT");
    });

    it("title input blur → onCommitEdit 被调用", () => {
      const onCommitEdit = vi.fn();
      render(
        <CardNode
          card={mockCard}
          style={{}}
          editingField="card-title"
          onCommitEdit={onCommitEdit}
        />,
      );
      const input = screen.getByDisplayValue("测试卡片标题") as HTMLInputElement;
      fireEvent.change(input, { target: { value: "新标题" } });
      fireEvent.blur(input);
      expect(onCommitEdit).toHaveBeenCalledWith(
        "card_test0001",
        "card-title",
        "新标题",
      );
    });

    it("title input Escape → onCancelEdit 被调用", () => {
      const onCancelEdit = vi.fn();
      render(
        <CardNode
          card={mockCard}
          style={{}}
          editingField="card-title"
          onCancelEdit={onCancelEdit}
        />,
      );
      const input = screen.getByDisplayValue("测试卡片标题") as HTMLInputElement;
      fireEvent.keyDown(input, { key: "Escape" });
      expect(onCancelEdit).toHaveBeenCalled();
    });

    it("understanding 为空时也能进入编辑（双击占位区）", () => {
      const onStartEdit = vi.fn();
      const cardWithUnderstanding = { ...mockCard, understanding: "原 understanding" };
      render(
        <CardNode
          card={cardWithUnderstanding}
          style={{}}
          onStartEdit={onStartEdit}
        />,
      );
      fireEvent.doubleClick(screen.getByText("原 understanding"));
      expect(onStartEdit).toHaveBeenCalledWith(
        "card_test0001",
        "card-understanding",
      );
    });

    it("editingField='card-understanding' → 渲染 textarea", () => {
      const cardWithUnderstanding = { ...mockCard, understanding: "原内容" };
      render(
        <CardNode
          card={cardWithUnderstanding}
          style={{}}
          editingField="card-understanding"
        />,
      );
      const textarea = screen.getByDisplayValue("原内容") as HTMLTextAreaElement;
      expect(textarea.tagName).toBe("TEXTAREA");
    });

    it("understanding textarea Cmd+Enter → 触发 commit（onBlur）", () => {
      const onCommitEdit = vi.fn();
      const cardWithUnderstanding = { ...mockCard, understanding: "old" };
      render(
        <CardNode
          card={cardWithUnderstanding}
          style={{}}
          editingField="card-understanding"
          onCommitEdit={onCommitEdit}
        />,
      );
      const textarea = screen.getByDisplayValue("old") as HTMLTextAreaElement;
      fireEvent.change(textarea, { target: { value: "new" } });
      fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });
      // Cmd+Enter triggers blur, which fires onBlur → commit
      fireEvent.blur(textarea);
      expect(onCommitEdit).toHaveBeenCalledWith(
        "card_test0001",
        "card-understanding",
        "new",
      );
    });
  });
});
