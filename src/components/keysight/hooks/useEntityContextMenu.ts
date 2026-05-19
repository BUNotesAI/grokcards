import { useCallback, useEffect, useState } from "react";
import type { Dispatch, RefObject, SetStateAction } from "react";
import type { Position } from "@/bindings";
import type { useCardActions } from "@/hooks/useCardActions";
import type { useNoteActions } from "@/hooks/useNoteActions";
import type { useTaskActions } from "@/hooks/useTaskActions";
import type { useSectionActions } from "@/hooks/useSectionActions";
import type { useAliasActions } from "@/hooks/useAliasActions";
import type { useQuestionActions } from "@/hooks/useQuestionActions";
import type { useLayoutActions } from "@/hooks/useLayoutActions";
import type { useEntityActions } from "@/hooks/useEntityActions";
import type { useViewport } from "@/components/keysight/useViewport";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { EntityWithPosition, GraphSelection } from "@/components/keysight/types";
import type { NodeContextMenuHandlers } from "@/components/keysight/nodes/EntityNode";
import { SECTION_PADDING } from "@/components/keysight/lib/sectionLayout";
import {
  TASK_JUMP_MIN_ZOOM, TASK_NODE_HEIGHT, TASK_NODE_WIDTH, taskPositionAboveTopmost,
} from "@/components/keysight/lib/taskLayout";
import type { EditingField } from "@/components/keysight/hooks/useEntityEditing";
import { useEntityMenuHandlers } from "@/components/keysight/hooks/useEntityMenuHandlers";

export interface UseEntityContextMenuDeps {
  cards: ReturnType<typeof useCardActions>;
  notes: ReturnType<typeof useNoteActions>;
  tasks: ReturnType<typeof useTaskActions>;
  sections: ReturnType<typeof useSectionActions>;
  aliases: ReturnType<typeof useAliasActions>;
  questions: ReturnType<typeof useQuestionActions>;
  layouts: ReturnType<typeof useLayoutActions>;
  entities: ReturnType<typeof useEntityActions>;
  data: WhiteboardData;
  currentWhiteboardId: string;
  effectivePositions: Record<string, Position>;
  allEntities: EntityWithPosition[];
  allDimensions: Record<string, { width: number; height: number }>;
  entityToSectionId: Record<string, string>;
  viewport: ReturnType<typeof useViewport>;
  getVisibleViewportSize: () => { width: number; height: number };
  newEntityPositionAtCenter: (width: number, height: number) => { x: number; y: number };
  lastDidDragRef: RefObject<boolean>;
  setEditing: Dispatch<SetStateAction<{ id: string; field: EditingField } | null>>;
  setLocalPositions: Dispatch<SetStateAction<Record<string, Position>>>;
  onSelectEntity: ((selection: GraphSelection | null) => void) | undefined;
  onWhiteboardChange: (whiteboardId: string) => void;
}

export interface UseEntityContextMenuResult {
  drawingState: { fromId: string } | null;
  setDrawingState: Dispatch<SetStateAction<{ fromId: string } | null>>;
  relatedPickerCardId: string | null;
  setRelatedPickerCardId: Dispatch<SetStateAction<string | null>>;
  relatedSearch: string;
  setRelatedSearch: Dispatch<SetStateAction<string>>;
  menuHandlers: NodeContextMenuHandlers;
  handleJumpToSection: (sectionId: string) => void;
  handleJumpToTask: (taskId: string) => Promise<void>;
  handleJumpToBoard: (whiteboardId: string) => void;
  handleSync: () => void;
  handleSelectEntity: (selection: GraphSelection) => void;
}

