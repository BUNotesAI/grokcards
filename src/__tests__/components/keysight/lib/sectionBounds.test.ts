import { describe, it, expect } from "vitest";
import {
  computeSectionBounds,
  type SectionMember,
} from "@/components/keysight/lib/sectionBounds";

describe("computeSectionBounds", () => {
  const PADDING = 40;

  it("无成员时返回最小尺寸", () => {
    const bounds = computeSectionBounds([], PADDING);
    expect(bounds).toEqual({ width: 200, height: 100 });
  });

  it("单个 note member — bounds 精确覆盖 member + padding*2", () => {
    const member: SectionMember = {
      position: { x: 0, y: 0 },
      width: 520,
      height: 180,
    };
    const bounds = computeSectionBounds([member], PADDING);
    // 右边缘 = x + width = 520；左边 section 从 0 - 40 = -40 开始
    // bounds.width 必须让 section 右边到 520 + 40 = 560
    // → bounds.width = 520 - 0 + 80 = 600
    expect(bounds.width).toBe(600);
    // 类似地：bounds.height = 180 - 0 + 80 = 260
    expect(bounds.height).toBe(260);
  });

  it("复现真实 bug：INBOX section 的三个 note member（DB 真实坐标）", () => {
    // 来自 DB 查询：sec_0bf78727 的三个 note member 的实际坐标
    // NoteNode 实际渲染宽度 520px（硬编码），高度 content-driven 估 180px
    const NOTE_W = 520;
    const NOTE_H = 180;
    const members: SectionMember[] = [
      { position: { x: -804, y: -742 }, width: NOTE_W, height: NOTE_H },
      { position: { x: -842, y: -578 }, width: NOTE_W, height: NOTE_H },
      { position: { x: -802, y: -473 }, width: NOTE_W, height: NOTE_H },
    ];
    const bounds = computeSectionBounds(members, PADDING);

    // 验证 bounds 能完整覆盖所有 member
    // section top-left = (min(member.x) - padding, min(member.y) - padding)
    //                  = (-842 - 40, -742 - 40) = (-882, -782)
    const sectionLeft = -842 - PADDING;
    const sectionTop = -742 - PADDING;
    const sectionRight = sectionLeft + bounds.width;
    const sectionBottom = sectionTop + bounds.height;

    // 每个 member 的 right/bottom 必须在 section 内部（含 padding）
    for (const m of members) {
      const memberRight = m.position.x + m.width;
      const memberBottom = m.position.y + m.height;
      expect(sectionRight).toBeGreaterThanOrEqual(memberRight + PADDING);
      expect(sectionBottom).toBeGreaterThanOrEqual(memberBottom + PADDING);
    }

    // 精确值：
    //   minX = -842, maxXRight = -802 + 520 = -282
    //   bounds.width = -282 - (-842) + 80 = 640
    //   minY = -742, maxYBottom = -473 + 180 = -293
    //   bounds.height = -293 - (-742) + 80 = 529
    expect(bounds.width).toBe(640);
    expect(bounds.height).toBe(529);
  });

  it("混合 kind：card + note 各有不同尺寸", () => {
    const members: SectionMember[] = [
      // card at (0, 0)：520 × 200
      { position: { x: 0, y: 0 }, width: 520, height: 200 },
      // note at (0, 300)：520 × 180
      { position: { x: 0, y: 300 }, width: 520, height: 180 },
      // task at (600, 0)：320 × 140
      { position: { x: 600, y: 0 }, width: 320, height: 140 },
    ];
    const bounds = computeSectionBounds(members, PADDING);
    // minX = 0, maxXRight = 600 + 320 = 920 → width = 920 + 80 = 1000
    expect(bounds.width).toBe(1000);
    // minY = 0, maxYBottom = 300 + 180 = 480 → height = 480 + 80 = 560
    expect(bounds.height).toBe(560);
  });
});
