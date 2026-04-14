/**
 * 节点菜单能力 catalog — 判别联合驱动的菜单能力声明。
 *
 * Phase B1 产物,替代 NodeContextMenu.tsx 里 5 份分散的 *MenuConfig interface。
 *
 * ## 设计
 *
 * - **静态 metadata**(NODE_CAPABILITIES):声明能力存在 + 属性,一份,不随 entity 实例变化。
 * - **运行时 handlers**(NodeMenuConfig 判别联合):per-node instance 提供,类型系统强制按 NodeKind 完整。
 * - **渲染 dispatch**(sub-stage 3 实现):循环 NODE_CAPABILITIES,按 applies_to 过滤,
 *   按 visible 判定,按 order 排序,按 ui_kind 分派渲染组件。
 *
 * ## 防火墙约束
 *
 * - NodeKind union 限定 6 种(含 task),applies_to 决定哪些能力落到 task。
 * - NodeMenuConfig per-kind handlers 强制完整(少一字段编译失败),不依赖注释或运行时校验。
 * - 每个 Capability variant 把 ui_kind / destructive 写死在类型里,const 初始化时
 *   shape 与 kind 不一致会编译失败(例如 set_color 的 ui_kind 只能是 "custom")。
 *
 * ## Task 已接入(3 项能力)
 *
 * Task 作为第 6 个 NodeKind 接入菜单,当前 scope 只含 **3 项能力**:
 *   - `copy_uuid_title` — Task 有 title,走统一 normalize pipeline
 *   - `move_to_section` — `section_members` schema 通用,接受任意 entity kind
 *   - `remove_from_group` — 同上
 *
 * 未接入的能力(留给后续 task):
 *   - `draw_connection` — Edge 判别联合编译期禁 Task 作 from(edge.rs:101-102)
 *   - `edit_title` / `delete` — 后端无 `task_update` / `task_delete`,需 Rust 扩张(Phase B2)
 *   - `set_color` — Task 模型无 color 字段,需 schema 扩张(Phase B2)
 */

// ============================================================================
// Types
// ============================================================================

/** 有菜单的节点类型 */
export type NodeKind = "card" | "alias" | "note" | "question" | "section" | "task";

/** 菜单项渲染形态 */
export type UiKind =
  | "plain"    // 普通 DropdownMenuItem(label + onClick)
  | "submenu"  // DropdownMenuSub + 动态列表(如 move_to_section 循环 sections)
  | "custom";  // 自绘组件(如 set_color 的 7 色块横排 ColorRow)

/** 能力可见性规则 — 省略 = "always" */
export type VisibilityRule =
  | "in_section"           // 仅当节点属于某 section (currentSectionId !== null)
  | "sections_not_empty";  // 仅当当前白板存在 section (sections.length > 0)

// ============================================================================
// Capability 判别联合
// ============================================================================

/** 能力元数据 base(所有 capability variant 继承) */
interface CapabilityMetaBase {
  readonly applies_to: ReadonlySet<NodeKind>;
  readonly order: number;
  readonly label: string;
  readonly visible?: VisibilityRule;
  readonly destructive?: true;
}

/** 复制 `UUID:{id} {normalize(title)}` 到剪贴板 */
export interface CopyUuidTitleCapability extends CapabilityMetaBase {
  readonly kind: "copy_uuid_title";
  readonly ui_kind: "plain";
}

/** 从当前节点开始画连线到下一次点击的目标实体 */
export interface DrawConnectionCapability extends CapabilityMetaBase {
  readonly kind: "draw_connection";
  readonly ui_kind: "plain";
}

/** Card 专属:打开 Related 视图查看关联卡片 */
export interface RelatedCapability extends CapabilityMetaBase {
  readonly kind: "related";
  readonly ui_kind: "plain";
}

/** Card 专属:为卡片创建 alias */
export interface CreateAliasCapability extends CapabilityMetaBase {
  readonly kind: "create_alias";
  readonly ui_kind: "plain";
}

/** Alias 专属:跳转到 source card */
export interface JumpToSourceCardCapability extends CapabilityMetaBase {
  readonly kind: "jump_to_source_card";
  readonly ui_kind: "plain";
}

/** 编辑节点标题(触发 inline editor) */
export interface EditTitleCapability extends CapabilityMetaBase {
  readonly kind: "edit_title";
  readonly ui_kind: "plain";
}

/** 把节点移入一个 section(子菜单展开 sections 列表) */
export interface MoveToSectionCapability extends CapabilityMetaBase {
  readonly kind: "move_to_section";
  readonly ui_kind: "submenu";
  readonly visible: "sections_not_empty";
}

