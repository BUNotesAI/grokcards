import { useRef, useState, useCallback, useEffect } from "react";
import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { GraphEdges } from "@/components/keysight/GraphEdges";
import { GraphToolbar } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import { useContainerSize } from "@/components/keysight/hooks/useContainerSize";
import { useWhiteboardData, useWhiteboardList } from "@/components/keysight/hooks/useWhiteboardData";
import { EntityNode } from "@/components/keysight/nodes/EntityNode";
import { WhiteboardNode } from "@/components/keysight/nodes/WhiteboardNode";
import { RenderedMarkdown } from "@/components/keysight/nodes/RenderedMarkdown";
import { perfLog } from "@/lib/perf";
import type { GraphSelection } from "@/components/keysight/types";
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
import { useEntityEditing } from "@/components/keysight/hooks/useEntityEditing";
import { useCreationModals } from "@/components/keysight/hooks/useCreationModals";
import { useDragInteraction } from "@/components/keysight/hooks/useDragInteraction";
import { useEntityContextMenu } from "@/components/keysight/hooks/useEntityContextMenu";
import { useViewportRecovery } from "@/components/keysight/hooks/useViewportRecovery";
import { useEntityIndexes } from "@/components/keysight/hooks/useEntityIndexes";
import { useGraphRenderState } from "@/components/keysight/hooks/useGraphRenderState";
import { useGraphFocus } from "@/components/keysight/hooks/useGraphFocus";


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

  // 子白板列表(仅根白板时加载)— 必须在 useEntityIndexes 之前定义,作为它的 dep
  const whiteboardListQuery = useWhiteboardList(currentWhiteboardId);
  const whiteboards = whiteboardListQuery.data ?? [];

  const {
    handleDragStart,
    lastDidDragRef,
    setLocalPositions,
    // 有效位置 = 服务器位置 + 本地覆盖(拖拽中的实时位置),由 useDragInteraction 内部 own
    effectivePositions,
  } = useDragInteraction({ viewport, layouts, data, currentWhiteboardId });

  const {
    allKinds, allEntities, allDimensions, entityToSectionId, menuSections,
    whiteboardEntities, cardsById, aliasesByTargetId, entityCounts, renderEdges,
  } = useEntityIndexes({ data, effectivePositions, currentWhiteboardId, whiteboards });

  // 合并实体和位置（用 effectivePositions 支持拖拽实时更新）
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

  // 子白板卡位置（从 positions 表读取 "wb:{id}" 格式的位置）
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

  const {
    visibleEntities, hasEntityInViewport, relatedPickerCandidates,
  } = useGraphRenderState({
    graphViewportRef, containerSize, viewport, allEntities, allDimensions,
    editing, expandedId, selectedEntityId, relatedPickerCardId,
    relatedSearch, data, effectivePositions,
  });

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
  // 所有位置映射（SectionNode bounds 计算用）
  // 当前白板的可渲染 edge 列表（从 cards/notes/aliases 数据派生）
  // 仅包含 from 和 to 都在当前白板内的 edge，避免渲染断头连线
  const hasExpandedSpotlight = expandedId !== null;


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


  useGraphFocus({
    focusTarget, allEntities, allDimensions, whiteboardEntities,
    viewport, getVisibleViewportSize, onSelectEntity,
  });

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
            positions={effectivePositions}
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
              allPositions={effectivePositions}
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
