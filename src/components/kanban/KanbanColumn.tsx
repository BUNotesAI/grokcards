import { Plus } from "lucide-react";
import type { ReactNode } from "react";
import { useDroppable } from "@dnd-kit/core";
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
 * 列本身是 dnd-kit 的 droppable(id = status 字符串),拖入卡片时 bg 稍亮提示。
 */
export function KanbanColumn({
  status,
  tasks,
  onAddTask,
  children,
}: KanbanColumnProps) {
  const config = COLUMNS[status];
  const { setNodeRef, isOver } = useDroppable({ id: status });

  return (
    <div
      ref={setNodeRef}
      className={`flex w-72 flex-col rounded-lg p-3 ${
        isOver ? "bg-muted/60" : "bg-muted/30"
      }`}
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
