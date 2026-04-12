import type { CSSProperties } from "react";
import type { AtomicCard, CardAlias } from "@/bindings";
import { CardNode } from "./CardNode";

interface AliasNodeProps {
  alias: CardAlias;
  /** 目标卡片（可能为 null，如果查不到则显示降级 UI） */
  targetCard: AtomicCard | null;
  /** cardId → AtomicCard 全局查找表，传给 CardNode 用于 related/linkTo 显示 */
  cardsById?: Record<string, AtomicCard>;
  /** 目标卡的反向 alias 索引 */
  aliasRefs?: Array<{ aliasId: string; aliasTitle: string }>;
  style: CSSProperties;
}

/**
 * 别名节点 — 复用 CardNode variant="alias"，渲染目标卡片完整内容
 *
 * 视觉上和 CardNode 几乎一致（完整内容），通过虚线边框 + "A" 图标 + 灰色 tag 样式区分。
 * 目标卡不存在时降级为占位。
 */
export function AliasNode({
  alias,
  targetCard,
  cardsById = {},
  aliasRefs = [],
  style,
}: AliasNodeProps) {
  if (!targetCard) {
    return (
      <div
        data-entity-id={alias.aliasId}
        className="select-none rounded-xl border border-dashed border-border/50 bg-card/40 px-4 py-3 text-xs text-muted-foreground/60"
        style={{ ...style, width: 360 }}
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
    />
  );
}