/** 把节点从当前 section 移除 */
export interface RemoveFromGroupCapability extends CapabilityMetaBase {
  readonly kind: "remove_from_group";
  readonly ui_kind: "plain";
  readonly visible: "in_section";
}

/** 设置节点背景色(自定义 7 色块组件) */
export interface SetColorCapability extends CapabilityMetaBase {
  readonly kind: "set_color";
  readonly ui_kind: "custom";
}

/** 删除节点(红色 destructive 样式) */
export interface DeleteCapability extends CapabilityMetaBase {
  readonly kind: "delete";
  readonly ui_kind: "plain";
  readonly destructive: true;
}

/** 节点菜单能力判别联合 */
export type NodeCapability =
  | CopyUuidTitleCapability
  | DrawConnectionCapability
  | RelatedCapability
  | CreateAliasCapability
  | JumpToSourceCardCapability
  | EditTitleCapability
  | MoveToSectionCapability
  | RemoveFromGroupCapability
  | SetColorCapability
  | DeleteCapability;

/** Capability 的 kind 全集 — 用作 handlers lookup 的 key */
export type CapabilityKind = NodeCapability["kind"];

// ============================================================================
// Catalog 静态声明 — 一份,不随 entity 实例变化
// ============================================================================

/**
 * 全局节点菜单能力 catalog。
 *
 * - 渲染顺序:按 `order` 从小到大,所有节点共享 canonical order。
 * - 新增/删除能力:更新此数组 + 相应 NodeMenuConfig per-kind handlers(两处同步)。
 * - 扩大 applies_to:只改此处 + 被扩的 NodeKind handlers interface。
 */
export const NODE_CAPABILITIES: readonly NodeCapability[] = [
  {
    kind: "copy_uuid_title",
    applies_to: new Set<NodeKind>(["card", "alias", "note", "question", "section", "task"]),
    ui_kind: "plain",
    order: 10,
    label: "Copy UUID + title",
  },
  {
    kind: "draw_connection",
    applies_to: new Set<NodeKind>(["card", "alias", "note", "question"]),
    ui_kind: "plain",
    order: 20,
    label: "Draw connection",
  },
  {
    kind: "related",
    applies_to: new Set<NodeKind>(["card"]),
    ui_kind: "plain",
    order: 30,
    label: "Related",
  },
  {
    kind: "create_alias",
    applies_to: new Set<NodeKind>(["card"]),
    ui_kind: "plain",
    order: 40,
    label: "Create alias",
  },
  {
    kind: "jump_to_source_card",
    applies_to: new Set<NodeKind>(["alias"]),
    ui_kind: "plain",
    order: 50,
    label: "→ Jump to source card",
  },
  {
    kind: "edit_title",
    applies_to: new Set<NodeKind>(["note", "question"]),
    ui_kind: "plain",
    order: 60,
    label: "Edit title",
  },
  {
    kind: "move_to_section",
    applies_to: new Set<NodeKind>(["card", "alias", "note", "question", "task"]),
    ui_kind: "submenu",
    order: 70,
    label: "Move to Section",
    visible: "sections_not_empty",
  },
  {
    kind: "remove_from_group",
    applies_to: new Set<NodeKind>(["card", "alias", "note", "question", "task"]),
    ui_kind: "plain",
    order: 80,
    label: "Remove from group",
    visible: "in_section",
  },
  {
    kind: "set_color",
    applies_to: new Set<NodeKind>(["card", "note", "question", "section", "task"]),
    ui_kind: "custom",
    order: 90,
    label: "Set color",
  },
  {
    kind: "delete",
    applies_to: new Set<NodeKind>(["alias", "note", "question", "section", "task"]),
    ui_kind: "plain",
    order: 100,
    label: "Delete",
    destructive: true,
  },
];

// ============================================================================
// Handler map — 单一声明源,per-kind handlers 从此 Pick
// ============================================================================

/**
 * 所有 capability 的 handler 签名全集,按 `CapabilityKind` 索引。
 *
 * 单一声明源:per-NodeKind handlers 用 `Pick<NodeCapabilityHandlerMap, ...>` 裁剪,
 * 保证任何节点类型的 handlers 字段签名都来自同一源头。
 * 参数无关于 NodeKind —— node-specific 信息(entity id / kind)在 closure 里捕获。
 */
export type NodeCapabilityHandlerMap = {
  copy_uuid_title: () => void;
  draw_connection: () => void;
  related: () => void;
  create_alias: () => void;
  jump_to_source_card: () => void;
  edit_title: () => void;
  move_to_section: (sectionId: string) => void;
  remove_from_group: () => void;
  set_color: (color: string) => void;
  delete: () => void;
};

