import type { QueryClient } from "@tanstack/react-query";

/**
 * 集中 invalidate 所有 task 相关 cache key —— 让 kanban 和 canvas 两边视图
 * 自动 refetch,保持与 SQLite 真源一致。
 *
 * 任何 task 写操作(create / update / delete / set_color)成功后必须调用此 helper,
 * 否则会出现两边数据漂移(L0 数据真源硬约束)。
 *
 * 如果知道具体 whiteboard_id(比如 kanban 创建 task 时能推导出 `projects/{name}`),
 * 精确 invalidate 只对该 wb 的 canvas cache;否则 fallback 到 broad invalidate。
 */
export function invalidateAllTaskCaches(
  queryClient: QueryClient,
  whiteboardId?: string,
) {
  queryClient.invalidateQueries({ queryKey: ["tasks-kanban"] });
  if (whiteboardId) {
    queryClient.invalidateQueries({ queryKey: ["tasks", whiteboardId] });
    queryClient.invalidateQueries({ queryKey: ["positions", whiteboardId] });
  } else {
    queryClient.invalidateQueries({ queryKey: ["tasks"] });
    queryClient.invalidateQueries({ queryKey: ["positions"] });
  }
}
