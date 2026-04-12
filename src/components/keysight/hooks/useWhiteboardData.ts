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
import { perfLog } from "@/lib/perf";

/** 包装 query 函数加 perf 日志：start + done(count) */
async function timedQuery<T>(
  name: string,
  fn: () => Promise<T>,
  countFn?: (r: T) => number,
): Promise<T> {
  perfLog(`query[${name}] start`);
  const t0 = performance.now();
  const result = await fn();
  const dur = performance.now() - t0;
  const count = countFn ? countFn(result) : -1;
  perfLog(
    `query[${name}] done in ${dur.toFixed(0)}ms${count >= 0 ? ` (n=${count})` : ""}`,
  );
  return result;
}

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
    queryFn: () =>
      timedQuery(
        "cards",
        () => unwrapCommand(commands.cardQueryAll(null, null)),
        (r) => r.length,
      ),
  });

  const sectionsQuery = useQuery({
    queryKey: ["sections", whiteboardId],
    queryFn: () =>
      timedQuery(
        `sections[${whiteboardId}]`,
        () => unwrapCommand(commands.sectionQueryAll(whiteboardId)),
        (r) => r.length,
      ),
  });

  const notesQuery = useQuery({
    queryKey: ["notes", whiteboardId],
    queryFn: () =>
      timedQuery(
        `notes[${whiteboardId}]`,
        () => unwrapCommand(commands.noteQueryAll(whiteboardId)),
        (r) => r.length,
      ),
  });

  const aliasesQuery = useQuery({
    queryKey: ["aliases", whiteboardId],
    queryFn: () =>
      timedQuery(
        `aliases[${whiteboardId}]`,
        () => unwrapCommand(commands.aliasQueryAll(whiteboardId)),
        (r) => r.length,
      ),
  });

  const tasksQuery = useQuery({
    queryKey: ["tasks", whiteboardId],
    queryFn: () =>
      timedQuery(
        `tasks[${whiteboardId}]`,
        () => unwrapCommand(commands.taskQueryAll(whiteboardId)),
        (r) => r.length,
      ),
  });

  const questionsQuery = useQuery({
    queryKey: ["questions", whiteboardId],
    queryFn: () =>
      timedQuery(
        `questions[${whiteboardId}]`,
        () => unwrapCommand(commands.questionQueryAll(whiteboardId)),
        (r) => r.length,
      ),
  });

  const positionsQuery = useQuery({
    queryKey: ["positions", whiteboardId],
    queryFn: () =>
      timedQuery(
        `positions[${whiteboardId}]`,
        () => unwrapCommand(commands.layoutQueryPositions(whiteboardId)),
        (r) => Object.keys(r).length,
      ),
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

/**
 * 加载子白板列表（仅在 wb_root 时启用）。
 */
export function useWhiteboardList(whiteboardId: string) {
  return useQuery({
    queryKey: ["whiteboards"],
    queryFn: () => unwrapCommand(commands.whiteboardList()),
    enabled: whiteboardId === "wb_root",
  });
}
