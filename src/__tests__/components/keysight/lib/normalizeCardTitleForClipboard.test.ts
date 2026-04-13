import { normalizeCardTitleForClipboard } from "@/components/keysight/lib/normalizeCardTitleForClipboard";

describe("normalizeCardTitleForClipboard", () => {
  it("去掉粗体定界符并清理 ASCII 标点转义", () => {
    expect(
      normalizeCardTitleForClipboard(String.raw`**Rust** \[Ownership\] \<Guide\> \#101`),
    ).toBe("Rust [Ownership] <Guide> #101");
  });

  it("保留字面反斜杠星号和双反斜杠", () => {
    expect(
      normalizeCardTitleForClipboard(String.raw`Keep \* literal and \\ slash and \[list\]`),
    ).toBe(String.raw`Keep \* literal and \\ slash and [list]`);
  });
});
