import { DndContext, type DragEndEvent } from "@dnd-kit/core";
import type { TaskEntity, TaskStatus } from "@/bindings";
import { KanbanColumn } from "./KanbanColumn";
import { KanbanCard } from "./KanbanCard";
import { COLUMN_ORDER } from "./columns";

interface KanbanBoardProps {
  tasks: TaskEntity[];
  /** "All projects" 模式下卡片显示 project 标签;单项目模式隐藏。 */
  showProjectTags: boolean;
  onAddTask: (status: TaskStatus) => void;
  /** dnd drop 落到合法列时触发,由 parent 调 taskUpdate。 */
  onTaskMove: (taskId: string, newStatus: TaskStatus) => void;
}

/**
 * 从 dnd-kit 的 DragEndEvent 提取 (taskId, newStatus)。
 *
 * 合法条件:
 * - `over` 非 null(落在某个 droppable)
 * - `over.id` 是 5 个合法 TaskStatus 之一(parse don't validate,非法直接丢弃)
 *
 * 返回 null 表示这次 drop 不合法,调用方应当 noop。
 *
 * 抽成独立 pure function 方便单元测试,避免在测试里 mock @dnd-kit 内部流程。
 */
export function parseDragEnd(
  event: DragEndEvent,
): { taskId: string; newStatus: TaskStatus } | null {
  const { active, over } = event;
  if (!over) return null;
  const taskId = String(active.id);
  const overId = String(over.id);
  if (
    overId !== "inbox" &&
    overId !== "next" &&
    overId !== "active" &&
    overId !== "blocked" &&
    overId !== "done"
  ) {
    return null;
  }
  return { taskId, newStatus: overId };
}

/**
 * Kanban 板面壳 —— 横向排列 5 个 KanbanColumn(顺序由 COLUMN_ORDER 决定),
 * 按 task.status 分组渲染 KanbanCard。用 DndContext 包裹,列为 droppable、
 * 卡片为 draggable,拖拽结束触发 onTaskMove。
 */
export function KanbanBoard({
  tasks,
  showProjectTags,
  onAddTask,
  onTaskMove,
}: KanbanBoardProps) {
  const handleDragEnd = (event: DragEndEvent) => {
    const parsed = parseDragEnd(event);
    if (!parsed) return;
    onTaskMove(parsed.taskId, parsed.newStatus);
  };

  return (
    <DndContext onDragEnd={handleDragEnd}>
      <div
        className="flex gap-4 overflow-x-auto p-4"
        data-testid="kanban-board"
      >
        {COLUMN_ORDER.map((status) => {
          const columnTasks = tasks.filter((t) => t.status === status);
          return (
            <KanbanColumn
              key={status}
              status={status}
              tasks={columnTasks}
              onAddTask={onAddTask}
            >
              {columnTasks.map((task) => (
                <KanbanCard
                  key={task.id}
                  task={task}
                  showProjectTag={showProjectTags}
                />
              ))}
            </KanbanColumn>
          );
        })}
      </div>
    </DndContext>
  );
}
