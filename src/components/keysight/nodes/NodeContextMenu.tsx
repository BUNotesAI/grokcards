import { Fragment, useMemo, type MouseEventHandler } from "react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  NODE_CAPABILITIES,
  type NodeCapability,
  type NodeCapabilityHandlerMap,
  type NodeMenuConfig,
} from "./NodeCapabilityCatalog";

// Re-export 类型别名 — 保持 CardNode / AliasNode / NoteNode / QuestionNode /
// SectionNode / EntityNode 的旧 import 路径继续工作,无需调整下游文件。
export type {
  NodeMenuConfig,
  CardMenuConfig,
  AliasMenuConfig,
  NoteMenuConfig,
  QuestionMenuConfig,
  SectionMenuConfig,
  TaskMenuConfig,
  NodeCapabilityHandlerMap,
} from "./NodeCapabilityCatalog";

/** Note 颜色面板 — 和旧 Obsidian 插件的 7 色便签色板对齐 */
export const NOTE_COLORS = [
  "#fff8b3",
  "#ffd6a5",
  "#ffadad",
  "#caffbf",
  "#a0c4ff",
  "#bdb2ff",
  "#ffc6ff",
] as const;

/** 当前白板的 section 精简列表(供 Move to Section 子菜单使用) */
export interface SectionListItem {
  id: string;
  title: string;
}

interface NodeContextMenuProps {
  menu: NodeMenuConfig;
  sections: SectionListItem[];
  /** 实体当前所属 section,为 null 时隐藏 "Remove from group" */
  currentSectionId: string | null;
}

/** 包装 Base UI trigger handler,同时阻止事件冒泡到节点 wrapper。 */
function withStopBubble<T extends HTMLElement>(
  handler?: MouseEventHandler<T>,
): MouseEventHandler<T> {
  return (event) => {
    event.stopPropagation();
    handler?.(event);
  };
}

/** 把 capability 归入三个渲染组之一,组间插入 DropdownMenuSeparator */
type CapabilityGroup = "normal" | "custom" | "destructive";

function groupOf(cap: NodeCapability): CapabilityGroup {
  if (cap.destructive) return "destructive";
  if (cap.ui_kind === "custom") return "custom";
  return "normal";
}

/** 判断 capability 是否应当显示(visibility rule 匹配上下文)*/
function isCapabilityVisible(
  cap: NodeCapability,
  ctx: { currentSectionId: string | null; sectionsCount: number },
): boolean {
  if (!cap.visible) return true;
  if (cap.visible === "in_section") return ctx.currentSectionId !== null;
  if (cap.visible === "sections_not_empty") return ctx.sectionsCount > 0;
  return true;
}

/**
 * 节点上下文菜单 — 右上角 ⋯ 按钮 + DropdownMenu。
 *
 * 渲染逻辑:从 `NODE_CAPABILITIES` 按 `applies_to.has(menu.kind)` 过滤,按 `visible`
 * 规则判定,按 `order` 升序排序,按 `ui_kind` 分派渲染组件(plain / submenu / custom)。
 * 所有节点类型走**同一渲染路径**,无 per-kind switch 分支。
 */
export function NodeContextMenu({
  menu,
  sections,
  currentSectionId,
}: NodeContextMenuProps) {
  // 先把 menu.handlers 当作 Partial<HandlerMap> 处理 — 结构子类型关系合法
  // (per-kind Pick 子集 → Partial 全集),运行时按 cap.kind 查找 handler
  const handlerMap = menu.handlers as Partial<NodeCapabilityHandlerMap>;

  const applicableCaps = useMemo(() => {
    const ctx = { currentSectionId, sectionsCount: sections.length };
    return NODE_CAPABILITIES
      .filter((cap) => cap.applies_to.has(menu.kind))
      .filter((cap) => isCapabilityVisible(cap, ctx))
      .slice()
      .sort((a, b) => a.order - b.order);
  }, [menu.kind, currentSectionId, sections.length]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={(props) => {
          const triggerProps = props as React.ButtonHTMLAttributes<HTMLButtonElement>;
          return (
            <button
              type="button"
              aria-label="Open menu"
              {...triggerProps}
              onMouseDown={withStopBubble(triggerProps.onMouseDown)}
              onClick={withStopBubble(triggerProps.onClick)}
              className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-md border-none bg-transparent text-[16px] leading-none text-foreground/45 transition-colors hover:bg-foreground/10 hover:text-foreground"
            >
              ⋯
            </button>
          );
        }}
      />
      <DropdownMenuContent align="end" className="min-w-[180px]">
        {applicableCaps.map((cap, idx) => {
          const prev = idx > 0 ? applicableCaps[idx - 1] : null;
          const needsSeparator = prev !== null && groupOf(prev) !== groupOf(cap);
          return (
            <Fragment key={cap.kind}>
              {needsSeparator && <DropdownMenuSeparator />}
              <CapabilityItem cap={cap} handlerMap={handlerMap} sections={sections} />
            </Fragment>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** 按 ui_kind 分派 capability 渲染 */
function CapabilityItem({
  cap,
  handlerMap,
  sections,
}: {
  cap: NodeCapability;
  handlerMap: Partial<NodeCapabilityHandlerMap>;
  sections: SectionListItem[];
}) {
  if (cap.ui_kind === "plain") {
    // 类型安全:plain capability 的 handler 都是 () => void
    const handler = handlerMap[cap.kind] as (() => void) | undefined;
    if (!handler) return null;
    return (
      <DropdownMenuItem
        onClick={handler}
        variant={cap.destructive ? "destructive" : undefined}
      >
        {cap.label}
      </DropdownMenuItem>
    );
  }

  if (cap.kind === "move_to_section") {
    const handler = handlerMap.move_to_section;
    if (!handler) return null;
    return (
      <MoveToSectionSubmenu
        label={cap.label}
        sections={sections}
        onMoveToSection={handler}
      />
    );
  }

  if (cap.kind === "set_color") {
    const handler = handlerMap.set_color;
    if (!handler) return null;
    return <NoteColorRow onSetColor={handler} />;
  }

  return null;
}

/** 共享的 "Move to Section" 子菜单 — 循环展开当前白板的 sections */
function MoveToSectionSubmenu({
  label,
  sections,
  onMoveToSection,
}: {
  label: string;
  sections: SectionListItem[];
  onMoveToSection: (sectionId: string) => void;
}) {
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>{label}</DropdownMenuSubTrigger>
      <DropdownMenuSubContent className="max-h-64 min-w-[160px] overflow-y-auto">
        {sections.map((s) => (
          <DropdownMenuItem key={s.id} onClick={() => onMoveToSection(s.id)}>
            <span className="truncate">{s.title || "(untitled)"}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}

/** Note/Section/... 颜色面板 — 菜单底部横排 7 色块 */
function NoteColorRow({ onSetColor }: { onSetColor: (color: string) => void }) {
  return (
    <div className="flex items-center gap-1 px-1.5 py-1.5">
      {NOTE_COLORS.map((color) => (
        <button
          key={color}
          type="button"
          aria-label={`Set color ${color}`}
          onClick={() => onSetColor(color)}
          className="h-5 w-5 rounded-full border border-foreground/15 ring-0 transition-transform hover:scale-110"
          style={{ background: color }}
        />
      ))}
    </div>
  );
}
