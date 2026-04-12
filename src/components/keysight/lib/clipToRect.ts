/**
 * 从中心点 (cx, cy) 出发朝目标 (tx, ty) 的射线，与矩形 (rx, ry, rw, rh) 边界的交点。
 *
 * 用途：edge 渲染让连线从卡片的边缘开始，而不是从卡片中心穿出。
 *
 * 算法：参数化射线 P(t) = (cx + t·dx, cy + t·dy)，对四条边分别求 t，
 * 取 t > 0 且交点落在边内的最小 t。
 *
 * 退化：起点等于目标 → 返回起点；无有效交点 → 返回起点。
 */
export function clipToRect(
  cx: number,
  cy: number,
  tx: number,
  ty: number,
  rx: number,
  ry: number,
  rw: number,
  rh: number,
): { x: number; y: number } {
  const dx = tx - cx;
  const dy = ty - cy;
  if (dx === 0 && dy === 0) return { x: cx, y: cy };

  let tMin = Infinity;
  // 左边界 x = rx
  if (dx !== 0) {
    const t = (rx - cx) / dx;
    if (t > 0) {
      const y = cy + t * dy;
      if (y >= ry && y <= ry + rh && t < tMin) tMin = t;
    }
  }
  // 右边界 x = rx + rw
  if (dx !== 0) {
    const t = (rx + rw - cx) / dx;
    if (t > 0) {
      const y = cy + t * dy;
      if (y >= ry && y <= ry + rh && t < tMin) tMin = t;
    }
  }
  // 上边界 y = ry
  if (dy !== 0) {
    const t = (ry - cy) / dy;
    if (t > 0) {
      const x = cx + t * dx;
      if (x >= rx && x <= rx + rw && t < tMin) tMin = t;
    }
  }
  // 下边界 y = ry + rh
  if (dy !== 0) {
    const t = (ry + rh - cy) / dy;
    if (t > 0) {
      const x = cx + t * dx;
      if (x >= rx && x <= rx + rw && t < tMin) tMin = t;
    }
  }

  if (tMin === Infinity) return { x: cx, y: cy };
  return { x: cx + tMin * dx, y: cy + tMin * dy };
}
