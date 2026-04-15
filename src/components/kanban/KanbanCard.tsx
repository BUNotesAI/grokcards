import { useDraggable } from "@dnd-kit/core";
import type { TaskEntity } from "@/bindings";

interface KanbanCardProps {
  task: TaskEntity;
  /** "All projects" 模式下显示 project 标签,单项目模式隐藏。 */
  showProjectTag?: boolean;
  /** 单击卡片的回调(V1.1 Phase 6.6 follow-up),由 KanbanView 打开 TaskEditModal。 */
  onClick?: (task: TaskEntity) => void;
}

/**
 * Kanban 单 task 卡片 —— 显示 title、可选 progress 徽章(V1.1)、可选 project 标签,
 * 左侧 color 条使用 task.color。卡片是 dnd-kit 的 draggable(id = task.id),拖动时
 * 透明度降低、平移跟随 pointer。
 *
 * V1.1 Phase 6.4: 右上角显示 `done/total` progress 徽章(仅当 subtasks 非空),
 * 让用户在 Kanban 卡片粒度直接看到 checklist 完成进度。
 *
 * V1.1 Phase 6.6 follow-up: `onClick` 单击打开 TaskEditModal。配合 KanbanBoard 的
 * `PointerSensor` `activationConstraint: { distance: 8 }`,drag 只在 pointer 移动
 * > 8px 后激活,纯 click 永不触发 drag;反过来 drag gesture(移动 > 8px)的 pointer
 * up 之后浏览器默认不会 fire click 事件(因为目标 hit test 判断发生移动),所以
 * click 和 drag 在事件层面天然互斥。
 */
export function KanbanCard({
  task,
  showProjectTag = false,
  onClick,
}: KanbanCardProps) {
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

  // V1.1 progress 徽章 —— subtasks 可能为 undefined(历史数据 / 空 body)
  const subs = task.subtasks ?? [];
  const total = subs.length;
  const done = subs.filter((s) => s.done).length;
  const showProgress = total > 0;

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      onClick={onClick ? () => onClick(task) : undefined}
      className="cursor-grab rounded-md border bg-card p-3 shadow-sm active:cursor-grabbing"
      style={style}
      data-testid={`kanban-card-${task.id}`}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="text-sm font-medium">{task.title}</div>
        {showProgress && (
          <div
            className="shrink-0 rounded bg-secondary px-1.5 py-0.5 text-xs text-secondary-foreground"
            data-testid={`kanban-card-progress-${task.id}`}
            aria-label={`${done} of ${total} subtasks done`}
          >
            {done}/{total}
          </div>
        )}
      </div>
      {showProjectTag && task.project && (
        <div className="mt-1 inline-block rounded bg-secondary px-1.5 py-0.5 text-xs text-secondary-foreground">
          {task.project}
        </div>
      )}
    </div>
  );
}
