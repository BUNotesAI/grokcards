import { useEffect, useRef } from "react";
import type { UseViewportReturn } from "@/components/keysight/useViewport";

interface GraphCanvasProps {
  /** 从外部注入的 viewport 状态（GraphView 创建，Toolbar 和 Canvas 共享） */
  viewport: UseViewportReturn;
  /** 外层画布光标，drawing 模式下切换为 crosshair */
  cursor?: React.CSSProperties["cursor"];
  /** 画布背景色；展开 spotlight 时切换为深色 */
  backgroundColor?: React.CSSProperties["backgroundColor"];
  children?: React.ReactNode;
}

/**
 * 知识图谱画布容器
 *
 * 提供可缩放、可平移的无限画布：
 * - 鼠标拖拽平移
 * - Cmd/Ctrl + 滚轮缩放（以鼠标位置为中心）
 * - Cmd/Ctrl + =/-/0 键盘快捷键（放大/缩小/重置）
 * - 普通滚轮平移
 *
 * viewport 由外部通过 props 注入，使 GraphView 可以同时给 Toolbar 和 Canvas 共享同一个 viewport 状态。
 * children 渲染在 transform 容器内，跟随视口变换。
 */
export function GraphCanvas({
  viewport,
  cursor = "grab",
  backgroundColor,
  children,
}: GraphCanvasProps) {
  const { state, handlers, actions } = viewport;
  const viewportRef = useRef<HTMLDivElement>(null);

  // 键盘快捷键：Cmd+= 放大，Cmd+- 缩小，Cmd+0 重置
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      switch (e.key) {
        case "=":
        case "+":
          e.preventDefault();
          actions.zoomIn();
          break;
        case "-":
          e.preventDefault();
          actions.zoomOut();
          break;
        case "0":
          e.preventDefault();
          actions.resetView();
          break;
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [actions]);

  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;

    const handleWheel: EventListener = (event) => {
      const wheelEvent = event as WheelEvent;
      handlers.onWheel({
        metaKey: wheelEvent.metaKey,
        ctrlKey: wheelEvent.ctrlKey,
        deltaX: wheelEvent.deltaX,
        deltaY: wheelEvent.deltaY,
        clientX: wheelEvent.clientX,
        clientY: wheelEvent.clientY,
        currentTarget: el,
        preventDefault: () => wheelEvent.preventDefault(),
      });
    };
    const listenerOptions: AddEventListenerOptions = { passive: false };

    el.addEventListener("wheel", handleWheel, listenerOptions);
    return () => {
      el.removeEventListener("wheel", handleWheel, false);
    };
  }, [handlers]);

  return (
    <div
      ref={viewportRef}
      data-testid="graph-viewport"
      className="relative h-full w-full overflow-hidden bg-muted/30"
      onMouseDown={handlers.onMouseDown}
      onMouseMove={handlers.onMouseMove}
      onMouseUp={handlers.onMouseUp}
      onMouseLeave={handlers.onMouseUp}
      style={{ cursor, backgroundColor }}
    >
      <div
        data-testid="graph-canvas"
        style={{
          transform: `translate(${state.panX}px, ${state.panY}px) scale(${state.zoom})`,
          transformOrigin: "0 0",
          position: "absolute",
          top: 0,
          left: 0,
        }}
      >
        {children}
      </div>
    </div>
  );
}
