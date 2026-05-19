import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { EntityKind } from "@/components/keysight/types";

/** 从 WhiteboardData 派生 entity id → kind 反向索引(纯函数,可被多个 hook 共享调用) */
export function deriveAllKinds(data: WhiteboardData): Record<string, EntityKind> {
  const kinds: Record<string, EntityKind> = {};
  for (const c of data.cards) kinds[c.id] = "card";
  for (const n of data.notes) kinds[n.id] = "note";
  for (const t of data.tasks) kinds[t.id] = "task";
  for (const q of data.questions) kinds[q.id] = "question";
  for (const s of data.sections) kinds[s.id] = "section";
  for (const a of data.aliases) kinds[a.aliasId] = "alias";
  return kinds;
}
