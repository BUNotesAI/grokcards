import { describe, expect, it } from "vitest";
import {
  normalizeLegacyToggleSyntax,
  parseToggleBlocks,
} from "@/components/keysight/lib/toggleMarkdown";

describe("toggleMarkdown", () => {
  it("解析单个 ?>> / ?<< toggle block", () => {
    const md = "before\n?>> Title\ncontent line\n?<<\nafter";
    const blocks = parseToggleBlocks(md);
    expect(blocks).toHaveLength(3);
    expect(blocks[0]).toBe("before");
    expect(blocks[1]).toEqual({ title: "Title", content: "content line" });
    expect(blocks[2]).toBe("after");
  });

  it("未闭合 ?>> block 按普通文本处理", () => {
    const md = "?>> Unclosed\ncontent";
    const blocks = parseToggleBlocks(md);
    expect(blocks).toHaveLength(1);
    expect(blocks[0]).toBe(md);
  });

  it("把 legacy <details>/<summary> 规范化成 ?>> / ?<<", () => {
    const md = [
      "before",
      "<details>",
      "<summary>折叠标题</summary>",
      "",
      "这里是内容",
      "</details>",
      "after",
    ].join("\n");

    expect(normalizeLegacyToggleSyntax(md)).toBe([
      "before",
      "?>> 折叠标题",
      "这里是内容",
      "?<<",
      "after",
    ].join("\n"));
  });

  it("已有 ?>> / ?<< 语法保持不变", () => {
    const md = "?>> 标题\n内容\n?<<";
    expect(normalizeLegacyToggleSyntax(md)).toBe(md);
  });
});
