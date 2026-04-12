import type { CSSProperties } from "react";
import type { AtomicCard, CardAlias } from "@/bindings";
import { CardNode } from "./CardNode";

interface AliasNodeProps {
  alias: CardAlias;
  /** 目标卡片（可能为 null，如果查不到则显示降级 UI） */
  targetCard: AtomicCard | null;
  cardsById?: Record<string, AtomicCard>;
  aliasRefs?: Array<{ aliasId: string; aliasTitle: string }>;
  style: CSSProperties;
  isExpanded?: boolean;
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
  isExpanded = false,
}: AliasNodeProps) {
  if (!targetCard) {
    return (
      <div
        data-entity-id={alias.aliasId}
        style={{
          ...style,
          width: 520,
          padding: "14px 18px",
          fontSize: 12,
          color: "#9ca3af",
          background: "#fafaf8",
          border: "1.5px dashed rgba(0, 0, 0, 0.12)",
          borderRadius: 10,
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
      isExpanded={isExpanded}
    />
  );
}
