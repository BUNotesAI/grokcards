import { render, screen, fireEvent } from "@testing-library/react";
import { renderHook } from "@testing-library/react";
import { GraphToolbar } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import { vi } from "vitest";

/** 创建真实 viewport */
function createTestViewport() {
  const { result } = renderHook(() => useViewport("test-wb"));
  return result.current;
}

describe("GraphToolbar", () => {
  it("显示实体计数", () => {
    const viewport = createTestViewport();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 2, notes: 1, sections: 1, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    expect(screen.getByText(/2 cards/)).toBeInTheDocument();
    expect(screen.getByText(/1 note/)).toBeInTheDocument();
  });

  it("显示 zoom 百分比", () => {
    const viewport = createTestViewport();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("Sync 按钮调用 onSync", () => {
    const viewport = createTestViewport();
    const onSync = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={onSync}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /sync/i }));
    expect(onSync).toHaveBeenCalledTimes(1);
  });

  it("搜索输入触发 onSearchChange", () => {
    const viewport = createTestViewport();
    const onSearchChange = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={onSearchChange}
      />,
    );
    fireEvent.change(screen.getByPlaceholderText(/search/i), {
      target: { value: "test" },
    });
    expect(onSearchChange).toHaveBeenCalledWith("test");
  });

  it("+ Section 按钮调用 onCreateSection", () => {
    const viewport = createTestViewport();
    const onCreateSection = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={onCreateSection}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /section/i }));
    expect(onCreateSection).toHaveBeenCalledTimes(1);
  });

  it("根白板时不显示返回按钮", () => {
    const viewport = createTestViewport();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
        currentWhiteboardId="wb_root"
      />,
    );
    expect(screen.queryByRole("button", { name: /back/i })).not.toBeInTheDocument();
  });

  it("子白板时显示返回按钮和白板名，点击触发 onNavigateBack", () => {
    const viewport = createTestViewport();
    const onNavigateBack = vi.fn();
    render(
      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 50, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        searchQuery=""
        onSearchChange={vi.fn()}
        currentWhiteboardId="rust"
        onNavigateBack={onNavigateBack}
      />,
    );
    expect(screen.getByText("rust")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(onNavigateBack).toHaveBeenCalledTimes(1);
  });
});
