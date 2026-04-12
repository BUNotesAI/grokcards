import type { CSSProperties } from "react";
import type { QuestionEntity } from "@/bindings";
import type { LodLevel } from "@/components/keysight/types";

interface QuestionNodeProps {
  question: QuestionEntity;
  style: CSSProperties;
  lodLevel?: LodLevel;
  selected?: boolean;
  highlighted?: boolean;
  dimmed?: boolean;
}

/** 问题状态 badge 样式映射 */
const STATUS_STYLES: Record<string, string> = {
  pending: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300",
  doing: "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300",
  done: "bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400",
  understood: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300",
};

/**
 * 问题节点 — 卡片变体 + status badge。
 * 宽度 320px，和 CardNode 同族但带问题状态标记。
 */
export function QuestionNode({
  question,
  style,
  lodLevel = 0,
  selected = false,
  highlighted = false,
  dimmed = false,
}: QuestionNodeProps) {
  const badgeClass = STATUS_STYLES[question.status] ?? STATUS_STYLES.pending;
  const ring = selected
    ? "0 0 0 2px rgba(245, 158, 11, 0.8)"
    : highlighted
      ? "0 0 0 2px rgba(16, 185, 129, 0.7)"
      : undefined;
  const opacity = dimmed ? 0.35 : 1;

  if (lodLevel === 2) {
    return (
      <div
        data-entity-id={question.id}
        className="select-none rounded-md border border-amber-200 bg-amber-200/70"
        style={{ ...style, width: 320, height: 18, opacity: dimmed ? 0.2 : 0.7, boxShadow: ring }}
      />
    );
  }

  return (
    <div
      data-entity-id={question.id}
      className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
      style={{ ...style, width: 320, opacity, boxShadow: ring }}
    >
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-amber-400 to-orange-500 text-[10px] font-bold text-white">
          Q
        </span>
        <h3 className="flex-1 truncate text-sm font-semibold text-foreground">{question.title}</h3>
        <span className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${badgeClass}`}>
          {question.status}
        </span>
      </div>
      {lodLevel === 0 && question.content && (
        <p className="px-4 pb-3 text-xs leading-relaxed text-muted-foreground">
          {question.content.length > 100
            ? question.content.slice(0, 100) + "..."
            : question.content}
        </p>
      )}
    </div>
  );
}
