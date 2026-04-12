import { renderHook, act } from "@testing-library/react";
import { useViewport } from "@/components/keysight/useViewport";

describe("useViewport", () => {
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
});
