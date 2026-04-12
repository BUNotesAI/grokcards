import { describe, it, expect } from "vitest";
import { clipToRect } from "@/components/keysight/lib/clipToRect";

describe("clipToRect", () => {
  // 矩形 (0, 0, 100, 100) — 中心在 (50, 50)
  const RECT = { rx: 0, ry: 0, rw: 100, rh: 100 };

  it("射线水平向右，从中心射出 → 与右边界相交", () => {
    const p = clipToRect(50, 50, 200, 50, RECT.rx, RECT.ry, RECT.rw, RECT.rh);
    expect(p.x).toBeCloseTo(100);
    expect(p.y).toBeCloseTo(50);
  });

  it("射线水平向左 → 与左边界相交", () => {
    const p = clipToRect(50, 50, -100, 50, RECT.rx, RECT.ry, RECT.rw, RECT.rh);
    expect(p.x).toBeCloseTo(0);
    expect(p.y).toBeCloseTo(50);
  });

  it("射线垂直向下 → 与下边界相交", () => {
    const p = clipToRect(50, 50, 50, 200, RECT.rx, RECT.ry, RECT.rw, RECT.rh);
    expect(p.x).toBeCloseTo(50);
    expect(p.y).toBeCloseTo(100);
  });

  it("射线垂直向上 → 与上边界相交", () => {
    const p = clipToRect(50, 50, 50, -100, RECT.rx, RECT.ry, RECT.rw, RECT.rh);
    expect(p.x).toBeCloseTo(50);
    expect(p.y).toBeCloseTo(0);
  });

  it("射线对角向右下 → 与右边界（不是下边界）相交", () => {
    // 从 (50, 50) 朝 (250, 100) — 斜率 0.25，先碰右边界 x=100
    const p = clipToRect(50, 50, 250, 100, RECT.rx, RECT.ry, RECT.rw, RECT.rh);
    expect(p.x).toBeCloseTo(100);
    expect(p.y).toBeCloseTo(62.5); // 50 + 50 * 0.25
  });

  it("射线对角向左下，斜率大 → 先碰下边界", () => {
    // 从 (50, 50) 朝 (40, 250) — 主要向下，会先碰下边界 y=100
    const p = clipToRect(50, 50, 40, 250, RECT.rx, RECT.ry, RECT.rw, RECT.rh);
    expect(p.y).toBeCloseTo(100);
    expect(p.x).toBeCloseTo(47.5); // 50 + (-10) * 0.25
  });

  it("起点等于目标 → 返回起点", () => {
    const p = clipToRect(50, 50, 50, 50, RECT.rx, RECT.ry, RECT.rw, RECT.rh);
    expect(p.x).toBe(50);
    expect(p.y).toBe(50);
  });

  it("矩形不在原点 — 偏移坐标系仍正确", () => {
    // 矩形 (200, 300, 100, 100) — 中心 (250, 350)
    const p = clipToRect(250, 350, 500, 350, 200, 300, 100, 100);
    expect(p.x).toBeCloseTo(300);
    expect(p.y).toBeCloseTo(350);
  });
});
