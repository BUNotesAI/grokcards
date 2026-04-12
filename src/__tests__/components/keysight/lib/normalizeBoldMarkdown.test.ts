import { describe, it, expect } from "vitest";
import { normalizeBoldMarkdown } from "@/components/keysight/lib/normalizeBoldMarkdown";

describe("normalizeBoldMarkdown", () => {
  it("空字符串 → 空字符串", () => {
    expect(normalizeBoldMarkdown("")).toBe("");
  });

  it("纯文本无变化", () => {
    expect(normalizeBoldMarkdown("hello world")).toBe("hello world");
  });

  it("规范的 **strong** 不变", () => {
    expect(normalizeBoldMarkdown("**bold**")).toBe("**bold**");
  });

  it("尾部空格 — `**X **` → `**X**`", () => {
    expect(normalizeBoldMarkdown("**Copy 和 Clone **的区别")).toBe(
      "**Copy 和 Clone**的区别",
    );
  });

  it("首部空格 — `** X**` → `**X**`", () => {
    expect(normalizeBoldMarkdown("** X**")).toBe("**X**");
  });

  it("两侧都有空格 — `** X **` → `**X**`", () => {
    expect(normalizeBoldMarkdown("** hello **")).toBe("**hello**");
  });

  it("多个 **strong** 同时存在 — 都修复", () => {
    const input = "**Copy 和 Clone **的区别 — **隐式 vs 显式**  | **位拷贝 vs 深拷贝**";
    const expected = "**Copy 和 Clone**的区别 — **隐式 vs 显式**  | **位拷贝 vs 深拷贝**";
    expect(normalizeBoldMarkdown(input)).toBe(expected);
  });

  it("`*italic*` 不动（只处理 **）", () => {
    expect(normalizeBoldMarkdown("*italic *trailing")).toBe("*italic *trailing");
  });

  it("空内容 `** **` 跳过（避免变 ****）", () => {
    expect(normalizeBoldMarkdown("** **")).toBe("** **");
  });

  it("跨行不匹配（避免误吞段落）", () => {
    // 第一行 **hello \n** 不应被合并；第二行 **world** 已规范，不变
    expect(normalizeBoldMarkdown("**hello \n**world**")).toBe(
      "**hello \n**world**",
    );
  });
});
