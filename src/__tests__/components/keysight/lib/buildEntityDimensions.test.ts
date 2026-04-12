import { describe, it, expect } from "vitest";
import { buildEntityDimensions } from "@/components/keysight/lib/buildEntityDimensions";
import {
  ENTITY_DIMENSIONS,
  type EntityKind,
} from "@/components/keysight/types";
import type { GraphSection, Position } from "@/bindings";

function mkSection(id: string, cardIds: string[]): GraphSection {
  return { id, title: id, cardIds, color: null };
}

describe("buildEntityDimensions", () => {
  it("非 section 实体用 ENTITY_DIMENSIONS by kind", () => {
    const kinds: Record<string, EntityKind> = {
      c1: "card",
      n1: "note",
      t1: "task",
    };
    const dims = buildEntityDimensions(kinds, [], {}, 40);
    expect(dims.c1).toEqual(ENTITY_DIMENSIONS.card);
    expect(dims.n1).toEqual(ENTITY_DIMENSIONS.note);
    expect(dims.t1).toEqual(ENTITY_DIMENSIONS.task);
  });

  it("section 用 computeSectionBounds 算真实尺寸 — 不是 ENTITY_DIMENSIONS.section placeholder", () => {
    // section 含两个 card 成员，分布在 x=[0, 1500]
    // 真实 width = (1500 + card.width) - 0 + 80 = (1500+520)-0+80 = 2100
    // 真实 height = (0 + card.height) - 0 + 80 = 220 + 80 = 300
    const kinds: Record<string, EntityKind> = {
      sec1: "section",
      c1: "card",
      c2: "card",
    };
    const sections = [mkSection("sec1", ["c1", "c2"])];
    const positions: Record<string, Position> = {
      c1: { x: 0, y: 0 },
      c2: { x: 1500, y: 0 },
    };
    const dims = buildEntityDimensions(kinds, sections, positions, 40);
    // 不应等于 placeholder
    expect(dims.sec1).not.toEqual(ENTITY_DIMENSIONS.section);
    // 应等于真实计算结果
    expect(dims.sec1.width).toBe(2100);
    expect(dims.sec1.height).toBe(300);
  });

  it("回归：section 跨大范围时 dimensions 必须能覆盖视口右侧（修 #section-cull-bug）", () => {
    // 这是用户报告的 bug：section 在 (0, 0)，成员 x=[0, 2000]
    // 旧 bug：section 用 ENTITY_DIMENSIONS.section.width=400 → pan 到右侧时被 cull
    // 修复：section.width 应能完整覆盖最远成员的右边界
    const kinds: Record<string, EntityKind> = {
      sec_wide: "section",
      ...Object.fromEntries(
        Array.from({ length: 5 }, (_, i) => [`c${i}`, "card" as const]),
      ),
    };
    const sections = [
      mkSection("sec_wide", ["c0", "c1", "c2", "c3", "c4"]),
    ];
    const positions: Record<string, Position> = {
      c0: { x: 0, y: 0 },
      c1: { x: 500, y: 0 },
      c2: { x: 1000, y: 0 },
      c3: { x: 1500, y: 0 },
      c4: { x: 2000, y: 0 },
    };
    const dims = buildEntityDimensions(kinds, sections, positions, 40);
    // 真实 right = 2000 + 520 = 2520, width = 2520 - 0 + 80 = 2600
    expect(dims.sec_wide.width).toBe(2600);
    // 远大于 ENTITY_DIMENSIONS.section.width (400) — 这是回归点
    expect(dims.sec_wide.width).toBeGreaterThan(
      ENTITY_DIMENSIONS.section.width * 5,
    );
  });

  it("section 没有成员位置 → 退化为 computeSectionBounds 的最小尺寸", () => {
    const kinds: Record<string, EntityKind> = { sec_empty: "section" };
    const sections = [mkSection("sec_empty", [])];
    const dims = buildEntityDimensions(kinds, sections, {}, 40);
    expect(dims.sec_empty).toEqual({ width: 200, height: 100 });
  });

  it("section 含 note 成员 → 用 note 尺寸而非 card 尺寸", () => {
    // 验证 buildEntityDimensions 按 member 真实 kind 查询尺寸
    // ENTITY_DIMENSIONS.note = {520, 180}
    const kinds: Record<string, EntityKind> = {
      sec1: "section",
      n1: "note",
    };
    const sections = [mkSection("sec1", ["n1"])];
    const positions: Record<string, Position> = { n1: { x: 0, y: 0 } };
    const dims = buildEntityDimensions(kinds, sections, positions, 40);
    // note 高 180, width 520, padding*2=80
    // expect width = 520 + 80 = 600, height = 180 + 80 = 260
    expect(dims.sec1).toEqual({ width: 600, height: 260 });
  });
});
