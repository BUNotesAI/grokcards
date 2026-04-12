import { memo, type CSSProperties, type MouseEvent as ReactMouseEvent } from "react";
import type { EntityKind, EntityWithPosition } from "@/components/keysight/types";
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
  /** 所有实体 id → kind 映射，SectionNode bounds 计算用 */
  allKinds: Record<string, EntityKind>;
  cardsById?: Record<string, AtomicCard>;
  aliasesByTargetId?: Record<string, Array<{ aliasId: string; aliasTitle: string }>>;
  /** 拖拽起始回调 — 按下鼠标左键时触发 */
  onDragStart?: (e: ReactMouseEvent, entityId: string) => void;
  /** 是否处于展开状态（展开后显示 body 内容） */
  isExpanded?: boolean;
  /** 切换展开状态的回调（click 且未发生拖拽时触发） */
  onToggleExpand?: (entityId: string) => void;
}

/**
 * 实体节点分发器 — 根据 kind 路由到对应节点组件。
 *
 * 所有节点共享一个 absolute positioned wrapper div，wrapper 处理 onMouseDown 实现拖拽。
 * 内部节点 style 不含 position，由 wrapper 承担。
 */
function EntityNodeImpl({
  entity,
  allPositions,
  allKinds,
  cardsById = {},
  aliasesByTargetId = {},
  onDragStart,
  isExpanded = false,
  onToggleExpand,
}: EntityNodeProps) {
  // 使用 transform 而非 left/top — GPU 合成，避免 layout reflow，拖拽更丝滑
  const wrapperStyle: CSSProperties = {
    position: "absolute",
    left: 0,
    top: 0,
    transform: `translate3d(${entity.position.x}px, ${entity.position.y}px, 0)`,
    willChange: "transform",
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

  // 仅点击 toggle 箭头时触发 — 从 CardNode/AliasNode 内部按钮回调
  const handleToggle = onToggleExpand
    ? (e: ReactMouseEvent) => {
        e.stopPropagation();
        onToggleExpand(entity.id);
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
            isExpanded={isExpanded}
            onToggleExpand={handleToggle}
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
        <SectionNode
          section={entity.entity}
          memberPositions={allPositions}
          memberKinds={allKinds}
          style={wrapperStyle}
        />
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
            isExpanded={isExpanded}
            onToggleExpand={handleToggle}
            style={innerStyle}
          />
        </div>
      );
    }
  }
}

/**
 * 自定义 memo 比较 — 只有当 entity 内容/位置/展开状态真的变化时才 re-render。
 *
 * 避免 mergeEntitiesWithPositions 每次返回新对象引用导致全量 re-render，
 * 拖拽时只有被拖的那个节点会 re-render。
 */
export const EntityNode = memo(EntityNodeImpl, (prev, next) => {
  if (prev.entity.kind !== next.entity.kind) return false;
  if (prev.entity.id !== next.entity.id) return false;
  if (prev.entity.position.x !== next.entity.position.x) return false;
  if (prev.entity.position.y !== next.entity.position.y) return false;
  if (prev.entity.entity !== next.entity.entity) return false;
  if (prev.allPositions !== next.allPositions) return false;
  if (prev.allKinds !== next.allKinds) return false;
  if (prev.cardsById !== next.cardsById) return false;
  if (prev.aliasesByTargetId !== next.aliasesByTargetId) return false;
  if (prev.onDragStart !== next.onDragStart) return false;
  if (prev.isExpanded !== next.isExpanded) return false;
  if (prev.onToggleExpand !== next.onToggleExpand) return false;
  return true;
});
