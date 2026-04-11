import { render, screen } from "@testing-library/react";
import { NoteNode } from "@/components/keysight/nodes/NoteNode";
import type { GraphNote } from "@/bindings";

const mockNote: GraphNote = {
  id: "note_test0001",
  title: "测试笔记",
  content: "笔记内容详情",
};

describe("NoteNode", () => {
  it("渲染笔记标题", () => {
    render(<NoteNode note={mockNote} style={{}} />);
    expect(screen.getByText("测试笔记")).toBeInTheDocument();
  });
  it("渲染笔记内容", () => {
    render(<NoteNode note={mockNote} style={{}} />);
    expect(screen.getByText("笔记内容详情")).toBeInTheDocument();
  });
  it("带 data-entity-id 属性", () => {
    const { container } = render(<NoteNode note={mockNote} style={{}} />);
    const node = container.firstElementChild;
    expect(node?.getAttribute("data-entity-id")).toBe("note_test0001");
  });
  it("无 color 时使用默认黄色", () => {
    render(<NoteNode note={mockNote} style={{}} />);
    // 无 color 时仍能正常渲染
    expect(screen.getByText("测试笔记")).toBeInTheDocument();
  });
  it("指定 color 时正常渲染", () => {
    const blueNote: GraphNote = { ...mockNote, color: "blue" };
    render(<NoteNode note={blueNote} style={{}} />);
    expect(screen.getByText("测试笔记")).toBeInTheDocument();
  });
});
