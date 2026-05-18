import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { CardId, WhiteboardId } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * Alias 实体的写操作集合。完成后 invalidate `["aliases", whiteboardId]`
 * + `["positions", whiteboardId]`(alias 创建会落新节点 position)
 * + `["cards"]`(alias 的 owning card 关系变化可能影响 card 查询)。
 */
export function useAliasActions(whiteboardId?: string) {
  const qc = useQueryClient();
  const invalidate = useCallback(() => {
    if (whiteboardId) {
      qc.invalidateQueries({ queryKey: ["aliases", whiteboardId] });
      qc.invalidateQueries({ queryKey: ["positions", whiteboardId] });
    } else {
      qc.invalidateQueries({ queryKey: ["aliases"] });
      qc.invalidateQueries({ queryKey: ["positions"] });
    }
    qc.invalidateQueries({ queryKey: ["cards"] });
  }, [qc, whiteboardId]);

  const create = useCallback(
    async (wb: string, cardId: string) => {
      const result = await unwrapCommand(commands.aliasCreate(wb as WhiteboardId, cardId as CardId));
      invalidate();
      return result;
    },
    [invalidate],
  );

  const remove = useCallback(
    async (id: string) => {
      await unwrapCommand(commands.aliasDelete(id));
      invalidate();
    },
    [invalidate],
  );

  return { create, remove };
}
