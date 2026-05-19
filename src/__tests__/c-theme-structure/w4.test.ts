import { describe, it, expect } from "vitest";
import useViewportRecoverySrc from "@/components/keysight/hooks/useViewportRecovery.ts?raw";
import graphViewSrc from "@/components/keysight/GraphView.tsx?raw";

describe("C Theme W4 — useViewportRecovery", () => {
  it("c_theme_w4_hook_file_exists", () => {
    // useViewportRecovery.ts exports
    expect(useViewportRecoverySrc).toMatch(/export\s+function\s+useViewportRecovery\b/);
    expect(useViewportRecoverySrc).toMatch(
      /export\s+(interface|type)\s+UseViewportRecoveryDeps\b/,
    );
    expect(useViewportRecoverySrc).toMatch(
      /export\s+(interface|type)\s+UseViewportRecoveryResult\b/,
    );

    // hook 内含三类 ref + 两个 callback
    expect(useViewportRecoverySrc).toMatch(/initializedRef/);
    expect(useViewportRecoverySrc).toMatch(/fitAttemptedRef/);
    expect(useViewportRecoverySrc).toMatch(/blankViewportRecoveryRef/);
    expect(useViewportRecoverySrc).toMatch(/firstVisiblePaintRef/);
    expect(useViewportRecoverySrc).toMatch(/handleFitContent/);
    expect(useViewportRecoverySrc).toMatch(/handleResetSavedViewport/);

    // GraphView.tsx 不再含 4 个顶层 useRef 声明
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+initializedRef\s*=\s*useRef</m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+fitAttemptedRef\s*=\s*useRef</m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+blankViewportRecoveryRef\s*=\s*useRef</m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+firstVisiblePaintRef\s*=\s*useRef</m,
    );

    // GraphView.tsx 不再含 handleFitContent / handleResetSavedViewport useCallback 定义
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleFitContent\s*=\s*useCallback/m);
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+handleResetSavedViewport\s*=\s*useCallback/m,
    );

    // GraphView.tsx import useViewportRecovery
    expect(graphViewSrc).toMatch(
      /import\s+\{[^}]*\buseViewportRecovery\b[^}]*\}\s+from\s+["']@\/components\/keysight\/hooks\/useViewportRecovery["']/,
    );
  });
});
