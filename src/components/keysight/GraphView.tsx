import { useMemo, useRef, useState, useCallback, useEffect } from "react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { GraphEdges } from "@/components/keysight/GraphEdges";
import { GraphToolbar, type EntityCounts } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useWhiteboardData, useWhiteboardList } from "@/components/keysight/hooks/useWhiteboardData";
import { useVisibleEntities } from "@/components/keysight/hooks/useVisibleEntities";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import { WhiteboardNode } from "@/components/keysight/nodes/WhiteboardNode";
import { buildEdges } from "@/components/keysight/lib/buildEdges";
import { buildEntityDimensions } from "@/components/keysight/lib/buildEntityDimensions";
import { RenderedMarkdown } from "@/components/keysight/nodes/RenderedMarkdown";
import { perfLog } from "@/lib/perf";
import type {
  AliasReference,
  EntityKind,
  GraphSelection,
} from "@/components/keysight/types";
import type { SectionListItem } from "@/components/keysight/nodes/NodeContextMenu";
import { useCardActions } from "@/hooks/useCardActions";
import { useNoteActions } from "@/hooks/useNoteActions";
import { useTaskActions } from "@/hooks/useTaskActions";
import { useSectionActions } from "@/hooks/useSectionActions";
import { useAliasActions } from "@/hooks/useAliasActions";
import { useQuestionActions } from "@/hooks/useQuestionActions";
import { useLayoutActions } from "@/hooks/useLayoutActions";
import { useWhiteboardActions } from "@/hooks/useWhiteboardActions";
import { useEntityActions } from "@/hooks/useEntityActions";
import {
  viewportCenterWorld,
  visibleViewportSize,
} from "@/components/keysight/lib/graphPositioning";
import { TASK_JUMP_MIN_ZOOM } from "@/components/keysight/lib/taskLayout";
import { mergeEntitiesWithPositions } from "@/components/keysight/lib/mergeEntitiesWithPositions";
import { useEntityEditing } from "@/components/keysight/hooks/useEntityEditing";
import { useCreationModals } from "@/components/keysight/hooks/useCreationModals";
import { useDragInteraction } from "@/components/keysight/hooks/useDragInteraction";
import { useEntityContextMenu } from "@/components/keysight/hooks/useEntityContextMenu";
import { useViewportRecovery } from "@/components/keysight/hooks/useViewportRecovery";


