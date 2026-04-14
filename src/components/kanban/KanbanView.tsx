import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { TaskStatus } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { KanbanBoard } from "./KanbanBoard";
import { KanbanToolbar } from "./KanbanToolbar";
import { CreateTaskModal } from "./CreateTaskModal";
import { invalidateAllTaskCaches } from "./invalidateAllTaskCaches";

interface ModalState {
  open: boolean;
  status: TaskStatus;
}

/**
 * Kanban 主页面壳。
 *
 * URL state: `?project={name}` 单项目模式,缺失时为 "All projects" 跨项目模式。
 * 主数据源: `useQuery(["tasks-kanban", project])` → `commands.taskQueryKanban`。
 *
 * 创建 task 数据流:
 * - 列头 "+" 或 toolbar "+ New task" → openModal(status) → modalState.open = true
 * - modal 受控渲染 `{modalState.open && <CreateTaskModal />}`(stale state 防护)
 * - modal onSubmit → commands.taskCreate → invalidateAllTaskCaches → 关闭 modal
 * - 错误通过 unwrapCommand throw,由 CreateTaskModal 内部 try/catch 显示
 */
export function KanbanView() {
  const [searchParams] = useSearchParams();
  const project = searchParams.get("project");
  const queryClient = useQueryClient();

  const tasksQuery = useQuery({
    queryKey: ["tasks-kanban", project],
    queryFn: () => unwrapCommand(commands.taskQueryKanban(project)),
  });

  const whiteboardsQuery = useQuery({
    queryKey: ["whiteboards"],
    queryFn: () => unwrapCommand(commands.whiteboardList()),
  });

  // projects 列表:从 whiteboardList 过滤 `projects/*` 前缀,剥掉前缀只留 project 名
  const projects = (whiteboardsQuery.data ?? [])
    .filter((wb) => wb.whiteboardId.startsWith("projects/"))
    .map((wb) => wb.whiteboardId.slice("projects/".length));

  const [modalState, setModalState] = useState<ModalState>({
    open: false,
    status: "inbox",
  });

  const openModal = (status: TaskStatus) =>
    setModalState({ open: true, status });

  const closeModal = () =>
    setModalState((prev) => ({ ...prev, open: false }));

  const handleCreateTask = async (data: {
    project: string;
    title: string;
    status: TaskStatus;
  }) => {
    await unwrapCommand(
      commands.taskCreate(
        data.project,
        data.title,
        null,
        data.status,
        null,
        null,
      ),
    );
    invalidateAllTaskCaches(queryClient, `projects/${data.project}`);
    closeModal();
  };

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
      <KanbanToolbar
        currentProject={project}
        projects={projects}
        onCreateTask={() => openModal("inbox")}
      />
      <div className="px-4 pt-2 text-xs text-muted-foreground">
        Kanban {project ? `— ${project}` : "(All projects)"}
      </div>
      <div className="flex-1 overflow-hidden">
        <KanbanBoard
          tasks={tasks}
          showProjectTags={showProjectTags}
          onAddTask={openModal}
        />
      </div>
      {modalState.open && (
        <CreateTaskModal
          initialStatus={modalState.status}
          initialProject={project}
          availableProjects={projects}
          onSubmit={handleCreateTask}
          onCancel={closeModal}
        />
      )}
    </div>
  );
}
