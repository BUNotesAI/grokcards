import type { TaskEntity, TaskStatus } from "@/bindings";
import { KanbanColumn } from "./KanbanColumn";
import { KanbanCard } from "./KanbanCard";
import { COLUMN_ORDER } from "./columns";

interface KanbanBoardProps {
  tasks: TaskEntity[];
  /** "All projects" 模式下卡片显示 project 标签;单项目模式隐藏。 */
  showProjectTags: boolean;
  onAddTask: (status: TaskStatus) => void;
}

/**
 * Kanban 板面壳 —— 横向排列 5 个 KanbanColumn(顺序由 COLUMN_ORDER 决定),
 * 按 task.status 分组渲染 KanbanCard。V1 无拖拽(Phase 4 加入)。
 *
 * 分组策略:按 COLUMN_ORDER 顺序 filter 一次 tasks,避免写入 Record 时的
 * 字符串-字面量协变陷阱。任务量很小(V1 kanban 通常 <100 task),O(5n) 是可接受的。
 * status 不在 COLUMN_ORDER 的 task 会被静默忽略 —— Rust 端 `parse_task_status`
 * 已经保证 status 合法,这种情况不会出现。
 */
export function KanbanBoard({
  tasks,
  showProjectTags,
  onAddTask,
}: KanbanBoardProps) {
  return (
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
  );
}