/** 根白板 ID — rust/chentian 等是子白板，根白板显示子白板预览卡 */
export const ROOT_WHITEBOARD = "wb_root";

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
  const graphViewportRef = useRef<HTMLDivElement>(null);
  const containerSize = useContainerSize(graphViewportRef);
  const getVisibleViewportSize = useCallback(
    () => visibleViewportSize(graphViewportRef.current, containerSize),
    [containerSize],
  );
  const data = useWhiteboardData(currentWhiteboardId);
  const cards = useCardActions();
  const notes = useNoteActions(currentWhiteboardId);
  const tasks = useTaskActions(currentWhiteboardId);
  const sections = useSectionActions(currentWhiteboardId);
  const aliases = useAliasActions(currentWhiteboardId);
  const questions = useQuestionActions(currentWhiteboardId);
  const layouts = useLayoutActions(currentWhiteboardId);
  const whiteboardActions = useWhiteboardActions();
  const entities = useEntityActions(currentWhiteboardId);
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



  // 画连线状态 — ⋯ 菜单 Draw connection 后进入两阶段点击模式
  // 第一次点菜单设置 fromId；第二次点其他实体触发 entityConnect + 清空。
  // edgeType 已从 drawingState 移除：后端按 from_id 前缀强类型派发(sub-stage 2a)


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

  const {
    handleDragStart,
    lastDidDragRef,
    setLocalPositions,
    // 有效位置 = 服务器位置 + 本地覆盖(拖拽中的实时位置),由 useDragInteraction 内部 own
    effectivePositions,
  } = useDragInteraction({
    viewport,
    layouts,
    data,
    currentWhiteboardId,
    allKinds,
  });

  // 合并实体和位置（用 effectivePositions 支持拖拽实时更新）
  const allEntities = useMemo(
    () => mergeEntitiesWithPositions(data, effectivePositions),
    [data, effectivePositions],
  );

  // 实体 id → 渲染尺寸映射（GraphEdges 的 clipToRect + useVisibleEntities 视口裁剪用）
  // section 用 computeSectionBounds 算真实尺寸（SectionNode.PADDING=40 对齐），
  // 否则视口偏离 section 左上角后，section 的 placeholder 400×300 会被错误 cull
  const allDimensions = useMemo(
    () => buildEntityDimensions(allKinds, data.sections, effectivePositions, 40),
    [allKinds, data.sections, effectivePositions],
  );

  const {
    editing,
    setEditing,
    handleStartEdit,
    handleCancelEdit,
    handleCommitEdit,
  } = useEntityEditing({
    lastDidDragRef,
    cards,
    notes,
    sections,
    questions,
    tasks,
  });

  // 展开状态 — 同时只有一张卡片展开显示 body / 关联列表
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const handleToggleExpand = useCallback(
    (entityId: string) => {
      // click 之前如果发生了拖拽就不切换
      if (lastDidDragRef.current) {
        lastDidDragRef.current = false;
        return;
      }
      setExpandedId((prev) => (prev === entityId ? null : entityId));
    },
    [lastDidDragRef],
  );

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


  // 把新实体放在当前视口中心 — 避免 random offset 把卡丢到视口外
  // section 用 400×300 placeholder,note 用 520×180(NoteNode 真实尺寸)
  // 加 ±60 / ±40 抖动避免连续创建堆叠
  const newEntityPositionAtCenter = useCallback(
    (width: number, height: number) => {
      const size = getVisibleViewportSize();
      const center = viewportCenterWorld(
        viewport.state.zoom,
        viewport.state.panX,
        viewport.state.panY,
        size.width,
        size.height,
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
      getVisibleViewportSize,
    ],
  );

  const {
    drawingState,
    setDrawingState,
    relatedPickerCardId,
    setRelatedPickerCardId,
    relatedSearch,
    setRelatedSearch,
    menuHandlers,
    handleJumpToSection,
    handleJumpToTask,
    handleJumpToBoard,
    handleSync,
    handleSelectEntity,
  } = useEntityContextMenu({
    cards,
    notes,
    tasks,
    sections,
    aliases,
    questions,
    layouts,
    entities,
    data,
    currentWhiteboardId,
    effectivePositions,
    allEntities,
    allDimensions,
    entityToSectionId,
    viewport,
    getVisibleViewportSize,
    newEntityPositionAtCenter,
    lastDidDragRef,
    setEditing,
    setLocalPositions,
    onSelectEntity,
    onWhiteboardChange,
  });

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
    [containerSize],
  );

  const visibleEntities = useVisibleEntities(
    allEntities,
    viewport.state,
    renderViewportSize,
    allDimensions,
    forceVisibleIds,
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
        entityLeft < viewRight &&
        entityRight > viewLeft &&
        entityTop < viewBottom &&
        entityBottom > viewTop
      );
    });
  }, [allDimensions, allEntities, containerSize, viewport.state]);

  const { handleFitContent, handleResetSavedViewport } = useViewportRecovery({
    viewport,
    layouts,
    data,
    currentWhiteboardId,
    allEntities,
    whiteboards,
    hasEntityInViewport,
    visibleEntitiesCount: visibleEntities.length,
    getVisibleViewportSize,
  });

  // 首次进入白板时自动居中到实体的中位数位置
  // 每个白板在 session 内只尝试一次，已保存的视口由 useViewport 恢复

  // 已保存 viewport 可能指向空白区域。每个白板首次发现"有实体但当前视口无可见实体"
  // 时自动 fit 一次，避免用户看到空画布后还需要手工清 localStorage。


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
    return buildEdges(data.cards, data.notes, data.aliases, data.questions, entitySet);
  }, [data.cards, data.notes, data.aliases, data.questions, allKinds]);

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

  const {
    creatingNote,
    noteDraft,
    setNoteDraft,
    creatingQuestion,
    questionDraft,
    setQuestionDraft,
    creatingTask,
    taskDraft,
    setTaskDraft,
    creatingWhiteboard,
    whiteboardDraft,
    setWhiteboardDraft,
    handleCreateSection,
    handleCreateNote,
    handleCancelCreateNote,
    handleSubmitCreateNote,
    handleCreateQuestion,
    handleCancelCreateQuestion,
    handleSubmitCreateQuestion,
    handleCreateTask,
    handleCancelCreateTask,
    handleSubmitCreateTask,
    handlePackTasks,
    handleCreateWhiteboard,
    handleCancelCreateWhiteboard,
    handleSubmitCreateWhiteboard,
  } = useCreationModals({
    notes,
    questions,
    tasks,
    whiteboardActions,
    sections,
    layouts,
    viewport,
    currentWhiteboardId,
    data,
    effectivePositions,
    allEntities,
    allDimensions,
    newEntityPositionAtCenter,
    getVisibleViewportSize,
    onSelectEntity,
    setEditing,
    setLocalPositions,
  });


  useEffect(() => {
    if (!focusTarget) return;
    const size = getVisibleViewportSize();
    if (size.width === 0 || size.height === 0) return;
    if (handledFocusNonceRef.current === focusTarget.nonce) return;

    const entity = allEntities.find((candidate) => candidate.id === focusTarget.id);
    if (entity) {
      const dim = allDimensions[entity.id];
      viewport.actions.centerOn(
        entity.position.x,
        entity.position.y,
        size.width,
        size.height,
        dim?.width ?? 320,
        dim?.height ?? 160,
        entity.kind === "task" ? { minZoom: TASK_JUMP_MIN_ZOOM } : undefined,
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
        size.width,
        size.height,
        320,
        130,
      );
      handledFocusNonceRef.current = focusTarget.nonce;
    }
  }, [
    allDimensions,
    allEntities,
    focusTarget,
    getVisibleViewportSize,
    onSelectEntity,
    viewport.actions.centerOn,
    whiteboardEntities,
  ]);

  // Loading 状态作为 overlay 渲染，而不是 early return
  // 原因：early return 会让 graphViewportRef 无法 attach，ResizeObserver 永远收不到尺寸
  return (
    <div className="relative flex h-full w-full flex-col">
      <GraphToolbar
        viewport={viewport}
        entityCounts={entityCounts}
        onSync={handleSync}
        onCreateSection={handleCreateSection}
        onCreateNote={handleCreateNote}
        creatingNote={creatingNote}
        noteDraft={noteDraft}
        onNoteDraftChange={setNoteDraft}
        onSubmitNote={handleSubmitCreateNote}
        onCancelNote={handleCancelCreateNote}
        onCreateQuestion={handleCreateQuestion}
        creatingQuestion={creatingQuestion}
        questionDraft={questionDraft}
        onQuestionDraftChange={setQuestionDraft}
        onSubmitQuestion={handleSubmitCreateQuestion}
        onCancelQuestion={handleCancelCreateQuestion}
        onCreateTask={handleCreateTask}
        creatingTask={creatingTask}
        taskDraft={taskDraft}
        onTaskDraftChange={setTaskDraft}
        onSubmitTask={handleSubmitCreateTask}
        onCancelTask={handleCancelCreateTask}
        onCreateWhiteboard={handleCreateWhiteboard}
        creatingWhiteboard={creatingWhiteboard}
        whiteboardDraft={whiteboardDraft}
        onWhiteboardDraftChange={setWhiteboardDraft}
        onSubmitWhiteboard={handleSubmitCreateWhiteboard}
        onCancelWhiteboard={handleCancelCreateWhiteboard}
        onShowOrphans={onShowOrphans}
        onFitContent={handleFitContent}
        onPackTasks={handlePackTasks}
        onResetSavedViewport={handleResetSavedViewport}
        currentWhiteboardId={currentWhiteboardId}
        onNavigateBack={() => {
          setDrawingState(null);
          setRelatedPickerCardId(null);
          setRelatedSearch("");
          onWhiteboardChange(ROOT_WHITEBOARD);
        }}
        sections={data.sections}
        tasks={data.tasks}
        whiteboards={whiteboards}
        onJumpToSection={handleJumpToSection}
        onJumpToTask={handleJumpToTask}
        onJumpToBoard={handleJumpToBoard}
      />
      {data.isLoading && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-background/50">
          <div className="text-sm text-muted-foreground">Loading whiteboard...</div>
        </div>
      )}
      <div ref={graphViewportRef} className="relative flex-1 overflow-hidden">
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
                        entities
                          .relate(relatedPickerCardId, card.id)
                          .catch((err: unknown) => console.error("建立 Related 失败:", err));
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
