import type { TaskStatus } from "@/bindings";

/**
 * 单列配置 —— label 是 UI 展示文案,description 是二级说明。
 */
export interface ColumnConfig {
  status: TaskStatus;
  label: string;
  description: string;
}

/**
 * 5 个 status 列的配置,用 `Record<TaskStatus, ...>` 强制穷尽 ——
 * 将来给 TaskStatus 加新 variant 时,这里编译期失败,所有引用点必须同步更新。
 */
export const COLUMNS: Record<TaskStatus, ColumnConfig> = {
  inbox: { status: "inbox", label: "Inbox", description: "收集需求" },
  next: { status: "next", label: "Next", description: "下一步" },
  active: { status: "active", label: "Active", description: "进行中" },
  blocked: { status: "blocked", label: "Blocked", description: "阻塞" },
  done: { status: "done", label: "Done", description: "完成" },
};

/**
 * 列在 board 上的固定显示顺序。
 */
export const COLUMN_ORDER: TaskStatus[] = [
  "inbox",
  "next",
  "active",
  "blocked",
  "done",
];
