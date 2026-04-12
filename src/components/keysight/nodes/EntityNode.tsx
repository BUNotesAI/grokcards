import type { CSSProperties } from "react";
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
  /** cardId → AtomicCard 映射，AliasNode 用来渲染目标卡片完整内容 */
  cardById?: Record<string, AtomicCard>;
}

/**
 * 实体节点分发器 — 根据 kind 路由到对应节点组件。
 * 负责绝对定位和分发，不含业务逻辑。
 */
export function EntityNode({ entity, allPositions, cardById = {} }: EntityNodeProps) {
  const posStyle: CSSProperties = {
    position: "absolute",
    left: entity.position.x,
    top: entity.position.y,
  };

  switch (entity.kind) {
    case "card":
      return <CardNode card={entity.entity} style={posStyle} />;
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
    case "alias":
      return (
        <AliasNode
          alias={entity.entity}
          targetCard={cardById[entity.entity.cardId] ?? null}
          style={posStyle}
        />
      );
  }
}
