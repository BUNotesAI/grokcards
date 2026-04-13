import { useMemo, useRef, useState, useCallback, useEffect, type MouseEvent as ReactMouseEvent } from "react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { GraphEdges } from "@/components/keysight/GraphEdges";
import { GraphToolbar, type EntityCounts } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useWhiteboardData, useWhiteboardList, type WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import { WhiteboardNode } from "@/components/keysight/nodes/WhiteboardNode";
import { buildEdges } from "@/components/keysight/lib/buildEdges";
import { buildEntityDimensions } from "@/components/keysight/lib/buildEntityDimensions";
import { normalizeCardTitleForClipboard } from "@/components/keysight/lib/normalizeCardTitleForClipboard";
import { RenderedMarkdown } from "@/components/keysight/nodes/RenderedMarkdown";
import { perfLog } from "@/lib/perf";
import type {
  AliasReference,
  EntityKind,
  EntityWithPosition,
  GraphSelection,
} from "@/components/keysight/types";
import type { EdgeType, Position } from "@/bindings";
import type { NodeContextMenuHandlers } from "@/components/keysight/nodes/EntityNode";
import type { SectionListItem } from "@/components/keysight/nodes/NodeContextMenu";
import { unwrapCommand } from "@/lib/commandResult";
import { commands } from "@/bindings";
import { useQueryClient } from "@tanstack/react-query";

/** 拖拽阈值 — 小于此距离视为 click 而非 drag（屏幕像素） */
const DRAG_THRESHOLD = 4;

/** 根白板 ID — rust/chentian 等是子白板，根白板显示子白板预览卡 */
export const ROOT_WHITEBOARD = "wb_root";

/** 把屏幕中心转成世界坐标 — 给"在视口中央创建新实体"用 */
function viewportCenterWorld(
  zoom: number,
  panX: number,
  panY: number,
  containerWidth: number,
  containerHeight: number,
): { x: number; y: number } {
  return {
    x: (-panX + containerWidth / 2) / zoom,
    y: (-panY + containerHeight / 2) / zoom,
  };
}

interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

function rectsOverlap(a: Rect, b: Rect): boolean {
  return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
}

function avoidSectionOverlap(
  preferred: { x: number; y: number },
  size: { width: number; height: number },
  existingRects: Rect[],
  gap = 40,
): { x: number; y: number } {
  let next = { ...preferred };

  for (let i = 0; i < existingRects.length * 2 + 1; i += 1) {
    const candidate: Rect = {
      left: next.x,
      top: next.y,
      right: next.x + size.width,
      bottom: next.y + size.height,
    };
    const hit = existingRects.find((rect) => rectsOverlap(candidate, rect));
    if (!hit) return next;
    next = {
      x: hit.right + gap,
      y: hit.top,
    };
  }

  return next;
}

export interface GraphFocusTarget {
  id: string;
  nonce: number;
}

function drawingHint(): string {
  return "Draw connection mode: click another node to create the edge";
}

