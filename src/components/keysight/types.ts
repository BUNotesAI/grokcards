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
export type EntityKind = "card" | "task" | "question" | "note" | "section" | "alias" | "whiteboard";
export type LodLevel = 0 | 1 | 2;
export type GraphSelectableKind = Exclude<EntityKind, "whiteboard">;

export interface GraphSelection {
  id: string;
  kind: GraphSelectableKind;
}

/** 各类型实体的联合 — 带位置信息 */
export type EntityWithPosition =
  | { kind: "card"; entity: AtomicCard; position: Position; id: string }
  | { kind: "task"; entity: TaskEntity; position: Position; id: string }
  | { kind: "question"; entity: QuestionEntity; position: Position; id: string }
  | { kind: "note"; entity: GraphNote; position: Position; id: string }
  | { kind: "section"; entity: GraphSection; position: Position; id: string }
  | { kind: "alias"; entity: CardAlias; position: Position; id: string };

/** 各类型实体的预设宽高（世界坐标像素）
 *
 * 宽度必须和节点组件里硬编码的 CSS `width` 保持一致（SectionNode bounds 计算依赖这个）：
 * - CardNode/NoteNode/AliasNode: 520（对齐旧 Obsidian）
 * - TaskNode/QuestionNode/WhiteboardNode: 320
 *
 * 高度是折叠状态下的典型值 + 适度富余（content-driven，常量只作为估计）。
 * 视口裁剪和 section bounds 计算用。
 */
export const ENTITY_DIMENSIONS: Record<EntityKind, { width: number; height: number }> = {
  card: { width: 520, height: 220 },
  task: { width: 320, height: 140 },
  question: { width: 320, height: 140 },
  note: { width: 520, height: 180 },
  section: { width: 400, height: 300 }, // section 的实际尺寸由 computeSectionBounds 动态计算
  alias: { width: 520, height: 220 },
  whiteboard: { width: 320, height: 130 },
};
