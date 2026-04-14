import { useState, type FormEventHandler } from "react";
import type { TaskStatus } from "@/bindings";
import { COLUMN_ORDER, COLUMNS } from "./columns";

interface CreateTaskModalProps {
  /** parent 每次打开时传入,由条件渲染 `{open && <CreateTaskModal .../>}` 保证 fresh */
  initialStatus: TaskStatus;
  /** 当前 view 的 project(单项目模式),null 时用 availableProjects[0] */
  initialProject: string | null;
  availableProjects: string[];
  onSubmit: (data: {
    project: string;
    title: string;
    status: TaskStatus;
  }) => Promise<void>;
  onCancel: () => void;
}

/**
 * 新建 task modal —— project / title / status 表单。
 *
 * **Stale state 防护**: 本组件**无 `open` prop**,也不做 `if (!open) return null`。
 * Parent 必须用 `{modalState.open && <CreateTaskModal ... />}` 条件渲染,
 * 保证每次打开是新实例,自动从 `initialStatus` / `initialProject` 取最新值。
 * 这样从 Inbox 列打开和从 Blocked 列打开不会复用 stale state(codex review §4)。
 */
export function CreateTaskModal({
  initialStatus,
  initialProject,
  availableProjects,
  onSubmit,
  onCancel,
}: CreateTaskModalProps) {
  const [project, setProject] = useState(
    initialProject ?? availableProjects[0] ?? "",
  );
  const [title, setTitle] = useState("");
  const [status, setStatus] = useState<TaskStatus>(initialStatus);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit: FormEventHandler<HTMLFormElement> = async (e) => {
    e.preventDefault();
    if (!title.trim() || !project) return;
    setSubmitting(true);
    setError(null);
    try {
      await onSubmit({ project, title: title.trim(), status });
      setTitle("");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
      data-testid="create-task-modal"
      role="dialog"
    >
      <form
        onSubmit={handleSubmit}
        className="w-96 rounded-lg bg-card p-6 shadow-xl"
      >
        <h3 className="mb-4 text-lg font-semibold">New task</h3>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Project</div>
          <select
            value={project}
            onChange={(e) => setProject(e.target.value)}
            className="w-full rounded border px-2 py-1.5"
            required
          >
            {availableProjects.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </label>

        <label className="mb-3 block">
          <div className="mb-1 text-sm font-medium">Title</div>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="w-full rounded border px-2 py-1.5"
            placeholder="What needs doing?"
            autoFocus
            required
          />
        </label>

        <label className="mb-4 block">
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

        {error && (
          <div className="mb-3 text-sm text-destructive">{error}</div>
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
            disabled={submitting || !title.trim() || !project}
            className="rounded bg-primary px-3 py-1 text-sm text-primary-foreground disabled:opacity-50"
          >
            {submitting ? "Creating..." : "Create"}
          </button>
        </div>
      </form>
    </div>
  );
}
