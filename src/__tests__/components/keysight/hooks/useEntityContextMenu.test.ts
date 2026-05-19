import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  useEntityContextMenu,
  type UseEntityContextMenuDeps,
} from "@/components/keysight/hooks/useEntityContextMenu";

function makeDeps(overrides: Partial<UseEntityContextMenuDeps> = {}): UseEntityContextMenuDeps {
  const lastDidDragRef = { current: false };
  return {
    cards: { setColor: vi.fn() } as unknown as UseEntityContextMenuDeps["cards"],
    notes: { update: vi.fn(), remove: vi.fn() } as unknown as UseEntityContextMenuDeps["notes"],
    tasks: { setColor: vi.fn(), remove: vi.fn() } as unknown as UseEntityContextMenuDeps["tasks"],
    sections: {
      remove: vi.fn(),
      update: vi.fn(),
      addMember: vi.fn(),
      removeMember: vi.fn(),
    } as unknown as UseEntityContextMenuDeps["sections"],
    aliases: {
      create: vi.fn().mockResolvedValue({ aliasId: "alias_x" }),
      remove: vi.fn(),
    } as unknown as UseEntityContextMenuDeps["aliases"],
    questions: { update: vi.fn(), remove: vi.fn() } as unknown as UseEntityContextMenuDeps["questions"],
    layouts: {
      setPosition: vi.fn(),
      invalidatePositions: vi.fn(),
    } as unknown as UseEntityContextMenuDeps["layouts"],
    entities: {
      connect: vi.fn().mockResolvedValue(undefined),
    } as unknown as UseEntityContextMenuDeps["entities"],
    data: {
      cards: [],
      notes: [],
      tasks: [],
      questions: [],
      sections: [],
      aliases: [],
      positions: {},
      syncVault: { mutate: vi.fn() },
    } as unknown as UseEntityContextMenuDeps["data"],
    currentWhiteboardId: "wb_test",
    effectivePositions: {},
    allEntities: [],
    allDimensions: {},
    entityToSectionId: {},
    viewport: {
      state: { zoom: 1, panX: 0, panY: 0 },
      actions: { centerOn: vi.fn(), fitToContent: vi.fn() },
    } as unknown as UseEntityContextMenuDeps["viewport"],
    getVisibleViewportSize: vi.fn(() => ({ width: 800, height: 600 })),
    newEntityPositionAtCenter: vi.fn(() => ({ x: 0, y: 0 })),
    lastDidDragRef,
    setEditing: vi.fn(),
    setLocalPositions: vi.fn(),
    onSelectEntity: vi.fn(),
    onWhiteboardChange: vi.fn(),
    ...overrides,
  };
}

describe("useEntityContextMenu", () => {
  it("c_theme_w3_draw_connection_mode_toggle", () => {
    const { result } = renderHook(() => useEntityContextMenu(makeDeps()));

    expect(result.current.drawingState).toBeNull();

    act(() => {
      result.current.menuHandlers.onDrawConnectionFrom("card_a");
    });
    expect(result.current.drawingState).toEqual({ fromId: "card_a" });
    expect(result.current.relatedPickerCardId).toBeNull();

    // 再点 selectEntity 同一 id → 取消 drawing
    act(() => {
      result.current.handleSelectEntity({ id: "card_a", kind: "card" });
    });
    expect(result.current.drawingState).toBeNull();
  });

  it("c_theme_w3_menu_edit_note_title_triggers_set_editing", () => {
    const setEditing = vi.fn();
    const { result } = renderHook(() => useEntityContextMenu(makeDeps({ setEditing })));

    act(() => {
      result.current.menuHandlers.onEditNoteTitle("note_x");
    });
    expect(setEditing).toHaveBeenCalledWith({ id: "note_x", field: "note-title" });
  });

  it("c_theme_w3_select_entity_in_drawing_mode_calls_entities_connect", async () => {
    const connect = vi.fn().mockResolvedValue(undefined);
    const entities = {
      connect,
    } as unknown as UseEntityContextMenuDeps["entities"];
    const { result } = renderHook(() => useEntityContextMenu(makeDeps({ entities })));

    act(() => {
      result.current.menuHandlers.onDrawConnectionFrom("card_a");
    });
    expect(result.current.drawingState).toEqual({ fromId: "card_a" });

    await act(async () => {
      result.current.handleSelectEntity({ id: "card_b", kind: "card" });
    });
    expect(connect).toHaveBeenCalledWith("card_a", "card_b");
    expect(result.current.drawingState).toBeNull();
  });
});
