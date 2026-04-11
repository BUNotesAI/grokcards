import { useCallback, useRef, useState } from "react";

const MIN_ZOOM = 0.05;
const MAX_ZOOM = 3.0;
const ZOOM_STEP = 1.2;

export interface ViewportState {
  zoom: number;
  panX: number;
  panY: number;
}

/** 将 zoom 值限制在 [MIN_ZOOM, MAX_ZOOM] 范围内 */
function clampZoom(z: number): number {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));
}

/**
 * 画布视口控制 hook
 *
 * 提供 zoom（缩放）和 pan（平移）能力：
 * - zoomIn / zoomOut：按 ZOOM_STEP 倍率缩放，受 [0.05, 3.0] 边界约束
 * - resetView：重置到初始状态（zoom=1, panX=0, panY=0）
 * - 鼠标拖拽平移
 * - Cmd/Ctrl + 滚轮缩放（以鼠标位置为中心）；普通滚轮平移
 */
export function useViewport() {
  const [state, setState] = useState<ViewportState>({
    zoom: 1,
    panX: 0,
    panY: 0,
  });

  // 拖拽状态用 ref，不触发渲染
  const dragRef = useRef<{
    dragging: boolean;
    startX: number;
    startY: number;
    startPanX: number;
    startPanY: number;
  }>({
    dragging: false,
    startX: 0,
    startY: 0,
    startPanX: 0,
    startPanY: 0,
  });

  const zoomIn = useCallback(() => {
    setState((s) => ({ ...s, zoom: clampZoom(s.zoom * ZOOM_STEP) }));
  }, []);

  const zoomOut = useCallback(() => {
    setState((s) => ({ ...s, zoom: clampZoom(s.zoom / ZOOM_STEP) }));
  }, []);

  const resetView = useCallback(() => {
    setState({ zoom: 1, panX: 0, panY: 0 });
  }, []);

  // Pan: 鼠标拖拽
  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      dragRef.current = {
        dragging: true,
        startX: e.clientX,
        startY: e.clientY,
        startPanX: state.panX,
        startPanY: state.panY,
      };
    },
    [state.panX, state.panY],
  );

  const onMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragRef.current.dragging) return;
    const dx = e.clientX - dragRef.current.startX;
    const dy = e.clientY - dragRef.current.startY;
    setState((s) => ({
      ...s,
      panX: dragRef.current.startPanX + dx,
      panY: dragRef.current.startPanY + dy,
    }));
  }, []);

  const onMouseUp = useCallback(() => {
    dragRef.current.dragging = false;
  }, []);

  // Zoom: Cmd/Ctrl + wheel = zoom（鼠标中心）；普通 wheel = pan
  const onWheel = useCallback((e: React.WheelEvent) => {
    if (e.metaKey || e.ctrlKey) {
      e.preventDefault();
      setState((s) => {
        const newZoom = clampZoom(s.zoom * (1 - e.deltaY * 0.001));
        const ratio = newZoom / s.zoom;
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;
        return {
          zoom: newZoom,
          panX: mouseX - (mouseX - s.panX) * ratio,
          panY: mouseY - (mouseY - s.panY) * ratio,
        };
      });
    } else {
      setState((s) => ({
        ...s,
        panX: s.panX - e.deltaX,
        panY: s.panY - e.deltaY,
      }));
    }
  }, []);

  return {
    state,
    handlers: { onMouseDown, onMouseMove, onMouseUp, onWheel },
    actions: { zoomIn, zoomOut, resetView },
  };
}
