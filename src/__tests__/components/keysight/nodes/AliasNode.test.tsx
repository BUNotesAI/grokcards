import { render, screen } from "@testing-library/react";
import { AliasNode } from "@/components/keysight/nodes/AliasNode";
import type { CardAlias } from "@/bindings";

const mockAlias: CardAlias = {
  aliasId: "alias_test001",
  cardId: "card_001",
};

describe("AliasNode", () => {
  it("渲染原卡片标题", () => {
    render(<AliasNode alias={mockAlias} originalTitle="原始卡片标题" style={{}} />);
    expect(screen.getByText("原始卡片标题")).toBeInTheDocument();
  });
  it("带 ghost/translucent 样式标记", () => {
    const { container } = render(
      <AliasNode alias={mockAlias} originalTitle="标题" style={{}} />,
    );
    const node = container.firstElementChild as HTMLElement;
    expect(node.className).toContain("opacity");
  });
  it("带 data-entity-id 属性（使用 aliasId）", () => {
    const { container } = render(
      <AliasNode alias={mockAlias} originalTitle="标题" style={{}} />,
    );
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("alias_test001");
  });
  it("显示 alias 标识", () => {
    render(<AliasNode alias={mockAlias} originalTitle="标题" style={{}} />);
    expect(screen.getByText(/alias/i)).toBeInTheDocument();
  });
});
