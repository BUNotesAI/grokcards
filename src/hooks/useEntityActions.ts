import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { CardId } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * 跨 entity 的图操作(connect / relate)。这两个 API 会改 edges 表,影响多个
 * entity 视图(notes / cards / sections),所以 invalidate 当前 whiteboard 下
 * 几乎全部 entity cache。
 */
export function useEntityActions(whiteboardId?: string) {
  const qc = useQueryClient();
  const invalidate = useCallback(() => {
    qc.invalidateQueries({ queryKey: ["cards"] });
    if (whiteboardId) {
      qc.invalidateQueries({ queryKey: ["notes", whiteboardId] });
      qc.invalidateQueries({ queryKey: ["sections", whiteboardId] });
      qc.invalidateQueries({ queryKey: ["aliases", whiteboardId] });
      qc.invalidateQueries({ queryKey: ["tasks", whiteboardId] });
      qc.invalidateQueries({ queryKey: ["questions", whiteboardId] });
    } else {
      qc.invalidateQueries({ queryKey: ["notes"] });
      qc.invalidateQueries({ queryKey: ["sections"] });
      qc.invalidateQueries({ queryKey: ["aliases"] });
      qc.invalidateQueries({ queryKey: ["tasks"] });
      qc.invalidateQueries({ queryKey: ["questions"] });
    }
  }, [qc, whiteboardId]);

  const connect = useCallback(
    async (fromId: string, toId: string) => {
      await unwrapCommand(commands.entityConnect(fromId, toId));
      invalidate();
    },
    [invalidate],
  );

  const relate = useCallback(
    async (fromCardId: string, toCardId: string) => {
      await unwrapCommand(commands.entityRelate(fromCardId as CardId, toCardId as CardId));
      invalidate();
    },
    [invalidate],
  );

  return { connect, relate };
}
