/**
 * scripts/brand-bindings.ts 的 fixture 测试。
 *
 * W0 阶段 src/bindings.ts 还没含 newtype type alias(还没有 command 用 WhiteboardId
 * 等)。所以测试用 fixture string + 临时文件,不依赖真实 bindings.ts。
 *
 * 测试不变量:
 * 1. 命中的 string alias 被改写为 `string & { readonly __brand: "XId" }` 形态
 * 2. EntityId 这种 tagged-union enum 即使存在也不被改写(不在 BRANDED_ID_TYPES)
 * 3. 已经 branded 或非 string 的 type alias 走 skipped 路径,不被二次改写
 *
 * 任务来源:task_ee2b6926 (A Theme W0,scenario 7)。
 */

import { describe, it, expect, afterEach } from "vitest";
import { writeFileSync, readFileSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { brandBindingsFile, BRANDED_ID_TYPES } from "./brand-bindings.ts";

const FIXTURE_PATH = join(tmpdir(), `keysight-brand-test-${process.pid}.ts`);

function writeFixture(content: string): void {
  writeFileSync(FIXTURE_PATH, content, "utf8");
}

function readResult(): string {
  return readFileSync(FIXTURE_PATH, "utf8");
}

afterEach(() => {
  try {
    unlinkSync(FIXTURE_PATH);
  } catch {
    // 文件可能本来就不存在,忽略
  }
});

describe("brand-bindings.ts", () => {
  it("把 7 个 newtype string alias 改写为 branded", () => {
    writeFixture(
      [
        "// fixture",
        "export type WhiteboardId = string;",
        "export type CardId = string;",
        "export type NoteId = string;",
        "export type AliasId = string;",
        "export type SectionId = string;",
        "export type QuestionId = string;",
        "export type TaskId = string;",
        "",
      ].join("\n"),
    );

    const result = brandBindingsFile(FIXTURE_PATH);

    expect(result.branded).toEqual([...BRANDED_ID_TYPES]);
    expect(result.skipped).toEqual([]);

    const output = readResult();
    for (const name of BRANDED_ID_TYPES) {
      expect(output).toContain(
        `export type ${name} = string & { readonly __brand: "${name}" };`,
      );
    }
  });

  it("不改写 EntityId tagged-union enum", () => {
    writeFixture(
      [
        '// fixture: EntityId is tagged-union, should NOT be branded',
        'export type EntityId = { Card: CardId } | { Note: NoteId };',
        'export type CardId = string;',
        'export type NoteId = string;',
        "",
      ].join("\n"),
    );

    const result = brandBindingsFile(FIXTURE_PATH);

    // EntityId 不在 BRANDED_ID_TYPES 列表里,被跳过
    expect(result.branded).toContain("CardId");
    expect(result.branded).toContain("NoteId");
    expect(result.branded).not.toContain("EntityId" as never);

    const output = readResult();
    expect(output).toContain(
      'export type EntityId = { Card: CardId } | { Note: NoteId };',
    );
  });

  it("已经 branded 的 type alias 不被二次改写", () => {
    writeFixture(
      [
        '// fixture: pre-branded',
        'export type CardId = string & { readonly __brand: "CardId" };',
        "",
      ].join("\n"),
    );

    const result = brandBindingsFile(FIXTURE_PATH);

    expect(result.branded).toEqual([]);
    // 至少 CardId 进入 skipped(其他 6 个不在 fixture 里也进 skipped)
    expect(result.skipped.length).toBeGreaterThanOrEqual(1);
    const cardSkipped = result.skipped.find((s) => s.name === "CardId");
    expect(cardSkipped).toBeDefined();
    expect(cardSkipped?.reason).toMatch(/not plain "string"/);

    // 输出应保持不变(没二次改写)
    const output = readResult();
    expect(output).toContain(
      'export type CardId = string & { readonly __brand: "CardId" };',
    );
    // 二次改写会产生 `string & {__brand:"CardId"} & {__brand:"CardId"}`,
    // 通过 `__brand:` 字面量出现次数 = 1 来验证未二次改写
    const occurrences = output.match(/__brand:/g)?.length ?? 0;
    expect(occurrences).toBe(1);
  });

  it("不存在的 type alias 走 skipped 路径不报错", () => {
    writeFixture(["// fixture: empty", ""].join("\n"));

    const result = brandBindingsFile(FIXTURE_PATH);

    expect(result.branded).toEqual([]);
    expect(result.skipped.length).toBe(BRANDED_ID_TYPES.length);
    for (const s of result.skipped) {
      expect(s.reason).toBe("type alias not found");
    }
  });
});
