import type { TaskEntity } from "@/bindings";

interface KanbanCardProps {
  task: TaskEntity;
  /** "All projects" 模式下显示 project 标签,单项目模式隐藏。 */
  showProjectTag?: boolean;
}

/**
 * Kanban 单 task 卡片 —— 显示 title,可选 project 标签,左侧 color 条使用 task.color。
 * V1 是纯展示,拖拽在 Phase 4 (Task 4.2) 加入。
 */
export function KanbanCard({ task, showProjectTag = false }: KanbanCardProps) {
  const color = task.color ?? undefined;
  return (
    <div
      className="rounded-md border bg-card p-3 shadow-sm"
      style={{
        borderLeftColor: color,
        borderLeftWidth: color ? 4 : 1,
      }}
      data-testid={`kanban-card-${task.id}`}
    >
      <div className="text-sm font-medium">{task.title}</div>
      {showProjectTag && task.project && (
        <div className="mt-1 inline-block rounded bg-secondary px-1.5 py-0.5 text-xs text-secondary-foreground">
          {task.project}
        </div>
      )}
    </div>
  );
}
