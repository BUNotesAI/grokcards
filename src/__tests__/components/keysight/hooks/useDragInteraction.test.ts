import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { MouseEvent as ReactMouseEvent } from "react";
import {
  useDragInteraction,
  type UseDragInteractionDeps,
} from "@/components/keysight/hooks/useDragInteraction";

function makeDeps(overrides: Partial<UseDragInteractionDeps> = {}): UseDragInteractionDeps {
  return {
    viewport: {
      state: { zoom: 1, panX: 0, panY: 0 },
      actions: { fitToContent: vi.fn() },
    } as unknown as UseDragInteractionDeps["viewport"],
    layouts: {
      setPositionWithoutInvalidate: vi.fn().mockResolvedValue(undefined),
      invalidatePositions: vi.fn(),
    } as unknown as UseDragInteractionDeps["layouts"],
    data: {
      // server positions:hook 内部 merge data.positions + localPositions = effectivePositions
      positions: { card_a: { x: 100, y: 100 } },
      sections: [],
      cards: [],
      tasks: [],
      questions: [],
      notes: [],
      aliases: [],
    } as unknown as UseDragInteractionDeps["data"],
    currentWhiteboardId: "wb_test",
    ...overrides,
  };
}

function mockMousedownEvent(clientX: number, clientY: number) {
  const el = document.createElement("div");
  return {
    clientX,
    clientY,
    currentTarget: el,
  } as unknown as ReactMouseEvent;
}

describe("useDragInteraction", () => {
  it("c_theme_w2_drag_single_entity_updates_local_positions_and_persists_on_mouseup", async () => {
    const deps = makeDeps();
    const { result } = renderHook(() => useDragInteraction(deps));

    act(() => {
      result.current.handleDragStart(mockMousedownEvent(50, 50), "card_a");
    });

    // 移动 20px(超过 DRAG_THRESHOLD=4)— localPositions 应该更新
    act(() => {
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 70, clientY: 70 }));
    });
    expect(result.current.localPositions["card_a"]).toEqual({ x: 120, y: 120 });

    // 释放 — 触发持久化
    act(() => {
      window.dispatchEvent(new MouseEvent("mouseup"));
    });
    expect(deps.layouts.setPositionWithoutInvalidate).toHaveBeenCalledWith(
      "wb_test",
      "card_a",
      120,
      120,
    );
    expect(result.current.lastDidDragRef.current).toBe(true);
  });

  it("c_theme_w2_drag_section_moves_all_member_cards", () => {
    const deps = makeDeps({
      data: {
        positions: { card_b: { x: 100, y: 100 }, card_c: { x: 200, y: 200 } },
        sections: [{ id: "sec_a", cardIds: ["card_b", "card_c"], title: "S" }],
        cards: [],
        tasks: [],
        questions: [],
        notes: [],
        aliases: [],
      } as unknown as UseDragInteractionDeps["data"],
    });
    const { result } = renderHook(() => useDragInteraction(deps));

    act(() => {
      result.current.handleDragStart(mockMousedownEvent(50, 50), "sec_a");
    });

    act(() => {
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 70, clientY: 70 }));
    });
    // dx=20, dy=20 → 两张卡都偏移 20px
    expect(result.current.localPositions["card_b"]).toEqual({ x: 120, y: 120 });
    expect(result.current.localPositions["card_c"]).toEqual({ x: 220, y: 220 });
  });

  it("c_theme_w2_drag_click_below_threshold_does_not_persist", () => {
    const deps = makeDeps();
    const { result } = renderHook(() => useDragInteraction(deps));

    act(() => {
      result.current.handleDragStart(mockMousedownEvent(50, 50), "card_a");
    });

    // 移动 2px(平方距离 8 < DRAG_THRESHOLD²=16)— 视为 click,不更新 localPositions
    act(() => {
      window.dispatchEvent(new MouseEvent("mousemove", { clientX: 52, clientY: 52 }));
    });
    expect(result.current.localPositions["card_a"]).toBeUndefined();

    act(() => {
      window.dispatchEvent(new MouseEvent("mouseup"));
    });
    expect(deps.layouts.setPositionWithoutInvalidate).not.toHaveBeenCalled();
    expect(result.current.lastDidDragRef.current).toBe(false);
  });
});
