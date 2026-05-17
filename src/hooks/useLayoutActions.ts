import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * Layout(节点位置)写操作。完成后 invalidate `["positions", whiteboardId]`。
 *
 * 注意:很多调用点(如 GraphView 创建 entity 后立刻 setPosition)调用本 hook
 * 前已经做了 entity 相关 invalidate,本 hook 只关心 positions 自身。
 */
export function useLayoutActions(whiteboardId?: string) {
  const qc = useQueryClient();
  const invalidate = useCallback(() => {
    const key = whiteboardId ? ["positions", whiteboardId] : ["positions"];
    qc.invalidateQueries({ queryKey: key });
  }, [qc, whiteboardId]);

  /**
   * 写单个 entity 的位置。`wb` 参数允许显式传(应对跨 wb 场景如根 wb 上记录子
   * wb summary 节点位置);默认情况下应与 hook 入参 `whiteboardId` 一致。
   */
  const setPosition = useCallback(
    async (wb: string, entityId: string, x: number, y: number) => {
      await unwrapCommand(commands.layoutSetPosition(wb, entityId, x, y));
      invalidate();
    },
    [invalidate],
  );

  /** 不 invalidate 的"裸"调用,供需要批量写 + 最后统一刷新的场景使用。 */
  const setPositionWithoutInvalidate = useCallback(
    async (wb: string, entityId: string, x: number, y: number) => {
      await unwrapCommand(commands.layoutSetPosition(wb, entityId, x, y));
    },
    [],
  );

  /** 手动触发一次 positions invalidate(配合 setPositionWithoutInvalidate 批量后用) */
  const invalidatePositions = invalidate;

  return { setPosition, setPositionWithoutInvalidate, invalidatePositions };
}
