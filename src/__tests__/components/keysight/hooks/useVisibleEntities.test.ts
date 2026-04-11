import { renderHook } from "@testing-library/react";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { ViewportState } from "@/components/keysight/useViewport";

/** 创建测试用 card 实体 */
function makeCard(id: string, x: number, y: number): EntityWithPosition {
  return {
    kind: "card",
    id,
    entity: {
      id,
      title: `Card ${id}`,
      content: "",
      filePath: "",
      tags: [],
      linkTo: [],
      related: [],
      understanding: "",
      source: "",
      seeAlso: [],
    },
    position: { x, y },
  };
}

const defaultViewport: ViewportState = { zoom: 1, panX: 0, panY: 0 };
const defaultSize = { width: 1000, height: 800 };

describe("useVisibleEntities", () => {
  it("视口内的实体被返回", () => {
    const entities = [makeCard("c1", 100, 100)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
    expect(result.current[0].id).toBe("c1");
  });

  it("视口外的实体被过滤", () => {
    // 实体在 (5000, 5000)，视口 1000x800，远超范围
    const entities = [makeCard("far", 5000, 5000)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(0);
  });

  it("300px buffer 内的边缘实体被保留", () => {
    // 实体在 (1200, 400)，视口宽 1000 + buffer 300 = 1300
    // 实体左边界 1200 < 视口右边界 1300 → 可见
    const entities = [makeCard("edge", 1200, 400)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
  });

  it("空实体列表返回空数组", () => {
    const { result } = renderHook(() =>
      useVisibleEntities([], defaultViewport, defaultSize),
    );
    expect(result.current).toHaveLength(0);
  });

  it("zoom 缩小时可见范围扩大", () => {
    // zoom=0.5 → 视口覆盖 2000x1600 世界坐标
    // 实体在 (1500, 1200) 应可见
    const viewport: ViewportState = { zoom: 0.5, panX: 0, panY: 0 };
    const entities = [makeCard("zoomed", 1500, 1200)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, viewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
  });

  it("pan 偏移后视口跟随", () => {
    // panX=-500 → 视口向右移 500px
    // 视口左边界（世界坐标）= -(-500)/1 - 300 = 200
    // 实体在 (300, 100)，右边界 = 300 + 320 = 620 > 200 → 可见
    const viewport: ViewportState = { zoom: 1, panX: -500, panY: 0 };
    const entities = [makeCard("panned", 300, 100)];
    const { result } = renderHook(() =>
      useVisibleEntities(entities, viewport, defaultSize),
    );
    expect(result.current).toHaveLength(1);
  });
});
