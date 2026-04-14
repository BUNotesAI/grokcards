import { render } from "@testing-library/react";
import { GraphEdges } from "@/components/keysight/GraphEdges";
import type { RenderEdge } from "@/components/keysight/lib/buildEdges";
import type { Position } from "@/bindings";

describe("GraphEdges", () => {
  const positions: Record<string, Position> = {
    c1: { x: 0, y: 0 },
    c2: { x: 600, y: 0 },
    n1: { x: 0, y: 400 },
    q1: { x: 610, y: 430 },
    s1: { x: 600, y: 400 },
  };
  const dimensions: Record<string, { width: number; height: number }> = {
    c1: { width: 520, height: 220 },
    c2: { width: 520, height: 220 },
    n1: { width: 520, height: 180 },
    q1: { width: 320, height: 140 },
    s1: { width: 400, height: 300 },
  };

  it("空 edges → 不渲染任何 path", () => {
    const { container } = render(
      <GraphEdges edges={[]} positions={positions} dimensions={dimensions} />,
    );
    expect(container.querySelectorAll("svg > path")).toHaveLength(0);
    expect(container.querySelectorAll("svg marker")).toHaveLength(0);
  });

  it("link_to edge → 渲染一条实线 + link 颜色 marker", () => {
    const edges: RenderEdge[] = [{ from: "c1", to: "c2", kind: "link_to" }];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    const paths = container.querySelectorAll("svg g > path[data-edge-kind]");
    const markers = container.querySelectorAll("svg marker");
    expect(paths).toHaveLength(1);
    expect(markers).toHaveLength(1);
    expect(paths[0].getAttribute("data-edge-kind")).toBe("link_to");
    expect(paths[0].getAttribute("stroke-dasharray")).toBeNull();
    expect(paths[0].getAttribute("marker-end")).toContain("ks-edge-arrow-link-");
  });

  it("note_link edge → 渲染一条虚线 + note 颜色 marker", () => {
    const edges: RenderEdge[] = [{ from: "n1", to: "c1", kind: "note_link" }];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    const paths = container.querySelectorAll("svg g > path[data-edge-kind]");
    const markers = container.querySelectorAll("svg marker");
    expect(paths).toHaveLength(1);
    expect(markers).toHaveLength(1);
    expect(paths[0].getAttribute("data-edge-kind")).toBe("note_link");
    expect(paths[0].getAttribute("stroke-dasharray")).toBe("8 4");
    expect(paths[0].getAttribute("marker-end")).toContain("ks-edge-arrow-note-");
  });

  it("缺少 from 位置 → 跳过该 edge", () => {
    const edges: RenderEdge[] = [
      { from: "missing", to: "c2", kind: "link_to" },
      { from: "c1", to: "c2", kind: "link_to" },
    ];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    expect(container.querySelectorAll("svg g > path[data-edge-kind]")).toHaveLength(
      1,
    );
  });

  it("混合多种 kind → 全部渲染", () => {
    const edges: RenderEdge[] = [
      { from: "c1", to: "c2", kind: "link_to" },
      { from: "n1", to: "c1", kind: "note_link" },
      { from: "n1", to: "s1", kind: "note_link" },
    ];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    expect(container.querySelectorAll("svg g > path[data-edge-kind]")).toHaveLength(
      3,
    );
    expect(container.querySelectorAll("svg marker")).toHaveLength(3);
  });

  it("近距离 note -> question 连线仍保留可见路径长度", () => {
    const edges: RenderEdge[] = [{ from: "n1", to: "q1", kind: "question_link" }];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    const path = container.querySelector("svg g > path[data-edge-kind='question_link']");
    expect(path).not.toBeNull();

    const d = path!.getAttribute("d")!;
    const match = d.match(/^M ([\d.-]+),([\d.-]+) Q .* ([\d.-]+),([\d.-]+)$/);
    expect(match).not.toBeNull();
    const startX = parseFloat(match![1]);
    const endX = parseFloat(match![3]);
    expect(endX - startX).toBeGreaterThan(30);
  });
});
