import { useViewport } from "@/components/keysight/useViewport";
import { useEffect } from "react";

interface GraphCanvasProps {
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
 * children 渲染在 transform 容器内，跟随视口变换
 */
export function GraphCanvas({ children }: GraphCanvasProps) {
  const { state, handlers, actions } = useViewport();

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

  return (
    <div
      data-testid="graph-viewport"
      className="relative h-full w-full overflow-hidden bg-muted/30"
      onMouseDown={handlers.onMouseDown}
      onMouseMove={handlers.onMouseMove}
      onMouseUp={handlers.onMouseUp}
      onMouseLeave={handlers.onMouseUp}
      onWheel={handlers.onWheel}
      style={{ cursor: "grab" }}
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
