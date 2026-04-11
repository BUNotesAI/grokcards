import type { CSSProperties } from "react";
import type { QuestionEntity } from "@/bindings";

interface QuestionNodeProps {
  question: QuestionEntity;
  style: CSSProperties;
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
export function QuestionNode({ question, style }: QuestionNodeProps) {
  const badgeClass = STATUS_STYLES[question.status] ?? STATUS_STYLES.pending;
  return (
    <div
      data-entity-id={question.id}
      className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
      style={{ ...style, width: 320 }}
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
      {question.content && (
        <p className="px-4 pb-3 text-xs leading-relaxed text-muted-foreground">
          {question.content.length > 100
            ? question.content.slice(0, 100) + "..."
            : question.content}
        </p>
      )}
    </div>
  );
}