interface GraphViewProps {
  currentWhiteboardId: string;
  onWhiteboardChange: (whiteboardId: string) => void;
  selectedEntityId?: string | null;
  onSelectEntity?: (selection: GraphSelection | null) => void;
  highlightedEntityIds?: Set<string>;
  focusTarget?: GraphFocusTarget | null;
  onShowOrphans?: () => void;
  onSearchTag?: (tag: string) => void;
  onOpenCard?: (cardId: string) => void;
  onOpenAlias?: (aliasId: string) => void;
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
export function GraphView({
  currentWhiteboardId,
  onWhiteboardChange,
  selectedEntityId = null,
  onSelectEntity,
  highlightedEntityIds,
  focusTarget = null,
  onShowOrphans,
  onSearchTag,
  onOpenCard,
  onOpenAlias,
}: GraphViewProps) {
  const viewport = useViewport(currentWhiteboardId);
  const containerRef = useRef<HTMLDivElement>(null);
  const containerSize = useContainerSize(containerRef);
  const data = useWhiteboardData(currentWhiteboardId);
  const queryClient = useQueryClient();
  const handledFocusNonceRef = useRef<number | null>(null);

  // 启动性能：mount + isLoading 转为 false 的时刻
  const mountedRef = useRef(false);
  if (!mountedRef.current) {
    mountedRef.current = true;
    perfLog("GraphView mount (first render)");
  }
  const wasLoadingRef = useRef(true);
  useEffect(() => {
    if (wasLoadingRef.current && !data.isLoading) {
      wasLoadingRef.current = false;
      perfLog(
        `GraphView data ready: cards=${data.cards.length} notes=${data.notes.length} sections=${data.sections.length} aliases=${data.aliases.length} tasks=${data.tasks.length} questions=${data.questions.length} positions=${Object.keys(data.positions).length}`,
      );
    }
  }, [data.isLoading, data.cards.length, data.notes.length, data.sections.length, data.aliases.length, data.tasks.length, data.questions.length, data.positions]);

  // 拖拽状态：ref 存储启动时的位置/屏幕坐标 + 被拖拽节点的 DOM 引用
  // mousemove 时直接 imperative 更新 el.style.transform，绕过 React 重渲染延迟
  // 同时也调用 setLocalPositions 让 React state 跟上（保持单向数据流一致性）
  const dragInfoRef = useRef<
    | null
    | {
        kind: "single";
        id: string;
        el: HTMLElement;
        startX: number;
        startY: number;
        origX: number;
        origY: number;
        didDrag: boolean;
      }
    | {
        kind: "section";
        id: string;
        startX: number;
        startY: number;
        origPositions: Record<string, Position>;
        movedIds: string[];
        didDrag: boolean;
      }
  >(null);
  // 记录上一次 mouseup 时是否发生了拖拽 — 给 onClick 用来判断是否跳转
  const lastDidDragRef = useRef(false);
  const [localPositions, setLocalPositions] = useState<Record<string, Position>>({});

  // 展开状态 — 同时只有一张卡片展开显示 body / 关联列表
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [creatingWhiteboard, setCreatingWhiteboard] = useState(false);
  const [whiteboardDraft, setWhiteboardDraft] = useState("");
  const handleToggleExpand = useCallback((entityId: string) => {
    // click 之前如果发生了拖拽就不切换
    if (lastDidDragRef.current) {
      lastDidDragRef.current = false;
      return;
    }
    setExpandedId((prev) => (prev === entityId ? null : entityId));
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (event.defaultPrevented) return;
      setExpandedId((prev) => (prev === null ? prev : null));
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  // 行内编辑状态：同时只能编辑一个字段
  // field 区分：card-title / card-understanding / note-title / note-body / section-title
  type EditingField =
    | "card-title"
    | "card-understanding"
    | "note-title"
    | "note-body"
    | "question-title"
    | "question-body"
    | "section-title";
  const [editing, setEditing] = useState<{ id: string; field: EditingField } | null>(null);

  // 画连线状态 — ⋯ 菜单 Draw connection 后进入两阶段点击模式
  // 第一次点菜单设置 fromId；第二次点其他实体触发 entityConnect(LinkTo) + 清空
  const [drawingState, setDrawingState] = useState<
    { fromId: string; edgeType: EdgeType } | null
  >(null);
  // Related picker — 对齐旧 Obsidian 交互：点击菜单后弹出候选列表，直接选择目标 card
  const [relatedPickerCardId, setRelatedPickerCardId] = useState<string | null>(null);
  const [relatedSearch, setRelatedSearch] = useState("");

  // 白板切换时终止画线模式，避免跨白板残留到下一次点击。
  useEffect(() => {
    setDrawingState(null);
    setRelatedPickerCardId(null);
    setRelatedSearch("");
  }, [currentWhiteboardId]);

  const handleStartEdit = useCallback((id: string, field: EditingField) => {
    if (lastDidDragRef.current) {
      lastDidDragRef.current = false;
      return;
    }
    setEditing({ id, field });
  }, []);

  const handleCancelEdit = useCallback(() => {
    setEditing(null);
  }, []);

  // 提交编辑 — 调用对应 Rust command 写回 + invalidate 查询
  const handleCommitEdit = useCallback(
    async (id: string, field: EditingField, value: string) => {
      try {
        switch (field) {
          case "card-title":
            await unwrapCommand(commands.cardEditTitle(id, value));
            break;
          case "card-understanding":
            await unwrapCommand(commands.cardUpdateUnderstanding(id, value));
            break;
          case "note-title":
            await unwrapCommand(commands.noteUpdate(id, value, null, null));
            break;
          case "note-body":
            await unwrapCommand(commands.noteUpdate(id, null, value, null));
            break;
          case "section-title":
            await unwrapCommand(commands.sectionUpdate(id, value, null));
            break;
          case "question-title":
            await unwrapCommand(commands.questionUpdate(id, value, null, null));
            break;
          case "question-body":
            await unwrapCommand(commands.questionUpdate(id, null, value, null));
            break;
        }
        if (field === "card-title" || field === "card-understanding") {
          queryClient.invalidateQueries({ queryKey: ["cards"] });
        } else if (field === "section-title") {
          queryClient.invalidateQueries({ queryKey: ["sections", currentWhiteboardId] });
        } else if (field === "question-title" || field === "question-body") {
          queryClient.invalidateQueries({ queryKey: ["questions", currentWhiteboardId] });
        } else {
          queryClient.invalidateQueries({ queryKey: ["notes", currentWhiteboardId] });
        }
      } catch (e) {
        console.error(`提交 ${field} 失败:`, e);
      } finally {
        setEditing(null);
      }
    },
    [queryClient, currentWhiteboardId],
  );

  // entityId → 所在 section id 的反向索引
  // 用于 ⋯ 菜单判断 "Remove from group" 是否显示
  const entityToSectionId = useMemo(() => {
    const map: Record<string, string> = {};
    for (const section of data.sections) {
      for (const memberId of section.cardIds) {
        map[memberId] = section.id;
      }
    }
    return map;
  }, [data.sections]);

  // 当前白板的 sections 精简列表（Move to Section 子菜单用）
  const menuSections = useMemo<SectionListItem[]>(
    () => data.sections.map((s) => ({ id: s.id, title: s.title })),
    [data.sections],
  );

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

  // 所有实体 id → kind 映射（SectionNode 按 kind 查询真实尺寸算 bounds 用）
  const allKinds = useMemo(() => {
    const kinds: Record<string, EntityKind> = {};
    for (const c of data.cards) kinds[c.id] = "card";
    for (const n of data.notes) kinds[n.id] = "note";
    for (const t of data.tasks) kinds[t.id] = "task";
    for (const q of data.questions) kinds[q.id] = "question";
    for (const s of data.sections) kinds[s.id] = "section";
    for (const a of data.aliases) kinds[a.aliasId] = "alias";
    return kinds;
  }, [data.cards, data.notes, data.tasks, data.questions, data.sections, data.aliases]);

  // 实体 id → 渲染尺寸映射（GraphEdges 的 clipToRect + useVisibleEntities 视口裁剪用）
  // section 用 computeSectionBounds 算真实尺寸（SectionNode.PADDING=40 对齐），
  // 否则视口偏离 section 左上角后，section 的 placeholder 400×300 会被错误 cull
  const allDimensions = useMemo(
    () => buildEntityDimensions(allKinds, data.sections, effectivePositions, 40),
    [allKinds, data.sections, effectivePositions],
  );

  // 子白板列表（仅根白板时加载）
  const whiteboardListQuery = useWhiteboardList(currentWhiteboardId);
  const whiteboards = whiteboardListQuery.data ?? [];

  // 子白板卡位置（从 positions 表读取 "wb:{id}" 格式的位置）
  const whiteboardEntities = useMemo(() => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return [];
    return whiteboards
      .map((wb) => {
        const pos = effectivePositions[`wb:${wb.whiteboardId}`];
        return pos ? { whiteboardId: wb.whiteboardId, position: pos } : null;
      })
      .filter(Boolean) as Array<{ whiteboardId: string; position: { x: number; y: number } }>;
  }, [whiteboards, effectivePositions, currentWhiteboardId]);

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

  // 视口裁剪 — 传入 allDimensions 让 section 用真实 bounds 而非 placeholder
  const forceVisibleIds = useMemo(() => {
    const ids = new Set<string>();
    if (expandedId) ids.add(expandedId);
    if (editing?.id) ids.add(editing.id);
    if (selectedEntityId) ids.add(selectedEntityId);
    if (relatedPickerCardId) ids.add(relatedPickerCardId);
    return ids;
  }, [editing, expandedId, selectedEntityId, relatedPickerCardId]);

  const visibleEntities = useVisibleEntities(
    allEntities,
    viewport.state,
    containerSize,
    allDimensions,
    forceVisibleIds,
  );

  // 启动性能：第一次有 visible entities
  const firstVisiblePaintRef = useRef(false);
  useEffect(() => {
    if (!firstVisiblePaintRef.current && visibleEntities.length > 0) {
      firstVisiblePaintRef.current = true;
      perfLog(
        `GraphView first visible paint: visible=${visibleEntities.length}/${allEntities.length}`,
      );
    }
  }, [visibleEntities.length, allEntities.length]);

  // 用 ref 跟随 effectivePositions，让 handleDragStart 保持 stable reference
  // 避免 onDragStart prop 每次渲染都变导致 EntityNode 全量 re-render
  const effectivePositionsRef = useRef(effectivePositions);
  useEffect(() => {
    effectivePositionsRef.current = effectivePositions;
  }, [effectivePositions]);

  const allKindsRef = useRef(allKinds);
  useEffect(() => {
    allKindsRef.current = allKinds;
  }, [allKinds]);

  const sectionsRef = useRef(data.sections);
  useEffect(() => {
    sectionsRef.current = data.sections;
  }, [data.sections]);

  // 拖拽：节点 mousedown 触发（stable reference）
  // 捕获被拖动节点的 wrapper DOM（e.currentTarget），mousemove 时直接更新它的 transform
  const handleDragStart = useCallback((e: ReactMouseEvent, entityId: string) => {
    if (allKindsRef.current[entityId] === "section") {
      const section = sectionsRef.current.find((item) => item.id === entityId);
      if (!section) return;

      const movedIds = section.cardIds.filter((id) => effectivePositionsRef.current[id] != null);
      const effectiveMovedIds = movedIds.length > 0 ? movedIds : [entityId];
      const origPositions: Record<string, Position> = {};
      for (const id of effectiveMovedIds) {
        const pos = effectivePositionsRef.current[id];
        if (pos) origPositions[id] = pos;
      }
      if (Object.keys(origPositions).length === 0) return;

      dragInfoRef.current = {
        kind: "section",
        id: entityId,
        startX: e.clientX,
        startY: e.clientY,
        origPositions,
        movedIds: Object.keys(origPositions),
        didDrag: false,
      };
      return;
    }

    const pos = effectivePositionsRef.current[entityId];
    if (!pos) return;
    dragInfoRef.current = {
      kind: "single",
      id: entityId,
      el: e.currentTarget as HTMLElement,
      startX: e.clientX,
      startY: e.clientY,
      origX: pos.x,
      origY: pos.y,
      didDrag: false,
    };
  }, []);

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
      if (info.kind === "single") {
        const newX = info.origX + dx;
        const newY = info.origY + dy;
        // Fast path：直接更新 DOM transform，每帧都立即响应
        // 不依赖 React 的 render 调度，避免任何 batching/scheduling 延迟导致视觉滞后
        info.el.style.transform = `translate3d(${newX}px, ${newY}px, 0)`;
        // Slow path：同步更新 React state 让其他依赖（section bounds 等）跟上
        setLocalPositions((prev) => ({
          ...prev,
          [info.id]: { x: newX, y: newY },
        }));
        return;
      }

      setLocalPositions((prev) => {
        const next = { ...prev };
        for (const id of info.movedIds) {
          const orig = info.origPositions[id];
          next[id] = {
            x: orig.x + dx,
            y: orig.y + dy,
          };
        }
        return next;
      });
    };

    const onUp = () => {
      const info = dragInfoRef.current;
      if (!info) return;
      dragInfoRef.current = null;
      lastDidDragRef.current = info.didDrag; // 给 onClick 用
      if (!info.didDrag) return; // 没移动足够距离 — click 不持久化
      // 持久化到 DB — 使用 setLocalPositions 的回调拿到最新值
      setLocalPositions((prev) => {
        const persistIds = info.kind === "single" ? [info.id] : info.movedIds;
        const writes: Promise<unknown>[] = [];
        for (const id of persistIds) {
          const pos = prev[id];
          if (!pos) continue;
          writes.push(
            unwrapCommand(
              commands.layoutSetPosition(currentWhiteboardId, id, pos.x, pos.y),
            ),
          );
        }

        if (writes.length > 0) {
          Promise.all(writes)
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
    const map: Record<string, AliasReference[]> = {};
    for (const alias of data.aliases) {
      if (!map[alias.cardId]) map[alias.cardId] = [];
      // 查找包含此 alias 的 section
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

  // 所有位置映射（SectionNode bounds 计算用）
  const allPositions = effectivePositions;

  // 当前白板的可渲染 edge 列表（从 cards/notes/aliases 数据派生）
  // 仅包含 from 和 to 都在当前白板内的 edge，避免渲染断头连线
  const renderEdges = useMemo(() => {
    const entitySet = new Set<string>(Object.keys(allKinds));
    return buildEdges(data.cards, data.notes, data.aliases, entitySet);
  }, [data.cards, data.notes, data.aliases, allKinds]);

  const hasExpandedSpotlight = expandedId !== null;

  const relatedPickerCandidates = useMemo(() => {
    if (!relatedPickerCardId) return [];

    const sourceCard = data.cards.find((card) => card.id === relatedPickerCardId);
    if (!sourceCard) return [];

    const excludedIds = new Set<string>([relatedPickerCardId]);
    for (const targetId of sourceCard.related ?? []) {
      excludedIds.add(targetId);
    }
    for (const card of data.cards) {
      if (card.related?.includes(relatedPickerCardId)) {
        excludedIds.add(card.id);
      }
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

  // 把新实体放在当前视口中心 — 避免 random offset 把卡丢到视口外
  // section 用 400×300 placeholder，note 用 520×180（NoteNode 真实尺寸）
  // 加 ±60 / ±40 抖动避免连续创建堆叠
  const newEntityPositionAtCenter = useCallback(
    (width: number, height: number) => {
      const center = viewportCenterWorld(
        viewport.state.zoom,
        viewport.state.panX,
        viewport.state.panY,
        containerSize.width,
        containerSize.height,
      );
      const jitterX = (Math.random() - 0.5) * 120;
      const jitterY = (Math.random() - 0.5) * 80;
      return {
        x: center.x - width / 2 + jitterX,
        y: center.y - height / 2 + jitterY,
      };
    },
    [
      viewport.state.zoom,
      viewport.state.panX,
      viewport.state.panY,
      containerSize.width,
      containerSize.height,
    ],
  );

  // 创建 Section 回调
  const handleCreateSection = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.sectionCreate(currentWhiteboardId, "New Section", null),
      );
      const preferredPos = newEntityPositionAtCenter(400, 300);
      const existingSectionRects = allEntities
        .filter((entity) => entity.kind === "section")
        .map((entity) => {
          const dim = allDimensions[entity.id] ?? { width: 400, height: 300 };
          return {
            left: entity.position.x,
            top: entity.position.y,
            right: entity.position.x + dim.width,
            bottom: entity.position.y + dim.height,
          };
        });
      const pos = avoidSectionOverlap(preferredPos, { width: 400, height: 300 }, existingSectionRects);
      await unwrapCommand(
        commands.layoutSetPosition(currentWhiteboardId, result.id, pos.x, pos.y),
      );
      onSelectEntity?.({ id: result.id, kind: "section" });
      setEditing({ id: result.id, field: "section-title" });
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 section 失败:", e);
    }
  }, [allDimensions, allEntities, onSelectEntity, queryClient, currentWhiteboardId, newEntityPositionAtCenter]);

  // 创建 Note 回调
  const handleCreateNote = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.noteCreate(currentWhiteboardId, "New Note", null, null),
      );
      const pos = newEntityPositionAtCenter(520, 180);
      await unwrapCommand(
        commands.layoutSetPosition(currentWhiteboardId, result.id, pos.x, pos.y),
      );
      onSelectEntity?.({ id: result.id, kind: "note" });
      setEditing({ id: result.id, field: "note-body" });
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 note 失败:", e);
    }
  }, [onSelectEntity, queryClient, currentWhiteboardId, newEntityPositionAtCenter]);

  const handleCreateQuestion = useCallback(async () => {
    try {
      const result = await unwrapCommand(
        commands.questionCreate(currentWhiteboardId, "New Question", null, null),
      );
      const pos = newEntityPositionAtCenter(320, 140);
      await unwrapCommand(
        commands.layoutSetPosition(currentWhiteboardId, result.id, pos.x, pos.y),
      );
      onSelectEntity?.({ id: result.id, kind: "question" });
      setEditing({ id: result.id, field: "question-body" });
      queryClient.invalidateQueries();
    } catch (e) {
      console.error("创建 question 失败:", e);
    }
  }, [onSelectEntity, queryClient, currentWhiteboardId, newEntityPositionAtCenter]);

  const handleCreateWhiteboard = useCallback(async () => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return;
    setWhiteboardDraft("");
    setCreatingWhiteboard(true);
  }, [currentWhiteboardId]);

