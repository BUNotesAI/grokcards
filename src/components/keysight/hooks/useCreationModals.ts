import { useCallback, useState, type Dispatch, type SetStateAction } from "react";
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

// 4 类 modal 的状态用 discriminated union 表达,让"同时多个 modal active"在类型层不可表达。
// kind: "idle" 显式表达"无 modal active",避免用 null/undefined 模糊态。
type Modal =
  | { kind: "idle" }
  | { kind: "note"; draft: string }
  | { kind: "question"; draft: string }
  | { kind: "task"; draft: string }
  | { kind: "whiteboard"; draft: string };

type ModalKind = "note" | "question" | "task" | "whiteboard";

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

// 从 union state 派生 4 个 (boolean, draft, setDraft) 三元组,JSX 侧 API 不变。
function modalAccessors(
  modal: Modal,
  setModal: Dispatch<SetStateAction<Modal>>,
  kind: ModalKind,
): { active: boolean; draft: string; setDraft: Dispatch<SetStateAction<string>> } {
  const setDraft: Dispatch<SetStateAction<string>> = (next) =>
    setModal((m) =>
      m.kind === kind
        ? { kind, draft: typeof next === "function" ? next(m.draft) : next }
        : m,
    );
  return {
    active: modal.kind === kind,
    draft: modal.kind === kind ? modal.draft : "",
    setDraft,
  };
}

export function useCreationModals(deps: UseCreationModalsDeps): UseCreationModalsResult {
  const {
    notes, questions, tasks, whiteboardActions, sections, layouts, viewport,
    currentWhiteboardId, data, effectivePositions, allEntities, allDimensions,
    newEntityPositionAtCenter, getVisibleViewportSize, onSelectEntity,
    setEditing, setLocalPositions,
  } = deps;

  const [modal, setModal] = useState<Modal>({ kind: "idle" });

  const note = modalAccessors(modal, setModal, "note");
  const question = modalAccessors(modal, setModal, "question");
  const task = modalAccessors(modal, setModal, "task");
  const whiteboard = modalAccessors(modal, setModal, "whiteboard");

  // section 没有 modal,直接创建 + 进入 inline 编辑
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
  }, [allDimensions, allEntities, onSelectEntity, currentWhiteboardId, newEntityPositionAtCenter, sections, layouts, setEditing]);

  const openNote = useCallback(() => setModal({ kind: "note", draft: "" }), []);
  const submitNote = useCallback(async () => {
    if (modal.kind !== "note") return;
    const title = modal.draft.trim();
    setModal({ kind: "idle" });
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
  }, [modal, onSelectEntity, currentWhiteboardId, newEntityPositionAtCenter, notes, layouts, setEditing]);

  const openQuestion = useCallback(async () => setModal({ kind: "question", draft: "" }), []);
  const submitQuestion = useCallback(async () => {
    if (modal.kind !== "question") return;
    const title = modal.draft.trim();
    setModal({ kind: "idle" });
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
  }, [modal, onSelectEntity, currentWhiteboardId, newEntityPositionAtCenter, questions, layouts, setEditing]);

  const openTask = useCallback(() => setModal({ kind: "task", draft: "" }), []);
  const submitTask = useCallback(async () => {
    if (modal.kind !== "task") return;
    const title = modal.draft.trim();
    setModal({ kind: "idle" });
    if (!title) return;
    // project 从 currentWhiteboardId 剥 "projects/" 前缀推导:按钮仅在该上下文渲染,
    // 但防御性地再校验一次,避免静默写入错 project
    if (!currentWhiteboardId.startsWith("projects/")) {
      console.error("submitTask 在非 project 白板被调用", { currentWhiteboardId });
      return;
    }
    const project = currentWhiteboardId.slice("projects/".length);
    if (!project) {
      console.error("submitTask 派生出空 project name", { currentWhiteboardId });
      return;
    }
    try {
      const fallbackPos = newEntityPositionAtCenter(320, 140);
      const pos = taskPositionAboveTopmost(data.tasks, effectivePositions, fallbackPos);
      const result = await tasks.create({
        project, title, content: null, status: "next", area: null, color: null, position: pos,
      });
      onSelectEntity?.({ id: result.id, kind: "task" });
    } catch (e) {
      console.error("创建 task 失败:", e);
    }
  }, [modal, data.tasks, effectivePositions, onSelectEntity, currentWhiteboardId, newEntityPositionAtCenter, tasks]);

  const cancelAny = useCallback(() => setModal({ kind: "idle" }), []);

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
  }, [currentWhiteboardId, data.tasks, getVisibleViewportSize, layouts, viewport.actions, viewport.state.panX, viewport.state.panY, viewport.state.zoom, setLocalPositions]);

  const openWhiteboard = useCallback(async () => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return;
    setModal({ kind: "whiteboard", draft: "" });
  }, [currentWhiteboardId]);
  const submitWhiteboard = useCallback(async () => {
    if (currentWhiteboardId !== ROOT_WHITEBOARD) return;
    if (modal.kind !== "whiteboard") return;
    const name = modal.draft.trim();
    setModal({ kind: "idle" });
    if (!name) return;
    try {
      await whiteboardActions.create(name);
    } catch (e) {
      console.error("创建 whiteboard 失败:", e);
    }
  }, [currentWhiteboardId, whiteboardActions, modal]);

  return {
    creatingNote: note.active,
    noteDraft: note.draft,
    setNoteDraft: note.setDraft,
    creatingQuestion: question.active,
    questionDraft: question.draft,
    setQuestionDraft: question.setDraft,
    creatingTask: task.active,
    taskDraft: task.draft,
    setTaskDraft: task.setDraft,
    creatingWhiteboard: whiteboard.active,
    whiteboardDraft: whiteboard.draft,
    setWhiteboardDraft: whiteboard.setDraft,
    handleCreateSection,
    handleCreateNote: openNote,
    handleCancelCreateNote: cancelAny,
    handleSubmitCreateNote: submitNote,
    handleCreateQuestion: openQuestion,
    handleCancelCreateQuestion: cancelAny,
    handleSubmitCreateQuestion: submitQuestion,
    handleCreateTask: openTask,
    handleCancelCreateTask: cancelAny,
    handleSubmitCreateTask: submitTask,
    handlePackTasks,
    handleCreateWhiteboard: openWhiteboard,
    handleCancelCreateWhiteboard: cancelAny,
    handleSubmitCreateWhiteboard: submitWhiteboard,
  };
}
