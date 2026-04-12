import { render, screen, fireEvent } from "@testing-library/react";
import { WhiteboardNode } from "@/components/keysight/nodes/WhiteboardNode";
import type { WhiteboardSummary } from "@/bindings";
import { vi } from "vitest";

const mockSummary: WhiteboardSummary = {
  whiteboardId: "rust",
  cards: 145,
  notes: 72,
  sections: 3,
  aliases: 6,
  tasks: 0,
  questions: 0,
};

describe("WhiteboardNode", () => {
  it("渲染白板名称和非零统计", () => {
    render(<WhiteboardNode summary={mockSummary} onNavigate={vi.fn()} style={{}} />);
    expect(screen.getByText("rust")).toBeInTheDocument();
    expect(screen.getByText(/145 cards/)).toBeInTheDocument();
    expect(screen.getByText(/72 notes/)).toBeInTheDocument();
    expect(screen.getByText(/6 aliases/)).toBeInTheDocument();
    // 零值不显示
    expect(screen.queryByText(/task/)).not.toBeInTheDocument();
    expect(screen.queryByText(/question/)).not.toBeInTheDocument();
  });

  it("点击触发 onNavigate", () => {
    const onNavigate = vi.fn();
    render(<WhiteboardNode summary={mockSummary} onNavigate={onNavigate} style={{}} />);
    fireEvent.click(screen.getByText("rust"));
    expect(onNavigate).toHaveBeenCalledWith("rust");
  });

  it("空白板显示 empty", () => {
    const emptySummary: WhiteboardSummary = {
      whiteboardId: "agent",
      cards: 0,
      notes: 0,
      sections: 0,
      aliases: 0,
      tasks: 0,
      questions: 0,
    };
    render(<WhiteboardNode summary={emptySummary} onNavigate={vi.fn()} style={{}} />);
    expect(screen.getByText("agent")).toBeInTheDocument();
    expect(screen.getByText("empty")).toBeInTheDocument();
  });
});
