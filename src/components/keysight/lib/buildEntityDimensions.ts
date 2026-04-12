import type { GraphSection, Position } from "@/bindings";
import {
  ENTITY_DIMENSIONS,
  type EntityKind,
} from "@/components/keysight/types";
import {
  computeSectionBounds,
  type SectionMember,
} from "./sectionBounds";

/**
 * 构造 entity id → 渲染尺寸 的映射，给视口裁剪 + edge clipToRect 用。
 *
 * - 非 section: 用 `ENTITY_DIMENSIONS[kind]` 作为估计尺寸
 * - section: 用成员位置 + `computeSectionBounds` 算真实尺寸
 *
 * **section 必须用真实尺寸**，因为 ENTITY_DIMENSIONS.section 只是 placeholder（400×300），
 * 实际 section 跨多个成员时宽度可能 2000+px。视口裁剪用 placeholder 会导致：
 * pan 到 section 右侧时，section "placeholder 右边" (= position.x + 400) 在视口左边外，
 * section 被错误 cull，背景和边框消失。
 */
export function buildEntityDimensions(
  allKinds: Record<string, EntityKind>,
  sections: GraphSection[],
  positions: Record<string, Position>,
  sectionPadding: number,
): Record<string, { width: number; height: number }> {
  const dims: Record<string, { width: number; height: number }> = {};

  for (const [id, kind] of Object.entries(allKinds)) {
    if (kind === "section") continue;
    dims[id] = ENTITY_DIMENSIONS[kind];
  }

  for (const section of sections) {
    const members: SectionMember[] = [];
    for (const memberId of section.cardIds) {
      const pos = positions[memberId];
      if (!pos) continue;
      const memberKind = allKinds[memberId] ?? "card";
      const memberDim = ENTITY_DIMENSIONS[memberKind];
      members.push({
        position: pos,
        width: memberDim.width,
        height: memberDim.height,
      });
    }
    dims[section.id] = computeSectionBounds(members, sectionPadding);
  }

  return dims;
}
