import { useMemo } from "react";
import type { Position } from "@/bindings";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type {
  AliasReference, EntityKind, EntityWithPosition,
} from "@/components/keysight/types";
import { ROOT_WHITEBOARD } from "@/components/keysight/GraphView";
import { mergeEntitiesWithPositions } from "@/components/keysight/lib/mergeEntitiesWithPositions";
import { buildEdges } from "@/components/keysight/lib/buildEdges";
import { buildEntityDimensions } from "@/components/keysight/lib/buildEntityDimensions";
import { deriveAllKinds } from "@/components/keysight/lib/deriveAllKinds";
import type { SectionListItem } from "@/components/keysight/nodes/NodeContextMenu";
import type { EntityCounts } from "@/components/keysight/GraphToolbar";

export interface UseEntityIndexesDeps {
  data: WhiteboardData;
  effectivePositions: Record<string, Position>;
  currentWhiteboardId: string;
  whiteboards: Array<{ whiteboardId: string }>;
}

export interface UseEntityIndexesResult {
  allKinds: Record<string, EntityKind>;
  allEntities: EntityWithPosition[];
  allDimensions: Record<string, { width: number; height: number }>;
  entityToSectionId: Record<string, string>;
  menuSections: SectionListItem[];
  whiteboardEntities: Array<{ whiteboardId: string; position: { x: number; y: number } }>;
  cardsById: Record<string, WhiteboardData["cards"][number]>;
  aliasesByTargetId: Record<string, AliasReference[]>;
  entityCounts: EntityCounts;
  renderEdges: ReturnType<typeof buildEdges>;
}

export function useEntityIndexes(deps: UseEntityIndexesDeps): UseEntityIndexesResult {
  const { data, effectivePositions, currentWhiteboardId, whiteboards } = deps;

  const allKinds = useMemo<Record<string, EntityKind>>(
    () => deriveAllKinds(data),
    [data.cards, data.notes, data.tasks, data.questions, data.sections, data.aliases],
  );

  const allEntities = useMemo(
    () => mergeEntitiesWithPositions(data, effectivePositions),
    [data, effectivePositions],
  );

  // section 用 computeSectionBounds 算真实尺寸(SectionNode.PADDING=40 对齐),
  // 否则视口偏离 section 左上角后,section 的 placeholder 400×300 会被错误 cull
  const allDimensions = useMemo(
    () => buildEntityDimensions(allKinds, data.sections, effectivePositions, 40),
    [allKinds, data.sections, effectivePositions],
  );

  // entityId → 所在 section id 的反向索引,用于 ⋯ 菜单判断 "Remove from group" 是否显示
  const entityToSectionId = useMemo(() => {
    const map: Record<string, string> = {};
    for (const section of data.sections) {
      for (const memberId of section.cardIds) {
        map[memberId] = section.id;
      }
    }
    return map;
  }, [data.sections]);

  // 当前白板的 sections 精简列表(Move to Section 子菜单用)
  const menuSections = useMemo<SectionListItem[]>(
    () => data.sections.map((s) => ({ id: s.id, title: s.title })),
    [data.sections],
  );

  // 子白板卡位置(从 positions 表读取 "wb:{id}" 格式的位置)
  const whiteboardEntities = useMemo(() => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return [];
    return whiteboards
      .map((wb) => {
        const pos = effectivePositions[`wb:${wb.whiteboardId}`];
        return pos ? { whiteboardId: wb.whiteboardId, position: pos } : null;
      })
      .filter(Boolean) as Array<{ whiteboardId: string; position: { x: number; y: number } }>;
  }, [whiteboards, effectivePositions, currentWhiteboardId]);

  // cardId → AtomicCard 全局映射(CardNode/AliasNode 渲染 related/linkTo 用)
  const cardsById = useMemo(() => {
    const map: Record<string, typeof data.cards[number]> = {};
    for (const c of data.cards) map[c.id] = c;
    return map;
  }, [data.cards]);

  // aliases 反向索引:targetCardId → [{ aliasId, 所属 section 标题 }]
  // 旧 Obsidian 插件的 ALIASES 区域显示的是 "包含此 card 别名的 section 标题"
  const aliasesByTargetId = useMemo(() => {
    const map: Record<string, AliasReference[]> = {};
    for (const alias of data.aliases) {
      if (!map[alias.cardId]) map[alias.cardId] = [];
      const containingSection = data.sections.find((s) => s.cardIds.includes(alias.aliasId));
      const title = containingSection?.title ?? alias.aliasId;
      map[alias.cardId].push({
        aliasId: alias.aliasId,
        aliasTitle: title,
        cardId: alias.cardId,
        sectionId: containingSection?.id ?? null,
        sectionTitle: containingSection?.title ?? null,
      });
    }
    return map;
  }, [data.aliases, data.sections]);

  const entityCounts = useMemo<EntityCounts>(
    () => ({
      cards: data.cards.length,
      notes: data.notes.length,
      sections: data.sections.length,
      aliases: data.aliases.length,
      tasks: data.tasks.length,
      questions: data.questions.length,
    }),
    [data.cards.length, data.notes.length, data.sections.length, data.aliases.length, data.tasks.length, data.questions.length],
  );

  // 当前白板的可渲染 edge 列表(从 cards/notes/aliases 数据派生)
  // 仅包含 from 和 to 都在当前白板内的 edge,避免渲染断头连线
  const renderEdges = useMemo(() => {
    const entitySet = new Set<string>(Object.keys(allKinds));
    return buildEdges(data.cards, data.notes, data.aliases, data.questions, entitySet);
  }, [data.cards, data.notes, data.aliases, data.questions, allKinds]);

  return {
    allKinds, allEntities, allDimensions, entityToSectionId, menuSections,
    whiteboardEntities, cardsById, aliasesByTargetId, entityCounts, renderEdges,
  };
}
