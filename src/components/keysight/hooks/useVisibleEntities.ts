import { useMemo } from "react";
import type { ViewportState } from "@/components/keysight/useViewport";
import type { EntityWithPosition } from "@/components/keysight/types";
import { ENTITY_DIMENSIONS } from "@/components/keysight/types";

/** 视口裁剪 buffer（世界坐标像素），避免滚动时边缘闪烁 */
const BUFFER = 300;

/**
 * 视口裁剪 hook — 只返回视口内可见的实体。
 *
 * 算法：
 * 1. 视口矩形从屏幕坐标转世界坐标
 * 2. 每个实体 (x, y, w, h) 与世界视口做矩形相交检测
 * 3. useMemo 缓存结果
 *
 * O(N) 遍历，面向 1000+ cards，<1ms。5f 阶段可升级为 Quadtree。
 */
export function useVisibleEntities(
  entities: EntityWithPosition[],
  viewport: ViewportState,
  containerSize: { width: number; height: number },
): EntityWithPosition[] {
  return useMemo(() => {
    const { zoom, panX, panY } = viewport;
    const { width, height } = containerSize;

    // 世界坐标下的视口边界（含 buffer）
    const viewLeft = -panX / zoom - BUFFER;
    const viewTop = -panY / zoom - BUFFER;
    const viewRight = (-panX + width) / zoom + BUFFER;
    const viewBottom = (-panY + height) / zoom + BUFFER;

    return entities.filter((e) => {
      const dim = ENTITY_DIMENSIONS[e.kind];
      const ex = e.position.x;
      const ey = e.position.y;
      const ew = dim.width;
      const eh = dim.height;

      // 矩形相交检测：两个矩形不相交的逆命题
      return !(
        ex + ew < viewLeft ||
        ex > viewRight ||
        ey + eh < viewTop ||
        ey > viewBottom
      );
    });
  }, [entities, viewport, containerSize]);
}
