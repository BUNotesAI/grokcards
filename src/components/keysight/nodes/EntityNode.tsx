import { memo, useMemo, type CSSProperties, type MouseEvent as ReactMouseEvent } from "react";
import type {
  AliasReference,
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
  QuestionMenuConfig,
  SectionMenuConfig,
  TaskMenuConfig,
  SectionListItem,
} from "./NodeContextMenu";

/**
 * 节点菜单回调集合 — 由 GraphView 传给 EntityNode,EntityNode 封装成 NodeMenuConfig
 * 的 handlers 对象透传给 NodeContextMenu。
 *
 * ## 统一 vs per-kind
 * - **统一 handler**(`onCopyEntityUuidTitle`):Phase B1 所有节点都支持"复制 UUID + title",
 *   共享同一 normalize pipeline `UUID:{id} {normalizeCardTitleForClipboard(title)}`,
 *   由 GraphView 集中在一处实现,kind 参数用于 entity 查找分派。
 * - **per-kind handler**(delete / 编辑 title 等):不同节点走不同 Tauri command
 *   (`alias_delete` / `note_delete` / `question_delete` / `section_delete` 等),
 *   语义差异大,保留 per-kind 接口。
 */
export interface NodeContextMenuHandlers {
  /** 共享:复制 UUID + normalized title(Card/Alias/Note/Question/Section/Task 共用) */
  onCopyEntityUuidTitle: (entityId: string, kind: EntityKind) => void;
  /** Card/Alias/Note/Question:从此节点开始画一条连线 */
  onDrawConnectionFrom: (id: string) => void;
  /** Card 专属:打开 Related 视图 */
  onRelatedFrom: (id: string) => void;
  /** Card 专属:为卡片创建 alias */
  onCreateAlias: (cardId: string) => void;
  /** Card 专属:设置卡片背景色(B2 新增) */
  onSetCardColor: (cardId: string, color: string) => void;
  /** Alias 专属:跳转到 source card */
  onJumpToSourceCard: (aliasId: string) => void;
  /** Alias 专属:删除 */
  onDeleteAlias: (aliasId: string) => void;
  /** Note 专属 */
  onEditNoteTitle: (noteId: string) => void;
  onDeleteNote: (noteId: string) => void;
  onSetNoteColor: (noteId: string, color: string) => void;
  /** Question 专属 */
  onEditQuestionTitle: (questionId: string) => void;
  onDeleteQuestion: (questionId: string) => void;
  /** Question 专属:设置背景色(B2 新增) */
  onSetQuestionColor: (questionId: string, color: string) => void;
  /** Section 专属 */
  onDeleteSection: (sectionId: string) => void;
  onSetSectionColor: (sectionId: string, color: string) => void;
  /** Task 专属(B2 新增):删除 + 背景色;P1 Task 4 新增:编辑标题 */
  onEditTaskTitle: (taskId: string) => void;
  onDeleteTask: (taskId: string) => void;
  onSetTaskColor: (taskId: string, color: string) => void;
  /** 共享:分组 */
  onMoveToSection: (entityId: string, sectionId: string) => void;
  onRemoveFromGroup: (entityId: string) => void;
}

/** 当前实体正在编辑的字段；null 表示该实体未在编辑 */
export type EditingField =
  | "card-title"
  | "card-understanding"
  | "note-title"
  | "note-body"
  | "question-title"
  | "question-body"
  | "section-title"
  | "task-title"
  | null;

