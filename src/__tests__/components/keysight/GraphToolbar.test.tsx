import { render, screen, fireEvent, renderHook } from "@testing-library/react";
import { MemoryRouter, Routes, Route, useLocation } from "react-router-dom";

// useNavigate in GraphToolbar requires a Router context,
// 本文件所有 render 用 renderT 包裹 MemoryRouter
function renderT(element: React.ReactElement) {
  return render(<MemoryRouter>{element}</MemoryRouter>);
}

// 跳转断言 helper:渲染当前 pathname + search,click 后断言位置变化
function LocationReadout() {
  const location = useLocation();
  return (
    <div data-testid="current-location">
      {location.pathname}
      {location.search}
    </div>
  );
}
import { GraphToolbar } from "@/components/keysight/GraphToolbar";
import { useViewport } from "@/components/keysight/useViewport";
import type { WhiteboardSummary } from "@/bindings";
import { vi } from "vitest";

/** 创建真实 viewport */
function createTestViewport() {
  const { result } = renderHook(() => useViewport("test-wb"));
  return result.current;
}

const mockWhiteboard: WhiteboardSummary = {
  whiteboardId: "rust",
  cards: 3,
  notes: 1,
  sections: 1,
  aliases: 0,
  tasks: 0,
  questions: 0,
};

