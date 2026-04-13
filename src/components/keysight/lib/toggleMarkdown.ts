export interface ToggleBlock {
  title: string;
  content: string;
}

export function parseToggleBlocks(markdown: string): Array<string | ToggleBlock> {
  const lines = markdown.split("\n");
  const result: Array<string | ToggleBlock> = [];
  let currentBlock: { title: string; lines: string[] } | null = null;
  let plainLines: string[] = [];

  for (const line of lines) {
    if (line.trimStart().startsWith("?>>")) {
      if (plainLines.length > 0) {
        result.push(plainLines.join("\n"));
        plainLines = [];
      }
      currentBlock = {
        title: line.trimStart().slice(3).trim(),
        lines: [],
      };
      continue;
    }

    if (line.trimStart().startsWith("?<<") && currentBlock) {
      result.push({
        title: currentBlock.title,
        content: currentBlock.lines.join("\n"),
      });
      currentBlock = null;
      continue;
    }

    if (currentBlock) {
      currentBlock.lines.push(line);
    } else {
      plainLines.push(line);
    }
  }

  if (currentBlock) {
    plainLines.push(`?>> ${currentBlock.title}`);
    plainLines.push(...currentBlock.lines);
  }

  if (plainLines.length > 0) {
    result.push(plainLines.join("\n"));
  }

  return result;
}

export function normalizeLegacyToggleSyntax(markdown: string): string {
  const lines = markdown.split("\n");
  const result: string[] = [];

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i]!;
    const trimmed = line.trim();
    if (!trimmed.startsWith("<details")) {
      result.push(line);
      continue;
    }

    const summaryLine = lines[i + 1];
    const summaryMatch = summaryLine?.trim().match(/^<summary>(.*)<\/summary>$/);
    if (!summaryMatch) {
      result.push(line);
      continue;
    }

    const contentLines: string[] = [];
    let cursor = i + 2;
    while (cursor < lines.length && lines[cursor]!.trim() === "") {
      cursor += 1;
    }

    while (cursor < lines.length && lines[cursor]!.trim() !== "</details>") {
      contentLines.push(lines[cursor]!);
      cursor += 1;
    }

    if (cursor >= lines.length) {
      result.push(line);
      continue;
    }

    while (
      contentLines.length > 0 &&
      contentLines[contentLines.length - 1]?.trim() === ""
    ) {
      contentLines.pop();
    }

    result.push(`?>> ${summaryMatch[1]!.trim()}`);
    result.push(...contentLines);
    result.push("?<<");
    i = cursor;
  }

  return result.join("\n");
}
