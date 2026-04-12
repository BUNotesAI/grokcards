import { memo, type CSSProperties, type MouseEvent as ReactMouseEvent } from "react";
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
  cardsById?: Record<string, AtomicCard>;
  aliasesByTargetId?: Record<string, Array<{ aliasId: string; aliasTitle: string }>>;
  /** 拖拽起始回调 — 按下鼠标左键时触发 */
  onDragStart?: (e: ReactMouseEvent, entityId: string) => void;
}

/**
 * 实体节点分发器 — 根据 kind 路由到对应节点组件。
 *
 * 所有节点共享一个 absolute positioned wrapper div，wrapper 处理 onMouseDown 实现拖拽。
 * 内部节点 style 不含 position，由 wrapper 承担。
 */
export const EntityNode = memo(function EntityNode({
  entity,
  allPositions,
  cardsById = {},
  aliasesByTargetId = {},
  onDragStart,
}: EntityNodeProps) {
  const wrapperStyle: CSSProperties = {
    position: "absolute",
    left: entity.position.x,
    top: entity.position.y,
  };

  const handleMouseDown = onDragStart
    ? (e: ReactMouseEvent) => {
        if (e.button !== 0) return;
        // 对于 section，不允许通过它拖动（它是背景层）
        if (entity.kind === "section") return;
        e.stopPropagation();
        onDragStart(e, entity.id);
      }
    : undefined;

  // 内部节点使用空 style — wrapper 负责定位
  const innerStyle: CSSProperties = {};

  switch (entity.kind) {
    case "card":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown}>
          <CardNode
            card={entity.entity}
            cardsById={cardsById}
            aliasRefs={aliasesByTargetId[entity.entity.id]}
            style={innerStyle}
          />
        </div>
      );
    case "task":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown}>
          <TaskNode task={entity.entity} style={innerStyle} />
        </div>
      );
    case "question":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown}>
          <QuestionNode question={entity.entity} style={innerStyle} />
        </div>
      );
    case "note":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown}>
          <NoteNode note={entity.entity} style={innerStyle} />
        </div>
      );
    case "section":
      return (
        <SectionNode section={entity.entity} memberPositions={allPositions} style={wrapperStyle} />
      );
    case "alias": {
      const target = cardsById[entity.entity.cardId] ?? null;
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown}>
          <AliasNode
            alias={entity.entity}
            targetCard={target}
            cardsById={cardsById}
            aliasRefs={target ? aliasesByTargetId[target.id] : []}
            style={innerStyle}
          />
        </div>
      );
    }
  }
});
