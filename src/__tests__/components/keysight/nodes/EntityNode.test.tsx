import { render, screen } from "@testing-library/react";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import type { EntityWithPosition } from "@/components/keysight/types";

describe("EntityNode", () => {
  it("kind=card 时渲染 CardNode", () => {
    const entity: EntityWithPosition = {
      kind: "card",
      id: "card_001",
      entity: {
        id: "card_001",
        title: "Test Card",
        content: "body",
        filePath: "",
        tags: [],
        linkTo: [],
        related: [],
        understanding: "",
        source: "",
        seeAlso: [],
      },
      position: { x: 0, y: 0 },
    };
    render(<EntityNode entity={entity} allPositions={{}} />);
    expect(screen.getByText("Test Card")).toBeInTheDocument();
  });
  it("kind=note 时渲染 NoteNode", () => {
    const entity: EntityWithPosition = {
      kind: "note",
      id: "note_001",
      entity: { id: "note_001", title: "Test Note", content: "note body" },
      position: { x: 0, y: 0 },
    };
    render(<EntityNode entity={entity} allPositions={{}} />);
    expect(screen.getByText("Test Note")).toBeInTheDocument();
  });
  it("kind=section 时渲染 SectionNode", () => {
    const entity: EntityWithPosition = {
      kind: "section",
      id: "sec_001",
      entity: { id: "sec_001", title: "Test Section", cardIds: [] },
      position: { x: 0, y: 0 },
    };
    render(<EntityNode entity={entity} allPositions={{}} />);
    expect(screen.getByText("Test Section")).toBeInTheDocument();
  });
  it("绝对定位到 position 坐标", () => {
    const entity: EntityWithPosition = {
      kind: "card",
      id: "card_002",
      entity: {
        id: "card_002",
        title: "Positioned",
        content: "",
        filePath: "",
        tags: [],
        linkTo: [],
        related: [],
        understanding: "",
        source: "",
        seeAlso: [],
      },
      position: { x: 150, y: 250 },
    };
    const { container } = render(<EntityNode entity={entity} allPositions={{}} />);
    const wrapper = container.firstElementChild as HTMLElement;
    expect(wrapper.style.left).toBe("150px");
    expect(wrapper.style.top).toBe("250px");
  });
});
