import { render, screen } from "@testing-library/react";
import { renderHook } from "@testing-library/react";
import { vi } from "vitest";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { useViewport } from "@/components/keysight/useViewport";

/** 创建一个真实的 viewport 用于测试 */
function createTestViewport() {
  const { result } = renderHook(() => useViewport("test-wb"));
  return result.current;
}

describe("GraphCanvas", () => {
  it("渲染 viewport 和 canvas 容器", () => {
    const viewport = createTestViewport();
    render(<GraphCanvas viewport={viewport} />);
    expect(screen.getByTestId("graph-viewport")).toBeInTheDocument();
    expect(screen.getByTestId("graph-canvas")).toBeInTheDocument();
  });

  it("canvas 的 transform 包含 translate 和 scale", () => {
    const viewport = createTestViewport();
    render(<GraphCanvas viewport={viewport} />);
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas.style.transform).toContain("translate");
    expect(canvas.style.transform).toContain("scale");
  });

  it("渲染传入的 children", () => {
    const viewport = createTestViewport();
    render(
      <GraphCanvas viewport={viewport}>
        <div data-testid="child-node">Hello</div>
      </GraphCanvas>,
    );
    expect(screen.getByTestId("child-node")).toBeInTheDocument();
    const canvas = screen.getByTestId("graph-canvas");
    expect(canvas).toContainElement(screen.getByTestId("child-node"));
  });

  it("为 wheel 注册 non-passive 原生监听，避免 Cmd/Ctrl + wheel 落回浏览器默认缩放", () => {
    const viewport = createTestViewport();
    const addEventListenerSpy = vi.spyOn(HTMLElement.prototype, "addEventListener");

    render(<GraphCanvas viewport={viewport} />);

    const hasNonPassiveWheelListener = addEventListenerSpy.mock.calls.some(
      ([type, listener, options]) =>
        type === "wheel" &&
        typeof listener === "function" &&
        typeof options === "object" &&
        options !== null &&
        "passive" in options &&
        options.passive === false,
    );

    expect(hasNonPassiveWheelListener).toBe(true);
  });
});
