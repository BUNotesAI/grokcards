import { memo, useMemo, type CSSProperties, type MouseEvent as ReactMouseEvent } from "react";
import type {
  EntityKind,
  EntityWithPosition,
  GraphSelectableKind,
  LodLevel,
} from "@/components/keysight/types";
import type { AtomicCard, Position } from "@/bindings";
import { CardNode } from "./CardNode";
import { TaskNode } from "./TaskNode";
import { QuestionNode } from "./QuestionNode";
import { NoteNode } from "./NoteNode";
import { SectionNode } from "./SectionNode";
import { AliasNode } from "./AliasNode";
import type {
  AliasMenuConfig,
  CardMenuConfig,
  NoteMenuConfig,
  SectionMenuConfig,
  SectionListItem,
} from "./NodeContextMenu";

/**
 * 节点菜单回调集合 — 所有回调都以 entityId 为首参数，保持 stable reference
 * 方便 EntityNode 的 memo 比较。
 */
export interface NodeContextMenuHandlers {
  /** Card 菜单 */
  onCopyCardTitle: (id: string) => void;
  onDrawConnectionFrom: (id: string) => void;
  onRelatedFrom: (id: string) => void;
  onCreateAlias: (cardId: string) => void;
  /** Alias 菜单 */
  onJumpToSourceCard: (aliasId: string) => void;
  onDeleteAlias: (aliasId: string) => void;
  /** Note 菜单 */
  onCopyNoteUuidTitle: (noteId: string) => void;
  onEditNoteTitle: (noteId: string) => void;
  onDeleteNote: (noteId: string) => void;
  onSetNoteColor: (noteId: string, color: string) => void;
  /** Section 菜单 */
  onDeleteSection: (sectionId: string) => void;
  /** 共享 */
  onMoveToSection: (entityId: string, sectionId: string) => void;
  onRemoveFromGroup: (entityId: string) => void;
}

/** 当前实体正在编辑的字段；null 表示该实体未在编辑 */
export type EditingField =
  | "card-title"
  | "card-understanding"
  | "note-title"
  | "note-body"
  | null;

