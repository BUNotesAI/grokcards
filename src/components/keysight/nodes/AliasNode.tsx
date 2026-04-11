import type { CSSProperties } from "react";
import type { CardAlias } from "@/bindings";

interface AliasNodeProps {
  alias: CardAlias;
  /** 原卡片标题，由调用方通过 cardTitles 查找后传入 */
  originalTitle: string;
  style: CSSProperties;
}

/**
 * 别名节点 — ghost/translucent 风格。
 * 宽度 280px，半透明显示原卡片标题，虚线边框区分别名与真实实体。
 */
export function AliasNode({ alias, originalTitle, style }: AliasNodeProps) {
  return (
    <div
      data-entity-id={alias.aliasId}
      className="select-none rounded-xl border border-dashed border-border/50 bg-card/60 opacity-70 shadow-[0_1px_2px_rgba(0,0,0,0.03)]"
      style={{ ...style, width: 280 }}
    >
      <div className="flex items-center gap-2 px-3 pt-2 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-gray-300 to-gray-400 text-[10px] font-bold text-white dark:from-gray-600 dark:to-gray-700">
          A
        </span>
        <h3 className="flex-1 truncate text-sm font-medium text-foreground/80">{originalTitle}</h3>
      </div>
      <p className="px-3 pb-2 text-[10px] text-muted-foreground/60">Alias of {alias.cardId}</p>
    </div>
  );
}
