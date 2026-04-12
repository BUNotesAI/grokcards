import { render, screen } from "@testing-library/react";
import { AliasNode } from "@/components/keysight/nodes/AliasNode";
import type { AtomicCard, CardAlias } from "@/bindings";

const mockAlias: CardAlias = {
  aliasId: "alias_test001",
  cardId: "card_001",
};

const mockCard: AtomicCard = {
  id: "card_001",
  filePath: "whiteboard/test.md",
  title: "目标卡片标题",
  content: "目标卡片的正文内容",
  tags: ["rust", "trait"],
  linkTo: [],
  related: [],
  understanding: "",
  source: "",
  seeAlso: [],
};

describe("AliasNode", () => {
  it("渲染目标卡片的标题", () => {
    render(<AliasNode alias={mockAlias} targetCard={mockCard} style={{}} />);
    expect(screen.getByText("目标卡片标题")).toBeInTheDocument();
  });

  it("渲染目标卡片的正文和 tags", () => {
    render(<AliasNode alias={mockAlias} targetCard={mockCard} style={{}} />);
    expect(screen.getByText(/目标卡片的正文内容/)).toBeInTheDocument();
    expect(screen.getByText("rust")).toBeInTheDocument();
    expect(screen.getByText("trait")).toBeInTheDocument();
  });

  it("带 data-entity-id 属性（使用 aliasId）", () => {
    const { container } = render(
      <AliasNode alias={mockAlias} targetCard={mockCard} style={{}} />,
    );
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("alias_test001");
  });

  it("目标卡片不存在时降级显示", () => {
    render(<AliasNode alias={mockAlias} targetCard={null} style={{}} />);
    expect(screen.getByText(/card_001/)).toBeInTheDocument();
    expect(screen.getByText(/不存在/)).toBeInTheDocument();
  });
});