interface EntityNodeProps {
  entity: EntityWithPosition;
  allPositions: Record<string, Position>;
  /** 所有实体 id → kind 映射，SectionNode bounds 计算用 */
  allKinds: Record<string, EntityKind>;
  cardsById?: Record<string, AtomicCard>;
  aliasesByTargetId?: Record<string, AliasReference[]>;
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
  onOpenCard?: (cardId: string) => void;
  onOpenAlias?: (aliasId: string) => void;
  onSelectTag?: (tag: string) => void;
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
  onOpenCard,
  onOpenAlias,
  onSelectTag,
}: EntityNodeProps) {
  // 按 entity.kind 派发构造对应 NodeMenuConfig 判别联合变体
  // 回调闭合 entity.id + entity.kind,这样 NodeContextMenu 内按 capability.kind 分派即可
  const cardMenu = useMemo<CardMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "card") return null;
    return {
      kind: "card",
      handlers: {
        copy_uuid_title: () => menuHandlers.onCopyEntityUuidTitle(entity.id, "card"),
        draw_connection: () => menuHandlers.onDrawConnectionFrom(entity.id),
        related: () => menuHandlers.onRelatedFrom(entity.id),
        create_alias: () => menuHandlers.onCreateAlias(entity.id),
        move_to_section: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
        remove_from_group: () => menuHandlers.onRemoveFromGroup(entity.id),
        set_color: (color) => menuHandlers.onSetCardColor(entity.id, color),
      },
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const noteMenu = useMemo<NoteMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "note") return null;
    return {
      kind: "note",
      handlers: {
        copy_uuid_title: () => menuHandlers.onCopyEntityUuidTitle(entity.id, "note"),
        draw_connection: () => menuHandlers.onDrawConnectionFrom(entity.id),
        edit_title: () => menuHandlers.onEditNoteTitle(entity.id),
        move_to_section: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
        remove_from_group: () => menuHandlers.onRemoveFromGroup(entity.id),
        set_color: (color) => menuHandlers.onSetNoteColor(entity.id, color),
        delete: () => menuHandlers.onDeleteNote(entity.id),
      },
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const questionMenu = useMemo<QuestionMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "question") return null;
    return {
      kind: "question",
      handlers: {
        copy_uuid_title: () => menuHandlers.onCopyEntityUuidTitle(entity.id, "question"),
        draw_connection: () => menuHandlers.onDrawConnectionFrom(entity.id),
        edit_title: () => menuHandlers.onEditQuestionTitle(entity.id),
        move_to_section: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
        remove_from_group: () => menuHandlers.onRemoveFromGroup(entity.id),
        delete: () => menuHandlers.onDeleteQuestion(entity.id),
        set_color: (color) => menuHandlers.onSetQuestionColor(entity.id, color),
      },
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const aliasMenu = useMemo<AliasMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "alias") return null;
    return {
      kind: "alias",
      handlers: {
        copy_uuid_title: () => menuHandlers.onCopyEntityUuidTitle(entity.id, "alias"),
        draw_connection: () => menuHandlers.onDrawConnectionFrom(entity.id),
        jump_to_source_card: () => menuHandlers.onJumpToSourceCard(entity.id),
        move_to_section: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
        remove_from_group: () => menuHandlers.onRemoveFromGroup(entity.id),
        delete: () => menuHandlers.onDeleteAlias(entity.id),
      },
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const sectionMenu = useMemo<SectionMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "section") return null;
    return {
      kind: "section",
      handlers: {
        copy_uuid_title: () => menuHandlers.onCopyEntityUuidTitle(entity.id, "section"),
        set_color: (color) => menuHandlers.onSetSectionColor(entity.id, color),
        delete: () => menuHandlers.onDeleteSection(entity.id),
      },
    };
  }, [menuHandlers, entity.id, entity.kind]);

  const taskMenu = useMemo<TaskMenuConfig | null>(() => {
    if (!menuHandlers || entity.kind !== "task") return null;
    return {
      kind: "task",
      handlers: {
        copy_uuid_title: () => menuHandlers.onCopyEntityUuidTitle(entity.id, "task"),
        edit_title: () => menuHandlers.onEditTaskTitle(entity.id),
        move_to_section: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
        remove_from_group: () => menuHandlers.onRemoveFromGroup(entity.id),
        set_color: (color) => menuHandlers.onSetTaskColor(entity.id, color),
        delete: () => menuHandlers.onDeleteTask(entity.id),
      },
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
  const questionEditingField =
    editing === "question-title" || editing === "question-body" ? editing : null;
  const sectionEditingField = editing === "section-title" ? editing : null;
  const taskEditingField = editing === "task-title" ? editing : null;

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
            onOpenCard={onOpenCard}
            onOpenAlias={onOpenAlias}
            onSelectTag={onSelectTag}
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
            editingField={taskEditingField}
            onStartEdit={onStartEdit}
            onCommitEdit={onCommitEdit}
            onCancelEdit={onCancelEdit}
            contextMenu={taskMenu}
            menuSections={menuSections}
            currentSectionId={currentSectionId}
          />
        </div>
      );
    case "question":
      return (
        <div style={wrapperStyle} onMouseDown={handleMouseDown} onClick={handleClick}>
          <QuestionNode
            question={entity.entity}
            editingField={questionEditingField}
            onStartEdit={onStartEdit}
            onCommitEdit={onCommitEdit}
            onCancelEdit={onCancelEdit}
            contextMenu={questionMenu}
            menuSections={menuSections}
            currentSectionId={currentSectionId}
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
            editingField={sectionEditingField}
            onStartEdit={onStartEdit}
            onCommitEdit={onCommitEdit}
            onCancelEdit={onCancelEdit}
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
            onOpenCard={onOpenCard}
            onOpenAlias={onOpenAlias}
            onSelectTag={onSelectTag}
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
