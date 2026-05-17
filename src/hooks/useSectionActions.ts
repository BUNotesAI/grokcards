import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { WhiteboardId } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/**
 * Section 实体的写操作集合(含成员管理)。`whiteboardId` 省略时 broadcast
 * invalidate(前缀匹配清所有 wb 的 sections/positions),用于跨 wb 上下文。
 */
export function useSectionActions(whiteboardId?: string) {
  const qc = useQueryClient();
  const invalidateSections = useCallback(() => {
    const key = whiteboardId ? ["sections", whiteboardId] : ["sections"];
    qc.invalidateQueries({ queryKey: key });
  }, [qc, whiteboardId]);
  const invalidateSectionsAndPositions = useCallback(() => {
    if (whiteboardId) {
      qc.invalidateQueries({ queryKey: ["sections", whiteboardId] });
      qc.invalidateQueries({ queryKey: ["positions", whiteboardId] });
    } else {
      qc.invalidateQueries({ queryKey: ["sections"] });
      qc.invalidateQueries({ queryKey: ["positions"] });
    }
  }, [qc, whiteboardId]);

  const create = useCallback(
    async (wb: string, title: string, color: string | null) => {
      const result = await unwrapCommand(
        commands.sectionCreate(wb as WhiteboardId, title, color),
      );
      invalidateSectionsAndPositions();
      return result;
    },
    [invalidateSectionsAndPositions],
  );

  const update = useCallback(
    async (id: string, title: string | null, color: string | null) => {
      await unwrapCommand(commands.sectionUpdate(id, title, color));
      invalidateSections();
    },
    [invalidateSections],
  );

  const remove = useCallback(
    async (id: string) => {
      await unwrapCommand(commands.sectionDelete(id));
      invalidateSectionsAndPositions();
    },
    [invalidateSectionsAndPositions],
  );

  const addMember = useCallback(
    async (sectionId: string, entityId: string) => {
      await unwrapCommand(commands.sectionAddMember(sectionId, entityId));
      invalidateSectionsAndPositions();
    },
    [invalidateSectionsAndPositions],
  );

  const removeMember = useCallback(
    async (sectionId: string, entityId: string) => {
      await unwrapCommand(commands.sectionRemoveMember(sectionId, entityId));
      invalidateSectionsAndPositions();
    },
    [invalidateSectionsAndPositions],
  );

  return { create, update, remove, addMember, removeMember };
}
