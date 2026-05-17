import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * Whiteboard 写操作。完成后 invalidate `["whiteboards"]`(列表)。
 * 查询 hook 在 `useWhiteboardData.useWhiteboardList`。
 */
export function useWhiteboardActions() {
  const qc = useQueryClient();
  const invalidate = useCallback(() => {
    qc.invalidateQueries({ queryKey: ["whiteboards"] });
  }, [qc]);

  const create = useCallback(
    async (name: string) => {
      const result = await unwrapCommand(commands.whiteboardCreate(name));
      invalidate();
      return result;
    },
    [invalidate],
  );

  return { create };
}
