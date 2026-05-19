/** Section 边距:绘图时 section 盒子的 top-left 是 min(member.x, member.y) - PADDING */
export const SECTION_PADDING = 40;

export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export function rectsOverlap(a: Rect, b: Rect): boolean {
  return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
}

/**
 * 给一个 preferred 坐标 + 待放置矩形 size,沿现有矩形右侧顺次试探,
 * 直到找到不重叠的位置;最多试 2N+1 次后兜底返回最后一次结果。
 */
export function avoidSectionOverlap(
  preferred: { x: number; y: number },
  size: { width: number; height: number },
  existingRects: Rect[],
  gap = 40,
): { x: number; y: number } {
  let next = { ...preferred };

  for (let i = 0; i < existingRects.length * 2 + 1; i += 1) {
    const candidate: Rect = {
      left: next.x,
      top: next.y,
      right: next.x + size.width,
      bottom: next.y + size.height,
    };
    const hit = existingRects.find((rect) => rectsOverlap(candidate, rect));
    if (!hit) return next;
    next = {
      x: hit.right + gap,
      y: hit.top,
    };
  }

  return next;
}
