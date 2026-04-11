import type { CSSProperties } from "react";
import type { AtomicCard } from "@/bindings";

interface CardNodeProps {
  card: AtomicCard;
  style: CSSProperties;
}

/** 内容截断到指定长度 */
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + "...";
}

/**
 * 原子卡片节点 — Clean Elevated 风格
 *
 * 白底、精致投影、渐变图标、pill 标签。
 * 宽度 320px，显示标题 + 内容摘要 + tags。
 */
export function CardNode({ card, style }: CardNodeProps) {
  return (
    <div
      data-entity-id={card.id}
      className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
      style={{ ...style, width: 320 }}
    >
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-blue-400 to-indigo-500 text-[10px] font-bold text-white">
          C
        </span>
        <h3 className="flex-1 truncate text-sm font-semibold text-foreground">
          {card.title}
        </h3>
      </div>
      {card.content && (
        <p className="px-4 pb-2 text-xs leading-relaxed text-muted-foreground">
          {truncate(card.content, 120)}
        </p>
      )}
      {card.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 px-4 pb-3">
          {card.tags.slice(0, 5).map((tag) => (
            <span
              key={tag}
              className="rounded-full bg-blue-50 px-2 py-0.5 text-[10px] font-medium text-blue-600 dark:bg-blue-950 dark:text-blue-300"
            >
              {tag}
            </span>
          ))}
          {card.tags.length > 5 && (
            <span className="rounded-full bg-muted px-2 py-0.5 text-[10px] text-muted-foreground">
              +{card.tags.length - 5}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
