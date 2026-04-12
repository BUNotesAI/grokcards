import { useMemo, useRef, useState, useCallback, useEffect } from "react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { GraphToolbar, type EntityCounts } from "@/components/keysight/GraphToolbar";
import { useViewport, hasSavedViewport } from "@/components/keysight/useViewport";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useWhiteboardData, useWhiteboardList, type WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import { WhiteboardNode } from "@/components/keysight/nodes/WhiteboardNode";
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
  const [currentWhiteboardId, setCurrentWhiteboardId] = useState(ROOT_WHITEBOARD);
  const viewport = useViewport(currentWhiteboardId);
  const containerRef = useRef<HTMLDivElement>(null);
  const containerSize = useContainerSize(containerRef);
  const data = useWhiteboardData(currentWhiteboardId);
  const queryClient = useQueryClient();

  // 搜索状态
  const [searchQuery, setSearchQuery] = useState("");

  // 合并实体和位置
  const allEntities = useMemo(() => mergeEntitiesWithPositions(data), [data]);

  // 首次进入没有保存视口的白板时，自适应内容居中
  const fitDoneRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (
      !data.isLoading &&
      allEntities.length > 0 &&
      !hasSavedViewport(currentWhiteboardId) &&
      !fitDoneRef.current.has(currentWhiteboardId) &&
      containerSize.width > 0
    ) {
      fitDoneRef.current.add(currentWhiteboardId);
      viewport.actions.fitToContent(
        allEntities.map((e) => e.position),
        containerSize.width,
        containerSize.height,
      );
    }
  }, [data.isLoading, allEntities, currentWhiteboardId, containerSize, viewport.actions]);

  // 子白板列表（仅根白板时加载）
  const whiteboardListQuery = useWhiteboardList(currentWhiteboardId);
  const whiteboards = whiteboardListQuery.data ?? [];

  // 子白板卡位置（从 positions 表读取 "wb:{id}" 格式的位置）
  const whiteboardEntities = useMemo(() => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return [];
    return whiteboards
      .map((wb) => {
        const pos = data.positions[`wb:${wb.whiteboardId}`];
        return pos ? { whiteboardId: wb.whiteboardId, position: pos } : null;
      })
      .filter(Boolean) as Array<{ whiteboardId: string; position: { x: number; y: number } }>;
  }, [whiteboards, data.positions, currentWhiteboardId]);

  // 无位置的子白板卡：自动计算初始坐标并写入 DB
  const initializedRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return;
    const unpositioned = whiteboards.filter(
      (wb) =>
        !data.positions[`wb:${wb.whiteboardId}`] &&
        !initializedRef.current.has(wb.whiteboardId),
    );
    if (unpositioned.length === 0) return;

    // 找现有实体的 bounding box 最大 y 值
    const maxY = allEntities.reduce((max, e) => Math.max(max, e.position.y + 200), 0);
    const startY = Math.max(maxY + 60, 100);
    const colWidth = 360; // 320px 卡 + 40px 间距
    const rowHeight = 170; // 130px 卡 + 40px 间距
    const cols = 2;

    const writes = unpositioned.map(async (wb, i) => {
      const col = i % cols;
      const row = Math.floor(i / cols);
      const x = 100 + col * colWidth;
      const y = startY + row * rowHeight;
      initializedRef.current.add(wb.whiteboardId);
      await unwrapCommand(
        commands.layoutSetPosition(currentWhiteboardId, `wb:${wb.whiteboardId}`, x, y),
      );
    });

    Promise.all(writes).then(() => {
      queryClient.invalidateQueries({ queryKey: ["positions", ROOT_WHITEBOARD] });
    });
  }, [whiteboards, data.positions, currentWhiteboardId, allEntities, queryClient]);

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
        commands.sectionCreate(currentWhiteboardId, "New Section", null),
      );
      const pos = randomOffset();
      await unwrapCommand(
        commands.layoutSetPosition(currentWhiteboardId, result.id, pos.x, pos.y),
      );
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 section 失败:", e);
    }
  }, [queryClient, currentWhiteboardId]);

  // 创建 Note 回调
  const handleCreateNote = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.noteCreate(currentWhiteboardId, "New Note", null, null),
      );
      const pos = randomOffset();
      await unwrapCommand(
        commands.layoutSetPosition(currentWhiteboardId, result.id, pos.x, pos.y),
      );
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 note 失败:", e);
    }
  }, [queryClient, currentWhiteboardId]);

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
        currentWhiteboardId={currentWhiteboardId}
        onNavigateBack={() => setCurrentWhiteboardId(ROOT_WHITEBOARD)}
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
          {whiteboardEntities.map((wb) => {
            const summary = whiteboards.find((s) => s.whiteboardId === wb.whiteboardId);
            if (!summary) return null;
            return (
              <WhiteboardNode
                key={`wb:${wb.whiteboardId}`}
                summary={summary}
                onNavigate={setCurrentWhiteboardId}
                style={{
                  position: "absolute",
                  left: wb.position.x,
                  top: wb.position.y,
                }}
              />
            );
          })}
        </GraphCanvas>
      </div>
    </div>
  );
}
