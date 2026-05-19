import { useMemo } from "react";
import type { RefObject } from "react";
import type { Position } from "@/bindings";
import type { useViewport } from "@/components/keysight/useViewport";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { EntityWithPosition } from "@/components/keysight/types";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import { visibleViewportSize } from "@/components/keysight/lib/graphPositioning";

export interface UseGraphRenderStateDeps {
  graphViewportRef: RefObject<HTMLDivElement | null>;
  containerSize: { width: number; height: number };
  viewport: ReturnType<typeof useViewport>;
  allEntities: EntityWithPosition[];
  allDimensions: Record<string, { width: number; height: number }>;
  // 想强制保持可见的实体集合(编辑中 / 展开中 / 选中 / picker 中)
  editing: { id: string; field: string } | null;
  expandedId: string | null;
  selectedEntityId: string | null;
  relatedPickerCardId: string | null;
  // Related picker 候选搜索
  relatedSearch: string;
  data: WhiteboardData;
  effectivePositions: Record<string, Position>;
}

export interface UseGraphRenderStateResult {
  forceVisibleIds: Set<string>;
  renderViewportSize: { width: number; height: number };
  visibleEntities: EntityWithPosition[];
  hasEntityInViewport: boolean;
  relatedPickerCandidates: WhiteboardData["cards"];
}

export function useGraphRenderState(deps: UseGraphRenderStateDeps): UseGraphRenderStateResult {
  const {
    graphViewportRef, containerSize, viewport, allEntities, allDimensions,
    editing, expandedId, selectedEntityId, relatedPickerCardId,
    relatedSearch, data,
  } = deps;

  // 视口裁剪 — 传入 allDimensions 让 section 用真实 bounds 而非 placeholder
  const forceVisibleIds = useMemo(() => {
    const ids = new Set<string>();
    if (expandedId) ids.add(expandedId);
    if (editing?.id) ids.add(editing.id);
    if (selectedEntityId) ids.add(selectedEntityId);
    if (relatedPickerCardId) ids.add(relatedPickerCardId);
    return ids;
  }, [editing, expandedId, selectedEntityId, relatedPickerCardId]);

  const renderViewportSize = useMemo(
    () => visibleViewportSize(graphViewportRef.current, containerSize),
    [containerSize, graphViewportRef],
  );

  const visibleEntities = useVisibleEntities(
    allEntities, viewport.state, renderViewportSize, allDimensions, forceVisibleIds,
  );

  const hasEntityInViewport = useMemo(() => {
    const size = visibleViewportSize(graphViewportRef.current, containerSize);
    if (size.width === 0 || size.height === 0) return true;
    const { zoom, panX, panY } = viewport.state;
    const viewLeft = -panX / zoom;
    const viewTop = -panY / zoom;
    const viewRight = (-panX + size.width) / zoom;
    const viewBottom = (-panY + size.height) / zoom;

    return allEntities.some((entity) => {
      const dim = allDimensions[entity.id] ?? { width: 320, height: 160 };
      const entityLeft = entity.position.x;
      const entityTop = entity.position.y;
      const entityRight = entity.position.x + dim.width;
      const entityBottom = entity.position.y + dim.height;
      return (
        entityLeft < viewRight && entityRight > viewLeft &&
        entityTop < viewBottom && entityBottom > viewTop
      );
    });
  }, [allDimensions, allEntities, containerSize, viewport.state, graphViewportRef]);

  const relatedPickerCandidates = useMemo(() => {
    if (!relatedPickerCardId) return [];
    const sourceCard = data.cards.find((card) => card.id === relatedPickerCardId);
    if (!sourceCard) return [];

    const excludedIds = new Set<string>([relatedPickerCardId]);
    for (const targetId of sourceCard.related ?? []) excludedIds.add(targetId);
    for (const card of data.cards) {
      if (card.related?.includes(relatedPickerCardId)) excludedIds.add(card.id);
    }

    const words = relatedSearch.toLowerCase().split(/\s+/).filter(Boolean);
    return data.cards
      .filter((card) => {
        if (excludedIds.has(card.id)) return false;
        if (words.length === 0) return true;
        const title = card.title.toLowerCase();
        const content = card.content.toLowerCase();
        return words.every((word) => title.includes(word) || content.includes(word));
      })
      .slice(0, 10);
  }, [data.cards, relatedPickerCardId, relatedSearch]);

  return {
    forceVisibleIds, renderViewportSize, visibleEntities,
    hasEntityInViewport, relatedPickerCandidates,
  };
}
