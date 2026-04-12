import type { SidebarTab, SidebarTabItem } from "@/components/keysight/sidebar/types";
import { cn } from "@/lib/utils";

interface SidebarTabsProps {
  tabs: SidebarTabItem[];
  activeTab: SidebarTab;
  onTabChange: (tab: SidebarTab) => void;
}

export function SidebarTabs({ tabs, activeTab, onTabChange }: SidebarTabsProps) {
  return (
    <div className="flex flex-wrap gap-1 rounded-2xl bg-[#f1ede2] p-1">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          onClick={() => onTabChange(tab.id)}
          className={cn(
            "rounded-xl px-3 py-1.5 text-xs font-semibold transition-colors",
            activeTab === tab.id
              ? "bg-[#1f2937] text-white shadow-sm"
              : "text-[#5b5346] hover:bg-white/70",
          )}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
