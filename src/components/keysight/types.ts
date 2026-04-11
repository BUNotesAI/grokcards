import type {
  AtomicCard,
  GraphSection,
  GraphNote,
  CardAlias,
  TaskEntity,
  QuestionEntity,
  Position,
} from "@/bindings";

/** 实体类型标识 */
export type EntityKind = "card" | "task" | "question" | "note" | "section" | "alias";

/** 各类型实体的联合 — 带位置信息 */
export type EntityWithPosition =
  | { kind: "card"; entity: AtomicCard; position: Position; id: string }
  | { kind: "task"; entity: TaskEntity; position: Position; id: string }
  | { kind: "question"; entity: QuestionEntity; position: Position; id: string }
  | { kind: "note"; entity: GraphNote; position: Position; id: string }
  | { kind: "section"; entity: GraphSection; position: Position; id: string }
  | { kind: "alias"; entity: CardAlias; position: Position; id: string };

/** 各类型实体的预设宽高（世界坐标像素），视口裁剪和 section bounds 计算用 */
export const ENTITY_DIMENSIONS: Record<EntityKind, { width: number; height: number }> = {
  card: { width: 320, height: 160 },
  task: { width: 320, height: 140 },
  question: { width: 320, height: 140 },
  note: { width: 200, height: 120 },
  section: { width: 400, height: 300 }, // section 的实际尺寸由成员位置动态计算
  alias: { width: 280, height: 100 },
};
