import type { UseViewportReturn } from "@/components/keysight/useViewport";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Minus, Plus, RotateCcw, RefreshCw, FolderPlus, StickyNote, Search, ArrowLeft } from "lucide-react";

/** 实体计数 */
export interface EntityCounts {
  cards: number;
  notes: number;
  sections: number;
  tasks: number;
  questions: number;
  aliases: number;
}

interface GraphToolbarProps {
  viewport: UseViewportReturn;
  entityCounts: EntityCounts;
  onSync: () => void;
  onCreateSection: () => void;
  onCreateNote: () => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  /** 当前白板 id，wb_root 表示根白板 */
  currentWhiteboardId?: string;
  /** 返回根白板的回调，仅子白板时使用 */
  onNavigateBack?: () => void;
}

/**
 * 画布工具栏 — 4 个区域
 *
 * 状态区：实体计数 + zoom 百分比
 * 创建区：+ Section / + Note
 * 操作区：Sync + zoom +/-/reset
 * 搜索区：输入框（debounce 由父组件管理）
 */
export function GraphToolbar({
  viewport,
  entityCounts,
  onSync,
  onCreateSection,
  onCreateNote,
  searchQuery,
  onSearchChange,
  currentWhiteboardId = "wb_root",
  onNavigateBack,
}: GraphToolbarProps) {
  const { state, actions } = viewport;
  const zoomPercent = Math.round(state.zoom * 100);

  const totalEntities =
    entityCounts.cards +
    entityCounts.notes +
    entityCounts.sections +
    entityCounts.tasks +
    entityCounts.questions +
    entityCounts.aliases;

  return (
    <div className="flex items-center gap-3 border-b border-border bg-background/80 px-4 py-2 backdrop-blur-sm">
      {/* 状态区 */}
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        {entityCounts.cards > 0 && (
          <span>{entityCounts.cards} card{entityCounts.cards !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.tasks > 0 && (
          <span>{entityCounts.tasks} task{entityCounts.tasks !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.questions > 0 && (
          <span>{entityCounts.questions} question{entityCounts.questions !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.notes > 0 && (
          <span>{entityCounts.notes} note{entityCounts.notes !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.sections > 0 && (
          <span>{entityCounts.sections} section{entityCounts.sections !== 1 ? "s" : ""}</span>
        )}
        {entityCounts.aliases > 0 && (
          <span>{entityCounts.aliases} alias{entityCounts.aliases !== 1 ? "es" : ""}</span>
        )}
        {totalEntities === 0 && <span>Empty whiteboard</span>}
        <span className="text-muted-foreground/50">|</span>
        <span>{zoomPercent}%</span>
      </div>

      <div className="flex-1" />

      {/* 创建区 */}
      <Button variant="ghost" size="sm" onClick={onCreateSection} aria-label="Create Section">
        <FolderPlus className="mr-1 h-4 w-4" />
        Section
      </Button>
      <Button variant="ghost" size="sm" onClick={onCreateNote} aria-label="Create Note">
        <StickyNote className="mr-1 h-4 w-4" />
        Note
      </Button>

      {/* 操作区 */}
      <div className="flex items-center gap-1 border-l border-border pl-3">
        <Button variant="ghost" size="icon" onClick={onSync} aria-label="Sync">
          <RefreshCw className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="icon" onClick={actions.zoomOut} aria-label="Zoom out">
          <Minus className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="icon" onClick={actions.zoomIn} aria-label="Zoom in">
          <Plus className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="icon" onClick={actions.resetView} aria-label="Reset zoom">
          <RotateCcw className="h-4 w-4" />
        </Button>
      </div>

      {/* 搜索区 */}
      <div className="relative w-48">
        <Search className="absolute left-2 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Search..."
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          className="h-8 pl-8 text-sm"
        />
      </div>

      {/* 白板导航 */}
      {currentWhiteboardId !== "wb_root" && (
        <div className="flex items-center gap-2 border-l border-border pl-3">
          <Button variant="ghost" size="sm" onClick={onNavigateBack} aria-label="Back to root">
            <ArrowLeft className="mr-1 h-4 w-4" />
            Root
          </Button>
          <span className="text-xs font-medium text-foreground">{currentWhiteboardId}</span>
        </div>
      )}
    </div>
  );
}
