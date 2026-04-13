import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import {
  SidebarProvider,
  useSidebar,
} from "@/components/ui/sidebar";

function SidebarStateProbe() {
  const { state } = useSidebar();
  return (
    <div>
      <div data-testid="sidebar-state">{state}</div>
      <input aria-label="editor-input" defaultValue="editable text" />
    </div>
  );
}

describe("ui Sidebar keyboard shortcut", () => {
  beforeEach(() => {
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation(() => ({
        matches: false,
        media: "",
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
  });

  it("编辑输入框聚焦时，Cmd/Ctrl+B 不应切换 sidebar", () => {
    render(
      <SidebarProvider>
        <SidebarStateProbe />
      </SidebarProvider>,
    );

    const input = screen.getByLabelText("editor-input");
    input.focus();

    fireEvent.keyDown(window, { key: "b", metaKey: true });

    expect(screen.getByTestId("sidebar-state")).toHaveTextContent("expanded");
  });
});
