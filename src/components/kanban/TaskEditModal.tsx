import { useState, type FormEventHandler } from "react";
import type { Subtask, TaskEntity, TaskStatus } from "@/bindings";
import { COLUMN_ORDER, COLUMNS } from "./columns";

/**
 * V1.1 Phase 6.5: Task 编辑 modal,支持 title / status / area / color 元数据
 * 编辑 + ChecklistEditor(增删勾改 subtask)。**无 content textarea** —— V1.1
 * 决策不做自由文本 body 编辑,只改 checklist;非 checklist 部分由 Rust 从
 * `current.content` 继承(merge base),延 V1.2 再做 markdown 编辑器。
 *
 * **Stale state 防护**: parent 必须用 `{editState.open && <TaskEditModal task={...} .../>}`
 * 条件渲染,每次打开是新实例,从 `task` prop 取最新值。
 *
 * **Project 只读**: `task::update` 的契约明确不允许改 project(project 决定文件
 * 目录,跨目录迁移是另一个独立操作)。Modal 只显示不允许编辑。
 */

interface TaskEditModalProps {
  task: TaskEntity;
  onSubmit: (data: {
    id: string;
    title: string;
    subtasks: Subtask[];
    status: TaskStatus;
    area: string | null;
    color: string | null;
  }) => Promise<void>;
  onCancel: () => void;
}

/** 给 Subtask 加本地 React key(纯 UI state,不进持久化)。 */
type SubtaskWithUiKey = Subtask & { uiKey: string };

function toWithKey(s: Subtask): SubtaskWithUiKey {
  return { ...s, uiKey: crypto.randomUUID() };
}

