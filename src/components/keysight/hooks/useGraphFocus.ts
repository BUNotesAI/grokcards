import { useEffect, useRef } from "react";
import type { GraphFocusTarget } from "@/components/keysight/GraphView";
import type { useViewport } from "@/components/keysight/useViewport";
import type { EntityWithPosition, GraphSelection } from "@/components/keysight/types";
import { TASK_JUMP_MIN_ZOOM } from "@/components/keysight/lib/taskLayout";

export interface UseGraphFocusDeps {
  focusTarget: GraphFocusTarget | null;
  allEntities: EntityWithPosition[];
  allDimensions: Record<string, { width: number; height: number }>;
  whiteboardEntities: Array<{ whiteboardId: string; position: { x: number; y: number } }>;
  viewport: ReturnType<typeof useViewport>;
  getVisibleViewportSize: () => { width: number; height: number };
  onSelectEntity: ((selection: GraphSelection | null) => void) | undefined;
}

/**
 * 外部传入的 focusTarget 触发"居中到该实体或子白板卡"。
 * 用 nonce 去重避免同一 target 反复 center(切走再切回也要响应,所以用 nonce 而非 id)。
 */
export function useGraphFocus(deps: UseGraphFocusDeps): void {
  const {
    focusTarget, allEntities, allDimensions, whiteboardEntities,
    viewport, getVisibleViewportSize, onSelectEntity,
  } = deps;
  const handledNonceRef = useRef<number | null>(null);

  useEffect(() => {
    if (!focusTarget) return;
    const size = getVisibleViewportSize();
    if (size.width === 0 || size.height === 0) return;
    if (handledNonceRef.current === focusTarget.nonce) return;

    const entity = allEntities.find((c) => c.id === focusTarget.id);
    if (entity) {
      const dim = allDimensions[entity.id];
      viewport.actions.centerOn(
        entity.position.x, entity.position.y, size.width, size.height,
        dim?.width ?? 320, dim?.height ?? 160,
        entity.kind === "task" ? { minZoom: TASK_JUMP_MIN_ZOOM } : undefined,
      );
      handledNonceRef.current = focusTarget.nonce;
      onSelectEntity?.({ id: entity.id, kind: entity.kind });
      return;
    }

    const wb = whiteboardEntities.find((c) => `wb:${c.whiteboardId}` === focusTarget.id);
    if (wb) {
      viewport.actions.centerOn(wb.position.x, wb.position.y, size.width, size.height, 320, 130);
      handledNonceRef.current = focusTarget.nonce;
    }
  }, [
    allDimensions, allEntities, focusTarget,
    getVisibleViewportSize, onSelectEntity, viewport.actions, whiteboardEntities,
  ]);
}
