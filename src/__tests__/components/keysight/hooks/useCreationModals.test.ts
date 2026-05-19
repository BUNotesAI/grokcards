import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  useCreationModals,
  type UseCreationModalsDeps,
} from "@/components/keysight/hooks/useCreationModals";

function makeDeps(): UseCreationModalsDeps {
  return {
    viewport: {
      state: { zoom: 1, panX: 0, panY: 0 },
      actions: { fitToContent: vi.fn() },
    } as unknown as UseCreationModalsDeps["viewport"],
    setLocalPositions: vi.fn(),
    notes: { create: vi.fn().mockResolvedValue({ id: "note_x" }) } as unknown as UseCreationModalsDeps["notes"],
    questions: {
      create: vi.fn().mockResolvedValue({ id: "q_x" }),
    } as unknown as UseCreationModalsDeps["questions"],
    tasks: {
      create: vi.fn().mockResolvedValue({ id: "task_x" }),
    } as unknown as UseCreationModalsDeps["tasks"],
    whiteboardActions: {
      create: vi.fn().mockResolvedValue({ id: "wb_x" }),
    } as unknown as UseCreationModalsDeps["whiteboardActions"],
    sections: {
      create: vi.fn().mockResolvedValue({ id: "sec_x" }),
    } as unknown as UseCreationModalsDeps["sections"],
    layouts: {
      setPosition: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseCreationModalsDeps["layouts"],
    currentWhiteboardId: "wb_test",
    data: {
      tasks: [],
    } as unknown as UseCreationModalsDeps["data"],
    effectivePositions: {},
    allEntities: [],
    allDimensions: {},
    newEntityPositionAtCenter: vi.fn(() => ({ x: 0, y: 0 })),
    getVisibleViewportSize: vi.fn(() => ({ width: 800, height: 600 })),
    onSelectEntity: vi.fn(),
    setEditing: vi.fn(),
  };
}

describe("useCreationModals", () => {
  it("c_theme_w1_use_creation_modals_note_open_cancel", () => {
    const { result } = renderHook(() => useCreationModals(makeDeps()));

    expect(result.current.creatingNote).toBe(false);
    expect(result.current.noteDraft).toBe("");

    act(() => {
      result.current.handleCreateNote();
    });
    expect(result.current.creatingNote).toBe(true);

    act(() => {
      result.current.setNoteDraft("draft text");
    });
    expect(result.current.noteDraft).toBe("draft text");

    act(() => {
      result.current.handleCancelCreateNote();
    });
    expect(result.current.creatingNote).toBe(false);
    expect(result.current.noteDraft).toBe("");
  });

  it("c_theme_w1_use_creation_modals_question_task_whiteboard_independent_state", () => {
    const { result } = renderHook(() => useCreationModals(makeDeps()));

    act(() => {
      result.current.handleCreateTask();
    });
    expect(result.current.creatingTask).toBe(true);
    expect(result.current.creatingNote).toBe(false);
    expect(result.current.creatingQuestion).toBe(false);
    expect(result.current.creatingWhiteboard).toBe(false);

    act(() => {
      result.current.handleCancelCreateTask();
    });
    expect(result.current.creatingTask).toBe(false);
  });
});
