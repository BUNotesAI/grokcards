/**
 * 折叠 `**X **` / `** X**` / `**  X  **` 这类不规范 strong 标记。
 *
 * Obsidian 的 markdown parser 比 CommonMark 宽松，容忍 `**` 闭合前的空格。
 * react-markdown 用严格 CommonMark — 闭合 `**` 前必须不能是空格，否则不识别为 strong。
 * 用户从 obsidian 迁移过来的内容里有大量这种"宽松写法"，渲染会丢失加粗。
 *
 * 这个函数把 `**...**` 内部首尾的空白折叠掉，让 react-markdown 也能识别为 strong。
 *
 * 例：
 * - `**Copy 和 Clone **的区别` → `**Copy 和 Clone**的区别`
 * - `** hello **` → `**hello**`
 * - `**hello**` → `**hello**` (unchanged)
 * - `*italic*` → unchanged (只处理 `**`)
 */
export function normalizeBoldMarkdown(text: string): string {
  // 匹配 `**X**` 其中 X 至少 1 个非 `*` 非换行字符；按位置非贪婪匹配
  // 跨行不处理（避免误吞段落）
  return text.replace(/\*\*([^*\n]+?)\*\*/g, (full, inner: string) => {
    const trimmed = inner.trim();
    if (trimmed.length === 0) return full;
    return `**${trimmed}**`;
  });
}
