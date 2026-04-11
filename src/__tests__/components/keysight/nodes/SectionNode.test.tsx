import { render, screen } from "@testing-library/react";
import { SectionNode } from "@/components/keysight/nodes/SectionNode";
import type { GraphSection, Position } from "@/bindings";

const mockSection: GraphSection = {
  id: "sec_test0001",
  title: "核心概念",
  cardIds: ["card_001", "card_002"],
  color: "blue",
};

const memberPositions: Record<string, Position> = {
  card_001: { x: 100, y: 100 },
  card_002: { x: 400, y: 300 },
};

describe("SectionNode", () => {
  it("渲染 section 标题", () => {
    render(<SectionNode section={mockSection} memberPositions={memberPositions} style={{}} />);
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });
  it("带 data-entity-id 属性", () => {
    const { container } = render(
      <SectionNode section={mockSection} memberPositions={memberPositions} style={{}} />,
    );
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("sec_test0001");
  });
  it("无成员位置时仍能渲染（使用最小尺寸）", () => {
    render(<SectionNode section={mockSection} memberPositions={{}} style={{}} />);
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });
  it("无 color 时使用默认色", () => {
    const noColor: GraphSection = { ...mockSection, color: undefined };
    render(<SectionNode section={noColor} memberPositions={memberPositions} style={{}} />);
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });
});
