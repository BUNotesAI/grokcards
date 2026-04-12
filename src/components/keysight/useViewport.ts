import { useCallback, useEffect, useRef, useState } from "react";

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

const DEFAULT_VIEWPORT: ViewportState = { zoom: 1, panX: 0, panY: 0 };
const STORAGE_PREFIX = "keysight:viewport:";
const SAVE_DEBOUNCE_MS = 500;

/** 检查白板是否有已保存的视口 */
export function hasSavedViewport(whiteboardId: string): boolean {
  return localStorage.getItem(STORAGE_PREFIX + whiteboardId) !== null;
}

/** 从 localStorage 读取白板视口 */
function loadViewport(whiteboardId: string): ViewportState {
  try {
    const raw = localStorage.getItem(STORAGE_PREFIX + whiteboardId);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (
        typeof parsed.zoom === "number" &&
        typeof parsed.panX === "number" &&
        typeof parsed.panY === "number"
      ) {
        return { zoom: clampZoom(parsed.zoom), panX: parsed.panX, panY: parsed.panY };
      }
    }
  } catch {
    // 损坏的 JSON，忽略
  }
  return { ...DEFAULT_VIEWPORT };
}

/** 保存白板视口到 localStorage */
function saveViewport(whiteboardId: string, state: ViewportState): void {
  localStorage.setItem(STORAGE_PREFIX + whiteboardId, JSON.stringify(state));
}

/**
 * 画布视口控制 hook
 *
 * 提供 zoom（缩放）和 pan（平移）能力：
 * - zoomIn / zoomOut：按 ZOOM_STEP 倍率缩放，受 [0.05, 3.0] 边界约束
 * - resetView：重置到初始状态（zoom=1, panX=0, panY=0）
 * - 鼠标拖拽平移
 * - Cmd/Ctrl + 滚轮缩放（以鼠标位置为中心）；普通滚轮平移
 * - 视口状态按白板 ID 持久化到 localStorage（500ms debounce）
 */
export function useViewport(whiteboardId: string) {
  const [state, setState] = useState<ViewportState>(() => loadViewport(whiteboardId));
  // 追踪当前白板是否需要 fit-to-content（在加载前、debounce 写入前判断）
  const [needsFit, setNeedsFit] = useState(() => !hasSavedViewport(whiteboardId));

  // 白板切换时加载对应视口
  const prevWhiteboardIdRef = useRef(whiteboardId);
  useEffect(() => {
    if (prevWhiteboardIdRef.current !== whiteboardId) {
      // 保存旧白板视口（立即写入，不等 debounce）
      saveViewport(prevWhiteboardIdRef.current, state);
      prevWhiteboardIdRef.current = whiteboardId;
      // 在加载前检查是否有保存的视口（debounce 还没写入）
      const hadSaved = hasSavedViewport(whiteboardId);
      setState(loadViewport(whiteboardId));
      setNeedsFit(!hadSaved);
    }
  }, [whiteboardId]); // eslint-disable-line react-hooks/exhaustive-deps

  // Debounced 持久化
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    debounceRef.current = setTimeout(() => {
      saveViewport(whiteboardId, state);
    }, SAVE_DEBOUNCE_MS);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [state, whiteboardId]);

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
    setState({ ...DEFAULT_VIEWPORT });
  }, []);

  /**
   * 定位到实体中心 — 以中位数位置为中心，zoom=1 显示
   *
   * 为什么用中位数而不是 bounding box fit：
   * - 旧数据实体分布不均（有的 whiteboard Y 跨度 30000+px）
   * - bounding box fit 会把 zoom 压到极小，看不清任何内容
   * - 中位数居中 + 默认 zoom 让用户能立即看清中心区域的卡片
   * - 用户可以从中心手动 pan/zoom 探索其他区域
   */
  const fitToContent = useCallback(
    (positions: Array<{ x: number; y: number }>, containerWidth: number, containerHeight: number) => {
      if (positions.length === 0) return;
      const sortedX = positions.map((p) => p.x).sort((a, b) => a - b);
      const sortedY = positions.map((p) => p.y).sort((a, b) => a - b);
      const medianX = sortedX[Math.floor(sortedX.length / 2)];
      const medianY = sortedY[Math.floor(sortedY.length / 2)];
      // 把中位数实体的中心放在屏幕中心（考虑估算的节点尺寸 320x160）
      const zoom = 1;
      const panX = containerWidth / 2 - (medianX + 160) * zoom;
      const panY = containerHeight / 2 - (medianY + 80) * zoom;
      setState({ zoom, panX, panY });
      setNeedsFit(false);
    },
    [],
  );

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
    needsFit,
    handlers: { onMouseDown, onMouseMove, onMouseUp, onWheel },
    actions: { zoomIn, zoomOut, resetView, fitToContent },
  };
}

/** useViewport hook 的返回值类型，供 GraphCanvas props 使用 */
export type UseViewportReturn = ReturnType<typeof useViewport>;
