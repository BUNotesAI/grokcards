import { useMemo, useRef, useState, useCallback, useEffect, type MouseEvent as ReactMouseEvent } from "react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { GraphToolbar, type EntityCounts } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
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

/** 拖拽阈值 — 小于此距离视为 click 而非 drag（屏幕像素） */
const DRAG_THRESHOLD = 4;

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
function mergeEntitiesWithPositions(
  data: WhiteboardData,
  positions: Record<string, Position>,
): EntityWithPosition[] {
  const result: EntityWithPosition[] = [];

  // Sections — 位置从成员动态计算（和旧 Obsidian 插件行为一致）
  // Section 盒子的 top-left 是 min(member.x, member.y) - PADDING
  // 如果没有任何成员有位置，section 不渲染
  const SECTION_PADDING = 40; // 和 SectionNode.PADDING 一致
  for (const section of data.sections) {
    const memberPosList = section.cardIds
      .map((id) => positions[id])
      .filter((p): p is { x: number; y: number } => p != null);

    if (memberPosList.length === 0) {
      // 无成员位置 — 回退到 section 自己的位置（兼容空 section）
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

  // 拖拽状态：ref 存储启动时的位置/屏幕坐标；state 存储拖拽中的本地位置覆盖
  const dragInfoRef = useRef<
    | null
    | {
        id: string;
        startX: number;
        startY: number;
        origX: number;
        origY: number;
        didDrag: boolean;
      }
  >(null);
  // 记录上一次 mouseup 时是否发生了拖拽 — 给 onClick 用来判断是否跳转
  const lastDidDragRef = useRef(false);
  const [localPositions, setLocalPositions] = useState<Record<string, Position>>({});

  // 有效位置 = 服务器位置 + 本地覆盖（拖拽中的实时位置）
  const effectivePositions = useMemo(
    () => ({ ...data.positions, ...localPositions }),
    [data.positions, localPositions],
  );

  // 合并实体和位置（用 effectivePositions 支持拖拽实时更新）
  const allEntities = useMemo(
    () => mergeEntitiesWithPositions(data, effectivePositions),
    [data, effectivePositions],
  );

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

  // 拖拽：节点 mousedown 触发
  const handleDragStart = useCallback(
    (e: ReactMouseEvent, entityId: string) => {
      const pos = effectivePositions[entityId];
      if (!pos) return;
      dragInfoRef.current = {
        id: entityId,
        startX: e.clientX,
        startY: e.clientY,
        origX: pos.x,
        origY: pos.y,
        didDrag: false,
      };
    },
    [effectivePositions],
  );

  // 全局 mousemove / mouseup — 拖拽中实时更新位置，释放时持久化
  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      const info = dragInfoRef.current;
      if (!info) return;
      const rawDx = e.clientX - info.startX;
      const rawDy = e.clientY - info.startY;
      if (!info.didDrag) {
        if (rawDx * rawDx + rawDy * rawDy < DRAG_THRESHOLD * DRAG_THRESHOLD) return;
        info.didDrag = true;
      }
      // 按当前 zoom 缩放屏幕坐标差 → 世界坐标差
      const zoom = viewport.state.zoom;
      const dx = rawDx / zoom;
      const dy = rawDy / zoom;
      setLocalPositions((prev) => ({
        ...prev,
        [info.id]: { x: info.origX + dx, y: info.origY + dy },
      }));
    };

    const onUp = () => {
      const info = dragInfoRef.current;
      if (!info) return;
      dragInfoRef.current = null;
      lastDidDragRef.current = info.didDrag; // 给 onClick 用
      if (!info.didDrag) return; // 没移动足够距离 — click 不持久化
      // 持久化到 DB — 使用 setLocalPositions 的回调拿到最新值
      setLocalPositions((prev) => {
        const pos = prev[info.id];
        if (pos) {
          unwrapCommand(
            commands.layoutSetPosition(currentWhiteboardId, info.id, pos.x, pos.y),
          )
            .then(() => {
              queryClient.invalidateQueries({
                queryKey: ["positions", currentWhiteboardId],
              });
            })
            .catch((err) => console.error("拖拽持久化失败:", err));
        }
        return prev;
      });
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, [viewport.state.zoom, currentWhiteboardId, queryClient]);

  // 服务器位置回来后，清理本地覆盖（避免 stale override）
  useEffect(() => {
    const toDelete: string[] = [];
    for (const [id, localPos] of Object.entries(localPositions)) {
      const serverPos = data.positions[id];
      if (
        serverPos &&
        Math.abs(serverPos.x - localPos.x) < 0.5 &&
        Math.abs(serverPos.y - localPos.y) < 0.5
      ) {
        toDelete.push(id);
      }
    }
    if (toDelete.length > 0) {
      setLocalPositions((prev) => {
        const next = { ...prev };
        for (const id of toDelete) delete next[id];
        return next;
      });
    }
  }, [data.positions, localPositions]);

  // 首次进入白板时自动居中到实体的中位数位置
  // 每个白板在 session 内只尝试一次，已保存的视口由 useViewport 恢复
  const fitAttemptedRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (data.isLoading || allEntities.length === 0 || containerSize.width === 0) return;
    if (fitAttemptedRef.current.has(currentWhiteboardId)) return;
    if (!viewport.needsFit) {
      // 有保存的视口 — 不强制 fit，但仍然标记已尝试避免后续触发
      fitAttemptedRef.current.add(currentWhiteboardId);
      return;
    }
    fitAttemptedRef.current.add(currentWhiteboardId);
    viewport.actions.fitToContent(
      allEntities.map((e) => e.position),
      containerSize.width,
      containerSize.height,
    );
  }, [
    data.isLoading,
    allEntities,
    viewport.needsFit,
    viewport.actions,
    containerSize,
    currentWhiteboardId,
  ]);

  // 搜索过滤（简单 title 匹配）
  const filteredEntities = useMemo(() => {
    if (!searchQuery.trim()) return visibleEntities;
    const q = searchQuery.toLowerCase();
    return visibleEntities.filter((e) => {
      const title = "entity" in e && "title" in e.entity ? (e.entity as any).title : "";
      return title.toLowerCase().includes(q);
    });
  }, [visibleEntities, searchQuery]);

  // cardId → AtomicCard 全局映射（CardNode/AliasNode 渲染 related/linkTo 用）
  const cardsById = useMemo(() => {
    const map: Record<string, typeof data.cards[number]> = {};
    for (const card of data.cards) {
      map[card.id] = card;
    }
    return map;
  }, [data.cards]);

  // aliases 反向索引：targetCardId → [{ aliasId, 所属 section 标题 }]
  // 旧 Obsidian 插件的 ALIASES 区域显示的是 "包含此 card 别名的 section 标题"
  const aliasesByTargetId = useMemo(() => {
    const map: Record<string, Array<{ aliasId: string; aliasTitle: string }>> = {};
    for (const alias of data.aliases) {
      if (!map[alias.cardId]) map[alias.cardId] = [];
      // 查找包含此 alias 的 section
      const containingSection = data.sections.find((s) => s.cardIds.includes(alias.aliasId));
      const title = containingSection?.title ?? alias.aliasId;
      map[alias.cardId].push({ aliasId: alias.aliasId, aliasTitle: title });
    }
    return map;
  }, [data.aliases, data.sections]);

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

  // Loading 状态作为 overlay 渲染，而不是 early return
  // 原因：early return 会让 containerRef 无法 attach，ResizeObserver 永远收不到尺寸
  return (
    <div ref={containerRef} className="relative flex h-full w-full flex-col">
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
      {data.isLoading && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-background/50">
          <div className="text-sm text-muted-foreground">Loading whiteboard...</div>
        </div>
      )}
      <div className="relative flex-1 overflow-hidden">
        <GraphCanvas viewport={viewport}>
          {filteredEntities.map((e) => (
            <EntityNode
              key={e.id}
              entity={e}
              allPositions={allPositions}
              cardsById={cardsById}
              aliasesByTargetId={aliasesByTargetId}
              onDragStart={handleDragStart}
            />
          ))}
          {whiteboardEntities.map((wb) => {
            const summary = whiteboards.find((s) => s.whiteboardId === wb.whiteboardId);
            if (!summary) return null;
            const dragId = `wb:${wb.whiteboardId}`;
            return (
              <div
                key={dragId}
                style={{
                  position: "absolute",
                  left: wb.position.x,
                  top: wb.position.y,
                }}
                onMouseDown={(e) => {
                  if (e.button !== 0) return;
                  e.stopPropagation();
                  handleDragStart(e, dragId);
                }}
                onClick={(e) => {
                  if (lastDidDragRef.current) {
                    lastDidDragRef.current = false;
                    return;
                  }
                  e.stopPropagation();
                  setCurrentWhiteboardId(wb.whiteboardId);
                }}
              >
                <WhiteboardNode
                  summary={summary}
                  onNavigate={() => {}}
                  style={{}}
                />
              </div>
            );
          })}
        </GraphCanvas>
      </div>
    </div>
  );
}
