import type { CSSProperties } from "react";
import type { TaskEntity } from "@/bindings";

interface TaskNodeProps {
  task: TaskEntity;
  style: CSSProperties;
}

/** 任务状态 badge 样式映射 */
const STATUS_STYLES: Record<string, string> = {
  next: "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300",
  active: "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300",
  done: "bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400",
  blocked: "bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300",
};

/**
 * 任务节点 — 卡片变体 + status badge + area/project 标签。
 * 宽度 320px，和 CardNode 同族但带状态标记。
 */
export function TaskNode({ task, style }: TaskNodeProps) {
  const badgeClass = STATUS_STYLES[task.status] ?? STATUS_STYLES.next;
  return (
    <div
      data-entity-id={task.id}
      className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
      style={{ ...style, width: 320 }}
    >
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-emerald-400 to-green-600 text-[10px] font-bold text-white">
          T
        </span>
        <h3 className="flex-1 truncate text-sm font-semibold text-foreground">{task.title}</h3>
        <span className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${badgeClass}`}>
          {task.status}
        </span>
      </div>
      {task.content && (
        <p className="px-4 pb-2 text-xs leading-relaxed text-muted-foreground">
          {task.content.length > 100 ? task.content.slice(0, 100) + "..." : task.content}
        </p>
      )}
      {(task.area || task.project) && (
        <div className="flex gap-1 px-4 pb-3">
          {task.area && (
            <span className="rounded-full bg-gray-100 px-2 py-0.5 text-[10px] font-medium text-gray-600 dark:bg-gray-800 dark:text-gray-300">
              {task.area}
            </span>
          )}
          {task.project && (
            <span className="rounded-full bg-violet-50 px-2 py-0.5 text-[10px] font-medium text-violet-600 dark:bg-violet-950 dark:text-violet-300">
              {task.project}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
