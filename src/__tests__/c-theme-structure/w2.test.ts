import { describe, it, expect } from "vitest";
import useDragInteractionSrc from "@/components/keysight/hooks/useDragInteraction.ts?raw";
import graphViewSrc from "@/components/keysight/GraphView.tsx?raw";

describe("C Theme W2 — useDragInteraction", () => {
  it("c_theme_w2_hook_file_exists", () => {
    // useDragInteraction.ts exports
    expect(useDragInteractionSrc).toMatch(/export\s+function\s+useDragInteraction\b/);
    expect(useDragInteractionSrc).toMatch(
      /export\s+(interface|type)\s+UseDragInteractionDeps\b/,
    );
    expect(useDragInteractionSrc).toMatch(
      /export\s+(interface|type)\s+UseDragInteractionResult\b/,
    );

    // hook 文件内含 handleDragStart + 全局 mousemove/mouseup 监听 + 服务器位置回流清理 三段逻辑
    expect(useDragInteractionSrc).toMatch(/handleDragStart/);
    expect(useDragInteractionSrc).toMatch(/addEventListener\(\s*["']mousemove["']/);
    expect(useDragInteractionSrc).toMatch(/addEventListener\(\s*["']mouseup["']/);
    expect(useDragInteractionSrc).toMatch(/data\.positions/);

    // GraphView.tsx 内不再含顶层 dragInfoRef + handleDragStart 定义
    expect(graphViewSrc).not.toMatch(/^\s*const\s+dragInfoRef\s*=\s*useRef/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleDragStart\s*=\s*useCallback/m);

    // GraphView.tsx 内不再含顶层 lastDidDragRef / localPositions 声明
    expect(graphViewSrc).not.toMatch(/^\s*const\s+lastDidDragRef\s*=\s*useRef/m);
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[localPositions,\s*setLocalPositions\]\s*=\s*useState/m,
    );

    // GraphView.tsx 内不再含监听全局 mousemove / mouseup 的 useEffect
    expect(graphViewSrc).not.toMatch(/window\.addEventListener\(\s*["']mousemove["']/);
    expect(graphViewSrc).not.toMatch(/window\.addEventListener\(\s*["']mouseup["']/);

    // GraphView.tsx 应该 import useDragInteraction
    expect(graphViewSrc).toMatch(
      /import\s+\{[^}]*\buseDragInteraction\b[^}]*\}\s+from\s+["']@\/components\/keysight\/hooks\/useDragInteraction["']/,
    );
  });
});
