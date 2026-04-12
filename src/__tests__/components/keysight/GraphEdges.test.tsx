import { render } from "@testing-library/react";
import { GraphEdges } from "@/components/keysight/GraphEdges";
import type { RenderEdge } from "@/components/keysight/lib/buildEdges";
import type { Position } from "@/bindings";

describe("GraphEdges", () => {
  const positions: Record<string, Position> = {
    c1: { x: 0, y: 0 },
    c2: { x: 600, y: 0 },
    n1: { x: 0, y: 400 },
    s1: { x: 600, y: 400 },
  };
  const dimensions: Record<string, { width: number; height: number }> = {
    c1: { width: 520, height: 220 },
    c2: { width: 520, height: 220 },
    n1: { width: 520, height: 180 },
    s1: { width: 400, height: 300 },
  };

  it("空 edges → 不渲染任何 path", () => {
    const { container } = render(
      <GraphEdges edges={[]} positions={positions} dimensions={dimensions} />,
    );
    // <defs> 里的 marker path 不算
    expect(container.querySelectorAll("svg > path")).toHaveLength(0);
  });

  it("link_to edge → 渲染一条实线 + link 颜色 marker", () => {
    const edges: RenderEdge[] = [{ from: "c1", to: "c2", kind: "link_to" }];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    const paths = container.querySelectorAll("svg > path");
    expect(paths).toHaveLength(1);
    expect(paths[0].getAttribute("data-edge-kind")).toBe("link_to");
    expect(paths[0].getAttribute("stroke-dasharray")).toBeNull();
    expect(paths[0].getAttribute("marker-end")).toContain("ks-edge-arrow-link");
  });

  it("note_link edge → 渲染一条虚线 + note 颜色 marker", () => {
    const edges: RenderEdge[] = [{ from: "n1", to: "c1", kind: "note_link" }];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    const paths = container.querySelectorAll("svg > path");
    expect(paths).toHaveLength(1);
    expect(paths[0].getAttribute("data-edge-kind")).toBe("note_link");
    expect(paths[0].getAttribute("stroke-dasharray")).toBe("8 4");
    expect(paths[0].getAttribute("marker-end")).toContain("ks-edge-arrow-note");
  });

  it("缺少 from 位置 → 跳过该 edge", () => {
    const edges: RenderEdge[] = [
      { from: "missing", to: "c2", kind: "link_to" },
      { from: "c1", to: "c2", kind: "link_to" },
    ];
    const { container } = render(
      <GraphEdges edges={edges} positions={positions} dimensions={dimensions} />,
    );
    expect(container.querySelectorAll("svg > path")).toHaveLength(1);
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
    expect(container.querySelectorAll("svg > path")).toHaveLength(3);
  });
});
