import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { TaskEditModal } from "@/components/kanban/TaskEditModal";
import type { TaskEntity } from "@/bindings";

const baseTask: TaskEntity = {
  id: "t1",
  title: "Original title",
  whiteboardId: "projects/super-tauri",
  content: "- [ ] a\n- [x] b",
  status: "next",
  area: "backend",
  project: "super-tauri",
  color: "#ffadad",
  subtasks: [
    { text: "a", done: false },
    { text: "b", done: true },
  ],
};

/**
 * 生成 modal 并返回 onSubmit/onCancel mocks。
 *
 * 工厂函数内创建 mocks 让 TS 从 render 的 prop 类型推导出 mock 签名,避开
 * `ReturnType<typeof vi.fn>` 的 `Mock<Procedure | Constructable>` 弱类型问题。
 */
function renderModal(task: TaskEntity = baseTask) {
  const onSubmit = vi.fn().mockResolvedValue(undefined);
  const onCancel = vi.fn();
  render(<TaskEditModal task={task} onSubmit={onSubmit} onCancel={onCancel} />);
  return { onSubmit, onCancel };
}

describe("TaskEditModal", () => {
  it("渲染并预填所有字段", () => {
    renderModal();
    expect(screen.getByTestId("task-edit-modal")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Original title")).toBeInTheDocument();
    expect(screen.getByDisplayValue("backend")).toBeInTheDocument();
    expect(screen.getByDisplayValue("#ffadad")).toBeInTheDocument();
  });

  it("project 以只读形式显示,不是 input", () => {
    renderModal();
    const readonly = screen.getByTestId("task-edit-project-readonly");
    expect(readonly).toHaveTextContent("super-tauri");
    // 没有 project 的 input element
    expect(screen.queryByDisplayValue("super-tauri")).not.toBeInTheDocument();
  });

  it("ChecklistEditor 显示所有初始 subtasks 和勾选状态", () => {
    renderModal();
    const textInputs = screen.getAllByPlaceholderText("subtask text");
    expect(textInputs).toHaveLength(2);
    expect((textInputs[0] as HTMLInputElement).value).toBe("a");
    expect((textInputs[1] as HTMLInputElement).value).toBe("b");
    const checkboxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    expect(checkboxes[0].checked).toBe(false);
    expect(checkboxes[1].checked).toBe(true);
  });

  it("勾选 checkbox 切换 done 状态", () => {
    renderModal();
    const checkboxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    fireEvent.click(checkboxes[0]);
    expect(checkboxes[0].checked).toBe(true);
  });

  it("点 Add subtask 追加一行", () => {
    renderModal();
    expect(screen.getAllByPlaceholderText("subtask text")).toHaveLength(2);
    fireEvent.click(screen.getByTestId("checklist-add-button"));
    expect(screen.getAllByPlaceholderText("subtask text")).toHaveLength(3);
  });

  it("点删除按钮移除 subtask", () => {
    renderModal();
    const removeButtons = screen.getAllByLabelText("remove subtask");
    expect(removeButtons).toHaveLength(2);
    fireEvent.click(removeButtons[0]);
    expect(screen.getAllByPlaceholderText("subtask text")).toHaveLength(1);
    expect(
      (screen.getByPlaceholderText("subtask text") as HTMLInputElement).value,
    ).toBe("b");
  });

  it("改 subtask text 更新内部 state", () => {
    renderModal();
    const textInputs = screen.getAllByPlaceholderText(
      "subtask text",
    ) as HTMLInputElement[];
    fireEvent.change(textInputs[0], { target: { value: "updated a" } });
    expect(textInputs[0].value).toBe("updated a");
  });

  it("Submit 调用 onSubmit 含清理后的 Subtask 数组(无 uiKey)", async () => {
    const { onSubmit } = renderModal();
    const form = screen.getByTestId("task-edit-modal").querySelector("form")!;
    fireEvent.submit(form);
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    const payload = onSubmit.mock.calls[0][0];
    expect(payload.id).toBe("t1");
    expect(payload.title).toBe("Original title");
    expect(payload.status).toBe("next");
    expect(payload.area).toBe("backend");
    expect(payload.color).toBe("#ffadad");
    expect(payload.subtasks).toEqual([
      { text: "a", done: false },
      { text: "b", done: true },
    ]);
    // 确认没有 uiKey 字段泄漏到 payload
    for (const sub of payload.subtasks) {
      expect(sub).not.toHaveProperty("uiKey");
    }
  });

  it("Submit 过滤掉 text 为空的 subtask(trim 后)", async () => {
    const { onSubmit } = renderModal();
    // 添加一个空项
    fireEvent.click(screen.getByTestId("checklist-add-button"));
    // 把第一个改成空白
    const texts = screen.getAllByPlaceholderText(
      "subtask text",
    ) as HTMLInputElement[];
    fireEvent.change(texts[0], { target: { value: "   " } });

    const form = screen.getByTestId("task-edit-modal").querySelector("form")!;
    fireEvent.submit(form);
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    const payload = onSubmit.mock.calls[0][0];
    // 剩下只有原 "b"(原 "a" 改空白被过滤,新加的 "" 也被过滤)
    expect(payload.subtasks).toEqual([{ text: "b", done: true }]);
  });

  it("Cancel 按钮调用 onCancel", () => {
    const { onCancel } = renderModal();
    fireEvent.click(screen.getByText("Cancel"));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("title 为空时 Submit 被禁用", () => {
    renderModal({ ...baseTask, title: "" });
    const saveBtn = screen.getByText("Save") as HTMLButtonElement;
    expect(saveBtn.disabled).toBe(true);
  });

  it("catch MultiBlockChecklist typed error 显示友好提示", async () => {
    const multiBlockErr = {
      kind: "MultiBlockChecklist",
      taskId: "t1",
      blockCount: 2,
    };
    const onSubmit = vi.fn().mockRejectedValue(multiBlockErr);
    const onCancel = vi.fn();
    render(
      <TaskEditModal task={baseTask} onSubmit={onSubmit} onCancel={onCancel} />,
    );
    const form = screen.getByTestId("task-edit-modal").querySelector("form")!;
    fireEvent.submit(form);

    await waitFor(() => {
      expect(screen.getByTestId("task-edit-error")).toHaveTextContent(
        /multiple non-contiguous checklist blocks/i,
      );
    });
  });

  it("一般 error 显示为字符串", async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error("boom"));
    const onCancel = vi.fn();
    render(
      <TaskEditModal task={baseTask} onSubmit={onSubmit} onCancel={onCancel} />,
    );
    const form = screen.getByTestId("task-edit-modal").querySelector("form")!;
    fireEvent.submit(form);
    await waitFor(() => {
      expect(screen.getByTestId("task-edit-error")).toHaveTextContent("boom");
    });
  });
});
