import { useCallback } from "react";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { commands, type Subtask, type TaskCreateRequest, type TaskId } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * 集中 invalidate 所有 task 相关 cache key —— 让 kanban 和 canvas 两边视图
 * 自动 refetch,保持与 SQLite 真源一致。
 *
 * 已知 wb 时精确 invalidate;否则 fallback 到 broad invalidate(整 entity 类)。
 */
function invalidateTaskCaches(qc: QueryClient, whiteboardId?: string) {
  qc.invalidateQueries({ queryKey: ["tasks-kanban"] });
  if (whiteboardId) {
    qc.invalidateQueries({ queryKey: ["tasks", whiteboardId] });
    qc.invalidateQueries({ queryKey: ["positions", whiteboardId] });
  } else {
    qc.invalidateQueries({ queryKey: ["tasks"] });
    qc.invalidateQueries({ queryKey: ["positions"] });
  }
}

/**
 * Task 实体的写操作集合。任何写后自动 invalidate kanban + canvas 两边的
 * `["tasks-kanban"]` / `["tasks", wb]` / `["positions", wb]` 三类 cache。
 *
 * `whiteboardId` 可省;未传时按 broad invalidate(用于 kanban 端不知道具体 wb 的场景)。
 */
export function useTaskActions(whiteboardId?: string) {
  const qc = useQueryClient();
  const invalidate = useCallback(
    () => invalidateTaskCaches(qc, whiteboardId),
    [qc, whiteboardId],
  );

  const create = useCallback(
    async (input: TaskCreateRequest) => {
      const result = await unwrapCommand(commands.taskCreate(input));
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
      status: "inbox" | "next" | "active" | "done" | "blocked" | null,
      area: string | null,
      color: string | null,
    ) => {
      await unwrapCommand(
        commands.taskUpdate(id as TaskId, title, content, status, area, color),
      );
      invalidate();
    },
    [invalidate],
  );

  const updateWithSubtasks = useCallback(
    async (
      id: string,
      title: string | null,
      subtasks: Subtask[],
      status: "inbox" | "next" | "active" | "done" | "blocked" | null,
      area: string | null,
      color: string | null,
    ) => {
      const result = await unwrapCommand(
        commands.taskUpdateWithSubtasks(id as TaskId, title, subtasks, status, area, color),
      );
      invalidate();
      return result;
    },
    [invalidate],
  );

  const remove = useCallback(
    async (id: string) => {
      await unwrapCommand(commands.taskDelete(id as TaskId));
      invalidate();
    },
    [invalidate],
  );

  const setColor = useCallback(
    async (id: string, color: string) => {
      await unwrapCommand(commands.taskSetColor(id as TaskId, color));
      invalidate();
    },
    [invalidate],
  );

  return { create, update, updateWithSubtasks, remove, setColor };
}