// ============================================================================
// Per-kind handlers — Pick 子集,字段与 NODE_CAPABILITIES.applies_to 对齐
// ============================================================================
//
// 这些类型的 key 集合必须**精确匹配** NODE_CAPABILITIES 里 applies_to 包含该 NodeKind
// 的 capability 集合。两处声明手动同步,漂移风险由组件测试兜底(测试触发每个 handler)。
// 渲染时把 menu.handlers 当作 `Partial<NodeCapabilityHandlerMap>` 处理,这是合法的
// 结构子类型关系(Pick 子集 → Partial 全集)。

/** Card 节点菜单 handlers (7 个) */
export type CardNodeHandlers = Pick<
  NodeCapabilityHandlerMap,
  "copy_uuid_title" | "draw_connection" | "related" | "create_alias" | "move_to_section" | "remove_from_group" | "set_color"
>;

/** Alias 节点菜单 handlers (6 个) */
export type AliasNodeHandlers = Pick<
  NodeCapabilityHandlerMap,
  "copy_uuid_title" | "draw_connection" | "jump_to_source_card" | "move_to_section" | "remove_from_group" | "delete"
>;

/** Note 节点菜单 handlers (7 个) */
export type NoteNodeHandlers = Pick<
  NodeCapabilityHandlerMap,
  "copy_uuid_title" | "draw_connection" | "edit_title" | "move_to_section" | "remove_from_group" | "set_color" | "delete"
>;

/** Question 节点菜单 handlers (7 个) */
export type QuestionNodeHandlers = Pick<
  NodeCapabilityHandlerMap,
  "copy_uuid_title" | "draw_connection" | "edit_title" | "move_to_section" | "remove_from_group" | "set_color" | "delete"
>;

/** Section 节点菜单 handlers (3 个) */
export type SectionNodeHandlers = Pick<
  NodeCapabilityHandlerMap,
  "copy_uuid_title" | "set_color" | "delete"
>;

/**
 * Task 节点菜单 handlers (5 个)
 *
 * 注意:`edit_title` 暂未纳入 —— TaskNode 当前无 inline editor(和
 * QuestionNode/NoteNode 的模式不同),加一个 edit_title 能力需要先在
 * TaskNode 加 inline editor,独立 task 处理。
 */
export type TaskNodeHandlers = Pick<
  NodeCapabilityHandlerMap,
  "copy_uuid_title" | "move_to_section" | "remove_from_group" | "set_color" | "delete"
>;

/**
 * 节点菜单运行时配置判别联合。
 *
 * NodeContextMenu 的渲染逻辑(单一路径,无 per-kind 分支):
 *   NODE_CAPABILITIES
 *     .filter(cap => cap.applies_to.has(menu.kind))
 *     .filter(cap => isVisible(cap, ctx))
 *     .sort((a,b) => a.order - b.order)
 *     .map(cap => renderByUiKind(cap, menu.handlers))
 */
export type NodeMenuConfig =
  | { readonly kind: "card"; readonly handlers: CardNodeHandlers }
  | { readonly kind: "alias"; readonly handlers: AliasNodeHandlers }
  | { readonly kind: "note"; readonly handlers: NoteNodeHandlers }
  | { readonly kind: "question"; readonly handlers: QuestionNodeHandlers }
  | { readonly kind: "section"; readonly handlers: SectionNodeHandlers }
  | { readonly kind: "task"; readonly handlers: TaskNodeHandlers };

// ============================================================================
// Backward compat 类型别名 —— 旧 NodeContextMenu.tsx 的 *MenuConfig 接口已被
// 新的 handlers 嵌套形态替换,但为了让 CardNode / AliasNode / NoteNode /
// QuestionNode / SectionNode / EntityNode 的 contextMenu prop 类型声明和旧 import
// 继续工作,这里 re-export 为 Extract<NodeMenuConfig, {kind}> 的别名。
// ============================================================================

export type CardMenuConfig = Extract<NodeMenuConfig, { readonly kind: "card" }>;
export type AliasMenuConfig = Extract<NodeMenuConfig, { readonly kind: "alias" }>;
export type NoteMenuConfig = Extract<NodeMenuConfig, { readonly kind: "note" }>;
export type QuestionMenuConfig = Extract<NodeMenuConfig, { readonly kind: "question" }>;
export type SectionMenuConfig = Extract<NodeMenuConfig, { readonly kind: "section" }>;
export type TaskMenuConfig = Extract<NodeMenuConfig, { readonly kind: "task" }>;
