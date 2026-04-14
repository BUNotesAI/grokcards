import { useNavigate } from "react-router-dom";
import { Plus, ArrowRight } from "lucide-react";

interface KanbanToolbarProps {
  /** 当前选中的 project 名字,null = "All projects" 模式 */
  currentProject: string | null;
  /** 可选的 project 列表(由 whiteboardList 过滤 projects/* 前缀得到) */
  projects: string[];
  /** 点击 "+ New task" 按钮的回调(由 parent 控制 modal 打开) */
  onCreateTask: () => void;
}

/**
 * Kanban 顶部工具条 —— project 下拉 + "+ New task" + "Reveal Graph"。
 *
 * "Reveal Graph" 按钮只在 specific project 选中时显示(跨 project 无 canvas 可跳),
 * 点击跳转到 `/keysight?wb=projects/{name}`,让 canvas 切到同一 project 的白板。
 */
export function KanbanToolbar({
  currentProject,
  projects,
  onCreateTask,
}: KanbanToolbarProps) {
  const navigate = useNavigate();

  const handleProjectChange = (value: string) => {
    if (value === "__all__") {
      navigate("/kanban");
    } else {
      navigate(`/kanban?project=${encodeURIComponent(value)}`);
    }
  };

  const handleRevealGraph = () => {
    if (currentProject) {
      navigate(`/keysight?wb=projects/${encodeURIComponent(currentProject)}`);
    }
  };

  return (
    <div
      className="flex items-center gap-3 border-b p-4"
      data-testid="kanban-toolbar"
    >
      <select
        value={currentProject ?? "__all__"}
        onChange={(e) => handleProjectChange(e.target.value)}
        className="rounded border px-2 py-1 text-sm"
        aria-label="切换 project"
      >
        <option value="__all__">All projects</option>
        {projects.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </select>

      <button
        type="button"
        onClick={onCreateTask}
        className="flex items-center gap-1 rounded bg-primary px-3 py-1 text-sm text-primary-foreground"
      >
        <Plus className="size-3" />
        New task
      </button>

      {currentProject && (
        <button
          type="button"
          onClick={handleRevealGraph}
          className="ml-auto flex items-center gap-1 rounded border px-3 py-1 text-sm"
          data-testid="reveal-graph-button"
        >
          Reveal Graph
          <ArrowRight className="size-3" />
        </button>
      )}
    </div>
  );
}
