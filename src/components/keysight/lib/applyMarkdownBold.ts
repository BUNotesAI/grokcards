type EditableElement = HTMLInputElement | HTMLTextAreaElement;

/**
 * 将当前选区包成 markdown 粗体；若无选区则插入 `****` 并把光标放到中间。
 */
export function applyMarkdownBold(el: EditableElement): void {
  const value = el.value;
  const start = el.selectionStart ?? value.length;
  const end = el.selectionEnd ?? value.length;
  const selected = value.slice(start, end);

  const hasWrappingBold =
    start >= 2 &&
    value.slice(start - 2, start) === "**" &&
    value.slice(end, end + 2) === "**";

  if (hasWrappingBold) {
    el.value = value.slice(0, start - 2) + selected + value.slice(end + 2);
    el.setSelectionRange(start - 2, end - 2);
    return;
  }

  const wrapped = `**${selected}**`;

  el.value = value.slice(0, start) + wrapped + value.slice(end);

  if (start === end) {
    const caret = start + 2;
    el.setSelectionRange(caret, caret);
    return;
  }

  el.setSelectionRange(start + 2, end + 2);
}
