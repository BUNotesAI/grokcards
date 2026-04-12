import type { CSSProperties, MouseEvent as ReactMouseEvent } from "react";
import type { AtomicCard, CardAlias } from "@/bindings";
import type { LodLevel } from "@/components/keysight/types";
import { CardNode } from "./CardNode";

interface AliasNodeProps {
  alias: CardAlias;
  /** 目标卡片（可能为 null，如果查不到则显示降级 UI） */
  targetCard: AtomicCard | null;
  cardsById?: Record<string, AtomicCard>;
  aliasRefs?: Array<{ aliasId: string; aliasTitle: string }>;
  style: CSSProperties;
  lodLevel?: LodLevel;
  selected?: boolean;
  highlighted?: boolean;
  dimmed?: boolean;
  isExpanded?: boolean;
  onToggleExpand?: (e: ReactMouseEvent) => void;
}

/**
 * 别名节点 — 复用 CardNode variant="alias" 渲染目标卡片完整内容。
 *
 * 拖拽由外层 EntityNode 的 wrapper 处理。
 */
export function AliasNode({
  alias,
  targetCard,
  cardsById = {},
  aliasRefs = [],
  style,
  lodLevel = 0,
  selected = false,
  highlighted = false,
  dimmed = false,
  isExpanded = false,
  onToggleExpand,
}: AliasNodeProps) {
  if (!targetCard) {
    return (
      <div
        data-entity-id={alias.aliasId}
        style={{
          ...style,
          width: 520,
          height: lodLevel === 2 ? 22 : undefined,
          padding: "14px 18px",
          fontSize: 12,
          color: "#9ca3af",
          background: "#fafaf8",
          border: "1.5px dashed rgba(0, 0, 0, 0.12)",
          borderRadius: 10,
          boxShadow: selected
            ? "0 0 0 2px rgba(59, 130, 246, 0.9)"
            : highlighted
              ? "0 0 0 2px rgba(16, 185, 129, 0.75)"
              : undefined,
          opacity: dimmed ? 0.35 : 1,
          userSelect: "none",
          cursor: "grab",
        }}
      >
        Alias: {alias.cardId}（目标卡片不存在）
      </div>
    );
  }

  return (
    <CardNode
      card={targetCard}
      cardsById={cardsById}
      aliasRefs={aliasRefs}
      style={style}
      variant="alias"
      lodLevel={lodLevel}
      selected={selected}
      highlighted={highlighted}
      dimmed={dimmed}
      isExpanded={isExpanded}
      onToggleExpand={onToggleExpand}
    />
  );
}
