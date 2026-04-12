import { memo, type CSSProperties } from "react";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { AtomicCard, Position } from "@/bindings";
import { CardNode } from "./CardNode";
import { TaskNode } from "./TaskNode";
import { QuestionNode } from "./QuestionNode";
import { NoteNode } from "./NoteNode";
import { SectionNode } from "./SectionNode";
import { AliasNode } from "./AliasNode";

interface EntityNodeProps {
  entity: EntityWithPosition;
  allPositions: Record<string, Position>;
  /** cardId → AtomicCard 映射，CardNode/AliasNode 用来查 related/linkTo */
  cardsById?: Record<string, AtomicCard>;
  /** targetCardId → aliases 反向索引 */
  aliasesByTargetId?: Record<string, Array<{ aliasId: string; aliasTitle: string }>>;
}

/**
 * 实体节点分发器 — 根据 kind 路由到对应节点组件。
 * 负责绝对定位和分发，不含业务逻辑。
 *
 * memo 化：每个 entity 独立渲染，props 不变时跳过 re-render，避免 200 个节点同时 re-render。
 */
export const EntityNode = memo(function EntityNode({
  entity,
  allPositions,
  cardsById = {},
  aliasesByTargetId = {},
}: EntityNodeProps) {
  const posStyle: CSSProperties = {
    position: "absolute",
    left: entity.position.x,
    top: entity.position.y,
  };

  switch (entity.kind) {
    case "card":
      return (
        <CardNode
          card={entity.entity}
          cardsById={cardsById}
          aliasRefs={aliasesByTargetId[entity.entity.id]}
          style={posStyle}
        />
      );
    case "task":
      return <TaskNode task={entity.entity} style={posStyle} />;
    case "question":
      return <QuestionNode question={entity.entity} style={posStyle} />;
    case "note":
      return <NoteNode note={entity.entity} style={posStyle} />;
    case "section":
      return (
        <SectionNode section={entity.entity} memberPositions={allPositions} style={posStyle} />
      );
    case "alias": {
      const target = cardsById[entity.entity.cardId] ?? null;
      return (
        <AliasNode
          alias={entity.entity}
          targetCard={target}
          cardsById={cardsById}
          aliasRefs={target ? aliasesByTargetId[target.id] : []}
          style={posStyle}
        />
      );
    }
  }
});
