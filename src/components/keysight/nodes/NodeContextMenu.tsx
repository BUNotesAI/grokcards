import type { MouseEvent as ReactMouseEvent } from "react";
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

/** Card 节点菜单配置 */
export interface CardMenuConfig {
  kind: "card";
  onCopyTitle: () => void;
  onDrawConnection: () => void;
  onRelated: () => void;
  onCreateAlias: () => void;
  onMoveToSection: (sectionId: string) => void;
  onRemoveFromGroup: () => void;
}

/** Alias 节点菜单配置 */
export interface AliasMenuConfig {
  kind: "alias";
  onJumpToSourceCard: () => void;
  onDrawConnection: () => void;
  onMoveToSection: (sectionId: string) => void;
  onRemoveFromGroup: () => void;
  onDelete: () => void;
}

/** Note 节点菜单配置 */
export interface NoteMenuConfig {
  kind: "note";
  onCopyUuidTitle: () => void;
  onDrawConnection: () => void;
  onEditTitle: () => void;
  onMoveToSection: (sectionId: string) => void;
  onRemoveFromGroup: () => void;
  onDelete: () => void;
  onSetColor: (color: string) => void;
}

/** 节点菜单判别联合 — 区分 Card / Alias / Note 三类菜单项 */
export type NodeMenuConfig = CardMenuConfig | AliasMenuConfig | NoteMenuConfig;

/** 当前白板的 section 精简列表（供 Move to Section 子菜单使用） */
export interface SectionListItem {
  id: string;
  title: string;
}

interface NodeContextMenuProps {
  menu: NodeMenuConfig;
  sections: SectionListItem[];
  /** 实体当前所属 section，为 null 时隐藏 "Remove from group" */
  currentSectionId: string | null;
}

/** 拦截菜单触发按钮的鼠标事件，避免触发节点拖拽或 click-select */
function stopBubble(e: ReactMouseEvent) {
  e.stopPropagation();
}

/** 共享的 "Move to Section" 子菜单 — 所有变体通用 */
function MoveToSectionSubmenu({
  sections,
  onMoveToSection,
}: {
  sections: SectionListItem[];
  onMoveToSection: (sectionId: string) => void;
}) {
  if (sections.length === 0) return null;
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>Move to Section</DropdownMenuSubTrigger>
      <DropdownMenuSubContent className="max-h-64 min-w-[160px] overflow-y-auto">
        {sections.map((s) => (
          <DropdownMenuItem
            key={s.id}
            onClick={() => onMoveToSection(s.id)}
          >
            <span className="truncate">{s.title || "(untitled)"}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}

/** Note 专属的 7 色块面板 — 菜单底部横排 */
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

/**
 * 节点上下文菜单 — 右上角 ⋯ 按钮 + DropdownMenu。
 *
 * 菜单项按 menu.kind 分派：
 * - card: Copy title / Draw connection / Related / Create alias / Move to Section / Remove from group
 * - alias: Remove from group / Jump to source card / Draw connection / Move to Section / Delete alias
 * - note: Copy UUID+title / Draw connection / Edit title / Move to Section / Remove from group / Delete / 7 色块
 */
export function NodeContextMenu({
  menu,
  sections,
  currentSectionId,
}: NodeContextMenuProps) {
  const showRemoveFromGroup = currentSectionId !== null;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={(props) => (
          <button
            type="button"
            aria-label="Open menu"
            onMouseDown={stopBubble}
            onClick={stopBubble}
            {...(props as React.ButtonHTMLAttributes<HTMLButtonElement>)}
            className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-md border-none bg-transparent text-[16px] leading-none text-foreground/45 transition-colors hover:bg-foreground/10 hover:text-foreground"
          >
            ⋯
          </button>
        )}
      />
      <DropdownMenuContent
        align="end"
        className="min-w-[180px]"
      >
        {menu.kind === "card" && (
          <>
            <DropdownMenuItem onClick={menu.onCopyTitle}>Copy title</DropdownMenuItem>
            <DropdownMenuItem onClick={menu.onDrawConnection}>Draw connection</DropdownMenuItem>
            <DropdownMenuItem onClick={menu.onRelated}>Related</DropdownMenuItem>
            <DropdownMenuItem onClick={menu.onCreateAlias}>Create alias</DropdownMenuItem>
            <MoveToSectionSubmenu
              sections={sections}
              onMoveToSection={menu.onMoveToSection}
            />
            {showRemoveFromGroup && (
              <DropdownMenuItem onClick={menu.onRemoveFromGroup}>
                Remove from group
              </DropdownMenuItem>
            )}
          </>
        )}

        {menu.kind === "alias" && (
          <>
            {showRemoveFromGroup && (
              <DropdownMenuItem onClick={menu.onRemoveFromGroup}>
                Remove from group
              </DropdownMenuItem>
            )}
            <DropdownMenuItem onClick={menu.onJumpToSourceCard}>
              → Jump to source card
            </DropdownMenuItem>
            <DropdownMenuItem onClick={menu.onDrawConnection}>Draw connection</DropdownMenuItem>
            <MoveToSectionSubmenu
              sections={sections}
              onMoveToSection={menu.onMoveToSection}
            />
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onClick={menu.onDelete}>
              Delete alias
            </DropdownMenuItem>
          </>
        )}

        {menu.kind === "note" && (
          <>
            <DropdownMenuItem onClick={menu.onCopyUuidTitle}>
              Copy UUID + title
            </DropdownMenuItem>
            <DropdownMenuItem onClick={menu.onDrawConnection}>Draw connection</DropdownMenuItem>
            <DropdownMenuItem onClick={menu.onEditTitle}>Edit title</DropdownMenuItem>
            <MoveToSectionSubmenu
              sections={sections}
              onMoveToSection={menu.onMoveToSection}
            />
            {showRemoveFromGroup && (
              <DropdownMenuItem onClick={menu.onRemoveFromGroup}>
                Remove from group
              </DropdownMenuItem>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onClick={menu.onDelete}>
              Delete
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <NoteColorRow onSetColor={menu.onSetColor} />
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
