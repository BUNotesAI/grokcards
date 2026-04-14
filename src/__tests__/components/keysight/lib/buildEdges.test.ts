import { describe, it, expect } from "vitest";
import { buildEdges } from "@/components/keysight/lib/buildEdges";
import type { AtomicCard, CardAlias, GraphNote, QuestionEntity } from "@/bindings";

function mkCard(id: string, linkTo: string[] = [], related: string[] = []): AtomicCard {
  return {
    id,
    title: id,
    content: "",
    filePath: "",
    tags: [],
    linkTo,
    related,
    seeAlso: [],
    understanding: "",
    source: "",
    mtime: null,
  };
}

function mkNote(
  id: string,
  linkedCardIds: string[] = [],
  linkedNoteIds: string[] = [],
  linkedSectionIds: string[] = [],
): GraphNote {
  return {
    id,
    title: id,
    content: "",
    color: null,
    linkedCardIds,
    linkedNoteIds,
    linkedSectionIds,
  };
}

function mkAlias(
  aliasId: string,
  cardId: string,
  linkedCardIds: string[] = [],
  incomingCardIds: string[] = [],
): CardAlias {
  return {
    aliasId,
    cardId,
    linkedCardIds,
    linkedNoteIds: [],
    linkedSectionIds: [],
    incomingCardIds,
  };
}

function mkQuestion(
  id: string,
  linkedCardIds: string[] = [],
  linkedNoteIds: string[] = [],
  linkedSectionIds: string[] = [],
  linkedQuestionIds: string[] = [],
  linkedTaskIds: string[] = [],
): QuestionEntity {
  return {
    id,
    title: id,
    content: "",
    whiteboardId: "wb_root",
    status: "pending",
    color: null,
    linkedCardIds,
    linkedNoteIds,
    linkedSectionIds,
    linkedQuestionIds,
    linkedTaskIds,
  };
}

describe("buildEdges", () => {
  it("空数据 → 返回空数组", () => {
    const edges = buildEdges([], [], [], [], new Set());
    expect(edges).toEqual([]);
  });

  it("card.linkTo → link_to edge（双方都存在）", () => {
    const cards = [mkCard("c1", ["c2"]), mkCard("c2")];
    const set = new Set(["c1", "c2"]);
    const edges = buildEdges(cards, [], [], [], set);
    expect(edges).toEqual([{ from: "c1", to: "c2", kind: "link_to" }]);
  });

  it("card.related 不产生 edge（只在卡片展开列表里显示）", () => {
    const cards = [mkCard("c1", [], ["c2"]), mkCard("c2")];
    const edges = buildEdges(cards, [], [], [], new Set(["c1", "c2"]));
    expect(edges).toEqual([]);
  });

  it("card.linkTo 指向不存在的实体 → 过滤掉", () => {
    const cards = [mkCard("c1", ["c2", "c_missing"])];
    const edges = buildEdges(cards, [], [], [], new Set(["c1", "c2"]));
    expect(edges).toEqual([{ from: "c1", to: "c2", kind: "link_to" }]);
  });

  it("note.linkedCardIds + linkedNoteIds + linkedSectionIds → note_link", () => {
    const notes = [mkNote("n1", ["c1"], ["n2"], ["s1"])];
    const set = new Set(["n1", "c1", "n2", "s1"]);
    const edges = buildEdges([], notes, [], [], set);
    expect(edges).toHaveLength(3);
    expect(edges).toContainEqual({ from: "n1", to: "c1", kind: "note_link" });
    expect(edges).toContainEqual({ from: "n1", to: "n2", kind: "note_link" });
    expect(edges).toContainEqual({ from: "n1", to: "s1", kind: "note_link" });
  });

  it("alias.linkedCardIds → alias_link", () => {
    const aliases = [mkAlias("a1", "c1", ["c2"])];
    const set = new Set(["a1", "c1", "c2"]);
    const edges = buildEdges([], [], aliases, [], set);
    expect(edges).toContainEqual({ from: "a1", to: "c2", kind: "alias_link" });
  });

  it("alias.incomingCardIds → alias_link（反向 card → alias）", () => {
    const aliases = [mkAlias("a1", "c1", [], ["c2", "c3"])];
    const set = new Set(["a1", "c1", "c2", "c3"]);
    const edges = buildEdges([], [], aliases, [], set);

    expect(edges).toContainEqual({ from: "c2", to: "a1", kind: "alias_link" });
    expect(edges).toContainEqual({ from: "c3", to: "a1", kind: "alias_link" });
  });

  it("question linked ids -> question_link", () => {
    const questions = [mkQuestion("q1", ["c1"], ["n1"], ["s1"], ["q2"], ["task1"])];
    const set = new Set(["q1", "c1", "n1", "s1", "q2", "task1"]);
    const edges = buildEdges([], [], [], questions, set);
    expect(edges).toHaveLength(5);
    expect(edges).toContainEqual({ from: "q1", to: "c1", kind: "question_link" });
    expect(edges).toContainEqual({ from: "q1", to: "n1", kind: "question_link" });
    expect(edges).toContainEqual({ from: "q1", to: "s1", kind: "question_link" });
    expect(edges).toContainEqual({ from: "q1", to: "q2", kind: "question_link" });
    expect(edges).toContainEqual({ from: "q1", to: "task1", kind: "question_link" });
  });

  it("question_link 指向不存在的实体 -> 过滤掉", () => {
    const questions = [mkQuestion("q1", ["c1"], [], [], [], ["task_missing"])];
    const set = new Set(["q1", "c1"]);
    const edges = buildEdges([], [], [], questions, set);
    expect(edges).toEqual([{ from: "q1", to: "c1", kind: "question_link" }]);
  });

  it("混合所有类型 — 整体场景", () => {
    const cards = [mkCard("c1", ["c2"], ["c3"]), mkCard("c2"), mkCard("c3")];
    const notes = [mkNote("n1", ["c1"])];
    const aliases = [mkAlias("a1", "c1", ["c3"])];
    const questions = [mkQuestion("q1", [], ["n1"])];
    const set = new Set(["c1", "c2", "c3", "n1", "a1", "q1"]);
    const edges = buildEdges(cards, notes, aliases, questions, set);
    // 1 link_to (c1→c2) + 1 note_link (n1→c1) + 1 alias_link (a1→c3)
    // + 1 question_link (q1→n1) = 4。related (c1→c3) 不产生 edge
    expect(edges).toHaveLength(4);
  });
});
