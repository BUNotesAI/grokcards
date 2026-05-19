/** 把屏幕中心转成世界坐标 — 给"在视口中央创建新实体"用 */
export function viewportCenterWorld(
  zoom: number,
  panX: number,
  panY: number,
  containerWidth: number,
  containerHeight: number,
): { x: number; y: number } {
  return {
    x: (-panX + containerWidth / 2) / zoom,
    y: (-panY + containerHeight / 2) / zoom,
  };
}

/**
 * 计算 wrapper div 在当前 window 里真正可见的尺寸。
 *
 * 为什么需要:body overflow 失控 / 嵌套 flex 异常时,wrapper 可以长出视口
 * 高度(如 contentRect.height = 12500),centerOn math 用这个错的高度
 * 会把 entity 推到屏幕外。此函数用 window.innerWidth/innerHeight 减去
 * rect.left/top 算可见区,作为 raw rect 的上限兜底。
 *
 * 1.5× buffer:允许 rect 比 visible 略大(scrollbar、sub-pixel rounding),
 * 超过则视为 layout 异常,用 visible 替换。阈值选 1.5 而不是 1.0 是为了
 * 避免临界尺寸下在 raw 和 visible 之间反复 flap 触发 re-render。
 */
export function visibleViewportSize(
  element: HTMLElement | null,
  fallback: { width: number; height: number },
): { width: number; height: number } {
  const rect = element?.getBoundingClientRect();
  const rawWidth = rect && rect.width > 0 ? rect.width : fallback.width;
  const rawHeight = rect && rect.height > 0 ? rect.height : fallback.height;
  if (rawWidth <= 0 || rawHeight <= 0) return { width: 0, height: 0 };

  const windowWidth = typeof window === "undefined" ? rawWidth : window.innerWidth;
  const windowHeight = typeof window === "undefined" ? rawHeight : window.innerHeight;
  const visibleWidth =
    rect && rect.width > 0 ? Math.max(0, windowWidth - Math.max(rect.left, 0)) : windowWidth;
  const visibleHeight =
    rect && rect.height > 0 ? Math.max(0, windowHeight - Math.max(rect.top, 0)) : windowHeight;

  return {
    width: rawWidth > visibleWidth * 1.5 ? visibleWidth : rawWidth,
    height: rawHeight > visibleHeight * 1.5 ? visibleHeight : rawHeight,
  };
}