interface EntityNodeProps {
  entity: EntityWithPosition;
  allPositions: Record<string, Position>;
  /** 所有实体 id → kind 映射，SectionNode bounds 计算用 */
  allKinds: Record<string, EntityKind>;
  cardsById?: Record<string, AtomicCard>;
  aliasesByTargetId?: Record<string, Array<{ aliasId: string; aliasTitle: string }>>;
  lodLevel?: LodLevel;
  selected?: boolean;
  highlighted?: boolean;
  dimmed?: boolean;
  spotlightDimmed?: boolean;
  /** 拖拽起始回调 — 按下鼠标左键时触发 */
  onDragStart?: (e: ReactMouseEvent, entityId: string) => void;
  /** 单击选中实体 */
  onSelect?: (selection: { id: string; kind: GraphSelectableKind }) => void;
  /** 是否处于展开状态（展开后显示 body 内容） */
  isExpanded?: boolean;
  /** 切换展开状态的回调（click 且未发生拖拽时触发） */
  onToggleExpand?: (entityId: string) => void;
  /** 当前实体正在编辑的字段；null 表示未编辑 */
  editing?: EditingField;
  /** 双击进入编辑模式回调 */
  onStartEdit?: (id: string, field: NonNullable<EditingField>) => void;
  /** 提交编辑回调（onBlur / Enter / Cmd+Enter 时触发） */
  onCommitEdit?: (
    id: string,
    field: NonNullable<EditingField>,
    value: string,
  ) => void;
  /** 取消编辑回调（Escape） */
  onCancelEdit?: () => void;
  /** ⋯ 菜单回调集合；null 表示不渲染菜单 */
  menuHandlers?: NodeContextMenuHandlers | null;
  /** 当前白板的 sections 列表（给 Move to Section 子菜单用） */
  menuSections?: SectionListItem[];
  /** 此实体所属 section id；null 表示未在任何 section */
  currentSectionId?: string | null;
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
  lodLevel = 0,
  selected = false,
  highlighted = false,
  dimmed = false,
  spotlightDimmed = false,
  onDragStart,
  onSelect,
  isExpanded = false,
  onToggleExpand,
  editing = null,
  onStartEdit,
  onCommitEdit,
  onCancelEdit,
  menuHandlers = null,
  menuSections = [],
  currentSectionId = null,
}: EntityNodeProps) {
  // 按 entity.kind 派发构造对应类型的菜单配置
  // 回调闭合 entity.id，这样 NodeContextMenu 内无需感知 id
  const cardMenu = useMemo<CardMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "card") return null;
    return {
      kind: "card",
      onCopyTitle: () => menuHandlers.onCopyCardTitle(entity.id),
      onDrawConnection: () => menuHandlers.onDrawConnectionFrom(entity.id),
      onRelated: () => menuHandlers.onRelatedFrom(entity.id),
      onCreateAlias: () => menuHandlers.onCreateAlias(entity.id),
      onMoveToSection: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
      onRemoveFromGroup: () => menuHandlers.onRemoveFromGroup(entity.id),
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const noteMenu = useMemo<NoteMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "note") return null;
    return {
      kind: "note",
      onCopyUuidTitle: () => menuHandlers.onCopyNoteUuidTitle(entity.id),
      onDrawConnection: () => menuHandlers.onDrawConnectionFrom(entity.id),
      onEditTitle: () => menuHandlers.onEditNoteTitle(entity.id),
      onMoveToSection: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
      onRemoveFromGroup: () => menuHandlers.onRemoveFromGroup(entity.id),
      onDelete: () => menuHandlers.onDeleteNote(entity.id),
      onSetColor: (color) => menuHandlers.onSetNoteColor(entity.id, color),
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const aliasMenu = useMemo<AliasMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "alias") return null;
    return {
      kind: "alias",
      onJumpToSourceCard: () => menuHandlers.onJumpToSourceCard(entity.id),
      onDrawConnection: () => menuHandlers.onDrawConnectionFrom(entity.id),
      onMoveToSection: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
      onRemoveFromGroup: () => menuHandlers.onRemoveFromGroup(entity.id),
      onDelete: () => menuHandlers.onDeleteAlias(entity.id),
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const sectionMenu = useMemo<SectionMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "section") return null;
    return {
      kind: "section",
      onDelete: () => menuHandlers.onDeleteSection(entity.id),
    };
  }, [menuHandlers, entity.id, entity.kind]);

  // 使用 transform 而非 left/top — GPU 合成，避免 layout reflow，拖拽更丝滑
  const wrapperStyle: CSSProperties = {
    position: "absolute",
    left: 0,
    top: 0,
    transform: `translate3d(${entity.position.x}px, ${entity.position.y}px, 0)`,
    willChange: "transform",
    opacity: spotlightDimmed ? 0.15 : 1,
  };

  const handleMouseDown = onDragStart
    ? (e: ReactMouseEvent) => {
        if (e.button !== 0) return;
        e.stopPropagation();
        onDragStart(e, entity.id);
      }
    : undefined;

  const handleClick = onSelect
    ? (e: ReactMouseEvent) => {
        e.stopPropagation();
        onSelect({ id: entity.id, kind: entity.kind });
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

  // Card / Note 行内编辑 — 仅当 editing.id === entity.id 时才传 field
  const cardEditingField =
    editing === "card-title" || editing === "card-understanding" ? editing : null;
  const noteEditingField =
    editing === "note-title" || editing === "note-body" ? editing : null;

  switch (entity.kind) {
    case "card":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown} onClick={handleClick}>
          <CardNode
            card={entity.entity}
            cardsById={cardsById}
            aliasRefs={aliasesByTargetId[entity.entity.id]}
            lodLevel={lodLevel}
            selected={selected}
            highlighted={highlighted}
            dimmed={dimmed}
            isExpanded={isExpanded}
            onToggleExpand={handleToggle}
            editingField={cardEditingField}
            onStartEdit={onStartEdit}
            onCommitEdit={onCommitEdit}
            onCancelEdit={onCancelEdit}
            contextMenu={cardMenu}
            menuSections={menuSections}
            currentSectionId={currentSectionId}
            style={innerStyle}
          />
        </div>
      );
    case "task":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown} onClick={handleClick}>
          <TaskNode
            task={entity.entity}
            style={innerStyle}
            lodLevel={lodLevel}
            selected={selected}
            highlighted={highlighted}
            dimmed={dimmed}
          />
        </div>
      );
    case "question":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown} onClick={handleClick}>
          <QuestionNode
            question={entity.entity}
            style={innerStyle}
            lodLevel={lodLevel}
            selected={selected}
            highlighted={highlighted}
            dimmed={dimmed}
          />
        </div>
      );
    case "note":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown} onClick={handleClick}>
          <NoteNode
            note={entity.entity}
            lodLevel={lodLevel}
            selected={selected}
            highlighted={highlighted}
            dimmed={dimmed}
            editingField={noteEditingField}
            onStartEdit={onStartEdit}
            onCommitEdit={onCommitEdit}
            onCancelEdit={onCancelEdit}
            contextMenu={noteMenu}
            menuSections={menuSections}
            currentSectionId={currentSectionId}
            style={innerStyle}
          />
        </div>
      );
    case "section":
      return (
        <div
          style={{ ...wrapperStyle, opacity: spotlightDimmed ? 0.1 : 1 }}
          onMouseDown={handleMouseDown}
          onClick={handleClick}
        >
          <SectionNode
            section={entity.entity}
            memberPositions={allPositions}
            memberKinds={allKinds}
            contextMenu={sectionMenu}
            style={{}}
          />
        </div>
      );
    case "alias": {
      const target = cardsById[entity.entity.cardId] ?? null;
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown} onClick={handleClick}>
          <AliasNode
            alias={entity.entity}
            targetCard={target}
            cardsById={cardsById}
            aliasRefs={target ? aliasesByTargetId[target.id] : []}
            lodLevel={lodLevel}
            selected={selected}
            highlighted={highlighted}
            dimmed={dimmed}
            isExpanded={isExpanded}
            onToggleExpand={handleToggle}
            contextMenu={aliasMenu}
            menuSections={menuSections}
            currentSectionId={currentSectionId}
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
  if (prev.lodLevel !== next.lodLevel) return false;
  if (prev.selected !== next.selected) return false;
  if (prev.highlighted !== next.highlighted) return false;
  if (prev.dimmed !== next.dimmed) return false;
  if (prev.spotlightDimmed !== next.spotlightDimmed) return false;
  if (prev.onDragStart !== next.onDragStart) return false;
  if (prev.onSelect !== next.onSelect) return false;
  if (prev.isExpanded !== next.isExpanded) return false;
  if (prev.onToggleExpand !== next.onToggleExpand) return false;
  if (prev.editing !== next.editing) return false;
  if (prev.onStartEdit !== next.onStartEdit) return false;
  if (prev.onCommitEdit !== next.onCommitEdit) return false;
  if (prev.onCancelEdit !== next.onCancelEdit) return false;
  if (prev.menuHandlers !== next.menuHandlers) return false;
  if (prev.menuSections !== next.menuSections) return false;
  if (prev.currentSectionId !== next.currentSectionId) return false;
  return true;
});
