import { describe, it, expect } from "vitest";
import useEntityContextMenuSrc from "@/components/keysight/hooks/useEntityContextMenu.ts?raw";
import graphViewSrc from "@/components/keysight/GraphView.tsx?raw";

describe("C Theme W3 — useEntityContextMenu", () => {
  it("c_theme_w3_hook_file_exists", () => {
    // useEntityContextMenu.ts exports
    expect(useEntityContextMenuSrc).toMatch(/export\s+function\s+useEntityContextMenu\b/);
    expect(useEntityContextMenuSrc).toMatch(
      /export\s+(interface|type)\s+UseEntityContextMenuDeps\b/,
    );
    expect(useEntityContextMenuSrc).toMatch(
      /export\s+(interface|type)\s+UseEntityContextMenuResult\b/,
    );

    // hook 内含 menuHandlers + drawingState + jump/select
    expect(useEntityContextMenuSrc).toMatch(/menuHandlers/);
    expect(useEntityContextMenuSrc).toMatch(/drawingState/);
    expect(useEntityContextMenuSrc).toMatch(/handleJumpToSection/);
    expect(useEntityContextMenuSrc).toMatch(/handleJumpToTask/);
    expect(useEntityContextMenuSrc).toMatch(/handleJumpToBoard/);
    expect(useEntityContextMenuSrc).toMatch(/handleSelectEntity/);

    // GraphView.tsx 内不再含 menuHandlers useMemo
    expect(graphViewSrc).not.toMatch(/^\s*const\s+menuHandlers\s*=\s*useMemo/m);

    // GraphView.tsx 内不再含 drawingState / relatedPickerCardId / relatedSearch useState 调用
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[drawingState,\s*setDrawingState\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[relatedPickerCardId,\s*setRelatedPickerCardId\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[relatedSearch,\s*setRelatedSearch\]\s*=\s*useState/m,
    );

    // GraphView.tsx 内不再含 5 个 jump/select handler 定义
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleJumpToSection\s*=\s*useCallback/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleJumpToTask\s*=\s*useCallback/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleJumpToBoard\s*=\s*useCallback/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleSync\s*=\s*useCallback/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleSelectEntity\s*=\s*useCallback/m);

    // GraphView.tsx 应该 import useEntityContextMenu
    expect(graphViewSrc).toMatch(
      /import\s+\{[^}]*\buseEntityContextMenu\b[^}]*\}\s+from\s+["']@\/components\/keysight\/hooks\/useEntityContextMenu["']/,
    );
  });
});
