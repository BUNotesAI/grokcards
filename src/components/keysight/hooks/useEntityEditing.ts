import { useCallback, useState } from "react";
import type { Dispatch, RefObject, SetStateAction } from "react";
import type { useCardActions } from "@/hooks/useCardActions";
import type { useNoteActions } from "@/hooks/useNoteActions";
import type { useSectionActions } from "@/hooks/useSectionActions";
import type { useQuestionActions } from "@/hooks/useQuestionActions";
import type { useTaskActions } from "@/hooks/useTaskActions";

// 8 个 EditingField variant 把"实体 × 可编辑字段"两维 flatten 成单一 union,
// 让 handleCommitEdit 内 switch 不能遗漏 case(TS exhaustiveness check)。
export type EditingField =
  | "card-title"
  | "card-understanding"
  | "note-title"
  | "note-body"
  | "question-title"
  | "question-body"
  | "section-title"
  | "task-title";

export interface UseEntityEditingDeps {
  // lastDidDragRef 由 GraphView 持有(W2 抽 useDragInteraction 时连同 ref 一起搬走),
  // 这里只读 .current 判断"刚结束 drag 时不要 startEdit"。
  lastDidDragRef: RefObject<boolean>;
  cards: ReturnType<typeof useCardActions>;
  notes: ReturnType<typeof useNoteActions>;
  sections: ReturnType<typeof useSectionActions>;
  questions: ReturnType<typeof useQuestionActions>;
  tasks: ReturnType<typeof useTaskActions>;
}

export interface UseEntityEditingResult {
  editing: { id: string; field: EditingField } | null;
  // setEditing 暴露给 useCreationModals(handleCreateSection 创建后立即进入 section-title 编辑)。
  setEditing: Dispatch<SetStateAction<{ id: string; field: EditingField } | null>>;
  handleStartEdit: (id: string, field: EditingField) => void;
  handleCancelEdit: () => void;
  handleCommitEdit: (id: string, field: EditingField, value: string) => Promise<void>;
}

export function useEntityEditing(deps: UseEntityEditingDeps): UseEntityEditingResult {
  const { lastDidDragRef, cards, notes, sections, questions, tasks } = deps;
  const [editing, setEditing] = useState<{ id: string; field: EditingField } | null>(null);

  const handleStartEdit = useCallback(
    (id: string, field: EditingField) => {
      // 刚结束 drag 的 mouseup 会冒泡到 onClick;此处吞掉一次 startEdit,把 ref 复位。
      if (lastDidDragRef.current) {
        lastDidDragRef.current = false;
        return;
      }
      setEditing({ id, field });
    },
    [lastDidDragRef],
  );

  const handleCancelEdit = useCallback(() => {
    setEditing(null);
  }, []);

  // 提交编辑 — 调用对应 entity hook(hook 内自动 invalidate 对应 query cache)
  const handleCommitEdit = useCallback(
    async (id: string, field: EditingField, value: string) => {
      try {
        switch (field) {
          case "card-title":
            await cards.editTitle(id, value);
            break;
          case "card-understanding":
            await cards.updateUnderstanding(id, value);
            break;
          case "note-title":
            await notes.update(id, value, null, null);
            break;
          case "note-body":
            await notes.update(id, null, value, null);
            break;
          case "section-title":
            await sections.update(id, value, null);
            break;
          case "question-title":
            await questions.update(id, value, null, null, null);
            break;
          case "question-body":
            await questions.update(id, null, value, null, null);
            break;
          case "task-title":
            await tasks.update(id, value, null, null, null, null);
            break;
        }
      } catch (e) {
        console.error(`提交 ${field} 失败:`, e);
      } finally {
        setEditing(null);
      }
    },
    [cards, notes, sections, questions, tasks],
  );

  return { editing, setEditing, handleStartEdit, handleCancelEdit, handleCommitEdit };
}
