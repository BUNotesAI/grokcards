import { describe, it, expect } from "vitest";
import { buildEdgePath } from "@/components/keysight/lib/edgePath";

describe("buildEdgePath", () => {
  // 两个 100×100 的矩形横向并排，间距 100
  const FROM = { x: 0, y: 0, width: 100, height: 100 };
  const TO = { x: 200, y: 0, width: 100, height: 100 };

  it("水平并排两矩形 → 返回 SVG path 字符串以 M 开头并含 Q", () => {
    const result = buildEdgePath(FROM, TO);
    expect(result).not.toBeNull();
    expect(result!.path).toMatch(/^M /);
    expect(result!.path).toContain(" Q ");
  });

  it("路径起点 = from 右边界（含 padding）", () => {
    const result = buildEdgePath(FROM, TO, 6);
    expect(result).not.toBeNull();
    // from 中心 (50, 50) 朝 to 中心 (250, 50)，碰 from 右边 x = 100 + 6 = 106
    const match = result!.path.match(/^M ([\d.-]+),([\d.-]+)/);
    expect(match).not.toBeNull();
    const startX = parseFloat(match![1]);
    expect(startX).toBeCloseTo(106, 0);
  });

  it("两矩形太近（重叠或几乎重叠）→ 返回 null", () => {
    const a = { x: 0, y: 0, width: 100, height: 100 };
    const b = { x: 5, y: 0, width: 100, height: 100 };
    const result = buildEdgePath(a, b);
    expect(result).toBeNull();
  });

  it("midX/midY 在两端点之间", () => {
    const result = buildEdgePath(FROM, TO);
    expect(result).not.toBeNull();
    // path 起点 ~106, 终点 ~194；midX 应在 (106, 194) 之间
    expect(result!.midX).toBeGreaterThan(100);
    expect(result!.midX).toBeLessThan(200);
  });

  it("padding=0 时起点正好在矩形右边界", () => {
    const result = buildEdgePath(FROM, TO, 0);
    expect(result).not.toBeNull();
    const match = result!.path.match(/^M ([\d.-]+),([\d.-]+)/);
    const startX = parseFloat(match![1]);
    expect(startX).toBeCloseTo(100, 0);
  });

  it("垂直布局时终点也应离目标卡片保留足够箭头空间", () => {
    const above = { x: 0, y: 0, width: 100, height: 100 };
    const below = { x: 0, y: 220, width: 100, height: 100 };
    const result = buildEdgePath(above, below);

    expect(result).not.toBeNull();
    const match = result!.path.match(/ ([\d.-]+),([\d.-]+)$/);
    expect(match).not.toBeNull();
    const endX = parseFloat(match![1]);
    const endY = parseFloat(match![2]);

    // 旧逻辑终点会直接贴在 208；现在会再沿切线回退，为箭头留出空间。
    expect(endY).toBeLessThan(200);
    expect(endY).toBeGreaterThan(190);
    expect(endX).toBeLessThan(50);
  });
});