export function useEntityContextMenu(deps: UseEntityContextMenuDeps): UseEntityContextMenuResult {
  const {
    cards, notes, tasks, sections, aliases, questions, layouts, entities,
    data, currentWhiteboardId, effectivePositions, allEntities, allDimensions,
    entityToSectionId, viewport, getVisibleViewportSize, newEntityPositionAtCenter,
    lastDidDragRef, setEditing, setLocalPositions, onSelectEntity, onWhiteboardChange,
  } = deps;

  // 画连线状态 — ⋯ 菜单 Draw connection 后进入两阶段点击模式
  // 第一次点菜单设置 fromId;第二次点其他实体触发 entityConnect + 清空。
  // edgeType 已从 drawingState 移除:后端按 from_id 前缀强类型派发(sub-stage 2a)
  const [drawingState, setDrawingState] = useState<{ fromId: string } | null>(null);
  // Related picker — 对齐旧 Obsidian 交互:点击菜单后弹出候选列表,直接选择目标 card
  const [relatedPickerCardId, setRelatedPickerCardId] = useState<string | null>(null);
  const [relatedSearch, setRelatedSearch] = useState("");

  // 白板切换时终止画线模式,避免跨白板残留到下一次点击。
  useEffect(() => {
    setDrawingState(null);
    setRelatedPickerCardId(null);
    setRelatedSearch("");
  }, [currentWhiteboardId]);

  const menuHandlers = useEntityMenuHandlers({
    cards, notes, tasks, sections, aliases, questions, layouts,
    data, currentWhiteboardId, effectivePositions, allDimensions, entityToSectionId,
    viewport, getVisibleViewportSize,
    setEditing, setDrawingState, setRelatedPickerCardId, setRelatedSearch,
  });

  // 跳转到指定 section — 用 allDimensions 算出真实尺寸后调 viewport.centerOn
  const handleJumpToSection = useCallback(
    (sectionId: string) => {
      const size = getVisibleViewportSize();
      if (size.width === 0 || size.height === 0) return;
      const dim = allDimensions[sectionId] ?? { width: 400, height: 300 };
      const sectionEntity = allEntities.find(
        (e) => e.kind === "section" && e.id === sectionId,
      );
      if (sectionEntity) {
        viewport.actions.centerOn(
          sectionEntity.position.x, sectionEntity.position.y,
          size.width, size.height, dim.width, dim.height,
        );
        return;
      }
      const section = data.sections.find((item) => item.id === sectionId);
      if (!section) return;
      const memberPosList = section.cardIds
        .map((id) => effectivePositions[id])
        .filter((p): p is { x: number; y: number } => p != null);
      if (memberPosList.length > 0) {
        const minX = Math.min(...memberPosList.map((p) => p.x));
        const minY = Math.min(...memberPosList.map((p) => p.y));
        viewport.actions.centerOn(
          minX - SECTION_PADDING, minY - SECTION_PADDING,
          size.width, size.height, dim.width, dim.height,
        );
        return;
      }
      const ownPos = effectivePositions[sectionId];
      if (!ownPos) return;
      viewport.actions.centerOn(ownPos.x, ownPos.y, size.width, size.height, dim.width, dim.height);
    },
    [allDimensions, allEntities, data.sections, effectivePositions, getVisibleViewportSize, viewport.actions],
  );

  const handleJumpToTask = useCallback(
    async (taskId: string) => {
      const task = data.tasks.find((item) => item.id === taskId);
      if (!task) return;
      const size = getVisibleViewportSize();
      if (size.width === 0 || size.height === 0) return;
      let position = effectivePositions[taskId];
      if (!position) {
        const fallbackPos = newEntityPositionAtCenter(TASK_NODE_WIDTH, TASK_NODE_HEIGHT);
        position = taskPositionAboveTopmost(data.tasks, effectivePositions, fallbackPos);
        setLocalPositions((prev) => ({ ...prev, [taskId]: position }));
        try {
          await layouts.setPosition(currentWhiteboardId, taskId, position.x, position.y);
        } catch (e) {
          console.error("定位 task 失败:", e);
          layouts.invalidatePositions();
          return;
        }
      }
      const dim = allDimensions[taskId] ?? { width: TASK_NODE_WIDTH, height: TASK_NODE_HEIGHT };
      viewport.actions.centerOn(
        position.x, position.y, size.width, size.height, dim.width, dim.height,
        { minZoom: TASK_JUMP_MIN_ZOOM },
      );
      onSelectEntity?.({ id: taskId, kind: "task" });
    },
    [
      allDimensions, currentWhiteboardId, data.tasks, effectivePositions, getVisibleViewportSize,
      newEntityPositionAtCenter, onSelectEntity, layouts, viewport.actions, setLocalPositions,
    ],
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
      // 画连线模式:第一次点 ⋯ 菜单设置 drawingState,下一次点目标实体触发 entityConnect
      if (drawingState && drawingState.fromId !== selection.id) {
        const { fromId } = drawingState;
        setDrawingState(null);
        entities
          .connect(fromId, selection.id)
          .catch((err) => console.error("建立连线失败:", err));
        return;
      }
      // 画连线模式下再点自己:取消 drawing 模式
      if (drawingState && drawingState.fromId === selection.id) {
        setDrawingState(null);
        return;
      }
      onSelectEntity?.(selection);
    },
    [onSelectEntity, drawingState, entities, relatedPickerCardId, lastDidDragRef],
  );

  return {
    drawingState, setDrawingState,
    relatedPickerCardId, setRelatedPickerCardId,
    relatedSearch, setRelatedSearch,
    menuHandlers,
    handleJumpToSection, handleJumpToTask, handleJumpToBoard, handleSync, handleSelectEntity,
  };
}
