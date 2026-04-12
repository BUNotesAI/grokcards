import { useMemo, useRef, useState, useCallback } from "react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { GraphToolbar, type EntityCounts } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useWhiteboardData, type WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { Position } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { commands } from "@/bindings";
import { useQueryClient } from "@tanstack/react-query";

/** 根白板 ID — rust/chentian 等是子白板，根白板显示子白板预览卡 */
const ROOT_WHITEBOARD = "wb_root";

/** 默认新实体位置（画布中心附近，带随机偏移防重叠） */
function randomOffset(): Position {
  return {
    x: 200 + Math.random() * 400,
    y: 200 + Math.random() * 300,
  };
}

/**
 * 合并所有实体和位置数据为 EntityWithPosition[]。
 *
 * 只有有位置数据的实体才会被渲染（没位置的卡片不在画布上显示）。
 */
function mergeEntitiesWithPositions(data: WhiteboardData): EntityWithPosition[] {
  const result: EntityWithPosition[] = [];
  const positions = data.positions;

  // Sections — 只渲染有 position 的
  for (const section of data.sections) {
    const pos = positions[section.id];
    if (pos) {
      result.push({ kind: "section", id: section.id, entity: section, position: pos });
    }
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

/**
 * KeySight 知识图谱主视图 — 数据枢纽
 *
 * 数据流: useWhiteboardData → mergeEntitiesWithPositions → useVisibleEntities → EntityNode 渲染
 * viewport 由 GraphView 创建，GraphToolbar 和 GraphCanvas 共享。
 */
export function GraphView() {
  const viewport = useViewport(ROOT_WHITEBOARD);
  const containerRef = useRef<HTMLDivElement>(null);
  const containerSize = useContainerSize(containerRef);
  const data = useWhiteboardData(ROOT_WHITEBOARD);
  const queryClient = useQueryClient();

  // 搜索状态
  const [searchQuery, setSearchQuery] = useState("");

  // 合并实体和位置
  const allEntities = useMemo(() => mergeEntitiesWithPositions(data), [data]);

  // 视口裁剪
  const visibleEntities = useVisibleEntities(allEntities, viewport.state, containerSize);

  // 搜索过滤（简单 title 匹配）
  const filteredEntities = useMemo(() => {
    if (!searchQuery.trim()) return visibleEntities;
    const q = searchQuery.toLowerCase();
    return visibleEntities.filter((e) => {
      const title = "entity" in e && "title" in e.entity ? (e.entity as any).title : "";
      return title.toLowerCase().includes(q);
    });
  }, [visibleEntities, searchQuery]);

  // 卡片标题映射（AliasNode 需要）
  const cardTitles = useMemo(() => {
    const map: Record<string, string> = {};
    for (const card of data.cards) {
      map[card.id] = card.title;
    }
    return map;
  }, [data.cards]);

  // 所有位置映射（SectionNode bounds 计算用）
  const allPositions = data.positions;

  // 实体计数
  const entityCounts: EntityCounts = useMemo(
    () => ({
      cards: data.cards.length,
      notes: data.notes.length,
      sections: data.sections.length,
      tasks: data.tasks.length,
      questions: data.questions.length,
      aliases: data.aliases.length,
    }),
    [data],
  );

  // 创建 Section 回调
  const handleCreateSection = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.sectionCreate(ROOT_WHITEBOARD, "New Section", null),
      );
      const pos = randomOffset();
      await unwrapCommand(
        commands.layoutSetPosition(ROOT_WHITEBOARD, result.id, pos.x, pos.y),
      );
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 section 失败:", e);
    }
  }, [queryClient]);

  // 创建 Note 回调
  const handleCreateNote = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.noteCreate(ROOT_WHITEBOARD, "New Note", null, null),
      );
      const pos = randomOffset();
      await unwrapCommand(
        commands.layoutSetPosition(ROOT_WHITEBOARD, result.id, pos.x, pos.y),
      );
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 note 失败:", e);
    }
  }, [queryClient]);

  // Sync 回调
  const handleSync = useCallback(() => {
    data.syncVault.mutate();
  }, [data.syncVault]);

  // Loading 状态
  if (data.isLoading) {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <div className="text-sm text-muted-foreground">Loading whiteboard...</div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="flex h-full w-full flex-col">
      <GraphToolbar
        viewport={viewport}
        entityCounts={entityCounts}
        onSync={handleSync}
        onCreateSection={handleCreateSection}
        onCreateNote={handleCreateNote}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
      />
      <div className="relative flex-1 overflow-hidden">
        <GraphCanvas viewport={viewport}>
          {filteredEntities.map((e) => (
            <EntityNode
              key={e.id}
              entity={e}
              allPositions={allPositions}
              cardTitles={cardTitles}
            />
          ))}
        </GraphCanvas>
      </div>
    </div>
  );
}
