import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { WhiteboardId } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * Question 实体的写操作集合。完成后 invalidate `["questions", whiteboardId]`;
 * create / delete 同时 invalidate `["positions", whiteboardId]`(节点变化)。
 */
export function useQuestionActions(whiteboardId?: string) {
  const qc = useQueryClient();
  const invalidateQuestions = useCallback(() => {
    const key = whiteboardId ? ["questions", whiteboardId] : ["questions"];
    qc.invalidateQueries({ queryKey: key });
  }, [qc, whiteboardId]);
  const invalidateQuestionsAndPositions = useCallback(() => {
    if (whiteboardId) {
      qc.invalidateQueries({ queryKey: ["questions", whiteboardId] });
      qc.invalidateQueries({ queryKey: ["positions", whiteboardId] });
    } else {
      qc.invalidateQueries({ queryKey: ["questions"] });
      qc.invalidateQueries({ queryKey: ["positions"] });
    }
  }, [qc, whiteboardId]);

  const create = useCallback(
    async (
      wb: string,
      title: string,
      content: string | null,
      status: string | null,
      color: string | null,
    ) => {
      const result = await unwrapCommand(
        commands.questionCreate(wb as WhiteboardId, title, content, status, color),
      );
      invalidateQuestionsAndPositions();
      return result;
    },
    [invalidateQuestionsAndPositions],
  );

  const update = useCallback(
    async (
      id: string,
      title: string | null,
      content: string | null,
      status: string | null,
      color: string | null,
    ) => {
      await unwrapCommand(
        commands.questionUpdate(id, title, content, status, color),
      );
      invalidateQuestions();
    },
    [invalidateQuestions],
  );

  const remove = useCallback(
    async (id: string) => {
      await unwrapCommand(commands.questionDelete(id));
      invalidateQuestionsAndPositions();
    },
    [invalidateQuestionsAndPositions],
  );

  return { create, update, remove };
}
