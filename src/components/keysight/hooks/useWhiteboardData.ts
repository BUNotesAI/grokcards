import { useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type {
  AtomicCard,
  GraphSection,
  GraphNote,
  CardAlias,
  TaskEntity,
  QuestionEntity,
  Position,
} from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";

/** useWhiteboardData 返回的数据结构 */
export interface WhiteboardData {
  cards: AtomicCard[];
  sections: GraphSection[];
  notes: GraphNote[];
  aliases: CardAlias[];
  tasks: TaskEntity[];
  questions: QuestionEntity[];
  positions: Record<string, Position>;
  isLoading: boolean;
  syncVault: { mutate: () => void; isPending: boolean };
}

/**
 * 加载指定白板的所有实体数据。
 *
 * 用 TanStack Query 封装 Tauri commands，白板切换时自动重新加载。
 * syncVault mutation 成功后 invalidateQueries 刷新全部。
 */
export function useWhiteboardData(whiteboardId: string): WhiteboardData {
  const queryClient = useQueryClient();

  // cards 是全局实体，不按白板过滤
  const cardsQuery = useQuery({
    queryKey: ["cards"],
    queryFn: () => unwrapCommand(commands.cardQueryAll(null, null)),
  });

  const sectionsQuery = useQuery({
    queryKey: ["sections", whiteboardId],
    queryFn: () => unwrapCommand(commands.sectionQueryAll(whiteboardId)),
  });

  const notesQuery = useQuery({
    queryKey: ["notes", whiteboardId],
    queryFn: () => unwrapCommand(commands.noteQueryAll(whiteboardId)),
  });

  const aliasesQuery = useQuery({
    queryKey: ["aliases", whiteboardId],
    queryFn: () => unwrapCommand(commands.aliasQueryAll(whiteboardId)),
  });

  const tasksQuery = useQuery({
    queryKey: ["tasks", whiteboardId],
    queryFn: () => unwrapCommand(commands.taskQueryAll(whiteboardId)),
  });

  const questionsQuery = useQuery({
    queryKey: ["questions", whiteboardId],
    queryFn: () => unwrapCommand(commands.questionQueryAll(whiteboardId)),
  });

  const positionsQuery = useQuery({
    queryKey: ["positions", whiteboardId],
    queryFn: () => unwrapCommand(commands.layoutQueryPositions(whiteboardId)),
  });

  const syncVaultMutation = useMutation({
    mutationFn: () => unwrapCommand(commands.syncVault()),
    onSuccess: () => {
      queryClient.invalidateQueries();
    },
  });

  const isLoading = useMemo(
    () =>
      cardsQuery.isLoading ||
      sectionsQuery.isLoading ||
      notesQuery.isLoading ||
      aliasesQuery.isLoading ||
      tasksQuery.isLoading ||
      questionsQuery.isLoading ||
      positionsQuery.isLoading,
    [
      cardsQuery.isLoading,
      sectionsQuery.isLoading,
      notesQuery.isLoading,
      aliasesQuery.isLoading,
      tasksQuery.isLoading,
      questionsQuery.isLoading,
      positionsQuery.isLoading,
    ],
  );

  return {
    cards: cardsQuery.data ?? [],
    sections: sectionsQuery.data ?? [],
    notes: notesQuery.data ?? [],
    aliases: aliasesQuery.data ?? [],
    tasks: tasksQuery.data ?? [],
    questions: questionsQuery.data ?? [],
    positions: positionsQuery.data ?? {},
    isLoading,
    syncVault: syncVaultMutation,
  };
}
