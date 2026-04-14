import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { TaskStatus } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { KanbanBoard } from "./KanbanBoard";

/**
 * Kanban 主页面壳。
 *
 * URL state: `?project={name}` 单项目模式,缺失时为 "All projects" 跨项目模式。
 * 主数据源: useQuery(["tasks-kanban", project]) → commands.taskQueryKanban。
 * 创建 task 的 modal state 暂置 placeholder,由 Task 3.5 接入 CreateTaskModal。
 */
export function KanbanView() {
  const [searchParams] = useSearchParams();
  const project = searchParams.get("project");

  const tasksQuery = useQuery({
    queryKey: ["tasks-kanban", project],
    queryFn: () => unwrapCommand(commands.taskQueryKanban(project)),
  });

  // 列头 + 按钮点击的 pending status,Task 3.5 会接入 CreateTaskModal
  const [, setCreatingStatus] = useState<TaskStatus | null>(null);

  if (tasksQuery.isLoading) {
    return (
      <div className="p-6 text-muted-foreground" data-testid="kanban-view">
        Loading kanban...
      </div>
    );
  }

  if (tasksQuery.isError) {
    return (
      <div className="p-6" data-testid="kanban-view">
        <div className="text-destructive">
          加载失败: {String(tasksQuery.error)}
        </div>
        <button
          type="button"
          onClick={() => tasksQuery.refetch()}
          className="mt-2 rounded border px-3 py-1"
        >
          Retry
        </button>
      </div>
    );
  }

  const tasks = tasksQuery.data ?? [];
  // "All projects" 模式才显示 project 标签(单项目模式无意义)
  const showProjectTags = !project;

  return (
    <div className="flex h-full flex-col" data-testid="kanban-view">
      <div className="border-b p-4">
        <h2 className="text-xl font-semibold">
          Kanban {project ? `— ${project}` : "(All projects)"}
        </h2>
      </div>
      <div className="flex-1 overflow-hidden">
        <KanbanBoard
          tasks={tasks}
          showProjectTags={showProjectTags}
          onAddTask={setCreatingStatus}
        />
      </div>
    </div>
  );
}
