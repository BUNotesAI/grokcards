import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { WhiteboardId } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * Note 实体的写操作集合(create / update / delete)。完成后 invalidate
 * `["notes", whiteboardId]`(notes 按 wb 切分)。`whiteboardId` 省略时
 * broadcast invalidate(`["notes"]` 前缀匹配,清所有 wb 的 notes cache),
 * 用于跨 wb 上下文(如 sidebar detail 不知具体 wb)。
 */
export function useNoteActions(whiteboardId?: string) {
  const qc = useQueryClient();
  const invalidate = useCallback(() => {
    const key = whiteboardId ? ["notes", whiteboardId] : ["notes"];
    qc.invalidateQueries({ queryKey: key });
  }, [qc, whiteboardId]);

  const create = useCallback(
    async (
      wb: string,
      title: string,
      content: string | null,
      color: string | null,
    ) => {
      const result = await unwrapCommand(
        commands.noteCreate(wb as WhiteboardId, title, content, color),
      );
      invalidate();
      return result;
    },
    [invalidate],
  );

  const update = useCallback(
    async (
      id: string,
      title: string | null,
      content: string | null,
      color: string | null,
    ) => {
      await unwrapCommand(commands.noteUpdate(id, title, content, color));
      invalidate();
    },
    [invalidate],
  );

  const remove = useCallback(
    async (id: string) => {
      await unwrapCommand(commands.noteDelete(id));
      invalidate();
    },
    [invalidate],
  );

  return { create, update, remove };
}