export function TaskEditModal({ task, onSubmit, onCancel }: TaskEditModalProps) {
  const [title, setTitle] = useState(task.title);
  const [subtasks, setSubtasks] = useState<SubtaskWithUiKey[]>(() =>
    (task.subtasks ?? []).map(toWithKey),
  );
  const [status, setStatus] = useState<TaskStatus>(task.status);
  const [area, setArea] = useState(task.area ?? "");
  const [color, setColor] = useState(task.color ?? "");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit: FormEventHandler<HTMLFormElement> = async (e) => {
    e.preventDefault();
    if (!title.trim()) return;
    setSubmitting(true);
    setError(null);
    try {
      // 剥掉 uiKey 还原 Subtask,顺便 trim text 并过滤空白项
      const cleanSubtasks: Subtask[] = subtasks
        .filter((s) => s.text.trim().length > 0)
        .map(({ uiKey: _uiKey, ...rest }) => ({
          ...rest,
          text: rest.text.trim(),
        }));

      await onSubmit({
        id: task.id,
        title: title.trim(),
        subtasks: cleanSubtasks,
        status,
        area: area.trim() || null,
        color: color.trim() || null,
      });
    } catch (err) {
      // V1.1 Phase 6.3: Rust 写路径遇多 block checklist 抛 typed error,
      // 这里按 discriminated union 穷尽 match,显示友好提示
      if (
        typeof err === "object" &&
        err !== null &&
        "kind" in err &&
        (err as { kind: unknown }).kind === "MultiBlockChecklist"
      ) {
        setError(
          "This task has multiple non-contiguous checklist blocks. Please edit the markdown file directly.",
        );
      } else {
        setError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
      data-testid="task-edit-modal"
      role="dialog"
    >
      <form
        onSubmit={handleSubmit}
        className="w-[28rem] rounded-lg bg-card p-6 shadow-xl"
      >
        <h3 className="mb-4 text-lg font-semibold">Edit task</h3>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Title</div>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="w-full rounded border px-2 py-1.5"
            autoFocus
            required
          />
        </label>

        <div className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Project</div>
          <div
            className="w-full rounded border bg-muted px-2 py-1.5 text-sm text-muted-foreground"
            data-testid="task-edit-project-readonly"
          >
            {task.project ?? "(no project)"}
          </div>
        </div>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Status</div>
          <select
            value={status}
            onChange={(e) => setStatus(e.target.value as TaskStatus)}
            className="w-full rounded border px-2 py-1.5"
          >
            {COLUMN_ORDER.map((s) => (
              <option key={s} value={s}>
                {COLUMNS[s].label}
              </option>
            ))}
          </select>
        </label>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Area</div>
          <input
            type="text"
            value={area}
            onChange={(e) => setArea(e.target.value)}
            className="w-full rounded border px-2 py-1.5"
            placeholder="(optional)"
          />
        </label>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Color</div>
          <input
            type="text"
            value={color}
            onChange={(e) => setColor(e.target.value)}
            className="w-full rounded border px-2 py-1.5"
            placeholder="#hex or leave empty"
          />
        </label>

        <div className="mb-4 block">
          <div className="mb-1 text-sm font-medium">Checklist</div>
          <ChecklistEditor subtasks={subtasks} onChange={setSubtasks} />
        </div>

        {error && (
          <div
            className="mb-3 text-sm text-destructive"
            data-testid="task-edit-error"
          >
            {error}
          </div>
        )}

        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded border px-3 py-1 text-sm"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting || !title.trim()}
            className="rounded bg-primary px-3 py-1 text-sm text-primary-foreground disabled:opacity-50"
          >
            {submitting ? "Saving..." : "Save"}
          </button>
        </div>
      </form>
    </div>
  );
}

// ============================================================
// ChecklistEditor 子组件
// ============================================================

interface ChecklistEditorProps {
  subtasks: SubtaskWithUiKey[];
  onChange: (next: SubtaskWithUiKey[]) => void;
}

/**
 * Checklist 编辑器 —— 支持勾选、改 text、增加、删除 subtask。
 *
 * uiKey 用 crypto.randomUUID() 本地生成,仅作 React list key,不进持久化
 * (符合 L0 硬约束 "ui_" 前缀例外 —— 纯 UI state key)。Submit 时 TaskEditModal
 * 会 map 剥掉 uiKey 还原为 `Subtask { text, done }`。
 */
function ChecklistEditor({ subtasks, onChange }: ChecklistEditorProps) {
  const toggle = (uiKey: string) =>
    onChange(
      subtasks.map((s) =>
        s.uiKey === uiKey ? { ...s, done: !s.done } : s,
      ),
    );

  const updateText = (uiKey: string, text: string) =>
    onChange(subtasks.map((s) => (s.uiKey === uiKey ? { ...s, text } : s)));

  const remove = (uiKey: string) =>
    onChange(subtasks.filter((s) => s.uiKey !== uiKey));

  const add = () =>
    onChange([
      ...subtasks,
      { text: "", done: false, uiKey: crypto.randomUUID() },
    ]);

  return (
    <div className="space-y-1" data-testid="checklist-editor">
      {subtasks.length === 0 && (
        <div className="text-xs text-muted-foreground">No subtasks yet.</div>
      )}
      {subtasks.map((s) => (
        <div key={s.uiKey} className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={s.done}
            onChange={() => toggle(s.uiKey)}
            aria-label={`toggle subtask ${s.text || "(empty)"}`}
            data-testid={`checklist-item-checkbox-${s.uiKey}`}
          />
          <input
            type="text"
            value={s.text}
            onChange={(e) => updateText(s.uiKey, e.target.value)}
            className="flex-1 rounded border px-2 py-1 text-sm"
            placeholder="subtask text"
            data-testid={`checklist-item-text-${s.uiKey}`}
          />
          <button
            type="button"
            onClick={() => remove(s.uiKey)}
            aria-label="remove subtask"
            className="text-muted-foreground hover:text-destructive"
            data-testid={`checklist-item-remove-${s.uiKey}`}
          >
            ×
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={add}
        className="mt-2 text-xs text-primary hover:underline"
        data-testid="checklist-add-button"
      >
        + Add subtask
      </button>
    </div>
  );
}
