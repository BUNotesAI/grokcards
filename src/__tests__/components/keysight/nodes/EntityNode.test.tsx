import { fireEvent, render, screen } from "@testing-library/react";
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
    render(<EntityNode entity={entity} allPositions={{}} allKinds={{}} />);
    expect(screen.getByText("Test Card")).toBeInTheDocument();
  });
  it("kind=note 时渲染 NoteNode", () => {
    const entity: EntityWithPosition = {
      kind: "note",
      id: "note_001",
      entity: { id: "note_001", title: "Test Note", content: "note body" },
      position: { x: 0, y: 0 },
    };
    render(<EntityNode entity={entity} allPositions={{}} allKinds={{}} />);
    expect(screen.getByText("Test Note")).toBeInTheDocument();
  });
  it("kind=section 时渲染 SectionNode", () => {
    const entity: EntityWithPosition = {
      kind: "section",
      id: "sec_001",
      entity: { id: "sec_001", title: "Test Section", cardIds: [] },
      position: { x: 0, y: 0 },
    };
    render(<EntityNode entity={entity} allPositions={{}} allKinds={{}} />);
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
    const { container } = render(<EntityNode entity={entity} allPositions={{}} allKinds={{}} />);
    const wrapper = container.firstElementChild as HTMLElement;
    // 现在用 transform 代替 left/top 以获得 GPU 合成的丝滑拖拽
    expect(wrapper.style.transform).toContain("150");
    expect(wrapper.style.transform).toContain("250");
  });

  it("section 节点允许拖拽", () => {
    const onDragStart = vi.fn();
    const entity: EntityWithPosition = {
      kind: "section",
      id: "sec_drag001",
      entity: { id: "sec_drag001", title: "Draggable Section", cardIds: [] },
      position: { x: 200, y: 120 },
    };
    const { container } = render(
      <EntityNode
        entity={entity}
        allPositions={{ sec_drag001: { x: 200, y: 120 } }}
        allKinds={{ sec_drag001: "section" }}
        onDragStart={onDragStart}
      />,
    );

    fireEvent.mouseDown(container.firstElementChild as HTMLElement, { button: 0 });

    expect(onDragStart).toHaveBeenCalledTimes(1);
    expect(onDragStart).toHaveBeenCalledWith(expect.anything(), "sec_drag001");
  });
});
