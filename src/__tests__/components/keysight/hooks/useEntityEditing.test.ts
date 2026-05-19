import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { RefObject } from "react";
import {
  useEntityEditing,
  type UseEntityEditingDeps,
} from "@/components/keysight/hooks/useEntityEditing";

// 真实 hook + mock actions:测试只关心 editing state 转移与 commit dispatch 路由,
// 不关心 actions 内部实现(那是 use*Actions 自己的测试覆盖)。
function makeDeps(overrides: Partial<UseEntityEditingDeps> = {}): UseEntityEditingDeps {
  const lastDidDragRef: RefObject<boolean> = { current: false };
  return {
    lastDidDragRef,
    cards: {
      editTitle: vi.fn().mockResolvedValue(undefined),
      updateUnderstanding: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseEntityEditingDeps["cards"],
    notes: {
      update: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseEntityEditingDeps["notes"],
    sections: {
      update: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseEntityEditingDeps["sections"],
    questions: {
      update: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseEntityEditingDeps["questions"],
    tasks: {
      update: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseEntityEditingDeps["tasks"],
    ...overrides,
  };
}

describe("useEntityEditing", () => {
  it("c_theme_w1_use_entity_editing_happy_path", () => {
    const { result } = renderHook(() => useEntityEditing(makeDeps()));

    expect(result.current.editing).toBeNull();

    act(() => {
      result.current.handleStartEdit("card_x", "card-title");
    });
    expect(result.current.editing).toEqual({ id: "card_x", field: "card-title" });

    act(() => {
      result.current.handleCancelEdit();
    });
    expect(result.current.editing).toBeNull();
  });

  it("c_theme_w1_use_entity_editing_skip_after_drag", () => {
    const lastDidDragRef: RefObject<boolean> = { current: true };
    const { result } = renderHook(() =>
      useEntityEditing(makeDeps({ lastDidDragRef })),
    );

    // 刚结束 drag 时 handleStartEdit 应跳过 setEditing,并把 ref 复位
    act(() => {
      result.current.handleStartEdit("card_x", "card-title");
    });
    expect(result.current.editing).toBeNull();
    expect(lastDidDragRef.current).toBe(false);
  });

  it("c_theme_w1_use_entity_editing_commit_dispatches_to_cards", async () => {
    const editTitle = vi.fn().mockResolvedValue(undefined);
    const cards = {
      editTitle,
      updateUnderstanding: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseEntityEditingDeps["cards"];
    const { result } = renderHook(() => useEntityEditing(makeDeps({ cards })));

    act(() => {
      result.current.handleStartEdit("card_y", "card-title");
    });

    await act(async () => {
      await result.current.handleCommitEdit("card_y", "card-title", "new title");
    });

    expect(editTitle).toHaveBeenCalledWith("card_y", "new title");
    expect(result.current.editing).toBeNull();
  });
});
