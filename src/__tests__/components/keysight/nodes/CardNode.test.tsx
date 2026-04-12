import { render, screen } from "@testing-library/react";
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
});
