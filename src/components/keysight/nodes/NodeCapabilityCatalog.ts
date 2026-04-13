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
 * - NodeKind union 限定 5 种,传 "task" 编译失败。
 * - NodeMenuConfig per-kind handlers 强制完整(少一字段编译失败),不依赖注释或运行时校验。
 * - 每个 Capability variant 把 ui_kind / destructive 写死在类型里,const 初始化时
 *   shape 与 kind 不一致会编译失败(例如 set_color 的 ui_kind 只能是 "custom")。
 *
 * ## Task 预留
 *
 * Task 是 EntityKind 的第 6 种(参考 types.ts),目前在 graph 已渲染(TaskNode.tsx)但无菜单。
 * B1 不接入,**类型层面不埋技术债**:NodeKind union 不含 "task",NODE_CAPABILITIES 的
 * applies_to 不含 "task"。未来接入(docs/progress/backend.md Next 段):
 *   1. 把 "task" 加入 NodeKind union
 *   2. 更新相应 capability 的 applies_to(copy_uuid_title / draw_connection / edit_title /
 *      move_to_section / remove_from_group / delete)
 *   3. 新增 TaskNodeHandlers interface + NodeMenuConfig.task 变体
 *   4. EntityNode.tsx 的 case "task" 装配 handlers
 */

// ============================================================================
// Types
// ============================================================================

/** 有菜单的节点类型(Task 暂不含,见文件头注释) */
export type NodeKind = "card" | "alias" | "note" | "question" | "section";

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
    applies_to: new Set<NodeKind>(["card", "alias", "note", "question", "section"]),
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
    applies_to: new Set<NodeKind>(["card", "alias", "note", "question"]),
    ui_kind: "submenu",
    order: 70,
    label: "Move to Section",
    visible: "sections_not_empty",
  },
  {
    kind: "remove_from_group",
    applies_to: new Set<NodeKind>(["card", "alias", "note", "question"]),
    ui_kind: "plain",
    order: 80,
    label: "Remove from group",
    visible: "in_section",
  },
  {
    kind: "set_color",
    applies_to: new Set<NodeKind>(["note", "section"]),
    ui_kind: "custom",
    order: 90,
    label: "Set color",
  },
  {
    kind: "delete",
    applies_to: new Set<NodeKind>(["alias", "note", "question", "section"]),
    ui_kind: "plain",
    order: 100,
    label: "Delete",
    destructive: true,
  },
];

// ============================================================================
// Per-kind handlers — 类型系统强制每个 NodeKind 提供完整 capability handlers 子集
// ============================================================================
//
// 这些 interface 的字段必须**精确匹配** NODE_CAPABILITIES 里 applies_to 包含该 NodeKind
// 的 capability 集合。两处声明手动同步,漂移风险由 sub-stage 3 的 Code Review 6 项检查
// 的"测试覆盖"项兜底(组件测试会触发每个 handler)。

/** Card 节点菜单 handlers (6 个) */
export interface CardNodeHandlers {
  readonly copy_uuid_title: () => void;
  readonly draw_connection: () => void;
  readonly related: () => void;
  readonly create_alias: () => void;
  readonly move_to_section: (sectionId: string) => void;
  readonly remove_from_group: () => void;
}

/** Alias 节点菜单 handlers (6 个) */
export interface AliasNodeHandlers {
  readonly copy_uuid_title: () => void;
  readonly draw_connection: () => void;
  readonly jump_to_source_card: () => void;
  readonly move_to_section: (sectionId: string) => void;
  readonly remove_from_group: () => void;
  readonly delete: () => void;
}

/** Note 节点菜单 handlers (7 个) */
export interface NoteNodeHandlers {
  readonly copy_uuid_title: () => void;
  readonly draw_connection: () => void;
  readonly edit_title: () => void;
  readonly move_to_section: (sectionId: string) => void;
  readonly remove_from_group: () => void;
  readonly set_color: (color: string) => void;
  readonly delete: () => void;
}

/** Question 节点菜单 handlers (6 个) */
export interface QuestionNodeHandlers {
  readonly copy_uuid_title: () => void;
  readonly draw_connection: () => void;
  readonly edit_title: () => void;
  readonly move_to_section: (sectionId: string) => void;
  readonly remove_from_group: () => void;
  readonly delete: () => void;
}

/** Section 节点菜单 handlers (3 个) */
export interface SectionNodeHandlers {
  readonly copy_uuid_title: () => void;
  readonly set_color: (color: string) => void;
  readonly delete: () => void;
}

/**
 * 节点菜单运行时配置判别联合。
 *
 * sub-stage 3 用这个替换 NodeContextMenu.tsx 里的 NodeMenuConfig props。
 * NodeContextMenu 的渲染逻辑:
 *   `NODE_CAPABILITIES.filter(cap => cap.applies_to.has(config.kind))`
 *     + visibility 判定 + order 排序 + ui_kind 分派
 *     → 每项从 `config.handlers` 里按 capability.kind 取 handler
 */
export type NodeMenuConfig =
  | { readonly kind: "card"; readonly handlers: CardNodeHandlers }
  | { readonly kind: "alias"; readonly handlers: AliasNodeHandlers }
  | { readonly kind: "note"; readonly handlers: NoteNodeHandlers }
  | { readonly kind: "question"; readonly handlers: QuestionNodeHandlers }
  | { readonly kind: "section"; readonly handlers: SectionNodeHandlers };
