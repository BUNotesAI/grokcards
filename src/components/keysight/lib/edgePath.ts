import { clipToRect } from "./clipToRect";

/** 矩形端点：左上角 + 宽高 */
export interface EdgeBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** 计算结果：SVG 曲线 path + 中点（给删除按钮等 UI 元素用） */
export interface EdgePath {
  path: string;
  midX: number;
  midY: number;
}

/**
 * 构造 from→to 两矩形之间的 SVG 二次贝塞尔曲线 path。
 *
 * 算法：
 * 1. 取两矩形中心 (cx1, cy1) (cx2, cy2)
 * 2. 用 clipToRect 求射线与 from 矩形边界的交点 p1
 * 3. 用 clipToRect 求反向射线与 to 矩形边界的交点 p2
 * 4. 中点 + 法向量偏移 curvature 构造控制点（让曲线略微弯曲，避免重叠）
 * 5. 返回 SVG 路径 `M p1 Q ctrl p2`
 *
 * 退化：两交点距离 < 10 → 返回 null（不渲染）
 *
 * @param from 起点矩形
 * @param to 终点矩形
 * @param padding 矩形外扩 padding（让 edge 端点离卡片有缝隙），默认 6
 */
export function buildEdgePath(
  from: EdgeBox,
  to: EdgeBox,
  padding = 6,
): EdgePath | null {
  // 矩形重叠时不渲染连线（视觉上无意义，且 clipToRect 会算出"反向"路径）
  const overlap = !(
    from.x + from.width <= to.x ||
    to.x + to.width <= from.x ||
    from.y + from.height <= to.y ||
    to.y + to.height <= from.y
  );
  if (overlap) return null;

  const cx1 = from.x + from.width / 2;
  const cy1 = from.y + from.height / 2;
  const cx2 = to.x + to.width / 2;
  const cy2 = to.y + to.height / 2;

  // 矩形外扩 padding，让连线端点离卡片边缘有缝隙
  const p1 = clipToRect(
    cx1,
    cy1,
    cx2,
    cy2,
    from.x - padding,
    from.y - padding,
    from.width + padding * 2,
    from.height + padding * 2,
  );
  const p2 = clipToRect(
    cx2,
    cy2,
    cx1,
    cy1,
    to.x - padding,
    to.y - padding,
    to.width + padding * 2,
    to.height + padding * 2,
  );

  const dx = p2.x - p1.x;
  const dy = p2.y - p1.y;
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 10) return null;

  // 二次贝塞尔控制点 — 中点 + 法向量偏移 curvature
  const midX = (p1.x + p2.x) / 2;
  const midY = (p1.y + p2.y) / 2;
  const curvature = Math.min(len * 0.15, 40);
  const nx = -dy / len; // 法向量
  const ny = dx / len;
  const cpX = midX + nx * curvature;
  const cpY = midY + ny * curvature;

  const path = `M ${p1.x},${p1.y} Q ${cpX},${cpY} ${p2.x},${p2.y}`;
  return { path, midX: cpX, midY: cpY };
}
