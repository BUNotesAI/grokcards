import type { Position } from "@/bindings";

/** Section 容器的几何尺寸（世界坐标像素） */
export interface SectionBounds {
  width: number;
  height: number;
}

/** 计算 bounds 所需的单个成员信息：位置 + 实际渲染尺寸 */
export interface SectionMember {
  position: Position;
  width: number;
  height: number;
}

/**
 * 计算 Section 容器的宽高，保证能包住所有 member 的 bounding box + padding。
 *
 * 契约：
 * - 前置条件：调用者必须传入每个 member 的**真实渲染尺寸**（不是 kind 通用估计）
 * - 无 member 时返回最小尺寸 { width: 200, height: 100 }
 * - 坐标系约定：position 是 member DOM 的 top-left
 *
 * 和 [`GraphView.mergeEntitiesWithPositions`] 里的 section 位置算法配对使用：
 * section 的 top-left = (minMember.x - padding, minMember.y - padding)
 * 所以 bounds.width/height 必须保证 section 的 right/bottom 覆盖所有 member 的 right/bottom
 */
export function computeSectionBounds(
  members: SectionMember[],
  padding: number,
): SectionBounds {
  if (members.length === 0) {
    return { width: 200, height: 100 };
  }

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const m of members) {
    if (m.position.x < minX) minX = m.position.x;
    if (m.position.y < minY) minY = m.position.y;
    const right = m.position.x + m.width;
    const bottom = m.position.y + m.height;
    if (right > maxX) maxX = right;
    if (bottom > maxY) maxY = bottom;
  }

  return {
    width: maxX - minX + padding * 2,
    height: maxY - minY + padding * 2,
  };
}
