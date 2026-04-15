import { useEffect, useRef, useState, type CSSProperties } from "react";
import type { TaskEntity } from "@/bindings";
import type { LodLevel } from "@/components/keysight/types";
import {
  NodeContextMenu,
  type TaskMenuConfig,
  type SectionListItem,
} from "./NodeContextMenu";

type TaskEditField = "task-title";

interface TaskNodeProps {
  task: TaskEntity;
  style: CSSProperties;
  lodLevel?: LodLevel;
  selected?: boolean;
  highlighted?: boolean;
  dimmed?: boolean;
  /** 当前正在编辑的字段;null 表示未编辑 */
  editingField?: TaskEditField | null;
  /** 双击进入编辑模式回调 */
  onStartEdit?: (id: string, field: TaskEditField) => void;
  /** 提交编辑回调(onBlur / Enter 时触发) */
  onCommitEdit?: (id: string, field: TaskEditField, value: string) => void;
  /** 取消编辑回调(Escape) */
  onCancelEdit?: () => void;
  /** ⋯ 菜单配置；null/undefined 时不渲染菜单 */
  contextMenu?: TaskMenuConfig | null;
  /** 可选择加入的 sections(当前白板) */
  menuSections?: SectionListItem[];
  /** 当前 task 所属 section;null 表示未在任何 section */
  currentSectionId?: string | null;
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
export function TaskNode({
  task,
  style,
  lodLevel = 0,
  selected = false,
  highlighted = false,
  dimmed = false,
  editingField = null,
  onStartEdit,
  onCommitEdit,
  onCancelEdit,
  contextMenu = null,
  menuSections = [],
  currentSectionId = null,
}: TaskNodeProps) {
  const badgeClass = STATUS_STYLES[task.status] ?? STATUS_STYLES.next;
  const ring = selected
    ? "0 0 0 2px rgba(59, 130, 246, 0.8)"
    : highlighted
      ? "0 0 0 2px rgba(16, 185, 129, 0.7)"
      : undefined;
  const opacity = dimmed ? 0.35 : 1;
  const backgroundColor = task.color ?? "#ffffff";
  const isEditingTitle = editingField === "task-title";

  const [draftTitle, setDraftTitle] = useState(task.title);
  const titleInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setDraftTitle(task.title);
  }, [task.title]);

  useEffect(() => {
    if (isEditingTitle) {
      titleInputRef.current?.focus();
      titleInputRef.current?.select();
    }
  }, [isEditingTitle]);

  if (lodLevel === 2) {
    return (
      <div
        data-entity-id={task.id}
        className="select-none rounded-md border border-emerald-200 bg-emerald-200/70"
        style={{ ...style, width: 320, height: 18, opacity: dimmed ? 0.2 : 0.7, boxShadow: ring }}
      />
    );
  }

  return (
      <div
        data-entity-id={task.id}
        className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
        style={{ ...style, width: 320, opacity, boxShadow: ring, backgroundColor }}
      >
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-emerald-400 to-green-600 text-[10px] font-bold text-white">
          T
        </span>
        {isEditingTitle ? (
          <input
            ref={titleInputRef}
            value={draftTitle}
            onChange={(event) => setDraftTitle(event.target.value)}
            onBlur={(event) =>
              onCommitEdit?.(task.id, "task-title", event.currentTarget.value)
            }
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                onCommitEdit?.(task.id, "task-title", draftTitle);
              } else if (event.key === "Escape") {
                event.preventDefault();
                onCancelEdit?.();
              }
            }}
            className="h-7 flex-1 rounded-md border border-emerald-300 bg-white px-2 text-sm font-semibold text-foreground outline-none"
          />
        ) : (
          <h3
            className="flex-1 truncate text-sm font-semibold text-foreground"
            onDoubleClick={() => onStartEdit?.(task.id, "task-title")}
          >
            {task.title}
          </h3>
        )}
        <span className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${badgeClass}`}>
          {task.status}
        </span>
        {contextMenu && (
          <NodeContextMenu
            menu={contextMenu}
            sections={menuSections}
            currentSectionId={currentSectionId}
          />
        )}
      </div>
      {lodLevel === 0 && task.subtasks && task.subtasks.length > 0 ? (
        <ul
          className="space-y-0.5 px-4 pb-2 text-xs leading-relaxed text-muted-foreground"
          data-testid={`task-node-subtasks-${task.id}`}
        >
          {task.subtasks.slice(0, 6).map((sub, idx) => (
            <li key={idx} className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={sub.done}
                readOnly
                disabled
                className="h-3 w-3 flex-shrink-0"
                aria-label={`subtask ${sub.text}${sub.done ? " (done)" : ""}`}
              />
              <span
                className={`truncate ${sub.done ? "opacity-60 line-through" : ""}`}
              >
                {sub.text}
              </span>
            </li>
          ))}
          {task.subtasks.length > 6 && (
            <li className="pl-5 text-[10px] italic opacity-60">
              + {task.subtasks.length - 6} more…
            </li>
          )}
        </ul>
      ) : lodLevel === 0 && task.content ? (
        <p className="whitespace-pre-wrap px-4 pb-2 text-xs leading-relaxed text-muted-foreground">
          {task.content.length > 100
            ? task.content.slice(0, 100) + "..."
            : task.content}
        </p>
      ) : null}
      {lodLevel === 0 && (task.area || task.project) && (
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
