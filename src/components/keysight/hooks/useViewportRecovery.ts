import { useCallback, useEffect, useRef } from "react";
import type { useViewport } from "@/components/keysight/useViewport";
import type { useLayoutActions } from "@/hooks/useLayoutActions";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { EntityWithPosition } from "@/components/keysight/types";
import { clearSavedViewport } from "@/components/keysight/useViewport";
import { perfLog } from "@/lib/perf";
import { ROOT_WHITEBOARD } from "@/components/keysight/GraphView";

export interface UseViewportRecoveryDeps {
  viewport: ReturnType<typeof useViewport>;
  layouts: ReturnType<typeof useLayoutActions>;
  data: WhiteboardData;
  currentWhiteboardId: string;
  allEntities: EntityWithPosition[];
  // 子白板列表(用于初始化定位 wb:{whiteboardId} 节点)
  whiteboards: Array<{ whiteboardId: string }>;
  hasEntityInViewport: boolean;
  visibleEntitiesCount: number;
  getVisibleViewportSize: () => { width: number; height: number };
}

export interface UseViewportRecoveryResult {
  handleFitContent: () => void;
  handleResetSavedViewport: () => void;
}

export function useViewportRecovery(
  deps: UseViewportRecoveryDeps,
): UseViewportRecoveryResult {
  const {
    viewport,
    layouts,
    data,
    currentWhiteboardId,
    allEntities,
    whiteboards,
    hasEntityInViewport,
    visibleEntitiesCount,
    getVisibleViewportSize,
  } = deps;

  // 无位置的子白板卡:自动计算初始坐标并写入 DB
  const initializedRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return;
    const unpositioned = whiteboards.filter(
      (wb) =>
        !data.positions[`wb:${wb.whiteboardId}`] &&
        !initializedRef.current.has(wb.whiteboardId),
    );
    if (unpositioned.length === 0) return;

    // 找现有实体的 bounding box 最大 y 值
    const maxY = allEntities.reduce((max, e) => Math.max(max, e.position.y + 200), 0);
    const startY = Math.max(maxY + 60, 100);
    const colWidth = 360; // 320px 卡 + 40px 间距
    const rowHeight = 170; // 130px 卡 + 40px 间距
    const cols = 2;

    const writes = unpositioned.map(async (wb, i) => {
      const col = i % cols;
      const row = Math.floor(i / cols);
      const x = 100 + col * colWidth;
      const y = startY + row * rowHeight;
      initializedRef.current.add(wb.whiteboardId);
      await layouts.setPositionWithoutInvalidate(
        currentWhiteboardId,
        `wb:${wb.whiteboardId}`,
        x,
        y,
      );
    });

    Promise.all(writes).then(() => {
      layouts.invalidatePositions();
    });
  }, [whiteboards, data.positions, currentWhiteboardId, allEntities, layouts]);

  // 启动性能:第一次有 visible entities
  const firstVisiblePaintRef = useRef(false);
  useEffect(() => {
    if (!firstVisiblePaintRef.current && visibleEntitiesCount > 0) {
      firstVisiblePaintRef.current = true;
      perfLog(
        `GraphView first visible paint: visible=${visibleEntitiesCount}/${allEntities.length}`,
      );
    }
  }, [visibleEntitiesCount, allEntities.length]);

  const handleFitContent = useCallback(() => {
    const size = getVisibleViewportSize();
    if (allEntities.length === 0 || size.width === 0 || size.height === 0) return;
    viewport.actions.fitToContent(
      allEntities.map((e) => e.position),
      size.width,
      size.height,
    );
  }, [allEntities, getVisibleViewportSize, viewport.actions]);

  // 首次进入白板时自动居中到实体的中位数位置
  // 每个白板在 session 内只尝试一次,已保存的视口由 useViewport 恢复
  const fitAttemptedRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    const size = getVisibleViewportSize();
    if (data.isLoading || allEntities.length === 0 || size.width === 0) return;
    if (fitAttemptedRef.current.has(currentWhiteboardId)) return;
    if (!viewport.needsFit) {
      // 有保存的视口 — 不强制 fit,但仍然标记已尝试避免后续触发
      fitAttemptedRef.current.add(currentWhiteboardId);
      return;
    }
    fitAttemptedRef.current.add(currentWhiteboardId);
    handleFitContent();
  }, [
    data.isLoading,
    allEntities,
    viewport.needsFit,
    handleFitContent,
    getVisibleViewportSize,
    currentWhiteboardId,
  ]);

  // 已保存 viewport 可能指向空白区域。每个白板首次发现"有实体但当前视口无可见实体"
  // 时自动 fit 一次,避免用户看到空画布后还需要手工清 localStorage。
  const blankViewportRecoveryRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (data.isLoading || viewport.needsFit) return;
    const size = getVisibleViewportSize();
    if (size.width === 0 || size.height === 0) return;
    if (allEntities.length === 0 || hasEntityInViewport) return;
    if (blankViewportRecoveryRef.current.has(currentWhiteboardId)) return;
    blankViewportRecoveryRef.current.add(currentWhiteboardId);
    handleFitContent();
  }, [
    allEntities.length,
    currentWhiteboardId,
    data.isLoading,
    getVisibleViewportSize,
    handleFitContent,
    hasEntityInViewport,
    viewport.needsFit,
  ]);

  const handleResetSavedViewport = useCallback(() => {
    clearSavedViewport(currentWhiteboardId);
    fitAttemptedRef.current.delete(currentWhiteboardId);
    blankViewportRecoveryRef.current.delete(currentWhiteboardId);
    const size = getVisibleViewportSize();

    if (allEntities.length > 0 && size.width > 0 && size.height > 0) {
      viewport.actions.fitToContent(
        allEntities.map((entity) => entity.position),
        size.width,
        size.height,
      );
      return;
    }

    viewport.actions.resetView();
  }, [
    allEntities,
    currentWhiteboardId,
    getVisibleViewportSize,
    viewport.actions,
  ]);

  return { handleFitContent, handleResetSavedViewport };
}
