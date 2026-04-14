import { useDraggable } from "@dnd-kit/core";
import type { TaskEntity } from "@/bindings";

interface KanbanCardProps {
  task: TaskEntity;
  /** "All projects" 模式下显示 project 标签,单项目模式隐藏。 */
  showProjectTag?: boolean;
}

/**
 * Kanban 单 task 卡片 —— 显示 title,可选 project 标签,左侧 color 条使用 task.color。
 * 卡片是 dnd-kit 的 draggable(id = task.id),拖动时透明度降低、平移跟随 pointer。
 */
export function KanbanCard({ task, showProjectTag = false }: KanbanCardProps) {
  const color = task.color ?? undefined;
  const { attributes, listeners, setNodeRef, transform, isDragging } =
    useDraggable({ id: task.id });

  const style: React.CSSProperties = {
    borderLeftColor: color,
    borderLeftWidth: color ? 4 : 1,
    transform: transform
      ? `translate3d(${transform.x}px, ${transform.y}px, 0)`
      : undefined,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className="cursor-grab rounded-md border bg-card p-3 shadow-sm active:cursor-grabbing"
      style={style}
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
