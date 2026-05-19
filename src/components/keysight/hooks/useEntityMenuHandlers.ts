import { useMemo } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { Position } from "@/bindings";
import type { useCardActions } from "@/hooks/useCardActions";
import type { useNoteActions } from "@/hooks/useNoteActions";
import type { useTaskActions } from "@/hooks/useTaskActions";
import type { useSectionActions } from "@/hooks/useSectionActions";
import type { useAliasActions } from "@/hooks/useAliasActions";
import type { useQuestionActions } from "@/hooks/useQuestionActions";
import type { useLayoutActions } from "@/hooks/useLayoutActions";
import type { useViewport } from "@/components/keysight/useViewport";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { NodeContextMenuHandlers } from "@/components/keysight/nodes/EntityNode";
import { normalizeCardTitleForClipboard } from "@/components/keysight/lib/normalizeCardTitleForClipboard";
import type { EditingField } from "@/components/keysight/hooks/useEntityEditing";

export interface UseEntityMenuHandlersDeps {
  cards: ReturnType<typeof useCardActions>;
  notes: ReturnType<typeof useNoteActions>;
  tasks: ReturnType<typeof useTaskActions>;
  sections: ReturnType<typeof useSectionActions>;
  aliases: ReturnType<typeof useAliasActions>;
  questions: ReturnType<typeof useQuestionActions>;
  layouts: ReturnType<typeof useLayoutActions>;
  data: WhiteboardData;
  currentWhiteboardId: string;
  effectivePositions: Record<string, Position>;
  allDimensions: Record<string, { width: number; height: number }>;
  entityToSectionId: Record<string, string>;
  viewport: ReturnType<typeof useViewport>;
  getVisibleViewportSize: () => { width: number; height: number };
  setEditing: Dispatch<SetStateAction<{ id: string; field: EditingField } | null>>;
  setDrawingState: Dispatch<SetStateAction<{ fromId: string } | null>>;
  setRelatedPickerCardId: Dispatch<SetStateAction<string | null>>;
  setRelatedSearch: Dispatch<SetStateAction<string>>;
}

/**
 * ⋯ 菜单回调集合 — 稳定 reference 传给 EntityNode,memo 比较依赖它不变。
 * 依赖 data/viewport/entity-actions,数据变化时整体替换(EntityNode 整体重渲染)。
 */
