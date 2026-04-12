import { render, screen } from "@testing-library/react";
import { SectionNode } from "@/components/keysight/nodes/SectionNode";
import type { EntityKind } from "@/components/keysight/types";
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

const memberKinds: Record<string, EntityKind> = {
  card_001: "card",
  card_002: "card",
};

describe("SectionNode", () => {
  it("渲染 section 标题", () => {
    render(
      <SectionNode
        section={mockSection}
        memberPositions={memberPositions}
        memberKinds={memberKinds}
        style={{}}
      />,
    );
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });
  it("带 data-entity-id 属性", () => {
    const { container } = render(
      <SectionNode
        section={mockSection}
        memberPositions={memberPositions}
        memberKinds={memberKinds}
        style={{}}
      />,
    );
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("sec_test0001");
  });
  it("无成员位置时仍能渲染（使用最小尺寸）", () => {
    render(
      <SectionNode
        section={mockSection}
        memberPositions={{}}
        memberKinds={{}}
        style={{}}
      />,
    );
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });
  it("无 color 时使用默认色", () => {
    const noColor: GraphSection = { ...mockSection, color: undefined };
    render(
      <SectionNode
        section={noColor}
        memberPositions={memberPositions}
        memberKinds={memberKinds}
        style={{}}
      />,
    );
    expect(screen.getByText("核心概念")).toBeInTheDocument();
  });
  it("note member 的 section bounds 用 note 尺寸而非 card 尺寸", () => {
    const noteSection: GraphSection = {
      id: "sec_with_notes",
      title: "INBOX",
      cardIds: ["note_a", "note_b", "note_c"],
      color: "default",
    };
    const notePositions: Record<string, Position> = {
      note_a: { x: -804, y: -742 },
      note_b: { x: -842, y: -578 },
      note_c: { x: -802, y: -473 },
    };
    const noteKinds: Record<string, EntityKind> = {
      note_a: "note",
      note_b: "note",
      note_c: "note",
    };
    const { container } = render(
      <SectionNode
        section={noteSection}
        memberPositions={notePositions}
        memberKinds={noteKinds}
        style={{}}
      />,
    );
    const el = container.firstElementChild as HTMLElement;
    // ENTITY_DIMENSIONS.note = {width: 520, height: 180}
    // minX=-842, maxXRight=-802+520=-282 → width = -282 - (-842) + 80 = 640
    // minY=-742, maxYBottom=-473+180=-293 → height = -293 - (-742) + 80 = 529
    expect(el.style.width).toBe("640px");
    expect(el.style.height).toBe("529px");
  });
});
