import { useSearchParams } from "react-router-dom";

/**
 * Kanban 主页面壳。
 *
 * URL state: `?project={name}` 单项目模式,缺失时为 "All projects" 跨项目模式。
 * V1 阶段先放空白 placeholder,后续 task 接入 toolbar / board / cards / modal。
 */
export function KanbanView() {
  const [searchParams] = useSearchParams();
  const project = searchParams.get("project");

  return (
    <div className="flex h-full flex-col p-6" data-testid="kanban-view">
      <h2 className="mb-4 text-xl font-semibold">
        Kanban {project ? `— ${project}` : "(All projects)"}
      </h2>
      <p className="text-muted-foreground">Kanban view 还在搭建中...</p>
    </div>
  );
}
