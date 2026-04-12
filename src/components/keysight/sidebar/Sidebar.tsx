import type { MouseEvent as ReactMouseEvent, ReactNode } from "react";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { SidebarTabs } from "@/components/keysight/sidebar/SidebarTabs";
import type { SidebarTab, SidebarTabItem } from "@/components/keysight/sidebar/types";
import { Button } from "@/components/ui/button";

interface SidebarProps {
  collapsed: boolean;
  width: number;
  tabs: SidebarTabItem[];
  activeTab: SidebarTab;
  onTabChange: (tab: SidebarTab) => void;
  onToggleCollapse: () => void;
  onResizeStart: (event: ReactMouseEvent<HTMLDivElement>) => void;
  children: ReactNode;
}

export function Sidebar({
  collapsed,
  width,
  tabs,
  activeTab,
  onTabChange,
  onToggleCollapse,
  onResizeStart,
  children,
}: SidebarProps) {
  if (collapsed) {
    return (
      <div className="relative h-full shrink-0">
        <Button
          variant="outline"
          size="icon-sm"
          onClick={onToggleCollapse}
          className="absolute right-3 top-3 z-20 bg-white/90"
          aria-label="Open sidebar"
        >
          <PanelLeftOpen className="size-4" />
        </Button>
      </div>
    );
  }

  return (
    <aside
      className="relative flex h-full shrink-0 flex-col border-l border-[#d8d0c1] bg-[#f8f4ea]"
      style={{ width }}
    >
      <div className="flex items-start gap-2 border-b border-[#e4dccd] px-4 py-3">
        <div className="min-w-0 flex-1">
          <div className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[#978d79]">
            Keysight
          </div>
          <div className="mt-1 text-lg font-semibold text-[#231f17]">Workspace</div>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onToggleCollapse}
          aria-label="Collapse sidebar"
          className="mt-0.5 text-[#6c6253]"
        >
          <PanelLeftClose className="size-4" />
        </Button>
      </div>

      <div className="border-b border-[#e4dccd] px-4 py-3">
        <SidebarTabs tabs={tabs} activeTab={activeTab} onTabChange={onTabChange} />
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">{children}</div>

      <div
        className="absolute left-0 top-0 h-full w-2 cursor-col-resize bg-transparent transition-colors hover:bg-[#d3c9b7]"
        onMouseDown={onResizeStart}
      />
    </aside>
  );
}
