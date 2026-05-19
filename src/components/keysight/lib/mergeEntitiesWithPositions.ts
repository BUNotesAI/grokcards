import type { Position } from "@/bindings";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { EntityWithPosition } from "@/components/keysight/types";
import { SECTION_PADDING } from "@/components/keysight/lib/sectionLayout";

/**
 * 合并所有实体和位置数据为 EntityWithPosition[]。
 *
 * 只有有位置数据的实体才会被渲染(没位置的卡片不在画布上显示)。
 */
export function mergeEntitiesWithPositions(
  data: WhiteboardData,
  positions: Record<string, Position>,
): EntityWithPosition[] {
  const result: EntityWithPosition[] = [];

  // Sections — 位置从成员动态计算(和旧 Obsidian 插件行为一致)
  // Section 盒子的 top-left 是 min(member.x, member.y) - PADDING
  // 如果没有任何成员有位置,section 不渲染
  for (const section of data.sections) {
    const memberPosList = section.cardIds
      .map((id) => positions[id])
      .filter((p): p is { x: number; y: number } => p != null);

    if (memberPosList.length === 0) {
      // 无成员位置 — 回退到 section 自己的位置(兼容空 section)
      const ownPos = positions[section.id];
      if (ownPos) {
        result.push({ kind: "section", id: section.id, entity: section, position: ownPos });
      }
      continue;
    }

    const minX = Math.min(...memberPosList.map((p) => p.x));
    const minY = Math.min(...memberPosList.map((p) => p.y));
    const effectivePos = { x: minX - SECTION_PADDING, y: minY - SECTION_PADDING };
    result.push({ kind: "section", id: section.id, entity: section, position: effectivePos });
  }

  // Cards
  for (const card of data.cards) {
    const pos = positions[card.id];
    if (pos) {
      result.push({ kind: "card", id: card.id, entity: card, position: pos });
    }
  }

  // Tasks
  for (const task of data.tasks) {
    const pos = positions[task.id];
    if (pos) {
      result.push({ kind: "task", id: task.id, entity: task, position: pos });
    }
  }

  // Questions
  for (const question of data.questions) {
    const pos = positions[question.id];
    if (pos) {
      result.push({ kind: "question", id: question.id, entity: question, position: pos });
    }
  }

  // Notes
  for (const note of data.notes) {
    const pos = positions[note.id];
    if (pos) {
      result.push({ kind: "note", id: note.id, entity: note, position: pos });
    }
  }

  // Aliases
  for (const alias of data.aliases) {
    const pos = positions[alias.aliasId];
    if (pos) {
      result.push({ kind: "alias", id: alias.aliasId, entity: alias, position: pos });
    }
  }

  return result;
}
