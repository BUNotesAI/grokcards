import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  useViewportRecovery,
  type UseViewportRecoveryDeps,
} from "@/components/keysight/hooks/useViewportRecovery";

function makeDeps(overrides: Partial<UseViewportRecoveryDeps> = {}): UseViewportRecoveryDeps {
  return {
    viewport: {
      state: { zoom: 1, panX: 0, panY: 0 },
      actions: {
        fitToContent: vi.fn(),
        resetView: vi.fn(),
        centerOn: vi.fn(),
      },
      needsFit: true,
    } as unknown as UseViewportRecoveryDeps["viewport"],
    layouts: {
      setPositionWithoutInvalidate: vi.fn().mockResolvedValue(undefined),
      invalidatePositions: vi.fn(),
    } as unknown as UseViewportRecoveryDeps["layouts"],
    data: {
      positions: {},
      isLoading: false,
      sections: [],
      cards: [],
      tasks: [],
      questions: [],
      notes: [],
      aliases: [],
    } as unknown as UseViewportRecoveryDeps["data"],
    currentWhiteboardId: "wb_test",
    allEntities: [{ kind: "card", id: "card_a", entity: {} as never, position: { x: 100, y: 100 } }],
    whiteboards: [],
    hasEntityInViewport: true,
    visibleEntitiesCount: 1,
    getVisibleViewportSize: vi.fn(() => ({ width: 800, height: 600 })),
    ...overrides,
  };
}

describe("useViewportRecovery", () => {
  it("c_theme_w4_fit_attempted_on_first_render_when_needs_fit", () => {
    const deps = makeDeps();
    renderHook(() => useViewportRecovery(deps));

    // 首次进入白板 + viewport.needsFit=true + allEntities 非空 → fitToContent 被调
    expect(deps.viewport.actions.fitToContent).toHaveBeenCalled();
  });

  it("c_theme_w4_blank_viewport_recovery_when_no_entity_in_view", () => {
    const deps = makeDeps({
      viewport: {
        state: { zoom: 1, panX: 0, panY: 0 },
        actions: {
          fitToContent: vi.fn(),
          resetView: vi.fn(),
          centerOn: vi.fn(),
        },
        needsFit: false, // 有保存的视口
      } as unknown as UseViewportRecoveryDeps["viewport"],
      hasEntityInViewport: false, // 但当前视口看不到任何 entity
    });
    renderHook(() => useViewportRecovery(deps));

    // 有 allEntities 但视口看不到 + 非 loading + size > 0 → recovery 触发 fit
    expect(deps.viewport.actions.fitToContent).toHaveBeenCalled();
  });

  it("c_theme_w4_handle_reset_clears_session_and_refits", () => {
    const deps = makeDeps({
      viewport: {
        state: { zoom: 1, panX: 0, panY: 0 },
        actions: {
          fitToContent: vi.fn(),
          resetView: vi.fn(),
          centerOn: vi.fn(),
        },
        needsFit: false,
      } as unknown as UseViewportRecoveryDeps["viewport"],
    });
    const { result } = renderHook(() => useViewportRecovery(deps));

    act(() => {
      result.current.handleResetSavedViewport();
    });

    // 有 entity + size>0 → 走 fitToContent 分支
    expect(deps.viewport.actions.fitToContent).toHaveBeenCalled();
  });
});
