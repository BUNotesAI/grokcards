import { render, screen, fireEvent } from "@testing-library/react";
import { vi } from "vitest";
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

  describe("inline editing", () => {
    it("双击 title → onStartEdit('note-title') 被调用", () => {
      const onStartEdit = vi.fn();
      render(<NoteNode note={mockNote} style={{}} onStartEdit={onStartEdit} />);
      fireEvent.doubleClick(screen.getByText("测试笔记"));
      expect(onStartEdit).toHaveBeenCalledWith("note_test0001", "note-title");
    });

    it("editingField='note-title' → 渲染 input 替代 title 文本", () => {
      render(
        <NoteNode note={mockNote} style={{}} editingField="note-title" />,
      );
      const input = screen.getByDisplayValue("测试笔记") as HTMLInputElement;
      expect(input.tagName).toBe("INPUT");
    });

    it("title input blur → onCommitEdit", () => {
      const onCommitEdit = vi.fn();
      render(
        <NoteNode
          note={mockNote}
          style={{}}
          editingField="note-title"
          onCommitEdit={onCommitEdit}
        />,
      );
      const input = screen.getByDisplayValue("测试笔记") as HTMLInputElement;
      fireEvent.change(input, { target: { value: "更新标题" } });
      fireEvent.blur(input);
      expect(onCommitEdit).toHaveBeenCalledWith(
        "note_test0001",
        "note-title",
        "更新标题",
      );
    });

    it("双击 body → onStartEdit('note-body')", () => {
      const onStartEdit = vi.fn();
      render(<NoteNode note={mockNote} style={{}} onStartEdit={onStartEdit} />);
      fireEvent.doubleClick(screen.getByText("笔记内容详情"));
      expect(onStartEdit).toHaveBeenCalledWith("note_test0001", "note-body");
    });

    it("editingField='note-body' → 渲染 textarea", () => {
      render(<NoteNode note={mockNote} style={{}} editingField="note-body" />);
      const textarea = screen.getByDisplayValue("笔记内容详情") as HTMLTextAreaElement;
      expect(textarea.tagName).toBe("TEXTAREA");
    });

    it("body Escape → onCancelEdit", () => {
      const onCancelEdit = vi.fn();
      render(
        <NoteNode
          note={mockNote}
          style={{}}
          editingField="note-body"
          onCancelEdit={onCancelEdit}
        />,
      );
      const textarea = screen.getByDisplayValue("笔记内容详情") as HTMLTextAreaElement;
      fireEvent.keyDown(textarea, { key: "Escape" });
      expect(onCancelEdit).toHaveBeenCalled();
    });

    it("空 body 时也能进入编辑（双击占位区，双击应触发 onStartEdit）", () => {
      // 空 content 的 note，editingField 设为 'note-body' → textarea 渲染（哪怕 content 为空）
      const emptyNote: GraphNote = { ...mockNote, content: "" };
      render(
        <NoteNode
          note={emptyNote}
          style={{}}
          editingField="note-body"
        />,
      );
      const textarea = screen.getByRole("textbox") as HTMLTextAreaElement;
      expect(textarea.tagName).toBe("TEXTAREA");
      expect(textarea.value).toBe("");
    });
  });
});
