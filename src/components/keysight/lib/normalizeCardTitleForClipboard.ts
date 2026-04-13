/**
 * 复制卡片标题到剪贴板前做轻量规范化：
 * - 去掉 markdown 粗体定界符 `**`
 * - 清理历史遗留的 `\` + ASCII 标点转义（保留 `\*` / `\\`）
 */
export function normalizeCardTitleForClipboard(title: string): string {
  const withoutBold = title.replace(/\*\*/g, "");
  let normalized = "";

  for (let i = 0; i < withoutBold.length; i += 1) {
    const ch = withoutBold[i];
    if (ch !== "\\") {
      normalized += ch;
      continue;
    }

    const next = withoutBold[i + 1];
    if (!next) {
      normalized += ch;
      continue;
    }

    if (next === "\\" || next === "*") {
      normalized += ch;
      continue;
    }

    if (/^[\x21-\x2F\x3A-\x40\x5B-\x60\x7B-\x7E]$/.test(next)) {
      normalized += next;
      i += 1;
      continue;
    }

    normalized += ch;
  }

  return normalized;
}
