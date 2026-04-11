import type { CSSProperties } from "react";
import type { GraphNote } from "@/bindings";

interface NoteNodeProps {
  note: GraphNote;
  style: CSSProperties;
}

/** 笔记颜色映射 — 便签纸配色 */
const NOTE_COLORS: Record<string, { bg: string; border: string }> = {
  yellow: {
    bg: "bg-amber-50 dark:bg-amber-950/40",
    border: "border-amber-200 dark:border-amber-800",
  },
  blue: {
    bg: "bg-blue-50 dark:bg-blue-950/40",
    border: "border-blue-200 dark:border-blue-800",
  },
  green: {
    bg: "bg-emerald-50 dark:bg-emerald-950/40",
    border: "border-emerald-200 dark:border-emerald-800",
  },
  pink: {
    bg: "bg-pink-50 dark:bg-pink-950/40",
    border: "border-pink-200 dark:border-pink-800",
  },
};

/**
 * 便签式笔记节点 — 暖色背景
 * 宽度 200px，显示标题 + 内容。
 */
export function NoteNode({ note, style }: NoteNodeProps) {
  const colors = NOTE_COLORS[note.color ?? "yellow"] ?? NOTE_COLORS.yellow;
  return (
    <div
      data-entity-id={note.id}
      className={`select-none rounded-lg border ${colors.border} ${colors.bg} shadow-[0_1px_2px_rgba(0,0,0,0.04)]`}
      style={{ ...style, width: 200 }}
    >
      <div className="px-3 pt-2 pb-1">
        <h3 className="truncate text-sm font-semibold text-foreground">
          {note.title}
        </h3>
      </div>
      {note.content && (
        <p className="px-3 pb-2 text-xs leading-relaxed text-muted-foreground">
          {note.content.length > 80
            ? note.content.slice(0, 80) + "..."
            : note.content}
        </p>
      )}
    </div>
  );
}
