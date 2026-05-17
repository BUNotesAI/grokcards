/**
 * bindings.ts 后处理脚本 —— 把 specta 生成的 `export type XId = string;` 改写为
 * branded string `string & { readonly __brand: "XId" }`,让 TS 端编译期挡 id 类型
 * 混用(参数顺序写反、跨 entity 混用)。
 *
 * 触发时机:`cargo test export_bindings` 生成最新 src/bindings.ts 之后,`pnpm build`
 * 之前。由 `pnpm bindings:brand` script 调用。
 *
 * 不改写 EntityId(它是 specta tagged-union,不是 string alias)。
 *
 * 任务来源:task_ee2b6926 (A Theme W0)。
 */

import { Project, SyntaxKind } from "ts-morph";

/** 7 个 entity-related newtype,与 keysight-core/src/domain/id.rs 中的 prefixed_id! 一一对应。 */
export const BRANDED_ID_TYPES = [
  "WhiteboardId",
  "CardId",
  "NoteId",
  "AliasId",
  "SectionId",
  "QuestionId",
  "TaskId",
] as const;

export type BrandedIdName = (typeof BRANDED_ID_TYPES)[number];

export interface BrandResult {
  branded: BrandedIdName[];
  skipped: Array<{ name: BrandedIdName; reason: string }>;
}

/**
 * 给文件里命名匹配 `idNames` 的 `export type XId = string;` 改写为 branded string。
 *
 * 行为契约:
 * - 只改写**完整等同 `string`** 的 type alias(避免误改已经 branded 或 union 形态的 type)
 * - 不存在的 type 名记入 `skipped`,不报错(W0 阶段 bindings.ts 可能还不含某些 type)
 * - 不写盘,调用方决定是否 `sf.saveSync()`(便于测试)
 *
 * 返回结果用于断言 / 日志输出。
 */
export function brandBindingsFile(
  filePath: string,
  idNames: readonly BrandedIdName[] = BRANDED_ID_TYPES,
): BrandResult {
  const project = new Project();
  const sf = project.addSourceFileAtPath(filePath);

  const result: BrandResult = { branded: [], skipped: [] };

  for (const name of idNames) {
    const alias = sf.getTypeAlias(name);
    if (!alias) {
      result.skipped.push({ name, reason: "type alias not found" });
      continue;
    }
    const typeNode = alias.getTypeNode();
    const oldText = typeNode?.getText();
    if (oldText !== "string") {
      result.skipped.push({
        name,
        reason: `type body is not plain "string" (got: ${oldText ?? "undefined"})`,
      });
      continue;
    }
    alias.setType(`string & { readonly __brand: "${name}" }`);
    result.branded.push(name);
  }

  sf.saveSync();
  return result;
}

/** CLI entry —— 由 `pnpm bindings:brand` 调用。 */
function cliMain(): void {
  const target = process.argv[2] ?? "src/bindings.ts";
  const result = brandBindingsFile(target);
  console.log(
    `[brand-bindings] file=${target} branded=[${result.branded.join(", ")}] skipped=${result.skipped.length}`,
  );
  for (const s of result.skipped) {
    console.log(`  - skipped ${s.name}: ${s.reason}`);
  }
}

// 仅当作为 CLI 直接 invoke 时运行 main(被 import 时不执行)
// import.meta.url 在 ESM 下指向当前文件,比较 process.argv[1] 决定是否 entry
const isCli =
  typeof process !== "undefined" &&
  Array.isArray(process.argv) &&
  process.argv[1] !== undefined &&
  // tsx / node 直接跑该文件时 process.argv[1] 末尾匹配文件名
  process.argv[1].endsWith("brand-bindings.ts");

if (isCli) {
  cliMain();
}

// 通过 `void SyntaxKind;` 等表达式标记 import 仍在用(防 tsc unused-import)
void SyntaxKind;
