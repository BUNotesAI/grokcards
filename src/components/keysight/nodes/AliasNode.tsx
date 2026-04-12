import type { CSSProperties } from "react";
import type { AtomicCard, CardAlias } from "@/bindings";

interface AliasNodeProps {
  alias: CardAlias;
  /** 目标卡片（可能为 null，如果查不到则显示降级 UI） */
  targetCard: AtomicCard | null;
  style: CSSProperties;
}

/** 内容截断到指定长度 */
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + "...";
}

/**
 * 别名节点 — 渲染目标卡片的完整内容（标题 + 正文 + tags）
 *
 * 和旧 Obsidian 插件行为一致：alias 就是另一张卡片的引用，
 * 视觉上和 CardNode 几乎一致，仅通过 "A" 图标 + 虚线边框 + 80% 透明度区分。
 */
export function AliasNode({ alias, targetCard, style }: AliasNodeProps) {
  // 降级：目标卡片不存在（可能跨白板或已删除）
  if (!targetCard) {
    return (
      <div
        data-entity-id={alias.aliasId}
        className="select-none rounded-xl border border-dashed border-border/50 bg-card/40 px-4 py-3 text-xs text-muted-foreground/60"
        style={{ ...style, width: 320 }}
      >
        <span>Alias: {alias.cardId}（目标卡片不存在）</span>
      </div>
    );
  }

  return (
    <div
      data-entity-id={alias.aliasId}
      className="select-none rounded-xl border border-dashed border-border/60 bg-card/90 shadow-[0_1px_3px_rgba(0,0,0,0.04),0_3px_10px_rgba(0,0,0,0.03)]"
      style={{ ...style, width: 320 }}
    >
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span
          className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-slate-400 to-slate-500 text-[10px] font-bold text-white"
          title="Alias"
        >
          A
        </span>
        <h3 className="flex-1 truncate text-sm font-semibold text-foreground/90">
          {targetCard.title}
        </h3>
      </div>
      {targetCard.content && (
        <p className="px-4 pb-2 text-xs leading-relaxed text-muted-foreground">
          {truncate(targetCard.content, 120)}
        </p>
      )}
      {targetCard.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 px-4 pb-3">
          {targetCard.tags.slice(0, 5).map((tag) => (
            <span
              key={tag}
              className="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-medium text-slate-600 dark:bg-slate-800 dark:text-slate-300"
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
