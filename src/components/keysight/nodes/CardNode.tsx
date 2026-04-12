import { memo, type CSSProperties } from "react";
import type { AtomicCard } from "@/bindings";
import { RenderedMarkdown } from "./RenderedMarkdown";

interface CardNodeProps {
  card: AtomicCard;
  /** cardId → AtomicCard 全局查找表，用于解析 related/linkTo 显示标题 */
  cardsById?: Record<string, AtomicCard>;
  /** 反向索引：指向此 card 的 aliases（含别名实体标题） */
  aliasRefs?: Array<{ aliasId: string; aliasTitle: string }>;
  style: CSSProperties;
  /** 外观变体：normal card vs alias ghost */
  variant?: "card" | "alias";
}

/** 区域标题 */
function SectionHeading({ label, count }: { label: string; count: number }) {
  return (
    <div className="mt-2 px-4 text-[10px] font-bold uppercase tracking-wider text-muted-foreground/70">
      {label} ({count})
    </div>
  );
}

/** 相关卡片/链接列表行 */
function LinkedRow({ title }: { title: string }) {
  return (
    <div className="px-4 py-0.5 text-xs text-foreground/80 truncate">{title}</div>
  );
}

/**
 * 原子卡片节点 — 全内容渲染（和旧 Obsidian 插件 InsightCard 对齐）
 *
 * 结构：
 * 1. 标题栏（带 C/A 图标）
 * 2. Understanding 字段（markdown 渲染，用户手写理解）
 * 3. Content 字段（markdown 渲染，卡片正文 — 代码块等）
 * 4. Tags（pill 样式）
 * 5. LINKED (N) — linkTo 列表
 * 6. RELATED (N) — related 列表
 * 7. ALIASES (N) — 反向索引，指向此卡的 aliases
 * 8. SEE ALSO (N) — seeAlso 字段
 *
 * 宽度固定 360px，max-height 800px 溢出滚动避免巨卡卡死画布。
 * variant="alias" 时外观变为虚线边框 + "A" 图标。
 */
export const CardNode = memo(function CardNode({
  card,
  cardsById = {},
  aliasRefs = [],
  style,
  variant = "card",
}: CardNodeProps) {
  const isAlias = variant === "alias";

  const linkedCards = (card.linkTo ?? [])
    .map((id) => cardsById[id])
    .filter((c): c is AtomicCard => c != null);
  const relatedCards = (card.related ?? [])
    .map((id) => cardsById[id])
    .filter((c): c is AtomicCard => c != null);

  return (
    <div
      data-entity-id={card.id}
      className={
        "select-none overflow-hidden rounded-xl bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)] " +
        (isAlias
          ? "border border-dashed border-border/60 bg-card/90"
          : "border border-border/50")
      }
      style={{ ...style, width: 360, maxHeight: 800, overflowY: "auto" }}
      onClick={(e) => e.stopPropagation()}
    >
      {/* 标题栏 */}
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span
          className={
            "flex h-5 w-5 shrink-0 items-center justify-center rounded text-[10px] font-bold text-white " +
            (isAlias
              ? "bg-gradient-to-br from-slate-400 to-slate-500"
              : "bg-gradient-to-br from-blue-400 to-indigo-500")
          }
          title={isAlias ? "Alias" : "Card"}
        >
          {isAlias ? "A" : "C"}
        </span>
        <div className="flex-1 min-w-0 text-sm font-semibold text-foreground">
          <RenderedMarkdown markdown={card.title} className="[&_p]:my-0" />
        </div>
      </div>

      {/* Understanding 字段 — 用户的思考 */}
      {card.understanding && (
        <div className="px-4 py-2 text-[11px] leading-relaxed text-foreground/90 border-t border-border/40 bg-amber-50/30 dark:bg-amber-950/10">
          <RenderedMarkdown markdown={card.understanding} />
        </div>
      )}

      {/* Content 字段 — 卡片正文，含代码块等 */}
      {card.content && (
        <div className="px-4 py-2 text-[11px] leading-relaxed text-muted-foreground">
          <RenderedMarkdown markdown={card.content} />
        </div>
      )}

      {/* Tags */}
      {card.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 px-4 pb-2">
          {card.tags.slice(0, 8).map((tag) => (
            <span
              key={tag}
              className={
                "rounded-full px-2 py-0.5 text-[10px] font-medium " +
                (isAlias
                  ? "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300"
                  : "bg-blue-50 text-blue-600 dark:bg-blue-950 dark:text-blue-300")
              }
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* LINKED */}
      {linkedCards.length > 0 && (
        <>
          <SectionHeading label="LINKED" count={linkedCards.length} />
          {linkedCards.map((c) => (
            <LinkedRow key={c.id} title={c.title} />
          ))}
        </>
      )}

      {/* RELATED */}
      {relatedCards.length > 0 && (
        <>
          <SectionHeading label="RELATED" count={relatedCards.length} />
          {relatedCards.map((c) => (
            <LinkedRow key={c.id} title={c.title} />
          ))}
        </>
      )}

      {/* ALIASES — 反向索引 */}
      {aliasRefs.length > 0 && (
        <>
          <SectionHeading label="ALIASES" count={aliasRefs.length} />
          {aliasRefs.map((a) => (
            <LinkedRow key={a.aliasId} title={a.aliasTitle} />
          ))}
        </>
      )}

      {/* SEE ALSO */}
      {card.seeAlso.length > 0 && (
        <>
          <SectionHeading label="SEE ALSO" count={card.seeAlso.length} />
          {card.seeAlso.map((entry, i) => (
            <LinkedRow key={i} title={entry} />
          ))}
        </>
      )}

      <div className="pb-2" />
    </div>
  );
});
