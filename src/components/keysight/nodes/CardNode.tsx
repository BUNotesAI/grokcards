import { memo, type CSSProperties } from "react";
import type { AtomicCard } from "@/bindings";
import { RenderedMarkdown } from "./RenderedMarkdown";

interface CardNodeProps {
  card: AtomicCard;
  /** cardId → AtomicCard 全局查找表，用于解析 related/linkTo 显示标题 */
  cardsById?: Record<string, AtomicCard>;
  /** 反向索引：指向此 card 的 aliases（含所属 section 标题） */
  aliasRefs?: Array<{ aliasId: string; aliasTitle: string }>;
  style: CSSProperties;
  /** 外观变体：normal card vs alias ghost */
  variant?: "card" | "alias";
}

/** 区域标题（LINKED / RELATED / ALIASES / SEE ALSO） */
function SectionHeading({ label, count }: { label: string; count: number }) {
  return (
    <div
      style={{
        marginTop: 8,
        padding: "4px 8px",
        fontSize: 11,
        fontWeight: 500,
        color: "#1D9E75",
        background: "rgba(29, 158, 117, 0.08)",
        borderRadius: 4,
      }}
    >
      ▸ {label} ({count})
    </div>
  );
}

/** 关联卡片行 — 左侧绿色竖条，和旧 ks-linked-card 样式对齐 */
function LinkedRow({ title }: { title: string }) {
  return (
    <div
      style={{
        marginTop: 4,
        padding: "6px 10px",
        borderRadius: 6,
        background: "#fafaf8",
        borderLeft: "3px solid #1D9E75",
        fontSize: 12,
        color: "#4a4a47",
        opacity: 0.88,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
      }}
    >
      {title}
    </div>
  );
}

/**
 * 原子卡片节点 — 严格对齐旧 Obsidian 插件样式
 *
 * 颜色 / 尺寸参考 styles.css 中的 .ks-graph-card 和 .ks-graph-card-understanding-view：
 * - 宽度 520px（旧插件 CARD_W = 520）
 * - 背景白 #ffffff，边框 1.5px rgba(0,0,0,0.10)，圆角 10px
 * - 标题 #2C2C2A, 14px, font-weight 600
 * - 加粗 rgb(229, 84, 3) 橙色
 * - Understanding 琥珀绿分区：#F9F8F5 底 + 左侧 3px #1D9E75 竖条
 * - Content 代码块 #1e1e1e 深色底
 * - max-height 700px 溢出滚动，避免巨卡卡死画布
 *
 * variant="alias" 时外观变为虚线边框 + 灰色竖条。
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
      style={{
        ...style,
        width: 520,
        maxHeight: 700,
        overflowY: "auto",
        background: "#ffffff",
        border: isAlias ? "1.5px dashed rgba(0, 0, 0, 0.12)" : "1.5px solid rgba(0, 0, 0, 0.10)",
        borderRadius: 10,
        boxShadow: "0 1px 4px rgba(0, 0, 0, 0.06), 0 0 0 0.5px rgba(0, 0, 0, 0.04)",
        userSelect: "none",
        cursor: "grab",
      }}
    >
      {/* 标题栏 */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          gap: 10,
          padding: "12px 14px 8px 14px",
          borderBottom: "1px solid #e8e7e3",
        }}
      >
        <span
          title={isAlias ? "Alias" : "Card"}
          style={{
            flexShrink: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            width: 20,
            height: 20,
            borderRadius: 4,
            fontSize: 10,
            fontWeight: 700,
            color: "#fff",
            background: isAlias
              ? "linear-gradient(to bottom right, #94a3b8, #64748b)"
              : "linear-gradient(to bottom right, #60a5fa, #4f46e5)",
          }}
        >
          {isAlias ? "A" : "C"}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <RenderedMarkdown markdown={card.title} variant="title" />
        </div>
      </div>

      {/* Understanding 字段 — 用户的洞察，琥珀色/绿色强调 */}
      {card.understanding && (
        <div
          style={{
            margin: "10px 14px 0 14px",
            padding: "14px 18px",
            borderRadius: 10,
            background: "#F9F8F5",
            borderLeft: "3px solid #1D9E75",
          }}
        >
          <RenderedMarkdown markdown={card.understanding} variant="understanding" />
        </div>
      )}

      {/* Content 字段 — 卡片正文（代码块等） */}
      {card.content && (
        <div style={{ padding: "10px 14px" }}>
          <RenderedMarkdown markdown={card.content} variant="body" />
        </div>
      )}

      {/* Tags */}
      {card.tags.length > 0 && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, padding: "0 14px 8px 14px" }}>
          {card.tags.slice(0, 8).map((tag) => (
            <span
              key={tag}
              style={{
                fontSize: 10,
                color: "#6b7280",
                background: "#f3f4f6",
                padding: "1px 6px",
                borderRadius: 4,
              }}
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* LINKED / RELATED / ALIASES / SEE ALSO */}
      <div style={{ padding: "0 14px 12px 14px" }}>
        {linkedCards.length > 0 && (
          <>
            <SectionHeading label="LINKED" count={linkedCards.length} />
            {linkedCards.map((c) => (
              <LinkedRow key={c.id} title={c.title} />
            ))}
          </>
        )}
        {relatedCards.length > 0 && (
          <>
            <SectionHeading label="RELATED" count={relatedCards.length} />
            {relatedCards.map((c) => (
              <LinkedRow key={c.id} title={c.title} />
            ))}
          </>
        )}
        {aliasRefs.length > 0 && (
          <>
            <SectionHeading label="ALIASES" count={aliasRefs.length} />
            {aliasRefs.map((a) => (
              <LinkedRow key={a.aliasId} title={a.aliasTitle} />
            ))}
          </>
        )}
        {card.seeAlso.length > 0 && (
          <>
            <SectionHeading label="SEE ALSO" count={card.seeAlso.length} />
            {card.seeAlso.map((entry, i) => (
              <LinkedRow key={i} title={entry} />
            ))}
          </>
        )}
      </div>
    </div>
  );
});
