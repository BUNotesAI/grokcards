import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { CardId } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * Card 实体的写操作集合。所有操作完成后自动 invalidate `["cards"]`
 * (cards 是全局实体,不按 whiteboard 切分)。
 *
 * 调用点用法:
 * ```ts
 * const cards = useCardActions();
 * await cards.editTitle(id, "new title");
 * ```
 */
export function useCardActions() {
  const qc = useQueryClient();
  const invalidate = useCallback(() => {
    qc.invalidateQueries({ queryKey: ["cards"] });
  }, [qc]);

  const editTitle = useCallback(
    async (id: string, title: string) => {
      await unwrapCommand(commands.cardEditTitle(id as CardId, title));
      invalidate();
    },
    [invalidate],
  );

  const updateUnderstanding = useCallback(
    async (id: string, value: string) => {
      await unwrapCommand(commands.cardUpdateUnderstanding(id as CardId, value));
      invalidate();
    },
    [invalidate],
  );

  const editBody = useCallback(
    async (id: string, content: string) => {
      await unwrapCommand(commands.cardEditBody(id as CardId, content));
      invalidate();
    },
    [invalidate],
  );

  const setColor = useCallback(
    async (id: string, color: string) => {
      await unwrapCommand(commands.cardSetColor(id as CardId, color));
      invalidate();
    },
    [invalidate],
  );

  return { editTitle, updateUnderstanding, editBody, setColor };
}
