import { renderHook, act } from "@testing-library/react";
import { vi } from "vitest";
import { clearSavedViewport, useViewport } from "@/components/keysight/useViewport";

describe("useViewport", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("初始状态: zoom=1, panX=0, panY=0", () => {
    const { result } = renderHook(() => useViewport("test-wb"));
    expect(result.current.state.zoom).toBe(1);
    expect(result.current.state.panX).toBe(0);
    expect(result.current.state.panY).toBe(0);
  });

  it("zoomIn 放大（×1.2）", () => {
    const { result } = renderHook(() => useViewport("test-wb"));
    act(() => result.current.actions.zoomIn());
    expect(result.current.state.zoom).toBeCloseTo(1.2);
  });

  it("zoomOut 缩小（÷1.2）", () => {
    const { result } = renderHook(() => useViewport("test-wb"));
    act(() => result.current.actions.zoomOut());
    expect(result.current.state.zoom).toBeCloseTo(1 / 1.2);
  });

  it("zoom 上边界不超过 3.0", () => {
    const { result } = renderHook(() => useViewport("test-wb"));
    for (let i = 0; i < 20; i++) {
      act(() => result.current.actions.zoomIn());
    }
    expect(result.current.state.zoom).toBeLessThanOrEqual(3.0);
  });

  it("zoom 下边界不低于 0.05", () => {
    const { result } = renderHook(() => useViewport("test-wb"));
    for (let i = 0; i < 50; i++) {
      act(() => result.current.actions.zoomOut());
    }
    expect(result.current.state.zoom).toBeGreaterThanOrEqual(0.05);
  });

  it("resetView 重置到初始状态", () => {
    const { result } = renderHook(() => useViewport("test-wb"));
    act(() => result.current.actions.zoomIn());
    act(() => result.current.actions.resetView());
    expect(result.current.state.zoom).toBe(1);
    expect(result.current.state.panX).toBe(0);
    expect(result.current.state.panY).toBe(0);
  });

  it("centerOn 支持最小可读 zoom，避免跳转后 task 太小", () => {
    const { result } = renderHook(() => useViewport("test-wb"));

    for (let i = 0; i < 12; i++) {
      act(() => result.current.actions.zoomOut());
    }

    act(() => {
      result.current.actions.centerOn(100, 200, 1000, 800, 320, 140, {
        minZoom: 0.75,
      });
    });

    expect(result.current.state.zoom).toBeCloseTo(0.75);
    expect(result.current.state.panX).toBeCloseTo(1000 / 2 - (100 + 160) * 0.75);
    expect(result.current.state.panY).toBeCloseTo(800 / 2 - (200 + 70) * 0.75);
  });

  it("clearSavedViewport 只清当前白板的 saved viewport", () => {
    localStorage.setItem("keysight:viewport:test-wb", "{\"zoom\":0.1}");
    localStorage.setItem("keysight:viewport:other-wb", "{\"zoom\":0.2}");

    clearSavedViewport("test-wb");

    expect(localStorage.getItem("keysight:viewport:test-wb")).toBeNull();
    expect(localStorage.getItem("keysight:viewport:other-wb")).toBe("{\"zoom\":0.2}");
  });

  it("Cmd/Ctrl + wheel 缩放时不依赖事件对象在回调返回后仍然可用", () => {
    const { result } = renderHook(() => useViewport("test-wb"));
    const viewportEl = document.createElement("div");
    vi.spyOn(viewportEl, "getBoundingClientRect").mockReturnValue({
      left: 10,
      top: 20,
      right: 410,
      bottom: 320,
      width: 400,
      height: 300,
      x: 10,
      y: 20,
      toJSON: () => ({}),
    });

    let released = false;
    const event = {
      metaKey: true,
      ctrlKey: false,
      deltaX: 0,
      deltaY: -120,
      clientX: 210,
      clientY: 170,
      preventDefault: vi.fn(),
      get currentTarget() {
        if (released) {
          throw new Error("wheel event currentTarget released");
        }
        return viewportEl;
      },
    };

    expect(() => {
      act(() => {
        result.current.handlers.onWheel(event as unknown as React.WheelEvent);
        released = true;
      });
    }).not.toThrow();

    expect(result.current.state.zoom).toBeGreaterThan(1);
    expect(event.preventDefault).toHaveBeenCalled();
  });
});
