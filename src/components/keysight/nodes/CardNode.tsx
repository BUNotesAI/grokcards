import { memo, type CSSProperties, type MouseEvent as ReactMouseEvent } from "react";
import type { AtomicCard } from "@/bindings";
import type { LodLevel } from "@/components/keysight/types";
import { applyMarkdownBold } from "@/components/keysight/lib/applyMarkdownBold";
import { RenderedMarkdown } from "./RenderedMarkdown";
import {
  NodeContextMenu,
  type NodeMenuConfig,
  type SectionListItem,
} from "./NodeContextMenu";

/** Card 行内编辑可选字段 */
type CardEditField = "card-title" | "card-understanding";

interface CardNodeProps {
  card: AtomicCard;
  /** cardId → AtomicCard 全局查找表，用于解析 related/linkTo 显示标题 */
  cardsById?: Record<string, AtomicCard>;
  /** 反向索引：指向此 card 的 aliases（含所属 section 标题） */
  aliasRefs?: Array<{ aliasId: string; aliasTitle: string }>;
  style: CSSProperties;
  /** 外观变体：normal card vs alias ghost */
  variant?: "card" | "alias";
  lodLevel?: LodLevel;
  selected?: boolean;
  highlighted?: boolean;
  dimmed?: boolean;
  /** 是否展开 — 折叠时只显示 title + understanding；展开后显示 content + 关联列表 */
  isExpanded?: boolean;
  /** 点击 toggle 箭头时的回调 */
  onToggleExpand?: (e: ReactMouseEvent) => void;
  /** 当前正在编辑哪个字段；null 表示未编辑 */
  editingField?: CardEditField | null;
  /** 双击进入编辑模式回调 */
  onStartEdit?: (id: string, field: CardEditField) => void;
  /** 提交编辑回调 */
  onCommitEdit?: (id: string, field: CardEditField, value: string) => void;
  /** 取消编辑回调（Escape） */
  onCancelEdit?: () => void;
  /** ⋯ 菜单配置；null/undefined 时不渲染菜单 */
  contextMenu?: NodeMenuConfig | null;
  /** 可选择加入的 sections（当前白板） */
  menuSections?: SectionListItem[];
  /** 当前 card/alias 所属 section；null 表示未在任何 section */
  currentSectionId?: string | null;
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
      <RenderedMarkdown markdown={title} variant="body" />
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
  lodLevel = 0,
  selected = false,
  highlighted = false,
  dimmed = false,
  isExpanded = false,
  onToggleExpand,
  editingField = null,
  onStartEdit,
  onCommitEdit,
  onCancelEdit,
  contextMenu = null,
  menuSections = [],
  currentSectionId = null,
}: CardNodeProps) {
  const isAlias = variant === "alias";
  const isEditingTitle = editingField === "card-title";
  const isEditingUnderstanding = editingField === "card-understanding";
  const emphasisRing = selected
    ? "0 0 0 2px rgba(59, 130, 246, 0.9), 0 10px 28px rgba(59, 130, 246, 0.15)"
    : highlighted
      ? "0 0 0 2px rgba(29, 158, 117, 0.85), 0 8px 24px rgba(29, 158, 117, 0.12)"
      : "0 1px 4px rgba(0, 0, 0, 0.06), 0 0 0 0.5px rgba(0, 0, 0, 0.04)";
  const opacity = dimmed ? 0.35 : 1;

  const linkedCards = (card.linkTo ?? [])
    .map((id) => cardsById[id])
    .filter((c): c is AtomicCard => c != null);
  const relatedCards = (card.related ?? [])
    .map((id) => cardsById[id])
    .filter((c): c is AtomicCard => c != null);

  if (lodLevel === 2) {
    return (
      <div
        data-entity-id={card.id}
        style={{
          ...style,
          width: 520,
          height: 22,
          borderRadius: 4,
          border: isAlias ? "1px dashed rgba(100, 116, 139, 0.7)" : "1px solid rgba(29, 158, 117, 0.22)",
          background: isAlias ? "rgba(148, 163, 184, 0.24)" : "rgba(29, 158, 117, 0.2)",
          boxShadow: emphasisRing,
          opacity: dimmed ? 0.2 : 0.75,
          userSelect: "none",
          cursor: "grab",
        }}
      />
    );
  }

  if (lodLevel === 1) {
    return (
      <div
        data-entity-id={card.id}
        style={{
          ...style,
          width: 520,
          padding: "4px 10px",
          borderRadius: 7,
          border: isAlias ? "1px dashed rgba(100, 116, 139, 0.45)" : "1px solid rgba(0, 0, 0, 0.12)",
          background: "#ffffff",
          boxShadow: emphasisRing,
          opacity,
          userSelect: "none",
          cursor: "grab",
          display: "flex",
          alignItems: "center",
          gap: 8,
        }}
      >
        <span
          style={{
            flexShrink: 0,
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            width: 18,
            height: 18,
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
        <div
          style={{
            minWidth: 0,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            fontSize: 11,
            lineHeight: 1.3,
            fontWeight: 600,
            color: "#2C2C2A",
          }}
        >
          {card.title}
        </div>
      </div>
    );
  }

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
        boxShadow: emphasisRing,
        opacity,
        userSelect: "none",
        cursor: "grab",
      }}
    >
      {/* 标题栏：toggle 箭头 + kind badge + 标题 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "11px 12px 11px 10px",
          borderBottom: isExpanded ? "1px solid #e8e7e3" : "none",
        }}
      >
        {/* Toggle 箭头按钮 — 点击展开/折叠 */}
        <button
          type="button"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation();
            onToggleExpand?.(e);
          }}
          title={isExpanded ? "Collapse" : "Expand"}
          style={{
            flexShrink: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            width: 20,
            height: 20,
            padding: 0,
            border: "none",
            borderRadius: 5,
            background: "transparent",
            color: "rgba(0, 0, 0, 0.35)",
            fontSize: 14,
            lineHeight: 1,
            cursor: "pointer",
            transition: "color 0.15s, background 0.15s",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.color = "#111214";
            (e.currentTarget as HTMLButtonElement).style.background = "rgba(0, 0, 0, 0.06)";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.color = "rgba(0, 0, 0, 0.35)";
            (e.currentTarget as HTMLButtonElement).style.background = "transparent";
          }}
        >
          {isExpanded ? "▾" : "›"}
        </button>

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
        <div
          style={{ flex: 1, minWidth: 0 }}
          onDoubleClick={(e) => {
            e.stopPropagation();
            onStartEdit?.(card.id, "card-title");
          }}
        >
          {isEditingTitle ? (
            <input
              autoFocus
              defaultValue={card.title}
              onMouseDown={(e) => e.stopPropagation()}
              onClick={(e) => e.stopPropagation()}
              onBlur={(e) => onCommitEdit?.(card.id, "card-title", e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  e.currentTarget.blur();
                } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "b") {
                  e.preventDefault();
                  applyMarkdownBold(e.currentTarget);
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  onCancelEdit?.();
                }
              }}
              style={{
                width: "100%",
                fontSize: 14,
                fontWeight: 600,
                color: "#2C2C2A",
                border: "1.5px solid #1D9E75",
                borderRadius: 4,
                padding: "2px 6px",
                outline: "none",
                background: "#fff",
                fontFamily: "inherit",
              }}
            />
          ) : (
            <RenderedMarkdown markdown={card.title} variant="title" />
          )}
        </div>
        {contextMenu && (
          <NodeContextMenu
            menu={contextMenu}
            sections={menuSections}
            currentSectionId={currentSectionId}
          />
        )}
      </div>

      {/* Understanding 字段 — 双击进入编辑模式；空内容也允许编辑（双击占位区） */}
      {(card.understanding || isEditingUnderstanding) && (
        <div
          style={{
            margin: "10px 14px 0 14px",
            padding: "14px 18px",
            borderRadius: 10,
            background: "#F9F8F5",
            borderLeft: "3px solid #1D9E75",
          }}
          onDoubleClick={(e) => {
            e.stopPropagation();
            onStartEdit?.(card.id, "card-understanding");
          }}
        >
          {isEditingUnderstanding ? (
            <textarea
              ref={(el) => {
                if (!el) return;
                el.style.height = "auto";
                el.style.height = `${el.scrollHeight}px`;
                el.focus();
                el.setSelectionRange(el.value.length, el.value.length);
              }}
              defaultValue={card.understanding}
              onInput={(e) => {
                const el = e.currentTarget;
                el.style.height = "auto";
                el.style.height = `${el.scrollHeight}px`;
              }}
              onMouseDown={(e) => e.stopPropagation()}
              onClick={(e) => e.stopPropagation()}
              onBlur={(e) =>
                onCommitEdit?.(card.id, "card-understanding", e.currentTarget.value)
              }
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  onCancelEdit?.();
                } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "b") {
                  e.preventDefault();
                  applyMarkdownBold(e.currentTarget);
                } else if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
                  e.preventDefault();
                  e.currentTarget.blur();
                }
              }}
              style={{
                width: "100%",
                fontSize: 13.5,
                color: "#5F5E5A",
                lineHeight: 1.7,
                border: "1.5px solid #1D9E75",
                borderRadius: 4,
                padding: "6px 8px",
                outline: "none",
                background: "#fff",
                fontFamily: "inherit",
                resize: "none",
                overflow: "hidden",
              }}
            />
          ) : (
            <RenderedMarkdown markdown={card.understanding} variant="understanding" />
          )}
        </div>
      )}

      {/* 以下内容仅在展开状态下显示：content / tags / 关联 */}
      {isExpanded && card.content && (
        <div style={{ padding: "10px 14px" }}>
          <RenderedMarkdown markdown={card.content} variant="body" />
        </div>
      )}

      {isExpanded && card.tags.length > 0 && (
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

      {/* LINKED / RELATED / ALIASES / SEE ALSO — 仅展开时显示 */}
      <div style={{ padding: "0 14px 12px 14px", display: isExpanded ? "block" : "none" }}>
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