  const handleCancelCreateWhiteboard = useCallback(() => {
    setCreatingWhiteboard(false);
    setWhiteboardDraft("");
  }, []);

  const handleSubmitCreateWhiteboard = useCallback(async () => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return;

    const name = whiteboardDraft.trim();
    setCreatingWhiteboard(false);
    setWhiteboardDraft("");
    if (!name) return;

    try {
      await unwrapCommand(commands.whiteboardCreate(name));
      queryClient.invalidateQueries({ queryKey: ["whiteboards"] });
    } catch (e) {
      console.error("创建 whiteboard 失败:", e);
    }
  }, [currentWhiteboardId, queryClient, whiteboardDraft]);

  // ⋯ 菜单回调集合 — 稳定 reference 传给 EntityNode，memo 比较依赖它不变
  // 依赖 data/viewport/queryClient，数据变化时整体替换（EntityNode 整体重渲染）
  const menuHandlers = useMemo<NodeContextMenuHandlers>(
    () => ({
      // Card: Copy title → 复制到剪贴板（去掉 markdown 加粗 + 转义反斜杠）
      onCopyCardTitle: (id) => {
        const card = data.cards.find((c) => c.id === id);
        if (!card) return;
        void navigator.clipboard.writeText(normalizeCardTitleForClipboard(card.title));
      },
      // Card/Alias: Draw connection → 进入 LinkTo 模式，等待下一次点击目标实体
      onDrawConnectionFrom: (id) => {
        setRelatedPickerCardId(null);
        setRelatedSearch("");
        setDrawingState({ fromId: id, edgeType: "LinkTo" });
      },
      // Card: Related → 打开 source card 右侧的 picker
      onRelatedFrom: (id) => {
        setDrawingState(null);
        setRelatedSearch("");
        setRelatedPickerCardId(id);
      },
      // Card: Create alias → 在原卡右侧 540px 位置创建 alias
      onCreateAlias: async (cardId) => {
        try {
          const cardPos = effectivePositions[cardId];
          const alias = await unwrapCommand(
            commands.aliasCreate(currentWhiteboardId, cardId),
          );
          if (cardPos) {
            await unwrapCommand(
              commands.layoutSetPosition(
                currentWhiteboardId,
                alias.aliasId,
                cardPos.x + 540,
                cardPos.y,
              ),
            );
          }
          queryClient.invalidateQueries();
        } catch (e) {
          console.error("创建 alias 失败:", e);
        }
      },
      // Alias: Jump to source card → 居中到目标 card
      onJumpToSourceCard: (aliasId) => {
        const alias = data.aliases.find((a) => a.aliasId === aliasId);
        if (!alias) return;
        const cardPos = effectivePositions[alias.cardId];
        if (!cardPos) return;
        const dim = allDimensions[alias.cardId];
        viewport.actions.centerOn(
          cardPos.x,
          cardPos.y,
          containerSize.width,
          containerSize.height,
          dim?.width ?? 520,
          dim?.height ?? 220,
        );
      },
      // Alias: Delete alias
      onDeleteAlias: async (aliasId) => {
        try {
          await unwrapCommand(commands.aliasDelete(aliasId));
          queryClient.invalidateQueries();
        } catch (e) {
          console.error("删除 alias 失败:", e);
        }
      },
      // Note: Copy UUID + title
      onCopyNoteUuidTitle: (noteId) => {
        const note = data.notes.find((n) => n.id === noteId);
        if (!note) return;
        void navigator.clipboard.writeText(`UUID:${note.id} ${note.title}`);
      },
      // Note: Edit title → 复用已有 inline 编辑
      onEditNoteTitle: (noteId) => setEditing({ id: noteId, field: "note-title" }),
      // Note: Delete
      onDeleteNote: async (noteId) => {
        try {
          await unwrapCommand(commands.noteDelete(noteId));
          queryClient.invalidateQueries();
        } catch (e) {
          console.error("删除 note 失败:", e);
        }
      },
      // Note: Set background color
      onSetNoteColor: async (noteId, color) => {
        try {
          await unwrapCommand(commands.noteUpdate(noteId, null, null, color));
          queryClient.invalidateQueries();
        } catch (e) {
          console.error("更新 note 颜色失败:", e);
        }
      },
      // Section: Delete
      onDeleteSection: async (sectionId) => {
        try {
          await unwrapCommand(commands.sectionDelete(sectionId));
          queryClient.invalidateQueries();
        } catch (e) {
          console.error("删除 section 失败:", e);
        }
      },
      // 共享: Move to Section — 先从旧 section 移除再加入新 section
      onMoveToSection: async (entityId, sectionId) => {
        try {
          const prevSection = entityToSectionId[entityId];
          if (prevSection && prevSection !== sectionId) {
            await unwrapCommand(commands.sectionRemoveMember(prevSection, entityId));
          }
          await unwrapCommand(commands.sectionAddMember(sectionId, entityId));
          queryClient.invalidateQueries();
        } catch (e) {
          console.error("移动到 section 失败:", e);
        }
      },
      // 共享: Remove from group — 从当前所属 section 移除
      onRemoveFromGroup: async (entityId) => {
        try {
          const sectionId = entityToSectionId[entityId];
          if (!sectionId) return;
          await unwrapCommand(commands.sectionRemoveMember(sectionId, entityId));
          queryClient.invalidateQueries();
        } catch (e) {
          console.error("从 section 移除失败:", e);
        }
      },
    }),
    [
      data.cards,
      data.notes,
      data.aliases,
      effectivePositions,
      entityToSectionId,
      allDimensions,
      containerSize,
      viewport.actions,
      currentWhiteboardId,
      queryClient,
      setRelatedPickerCardId,
      setRelatedSearch,
    ],
  );

  // Sync 回调
  // 跳转到指定 section — 用 allDimensions 算出真实尺寸后调 viewport.centerOn
  const handleJumpToSection = useCallback(
    (sectionId: string) => {
      const sectionEntity = allEntities.find(
        (e) => e.kind === "section" && e.id === sectionId,
      );
      if (!sectionEntity) return;
      const dim = allDimensions[sectionId];
      if (!dim) return;
      viewport.actions.centerOn(
        sectionEntity.position.x,
        sectionEntity.position.y,
        containerSize.width,
        containerSize.height,
        dim.width,
        dim.height,
      );
    },
    [allEntities, allDimensions, containerSize, viewport.actions],
  );

  // 跳转到指定白板 — 直接切 currentWhiteboardId
  const handleJumpToBoard = useCallback(
    (whiteboardId: string) => {
      setDrawingState(null);
      setRelatedPickerCardId(null);
      setRelatedSearch("");
      onWhiteboardChange(whiteboardId);
    },
    [onWhiteboardChange],
  );

  const handleSync = useCallback(() => {
    data.syncVault.mutate();
  }, [data.syncVault]);

  const handleSelectEntity = useCallback(
    (selection: GraphSelection) => {
      if (lastDidDragRef.current) {
        lastDidDragRef.current = false;
        return;
      }
      if (relatedPickerCardId) {
        setRelatedPickerCardId(null);
        setRelatedSearch("");
      }
      // 画连线模式：第一次点 ⋯ 菜单设置 drawingState，下一次点目标实体触发 entityConnect
      if (drawingState && drawingState.fromId !== selection.id) {
        const { fromId, edgeType } = drawingState;
        setDrawingState(null);
        unwrapCommand(
          commands.entityConnect(fromId, selection.id, edgeType, null, null),
        )
          .then(() => {
            queryClient.invalidateQueries();
          })
          .catch((err) => console.error(`建立 ${edgeType} 边失败:`, err));
        return;
      }
      // 画连线模式下再点自己：取消 drawing 模式
      if (drawingState && drawingState.fromId === selection.id) {
        setDrawingState(null);
        return;
      }
      onSelectEntity?.(selection);
    },
    [onSelectEntity, drawingState, queryClient, relatedPickerCardId],
  );

  useEffect(() => {
    if (!focusTarget || containerSize.width === 0 || containerSize.height === 0) return;
    if (handledFocusNonceRef.current === focusTarget.nonce) return;

    const entity = allEntities.find((candidate) => candidate.id === focusTarget.id);
    if (entity) {
      const dim = allDimensions[entity.id];
      viewport.actions.centerOn(
        entity.position.x,
        entity.position.y,
        containerSize.width,
        containerSize.height,
        dim?.width ?? 320,
        dim?.height ?? 160,
      );
      handledFocusNonceRef.current = focusTarget.nonce;
      onSelectEntity?.({ id: entity.id, kind: entity.kind });
      return;
    }

    const whiteboard = whiteboardEntities.find((candidate) => `wb:${candidate.whiteboardId}` === focusTarget.id);
    if (whiteboard) {
      viewport.actions.centerOn(
        whiteboard.position.x,
        whiteboard.position.y,
        containerSize.width,
        containerSize.height,
        320,
        130,
      );
      handledFocusNonceRef.current = focusTarget.nonce;
    }
  }, [
    allDimensions,
    allEntities,
    containerSize.height,
    containerSize.width,
    focusTarget,
    onSelectEntity,
    viewport.actions.centerOn,
    whiteboardEntities,
  ]);

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
        onCreateQuestion={handleCreateQuestion}
        onCreateWhiteboard={handleCreateWhiteboard}
        creatingWhiteboard={creatingWhiteboard}
        whiteboardDraft={whiteboardDraft}
        onWhiteboardDraftChange={setWhiteboardDraft}
        onSubmitWhiteboard={handleSubmitCreateWhiteboard}
        onCancelWhiteboard={handleCancelCreateWhiteboard}
        onShowOrphans={onShowOrphans}
        currentWhiteboardId={currentWhiteboardId}
        onNavigateBack={() => {
          setDrawingState(null);
          setRelatedPickerCardId(null);
          setRelatedSearch("");
          onWhiteboardChange(ROOT_WHITEBOARD);
        }}
        sections={data.sections}
        whiteboards={whiteboards}
        onJumpToSection={handleJumpToSection}
        onJumpToBoard={handleJumpToBoard}
      />
      {data.isLoading && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-background/50">
          <div className="text-sm text-muted-foreground">Loading whiteboard...</div>
        </div>
      )}
      <div className="relative flex-1 overflow-hidden">
        {drawingState && (
          <div className="pointer-events-none absolute left-4 top-3 z-40 rounded-md border border-sky-200 bg-sky-50/95 px-3 py-1.5 text-xs font-medium text-sky-900 shadow-sm">
            {drawingHint()}
          </div>
        )}
        <GraphCanvas
          viewport={viewport}
          cursor={drawingState ? "crosshair" : "grab"}
          backgroundColor={hasExpandedSpotlight ? "#2c2c2e" : undefined}
        >
          {/* edges 渲染在 entity 节点之下作为背景层 */}
          <GraphEdges
            edges={renderEdges}
            positions={allPositions}
            dimensions={allDimensions}
            opacity={hasExpandedSpotlight ? 0.08 : 1}
          />
          {visibleEntities.map((e) => (
            (() => {
              const highlightedDimmed = Boolean(
                highlightedEntityIds &&
                highlightedEntityIds.size > 0 &&
                !highlightedEntityIds.has(e.id) &&
                selectedEntityId !== e.id,
              );
              const spotlightDimmed = hasExpandedSpotlight && expandedId !== e.id;
              return (
            <EntityNode
              key={e.id}
              entity={e}
              allPositions={allPositions}
              allKinds={allKinds}
              cardsById={cardsById}
              aliasesByTargetId={aliasesByTargetId}
              lodLevel={viewport.lodLevel}
              selected={selectedEntityId === e.id}
              highlighted={highlightedEntityIds?.has(e.id) ?? false}
              dimmed={highlightedDimmed}
              spotlightDimmed={spotlightDimmed}
              onDragStart={handleDragStart}
              onSelect={handleSelectEntity}
              isExpanded={expandedId === e.id}
              onToggleExpand={handleToggleExpand}
              editing={editing && editing.id === e.id ? editing.field : null}
              onStartEdit={handleStartEdit}
              onCommitEdit={handleCommitEdit}
              onCancelEdit={handleCancelEdit}
              menuHandlers={menuHandlers}
              menuSections={menuSections}
              currentSectionId={entityToSectionId[e.id] ?? null}
              onOpenCard={onOpenCard}
              onOpenAlias={onOpenAlias}
              onSelectTag={onSearchTag}
            />
              );
            })()
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
                  left: 0,
                  top: 0,
                  transform: `translate3d(${wb.position.x}px, ${wb.position.y}px, 0)`,
                  willChange: "transform",
                  opacity: hasExpandedSpotlight ? 0.1 : 1,
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
                  setDrawingState(null);
                  setRelatedPickerCardId(null);
                  setRelatedSearch("");
                  onWhiteboardChange(wb.whiteboardId);
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
          {relatedPickerCardId && (() => {
            const sourcePos = effectivePositions[relatedPickerCardId];
            if (!sourcePos) return null;

            return (
              <div
                data-testid="related-picker"
                style={{
                  position: "absolute",
                  left: 0,
                  top: 0,
                  width: 320,
                  maxHeight: 320,
                  transform: `translate3d(${sourcePos.x + 528}px, ${sourcePos.y}px, 0)`,
                  borderRadius: 10,
                  border: "1px solid rgba(15, 23, 42, 0.12)",
                  background: "rgba(255, 255, 255, 0.98)",
                  boxShadow: "0 18px 40px rgba(15, 23, 42, 0.16)",
                  overflow: "hidden",
                  pointerEvents: "auto",
                }}
                onMouseDown={(e) => e.stopPropagation()}
                onClick={(e) => e.stopPropagation()}
              >
                <input
                  autoFocus
                  placeholder="Search cards..."
                  value={relatedSearch}
                  onChange={(e) => setRelatedSearch(e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") {
                      setRelatedPickerCardId(null);
                      setRelatedSearch("");
                    }
                  }}
                  style={{
                    width: "100%",
                    border: "none",
                    borderBottom: "1px solid rgba(15, 23, 42, 0.08)",
                    padding: "10px 12px",
                    fontSize: 13,
                    outline: "none",
                    background: "transparent",
                  }}
                />
                <div style={{ maxHeight: 272, overflowY: "auto" }}>
                  {relatedPickerCandidates.map((card) => (
                    <button
                      key={card.id}
                      data-testid={`related-picker-item-${card.id}`}
                      type="button"
                      onClick={() => {
                        setRelatedPickerCardId(null);
                        setRelatedSearch("");
                        unwrapCommand(
                          commands.entityConnect(
                            relatedPickerCardId,
                            card.id,
                            "Related",
                            null,
                            null,
                          ),
                        )
                          .then(() => {
                            queryClient.invalidateQueries();
                          })
                          .catch((err) => console.error("建立 Related 失败:", err));
                      }}
                      style={{
                        display: "block",
                        width: "100%",
                        border: "none",
                        borderBottom: "1px solid rgba(15, 23, 42, 0.05)",
                        padding: "10px 12px",
                        textAlign: "left",
                        background: "transparent",
                        cursor: "pointer",
                      }}
                    >
                      <div
                        style={{
                          fontSize: 13,
                          fontWeight: 600,
                          color: "#1f2937",
                        }}
                      >
                        <RenderedMarkdown markdown={card.title} variant="title" />
                      </div>
                    </button>
                  ))}
                  {relatedPickerCandidates.length === 0 && (
                    <div
                      style={{
                        padding: "12px",
                        fontSize: 12,
                        color: "#6b7280",
                      }}
                    >
                      No matches
                    </div>
                  )}
                </div>
              </div>
            );
          })()}
        </GraphCanvas>
      </div>
    </div>
  );
}