describe("GraphToolbar", () => {
  it("显示实体计数", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 2, notes: 1, sections: 1, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
      />,
    );
    expect(screen.getByText("2 Cards")).toBeInTheDocument();
    expect(screen.getByText("1 Notes")).toBeInTheDocument();
  });

  it("显示 zoom 百分比", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
      />,
    );
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("子白板时显示面包屑导航", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        currentWhiteboardId="rust"
        onNavigateBack={vi.fn()}
      />,
    );

    expect(screen.getByText("Root")).toBeInTheDocument();
    expect(screen.getByText("rust")).toBeInTheDocument();
  });

  it("顶部工具栏不显示搜索框，并按 Section / Note / Question / Whiteboard / 孤儿 / Sections / Boards 排列", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 2, notes: 1, sections: 1, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        sections={[{ id: "sec_inbox", title: "Inbox", cardIds: [] }]}
        whiteboards={[mockWhiteboard]}
      />,
    );

    expect(screen.queryByPlaceholderText(/search/i)).not.toBeInTheDocument();

    const buttons = screen
      .getAllByRole("button")
      .map((button) => ({
        text: button.textContent?.replace(/\s+/g, " ").trim() ?? "",
        label: button.getAttribute("aria-label") ?? "",
      }));

    const sectionIndex = buttons.findIndex((button) => button.text === "Section");
    const noteIndex = buttons.findIndex((button) => button.text === "Note");
    const questionIndex = buttons.findIndex((button) => button.text === "Question");
    const whiteboardIndex = buttons.findIndex((button) => button.text === "Whiteboard");
    const orphanIndex = buttons.findIndex((button) => button.label === "Show orphans");
    const sectionsIndex = buttons.findIndex((button) => button.text === "Sections");
    const boardsIndex = buttons.findIndex((button) => button.text === "Boards");

    expect(sectionIndex).toBeGreaterThanOrEqual(0);
    expect(noteIndex).toBeGreaterThan(sectionIndex);
    expect(questionIndex).toBeGreaterThan(noteIndex);
    expect(whiteboardIndex).toBeGreaterThan(questionIndex);
    expect(orphanIndex).toBeGreaterThan(whiteboardIndex);
    expect(sectionsIndex).toBeGreaterThan(orphanIndex);
    expect(boardsIndex).toBeGreaterThan(sectionsIndex);
  });

  it("Sync 按钮调用 onSync", () => {
    const viewport = createTestViewport();
    const onSync = vi.fn();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={onSync}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /sync/i }));
    expect(onSync).toHaveBeenCalledTimes(1);
  });

  it("无文字按钮提供鼠标悬停提示", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: /sync/i })).toHaveAttribute("title");
    expect(screen.getByRole("button", { name: /zoom out/i })).toHaveAttribute("title");
    expect(screen.getByRole("button", { name: /zoom in/i })).toHaveAttribute("title");
    expect(screen.getByRole("button", { name: /reset zoom/i })).toHaveAttribute("title");
  });

  it("+ Section 按钮调用 onCreateSection", () => {
    const viewport = createTestViewport();
    const onCreateSection = vi.fn();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={onCreateSection}
        onCreateNote={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /section/i }));
    expect(onCreateSection).toHaveBeenCalledTimes(1);
  });

  it("+ Question 按钮调用 onCreateQuestion", () => {
    const viewport = createTestViewport();
    const onCreateQuestion = vi.fn();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        onCreateQuestion={onCreateQuestion}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /create question/i }));
    expect(onCreateQuestion).toHaveBeenCalledTimes(1);
  });

  it("创建 note 时显示内联输入框", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        creatingNote
        noteDraft="My Note"
      />,
    );

    expect(screen.getByRole("textbox", { name: /note title/i })).toHaveValue("My Note");
    expect(screen.queryByRole("button", { name: /create note/i })).not.toBeInTheDocument();
  });

  it("创建 question 时显示内联输入框", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        creatingQuestion
        questionDraft="Why?"
      />,
    );

    expect(screen.getByRole("textbox", { name: /question title/i })).toHaveValue("Why?");
    expect(screen.queryByRole("button", { name: /create question/i })).not.toBeInTheDocument();
  });

  it("根白板时显示 Whiteboard 按钮并调用 onCreateWhiteboard", () => {
    const viewport = createTestViewport();
    const onCreateWhiteboard = vi.fn();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        onCreateWhiteboard={onCreateWhiteboard}
        currentWhiteboardId="wb_root"
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /create whiteboard/i }));
    expect(onCreateWhiteboard).toHaveBeenCalledTimes(1);
  });

  it("创建 whiteboard 时显示内联输入框", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        creatingWhiteboard
        whiteboardDraft="agent"
      />,
    );

    expect(screen.getByRole("textbox", { name: /whiteboard name/i })).toHaveValue("agent");
    expect(screen.queryByRole("button", { name: /create whiteboard/i })).not.toBeInTheDocument();
  });

  it("根白板时不显示返回按钮", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        currentWhiteboardId="wb_root"
      />,
    );
    expect(screen.queryByText("rust")).not.toBeInTheDocument();
  });

  it("子白板时不显示 Whiteboard 创建按钮", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        currentWhiteboardId="rust"
      />,
    );

    expect(screen.queryByRole("button", { name: /create whiteboard/i })).not.toBeInTheDocument();
  });

  it("子白板时显示返回按钮和白板名，点击触发 onNavigateBack", () => {
    const viewport = createTestViewport();
    const onNavigateBack = vi.fn();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 50, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        currentWhiteboardId="rust"
        onNavigateBack={onNavigateBack}
      />,
    );
    expect(screen.getByText("rust")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Root"));
    expect(onNavigateBack).toHaveBeenCalledTimes(1);
  });

  it("project 白板时显示 Task 按钮并调用 onCreateTask", () => {
    const viewport = createTestViewport();
    const onCreateTask = vi.fn();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        onCreateTask={onCreateTask}
        currentWhiteboardId="projects/super-tauri"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /create task/i }));
    expect(onCreateTask).toHaveBeenCalledTimes(1);
  });

  it("根白板时不显示 Task 按钮", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        onCreateTask={vi.fn()}
        currentWhiteboardId="wb_root"
      />,
    );
    expect(screen.queryByRole("button", { name: /create task/i })).not.toBeInTheDocument();
  });

  it("非 project 子白板时不显示 Task 按钮", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        onCreateTask={vi.fn()}
        currentWhiteboardId="rust"
      />,
    );
    expect(screen.queryByRole("button", { name: /create task/i })).not.toBeInTheDocument();
  });

  it("创建 task 时显示内联输入框", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        creatingTask
        taskDraft="My Task"
        currentWhiteboardId="projects/super-tauri"
      />,
    );
    expect(screen.getByRole("textbox", { name: /task title/i })).toHaveValue("My Task");
    expect(screen.queryByRole("button", { name: /create task/i })).not.toBeInTheDocument();
  });

  it("project 白板显示 Show Kanban 按钮", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        currentWhiteboardId="projects/super-tauri"
      />,
    );
    expect(screen.getByRole("button", { name: /show kanban/i })).toBeInTheDocument();
  });

  it("非 project 白板不显示 Show Kanban 按钮", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        currentWhiteboardId="wb_root"
      />,
    );
    expect(screen.queryByRole("button", { name: /show kanban/i })).not.toBeInTheDocument();
  });

  it("root 白板不显示 Show Kanban 按钮", () => {
    const viewport = createTestViewport();
    renderT(      <GraphToolbar
        viewport={viewport}
        entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
        onSync={vi.fn()}
        onCreateSection={vi.fn()}
        onCreateNote={vi.fn()}
        currentWhiteboardId="rust"
      />,
    );
    expect(screen.queryByRole("button", { name: /show kanban/i })).not.toBeInTheDocument();
  });

  it("点击 Show Kanban 导航到 /kanban?project={name}", () => {
    const viewport = createTestViewport();
    render(
      <MemoryRouter initialEntries={["/keysight?wb=projects/super-tauri"]}>
        <Routes>
          <Route
            path="*"
            element={
              <>
                <GraphToolbar
                  viewport={viewport}
                  entityCounts={{ cards: 0, notes: 0, sections: 0, tasks: 0, questions: 0, aliases: 0 }}
                  onSync={vi.fn()}
                  onCreateSection={vi.fn()}
                  onCreateNote={vi.fn()}
                  currentWhiteboardId="projects/super-tauri"
                />
                <LocationReadout />
              </>
            }
          />
        </Routes>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole("button", { name: /show kanban/i }));
    expect(screen.getByTestId("current-location")).toHaveTextContent(
      "/kanban?project=super-tauri",
    );
  });
});
