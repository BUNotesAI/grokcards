import type { AtomicCard, CardAlias, GraphNote, QuestionEntity } from "@/bindings";

/** 可渲染的 edge 类型 */
export type EdgeKind = "link_to" | "note_link" | "alias_link" | "question_link";

export interface RenderEdge {
  from: string;
  to: string;
  kind: EdgeKind;
}

/**
 * 从当前白板的 cards / notes / aliases / questions 数据构建可渲染的 edge 列表。
 *
 * 包含的 edge 类型：
 * - card.linkTo → link_to edge（橙色实线）
 * - note.linkedCardIds / linkedNoteIds / linkedSectionIds / linkedQuestionIds / linkedTaskIds → note_link edge（青色虚线）
 * - alias.linkedCardIds / linkedNoteIds / linkedSectionIds / linkedQuestionIds / linkedTaskIds → alias_link edge（青色虚线）
 * - question.linkedCardIds / linkedNoteIds / linkedSectionIds / linkedQuestionIds / linkedTaskIds → question_link edge（青色虚线）
 *
 * 不包含：
 * - card.related — 通过 picker 添加，只在卡片展开后的 Related 列表里显示
 * - card.seeAlso — see-also 指向 vault 文件而非实体，画线没意义
 *
 * 过滤：from 和 to 必须都在 entitySet 中，避免渲染断头连线（指向不存在的实体）。
 */
export function buildEdges(
  cards: AtomicCard[],
  notes: GraphNote[],
  aliases: CardAlias[],
  questions: QuestionEntity[],
  entitySet: Set<string>,
): RenderEdge[] {
  const edges: RenderEdge[] = [];

  for (const card of cards) {
    if (!entitySet.has(card.id)) continue;
    for (const target of card.linkTo ?? []) {
      if (target === card.id || !entitySet.has(target)) continue;
      edges.push({ from: card.id, to: target, kind: "link_to" });
    }
  }

  for (const note of notes) {
    if (!entitySet.has(note.id)) continue;
    const targets = [
      ...(note.linkedCardIds ?? []),
      ...(note.linkedNoteIds ?? []),
      ...(note.linkedSectionIds ?? []),
      ...(note.linkedQuestionIds ?? []),
      ...(note.linkedTaskIds ?? []),
    ];
    for (const target of targets) {
      if (target === note.id || !entitySet.has(target)) continue;
      edges.push({ from: note.id, to: target, kind: "note_link" });
    }
  }

  for (const alias of aliases) {
    if (!entitySet.has(alias.aliasId)) continue;
    // 出边：alias → linkedXxx
    const targets = [
      ...(alias.linkedCardIds ?? []),
      ...(alias.linkedNoteIds ?? []),
      ...(alias.linkedSectionIds ?? []),
      ...(alias.linkedQuestionIds ?? []),
      ...(alias.linkedTaskIds ?? []),
    ];
    for (const target of targets) {
      if (target === alias.aliasId || !entitySet.has(target)) continue;
      edges.push({ from: alias.aliasId, to: target, kind: "alias_link" });
    }
    // 入边：incomingCardIds → alias（反向 card → alias 引用）
    for (const cardId of alias.incomingCardIds ?? []) {
      if (cardId === alias.aliasId || !entitySet.has(cardId)) continue;
      edges.push({ from: cardId, to: alias.aliasId, kind: "alias_link" });
    }
  }

  for (const question of questions) {
    if (!entitySet.has(question.id)) continue;
    const targets = [
      ...(question.linkedCardIds ?? []),
      ...(question.linkedNoteIds ?? []),
      ...(question.linkedSectionIds ?? []),
      ...(question.linkedQuestionIds ?? []),
      ...(question.linkedTaskIds ?? []),
    ];
    for (const target of targets) {
      if (target === question.id || !entitySet.has(target)) continue;
      edges.push({ from: question.id, to: target, kind: "question_link" });
    }
  }

  return edges;
}
