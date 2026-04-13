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

  it("展开时渲染目标卡片的正文和 tags", () => {
    render(<AliasNode alias={mockAlias} targetCard={mockCard} style={{}} isExpanded />);
    expect(screen.getByText(/目标卡片的正文内容/)).toBeInTheDocument();
    expect(screen.getByText("rust")).toBeInTheDocument();
    expect(screen.getByText("trait")).toBeInTheDocument();
  });

  it("折叠状态下也显示目标卡片的 tags 和 related", () => {
    render(
      <AliasNode
        alias={mockAlias}
        targetCard={{
          ...mockCard,
          related: ["card_related"],
        }}
        cardsById={{
          card_related: {
            ...mockCard,
            id: "card_related",
            title: "Related Alias Card",
          },
        }}
        aliasRefs={[
          { aliasId: "alias_1", aliasTitle: "Trait Basics" },
          { aliasId: "alias_2", aliasTitle: "Advanced Rust" },
        ]}
        style={{}}
      />,
    );

    expect(screen.getByRole("button", { name: /rust/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /trait/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Related Alias Card/i })).toBeInTheDocument();
    expect(screen.getByText("Aliases (2)")).toBeInTheDocument();
    expect(screen.getAllByText("Trait Basics").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Advanced Rust").length).toBeGreaterThan(0);
  });

  it("委托给 CardNode，使用目标卡片的 data-entity-id", () => {
    const { container } = render(
      <AliasNode alias={mockAlias} targetCard={mockCard} style={{}} />,
    );
    // AliasNode 现在复用 CardNode variant="alias"，data-entity-id 是目标卡的 id
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("card_001");
  });

  it("目标卡片不存在时降级显示", () => {
    render(<AliasNode alias={mockAlias} targetCard={null} style={{}} />);
    expect(screen.getByText(/card_001/)).toBeInTheDocument();
    expect(screen.getByText(/不存在/)).toBeInTheDocument();
  });
});
