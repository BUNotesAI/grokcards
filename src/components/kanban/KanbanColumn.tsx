import { Plus } from "lucide-react";
import type { ReactNode } from "react";
import type { TaskEntity, TaskStatus } from "@/bindings";
import { COLUMNS } from "./columns";

interface KanbanColumnProps {
  status: TaskStatus;
  tasks: TaskEntity[];
  onAddTask: (status: TaskStatus) => void;
  children?: ReactNode;
}

/**
 * Kanban 单列 —— 列头(label + count + "+" button)+ 内容区(空态文案或 children 卡片)。
 * 列的 label / 描述由 `COLUMNS[status]` 提供,不在组件里写死。
 */
export function KanbanColumn({
  status,
  tasks,
  onAddTask,
  children,
}: KanbanColumnProps) {
  const config = COLUMNS[status];

  return (
    <div
      className="flex w-72 flex-col rounded-lg bg-muted/30 p-3"
      data-testid={`kanban-column-${status}`}
    >
      <div className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-sm font-semibold">{config.label}</div>
          <div className="text-xs text-muted-foreground">{tasks.length}</div>
        </div>
        <button
          type="button"
          aria-label={`新建 ${config.label} task`}
          onClick={() => onAddTask(status)}
          className="rounded p-1 hover:bg-muted"
        >
          <Plus className="size-4" />
        </button>
      </div>
      <div className="flex flex-1 flex-col gap-2">
        {tasks.length === 0 ? (
          <div className="rounded border border-dashed p-4 text-center text-xs text-muted-foreground">
            还没有 task。点 + 创建第一个
          </div>
        ) : (
          children
        )}
      </div>
    </div>
  );
}
