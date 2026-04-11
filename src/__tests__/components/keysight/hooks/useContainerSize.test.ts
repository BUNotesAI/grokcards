import { renderHook } from "@testing-library/react";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useRef } from "react";

// mock ResizeObserver — jsdom 不支持
const mockObserve = vi.fn();
const mockDisconnect = vi.fn();

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", class {
    constructor(_cb: ResizeObserverCallback) {}
    observe = mockObserve;
    unobserve = vi.fn();
    disconnect = mockDisconnect;
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("useContainerSize", () => {
  it("返回初始尺寸 { width: 0, height: 0 }", () => {
    const { result } = renderHook(() => {
      const ref = useRef<HTMLDivElement>(null);
      return useContainerSize(ref);
    });
    expect(result.current.width).toBe(0);
    expect(result.current.height).toBe(0);
  });

  it("挂载后调用 ResizeObserver.observe", () => {
    const div = document.createElement("div");
    renderHook(() => {
      const ref = useRef<HTMLDivElement>(div);
      // 手动设置 current 模拟 ref 已挂载
      ref.current = div;
      return useContainerSize(ref);
    });
    expect(mockObserve).toHaveBeenCalled();
  });
});
