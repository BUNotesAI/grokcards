import { render, screen } from "@testing-library/react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";

describe("GraphCanvas", () => {
  it("渲染 viewport 和 canvas 容器", () => {
    render(<GraphCanvas />);
    expect(screen.getByTestId("graph-viewport")).toBeInTheDocument();
    expect(screen.getByTestId("graph-canvas")).toBeInTheDocument();
  });

  it("canvas 的 transform 包含 translate 和 scale", () => {
    render(<GraphCanvas />);
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas.style.transform).toContain("translate");
    expect(canvas.style.transform).toContain("scale");
  });

  it("渲染传入的 children", () => {
    render(
      <GraphCanvas>
        <div data-testid="child-node">Hello</div>
      </GraphCanvas>,
    );
    expect(screen.getByTestId("child-node")).toBeInTheDocument();
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas).toContainElement(screen.getByTestId("child-node"));
  });
});
