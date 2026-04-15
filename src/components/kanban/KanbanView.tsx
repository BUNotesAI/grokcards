import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { Subtask, TaskEntity, TaskStatus } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { KanbanBoard } from "./KanbanBoard";
import { KanbanToolbar } from "./KanbanToolbar";
import { CreateTaskModal } from "./CreateTaskModal";
import { TaskEditModal } from "./TaskEditModal";
import { invalidateAllTaskCaches } from "./invalidateAllTaskCaches";
import { COLUMN_ORDER } from "./columns";

interface ModalState {
  open: boolean;
  status: TaskStatus;
}

interface EditState {
  open: boolean;
  task: TaskEntity | null;
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

  // V1.1 Phase 6.6: 双击打开 TaskEditModal
  const [editState, setEditState] = useState<EditState>({
    open: false,
    task: null,
  });

  const openModal = (status: TaskStatus) =>
    setModalState({ open: true, status });

  const closeModal = () =>
    setModalState((prev) => ({ ...prev, open: false }));

  const openEdit = (task: TaskEntity) => setEditState({ open: true, task });

  const closeEdit = () => setEditState((prev) => ({ ...prev, open: false }));

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

  const handleTaskMove = async (taskId: string, newStatus: TaskStatus) => {
    try {
      await unwrapCommand(
        commands.taskUpdate(taskId, null, null, newStatus, null, null),
      );
      invalidateAllTaskCaches(queryClient);
    } catch (err) {
      // V1 不做 optimistic update,失败时 loud 报 console + query refetch 自动回滚视觉
      console.error("Task move failed:", err);
    }
  };

  // V1.1 Phase 6.6: modal submit → 单个 command `task_update_with_subtasks`
  const handleEditSubmit = async (data: {
    id: string;
    title: string;
    subtasks: Subtask[];
    status: TaskStatus;
    area: string | null;
    color: string | null;
  }) => {
    await unwrapCommand(
      commands.taskUpdateWithSubtasks(
        data.id,
        data.title,
        data.subtasks,
        data.status,
        data.area,
        data.color,
      ),
    );
    invalidateAllTaskCaches(queryClient);
    closeEdit();
  };

  if (tasksQuery.isLoading) {
    return (
      <div
        className="flex h-full flex-col"
        data-testid="kanban-view"
        aria-busy="true"
      >
        <div className="border-b p-4 text-muted-foreground">
          Kanban — loading...
        </div>
        <div className="flex gap-4 p-4">
          {COLUMN_ORDER.map((status) => (
            <div
              key={status}
              className="w-72 animate-pulse rounded-lg bg-muted/30 p-3"
              data-testid={`kanban-column-${status}-skeleton`}
            >
              <div className="mb-3 h-6 w-20 rounded bg-muted" />
              {[1, 2].map((i) => (
                <div key={i} className="mb-2 h-12 rounded bg-muted" />
              ))}
            </div>
          ))}
        </div>
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
          onTaskMove={handleTaskMove}
          onTaskClick={openEdit}
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
      {editState.open && editState.task && (
        <TaskEditModal
          task={editState.task}
          onSubmit={handleEditSubmit}
          onCancel={closeEdit}
        />
      )}
    </div>
  );
}