export function useEntityMenuHandlers(deps: UseEntityMenuHandlersDeps): NodeContextMenuHandlers {
  const {
    cards, notes, tasks, sections, aliases, questions, layouts,
    data, currentWhiteboardId, effectivePositions, allDimensions, entityToSectionId,
    viewport, getVisibleViewportSize,
    setEditing, setDrawingState, setRelatedPickerCardId, setRelatedSearch,
  } = deps;

  return useMemo<NodeContextMenuHandlers>(
    () => ({
      // 共享:Copy UUID + normalized title(所有节点)
      // pipeline: `UUID:${id} ${normalizeCardTitleForClipboard(title)}`
      // kind 决定 entity 查找位置;Alias 借 source card 的 title
      onCopyEntityUuidTitle: (entityId, kind) => {
        let title: string | undefined;
        switch (kind) {
          case "card":
            title = data.cards.find((c) => c.id === entityId)?.title;
            break;
          case "note":
            title = data.notes.find((n) => n.id === entityId)?.title;
            break;
          case "question":
            title = data.questions.find((q) => q.id === entityId)?.title;
            break;
          case "section":
            title = data.sections.find((s) => s.id === entityId)?.title;
            break;
          case "alias": {
            const alias = data.aliases.find((a) => a.aliasId === entityId);
            if (!alias) break;
            title = data.cards.find((c) => c.id === alias.cardId)?.title;
            break;
          }
          case "task":
            title = data.tasks.find((t) => t.id === entityId)?.title;
            break;
          case "whiteboard":
            return; // whiteboard 无菜单
        }
        if (title === undefined) return;
        void navigator.clipboard.writeText(
          `UUID:${entityId} ${normalizeCardTitleForClipboard(title)}`,
        );
      },
      onDrawConnectionFrom: (id) => {
        setRelatedPickerCardId(null);
        setRelatedSearch("");
        setDrawingState({ fromId: id });
      },
      onRelatedFrom: (id) => {
        setDrawingState(null);
        setRelatedSearch("");
        setRelatedPickerCardId(id);
      },
      onCreateAlias: async (cardId) => {
        try {
          const cardPos = effectivePositions[cardId];
          const alias = await aliases.create(currentWhiteboardId, cardId);
          if (cardPos) {
            await layouts.setPosition(
              currentWhiteboardId, alias.aliasId, cardPos.x + 540, cardPos.y,
            );
          }
        } catch (e) {
          console.error("创建 alias 失败:", e);
        }
      },
      onJumpToSourceCard: (aliasId) => {
        const alias = data.aliases.find((a) => a.aliasId === aliasId);
        if (!alias) return;
        const cardPos = effectivePositions[alias.cardId];
        if (!cardPos) return;
        const size = getVisibleViewportSize();
        if (size.width === 0 || size.height === 0) return;
        const dim = allDimensions[alias.cardId];
        viewport.actions.centerOn(
          cardPos.x, cardPos.y, size.width, size.height,
          dim?.width ?? 520, dim?.height ?? 220,
        );
      },
      onDeleteAlias: async (aliasId) => {
        try { await aliases.remove(aliasId); } catch (e) { console.error("删除 alias 失败:", e); }
      },
      onEditNoteTitle: (noteId) => setEditing({ id: noteId, field: "note-title" }),
      onDeleteNote: async (noteId) => {
        try { await notes.remove(noteId); } catch (e) { console.error("删除 note 失败:", e); }
      },
      onSetNoteColor: async (noteId, color) => {
        try { await notes.update(noteId, null, null, color); } catch (e) { console.error("更新 note 颜色失败:", e); }
      },
      onEditQuestionTitle: (questionId) => setEditing({ id: questionId, field: "question-title" }),
      onDeleteQuestion: async (questionId) => {
        try { await questions.remove(questionId); } catch (e) { console.error("删除 question 失败:", e); }
      },
      onDeleteSection: async (sectionId) => {
        try { await sections.remove(sectionId); } catch (e) { console.error("删除 section 失败:", e); }
      },
      onSetSectionColor: async (sectionId, color) => {
        try { await sections.update(sectionId, null, color); } catch (e) { console.error("更新 section 颜色失败:", e); }
      },
      onMoveToSection: async (entityId, sectionId) => {
        try {
          const prevSection = entityToSectionId[entityId];
          if (prevSection && prevSection !== sectionId) {
            await sections.removeMember(prevSection, entityId);
          }
          await sections.addMember(sectionId, entityId);
        } catch (e) {
          console.error("移动到 section 失败:", e);
        }
      },
      onRemoveFromGroup: async (entityId) => {
        try {
          const sectionId = entityToSectionId[entityId];
          if (!sectionId) return;
          await sections.removeMember(sectionId, entityId);
        } catch (e) {
          console.error("从 section 移除失败:", e);
        }
      },
      onSetCardColor: async (cardId, color) => {
        try { await cards.setColor(cardId, color); } catch (e) { console.error("更新 card 颜色失败:", e); }
      },
      onSetQuestionColor: async (questionId, color) => {
        try { await questions.update(questionId, null, null, null, color); } catch (e) { console.error("更新 question 颜色失败:", e); }
      },
      onEditTaskTitle: (taskId) => setEditing({ id: taskId, field: "task-title" }),
      onDeleteTask: async (taskId) => {
        try { await tasks.remove(taskId); } catch (e) { console.error("删除 task 失败:", e); }
      },
      onSetTaskColor: async (taskId, color) => {
        try { await tasks.setColor(taskId, color); } catch (e) { console.error("更新 task 颜色失败:", e); }
      },
    }),
    [
      data.cards, data.notes, data.questions, data.aliases, data.sections, data.tasks,
      effectivePositions, entityToSectionId, allDimensions, getVisibleViewportSize,
      viewport.actions, currentWhiteboardId,
      cards, notes, sections, questions, tasks, aliases, layouts,
      setEditing, setDrawingState, setRelatedPickerCardId, setRelatedSearch,
    ],
  );
}
