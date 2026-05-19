import { useCallback, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { Position } from "@/bindings";
import type { useNoteActions } from "@/hooks/useNoteActions";
import type { useQuestionActions } from "@/hooks/useQuestionActions";
import type { useTaskActions } from "@/hooks/useTaskActions";
import type { useWhiteboardActions } from "@/hooks/useWhiteboardActions";
import type { useSectionActions } from "@/hooks/useSectionActions";
import type { useLayoutActions } from "@/hooks/useLayoutActions";
import type { useViewport } from "@/components/keysight/useViewport";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { EntityWithPosition, GraphSelection } from "@/components/keysight/types";
import { ROOT_WHITEBOARD } from "@/components/keysight/GraphView";
import { avoidSectionOverlap } from "@/components/keysight/lib/sectionLayout";
import {
  packTaskPositions,
  taskPositionAboveTopmost,
} from "@/components/keysight/lib/taskLayout";
import { viewportCenterWorld } from "@/components/keysight/lib/graphPositioning";
import type { EditingField } from "@/components/keysight/hooks/useEntityEditing";

export interface UseCreationModalsDeps {
  notes: ReturnType<typeof useNoteActions>;
  questions: ReturnType<typeof useQuestionActions>;
  tasks: ReturnType<typeof useTaskActions>;
  whiteboardActions: ReturnType<typeof useWhiteboardActions>;
  sections: ReturnType<typeof useSectionActions>;
  layouts: ReturnType<typeof useLayoutActions>;
  viewport: ReturnType<typeof useViewport>;
  currentWhiteboardId: string;
  data: WhiteboardData;
  effectivePositions: Record<string, Position>;
  allEntities: EntityWithPosition[];
  allDimensions: Record<string, { width: number; height: number }>;
  newEntityPositionAtCenter: (width: number, height: number) => { x: number; y: number };
  getVisibleViewportSize: () => { width: number; height: number };
  onSelectEntity: ((selection: GraphSelection | null) => void) | undefined;
  setEditing: Dispatch<SetStateAction<{ id: string; field: EditingField } | null>>;
  // setLocalPositions 在 W2 抽 useDragInteraction 后会重整,这里暂以 setter 形式 inject。
  setLocalPositions: Dispatch<SetStateAction<Record<string, Position>>>;
}

export interface UseCreationModalsResult {
  creatingNote: boolean;
  noteDraft: string;
  setNoteDraft: Dispatch<SetStateAction<string>>;
  creatingQuestion: boolean;
  questionDraft: string;
  setQuestionDraft: Dispatch<SetStateAction<string>>;
  creatingTask: boolean;
  taskDraft: string;
  setTaskDraft: Dispatch<SetStateAction<string>>;
  creatingWhiteboard: boolean;
  whiteboardDraft: string;
  setWhiteboardDraft: Dispatch<SetStateAction<string>>;

  handleCreateSection: () => Promise<void>;
  handleCreateNote: () => void;
  handleCancelCreateNote: () => void;
  handleSubmitCreateNote: () => Promise<void>;
  handleCreateQuestion: () => Promise<void>;
  handleCancelCreateQuestion: () => void;
  handleSubmitCreateQuestion: () => Promise<void>;
  handleCreateTask: () => void;
  handleCancelCreateTask: () => void;
  handleSubmitCreateTask: () => Promise<void>;
  handlePackTasks: () => Promise<void>;
  handleCreateWhiteboard: () => Promise<void>;
  handleCancelCreateWhiteboard: () => void;
  handleSubmitCreateWhiteboard: () => Promise<void>;
}

export function useCreationModals(deps: UseCreationModalsDeps): UseCreationModalsResult {
  const {
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
  } = deps;

  const [creatingNote, setCreatingNote] = useState(false);
  const [noteDraft, setNoteDraft] = useState("");
  const [creatingQuestion, setCreatingQuestion] = useState(false);
  const [questionDraft, setQuestionDraft] = useState("");
  const [creatingWhiteboard, setCreatingWhiteboard] = useState(false);
  const [whiteboardDraft, setWhiteboardDraft] = useState("");
  const [creatingTask, setCreatingTask] = useState(false);
  const [taskDraft, setTaskDraft] = useState("");

  // 创建 Section 回调
  const handleCreateSection = useCallback(async () => {
    try {
      const result = await sections.create(currentWhiteboardId, "New Section", null);
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
      const pos = avoidSectionOverlap(
        preferredPos,
        { width: 400, height: 300 },
        existingSectionRects,
      );
      await layouts.setPosition(currentWhiteboardId, result.id, pos.x, pos.y);
      onSelectEntity?.({ id: result.id, kind: "section" });
      setEditing({ id: result.id, field: "section-title" });
    } catch (e) {
      console.error("创建 section 失败:", e);
    }
  }, [
    allDimensions,
    allEntities,
    onSelectEntity,
    currentWhiteboardId,
    newEntityPositionAtCenter,
    sections,
    layouts,
    setEditing,
  ]);

  // 创建 Note 回调
  const handleCreateNote = useCallback(() => {
    setCreatingWhiteboard(false);
    setWhiteboardDraft("");
    setCreatingQuestion(false);
    setQuestionDraft("");
    setCreatingTask(false);
    setTaskDraft("");
    setNoteDraft("");
    setCreatingNote(true);
  }, []);

  const handleCancelCreateNote = useCallback(() => {
    setCreatingNote(false);
    setNoteDraft("");
  }, []);

  const handleSubmitCreateNote = useCallback(async () => {
    const title = noteDraft.trim();
    setCreatingNote(false);
    setNoteDraft("");
    if (!title) return;

    try {
      const result = await notes.create(currentWhiteboardId, title, null, null);
      const pos = newEntityPositionAtCenter(520, 180);
      await layouts.setPosition(currentWhiteboardId, result.id, pos.x, pos.y);
      onSelectEntity?.({ id: result.id, kind: "note" });
      setEditing({ id: result.id, field: "note-body" });
    } catch (e) {
      console.error("创建 note 失败:", e);
    }
  }, [
    noteDraft,
    onSelectEntity,
    currentWhiteboardId,
    newEntityPositionAtCenter,
    notes,
    layouts,
    setEditing,
  ]);

  const handleCreateQuestion = useCallback(async () => {
    setCreatingWhiteboard(false);
    setWhiteboardDraft("");
    setCreatingNote(false);
    setNoteDraft("");
    setCreatingTask(false);
    setTaskDraft("");
    setQuestionDraft("");
    setCreatingQuestion(true);
  }, []);

  const handleCancelCreateQuestion = useCallback(() => {
    setCreatingQuestion(false);
    setQuestionDraft("");
  }, []);

  const handleSubmitCreateQuestion = useCallback(async () => {
    const title = questionDraft.trim();
    setCreatingQuestion(false);
    setQuestionDraft("");
    if (!title) return;

    try {
      const result = await questions.create(currentWhiteboardId, title, null, null, null);
      const pos = newEntityPositionAtCenter(320, 140);
      await layouts.setPosition(currentWhiteboardId, result.id, pos.x, pos.y);
      onSelectEntity?.({ id: result.id, kind: "question" });
      setEditing({ id: result.id, field: "question-body" });
    } catch (e) {
      console.error("创建 question 失败:", e);
    }
  }, [
    questionDraft,
    onSelectEntity,
    currentWhiteboardId,
    newEntityPositionAtCenter,
    questions,
    layouts,
    setEditing,
  ]);

  // 创建 Task 回调 — 仅在 currentWhiteboardId 形如 "projects/{name}" 时由 GraphToolbar 触发
  const handleCreateTask = useCallback(() => {
    setCreatingWhiteboard(false);
    setWhiteboardDraft("");
    setCreatingNote(false);
    setNoteDraft("");
    setCreatingQuestion(false);
    setQuestionDraft("");
    setTaskDraft("");
    setCreatingTask(true);
  }, []);

  const handleCancelCreateTask = useCallback(() => {
    setCreatingTask(false);
    setTaskDraft("");
  }, []);

  const handleSubmitCreateTask = useCallback(async () => {
    const title = taskDraft.trim();
    setCreatingTask(false);
    setTaskDraft("");
    if (!title) return;

    // project 从 currentWhiteboardId 剥 "projects/" 前缀推导:按钮仅在该上下文渲染,
    // 但防御性地再校验一次,避免静默写入错 project
    const projectPrefix = "projects/";
    if (!currentWhiteboardId.startsWith(projectPrefix)) {
      console.error("handleSubmitCreateTask 在非 project 白板被调用", { currentWhiteboardId });
      return;
    }
    const project = currentWhiteboardId.slice(projectPrefix.length);
    if (!project) {
      console.error("handleSubmitCreateTask 派生出空 project name", { currentWhiteboardId });
      return;
    }

    try {
      const fallbackPos = newEntityPositionAtCenter(320, 140);
      const pos = taskPositionAboveTopmost(data.tasks, effectivePositions, fallbackPos);
      const result = await tasks.create({
        project,
        title,
        content: null,
        status: "next",
        area: null,
        color: null,
        position: pos,
      });
      onSelectEntity?.({ id: result.id, kind: "task" });
    } catch (e) {
      console.error("创建 task 失败:", e);
    }
  }, [
    taskDraft,
    data.tasks,
    effectivePositions,
    onSelectEntity,
    currentWhiteboardId,
    newEntityPositionAtCenter,
    tasks,
  ]);

  const handlePackTasks = useCallback(async () => {
    if (!currentWhiteboardId.startsWith("projects/")) return;
    if (data.tasks.length === 0) return;
    const size = getVisibleViewportSize();
    if (size.width === 0 || size.height === 0) return;

    const confirmed = window.confirm(
      "Pack all project tasks into visible columns? This rewrites task positions only.",
    );
    if (!confirmed) return;

    const center = viewportCenterWorld(
      viewport.state.zoom,
      viewport.state.panX,
      viewport.state.panY,
      size.width,
      size.height,
    );
    const packed = packTaskPositions(data.tasks, center);
    const packedPositions = Object.values(packed);

    setLocalPositions((prev) => ({ ...prev, ...packed }));

    try {
      // Promise.all 并发写,部分失败不 rollback 已成功的写入。catch 块
      // invalidate 让 query refetch 拿回 server 真实状态,UI 显示 partial。
      // 如未来需要原子语义,加 layout_set_positions_bulk command 走单事务。
      await Promise.all(
        Object.entries(packed).map(([taskId, position]) =>
          layouts.setPositionWithoutInvalidate(
            currentWhiteboardId,
            taskId,
            position.x,
            position.y,
          ),
        ),
      );
      layouts.invalidatePositions();
      viewport.actions.fitToContent(packedPositions, size.width, size.height);
    } catch (e) {
      console.error("整理 task 位置失败:", e);
      layouts.invalidatePositions();
    }
  }, [
    currentWhiteboardId,
    data.tasks,
    getVisibleViewportSize,
    layouts,
    viewport.actions,
    viewport.state.panX,
    viewport.state.panY,
    viewport.state.zoom,
    setLocalPositions,
  ]);

  const handleCreateWhiteboard = useCallback(async () => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return;
    setCreatingNote(false);
    setNoteDraft("");
    setCreatingQuestion(false);
    setQuestionDraft("");
    setCreatingTask(false);
    setTaskDraft("");
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
      await whiteboardActions.create(name);
    } catch (e) {
      console.error("创建 whiteboard 失败:", e);
    }
  }, [currentWhiteboardId, whiteboardActions, whiteboardDraft]);

  return {
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
  };
}
