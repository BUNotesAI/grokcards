import type { CSSProperties } from "react";
import type { EntityWithPosition } from "@/components/keysight/types";
import type { Position } from "@/bindings";
import { CardNode } from "./CardNode";
import { NoteNode } from "./NoteNode";
import { SectionNode } from "./SectionNode";

interface EntityNodeProps {
  entity: EntityWithPosition;
  allPositions: Record<string, Position>;
}

/**
 * 实体节点分发器 — 根据 kind 路由到对应节点组件。
 * 负责绝对定位和分发，不含业务逻辑。
 * TaskNode / QuestionNode / AliasNode 在 Task 6 补充。
 */
export function EntityNode({ entity, allPositions }: EntityNodeProps) {
  const posStyle: CSSProperties = {
    position: "absolute",
    left: entity.position.x,
    top: entity.position.y,
  };

  switch (entity.kind) {
    case "card":
      return <CardNode card={entity.entity} style={posStyle} />;
    case "note":
      return <NoteNode note={entity.entity} style={posStyle} />;
    case "section":
      return (
        <SectionNode
          section={entity.entity}
          memberPositions={allPositions}
          style={posStyle}
        />
      );
    case "task":
      // Task 6 实现，临时 fallback
      return (
        <div
          data-entity-id={entity.id}
          style={{ ...posStyle, width: 320 }}
          className="rounded-xl border bg-card p-3 text-sm"
        >
          {entity.entity.title}
        </div>
      );
    case "question":
      // Task 6 实现，临时 fallback
      return (
        <div
          data-entity-id={entity.id}
          style={{ ...posStyle, width: 320 }}
          className="rounded-xl border bg-card p-3 text-sm"
        >
          {entity.entity.title}
        </div>
      );
    case "alias":
      // Task 6 实现，临时 fallback
      return (
        <div
          data-entity-id={entity.id}
          style={{ ...posStyle, width: 280, opacity: 0.6 }}
          className="rounded-xl border bg-card p-3 text-sm"
        >
          Alias: {entity.entity.cardId}
        </div>
      );
  }
}
