import type { ComponentProps } from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MemoryRouter, Routes, Route, useLocation } from "react-router-dom";
import { KanbanToolbar } from "@/components/kanban/KanbanToolbar";

// 测试跳转断言用:把当前 pathname + search 渲染成 data-testid,断言后 click
// 后的路径是否改变。比 mock useNavigate 更简洁,也覆盖了 navigate 的真实行为。
function LocationReadout() {
  const location = useLocation();
  return (
    <div data-testid="current-location">
      {location.pathname}
      {location.search}
    </div>
  );
}

function renderToolbar(
  overrides: Partial<ComponentProps<typeof KanbanToolbar>> = {},
) {
  return render(
    <MemoryRouter initialEntries={["/kanban"]}>
      <KanbanToolbar
        currentProject={null}
        projects={["alpha", "beta"]}
        onCreateTask={vi.fn()}
        {...overrides}
      />
    </MemoryRouter>,
  );
}

describe("KanbanToolbar", () => {
  it("渲染 project 下拉含 All projects + 每个 project", () => {
    renderToolbar();
    expect(screen.getByLabelText(/切换 project/)).toBeInTheDocument();
    expect(screen.getByText("All projects")).toBeInTheDocument();
    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("beta")).toBeInTheDocument();
  });

  it("未选中具体 project 时隐藏 Reveal Graph", () => {
    renderToolbar({ currentProject: null });
    expect(
      screen.queryByTestId("reveal-graph-button"),
    ).not.toBeInTheDocument();
  });

  it("选中具体 project 时显示 Reveal Graph", () => {
    renderToolbar({ currentProject: "alpha" });
    expect(
      screen.getByTestId("reveal-graph-button"),
    ).toBeInTheDocument();
  });

  it("点击 New task 调 onCreateTask 回调", () => {
    const handler = vi.fn();
    renderToolbar({ onCreateTask: handler });
    fireEvent.click(screen.getByText("New task"));
    expect(handler).toHaveBeenCalled();
  });

  it("点击 Reveal Graph 导航到 /keysight?wb=projects/{name}", () => {
    render(
      <MemoryRouter initialEntries={["/kanban?project=alpha"]}>
        <Routes>
          <Route
            path="*"
            element={
              <>
                <KanbanToolbar
                  currentProject="alpha"
                  projects={["alpha"]}
                  onCreateTask={vi.fn()}
                />
                <LocationReadout />
              </>
            }
          />
        </Routes>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByTestId("reveal-graph-button"));
    // react-router v7 不对路径里的斜杠做 percent-encoding
    expect(screen.getByTestId("current-location")).toHaveTextContent(
      "/keysight?wb=projects/alpha",
    );
  });
});
