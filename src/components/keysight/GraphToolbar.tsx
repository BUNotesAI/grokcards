import type { UseViewportReturn } from "@/components/keysight/useViewport";
import type { GraphSection, WhiteboardSummary } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  BookOpen,
  Boxes,
  CheckSquare,
  CircleHelp,
  FolderKanban,
  FolderPlus,
  Lightbulb,
  Minus,
  NotebookPen,
  PenSquare,
  Plus,
  RotateCcw,
  RefreshCw,
  ChevronRight,
  MapPinned,
  Rows3,
  LibraryBig,
} from "lucide-react";

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
  creatingNote?: boolean;
  noteDraft?: string;
  onNoteDraftChange?: (value: string) => void;
  onSubmitNote?: () => void;
  onCancelNote?: () => void;
  onCreateQuestion?: () => void;
  creatingQuestion?: boolean;
  questionDraft?: string;
  onQuestionDraftChange?: (value: string) => void;
  onSubmitQuestion?: () => void;
  onCancelQuestion?: () => void;
  onCreateTask?: () => void;
  creatingTask?: boolean;
  taskDraft?: string;
  onTaskDraftChange?: (value: string) => void;
  onSubmitTask?: () => void;
  onCancelTask?: () => void;
  onCreateWhiteboard?: () => void;
  creatingWhiteboard?: boolean;
  whiteboardDraft?: string;
  onWhiteboardDraftChange?: (value: string) => void;
  onSubmitWhiteboard?: () => void;
  onCancelWhiteboard?: () => void;
  onShowOrphans?: () => void;
  /** 当前白板 id，wb_root 表示根白板 */
  currentWhiteboardId?: string;
  /** 返回根白板的回调，仅子白板时使用 */
  onNavigateBack?: () => void;
  /** 当前白板的 sections，给 Sections 下拉菜单用 */
  sections?: GraphSection[];
  /** 所有子白板列表，给 Boards 下拉菜单用 */
  whiteboards?: WhiteboardSummary[];
  /** 跳转到某 section 的回调（在 graph 中居中显示） */
  onJumpToSection?: (sectionId: string) => void;
  /** 跳转到某白板的回调（切换 currentWhiteboardId） */
  onJumpToBoard?: (whiteboardId: string) => void;
}

/**
 * 画布工具栏 — 4 个区域
 *
 * 统计区：实体计数
 * 创建区：Section / Note / 孤儿
 * 操作区：Sync + zoom +/-/reset
 * 跳转区：Sections / Boards / Root
 */
export function GraphToolbar({
  viewport,
  entityCounts,
  onSync,
  onCreateSection,
  onCreateNote,
  creatingNote = false,
  noteDraft = "",
  onNoteDraftChange,
  onSubmitNote,
  onCancelNote,
  onCreateQuestion,
  creatingQuestion = false,
  questionDraft = "",
  onQuestionDraftChange,
  onSubmitQuestion,
  onCancelQuestion,
  onCreateTask,
  creatingTask = false,
  taskDraft = "",
  onTaskDraftChange,
  onSubmitTask,
  onCancelTask,
  onCreateWhiteboard,
  creatingWhiteboard = false,
  whiteboardDraft = "",
  onWhiteboardDraftChange,
  onSubmitWhiteboard,
  onCancelWhiteboard,
  onShowOrphans,
  currentWhiteboardId = "wb_root",
  onNavigateBack,
  sections = [],
  whiteboards = [],
  onJumpToSection,
  onJumpToBoard,
}: GraphToolbarProps) {
  const { state, actions } = viewport;
  const zoomPercent = Math.round(state.zoom * 100);
  const coordX = Math.round(-state.panX / Math.max(state.zoom, 0.001));
  const coordY = Math.round(-state.panY / Math.max(state.zoom, 0.001));

  return (
    <div className="flex flex-wrap items-center gap-3 border-b border-[#dbd6cc] bg-[#f6f4ef] px-4 py-2 text-[13px] text-[#5c5548]">
      <div className="flex items-center gap-1.5">
        <button
          type="button"
          onClick={currentWhiteboardId !== "wb_root" ? onNavigateBack : undefined}
          className="rounded-md px-1.5 py-0.5 text-[#4b4efc] hover:bg-[#ebe8df]"
        >
          Root
        </button>
        {currentWhiteboardId !== "wb_root" && (
          <>
            <ChevronRight className="size-3.5 text-[#9b9383]" />
            <span className="rounded-md px-1.5 py-0.5 text-[#3a342b]">{currentWhiteboardId}</span>
          </>
        )}
      </div>

      <div className="h-5 w-px bg-[#d8d2c6]" />

      <div className="flex items-center gap-3">
        <span className="inline-flex items-center gap-1.5">
          <BookOpen className="size-3.5 text-[#7d725f]" />
          <span>{entityCounts.cards} Cards</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <Boxes className="size-3.5 text-[#7d725f]" />
          <span>{entityCounts.aliases} Aliases</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <NotebookPen className="size-3.5 text-[#7d725f]" />
          <span>{entityCounts.notes} Notes</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <Rows3 className="size-3.5 text-[#7d725f]" />
          <span>{entityCounts.sections} Sections</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <MapPinned className="size-3.5 text-[#7d725f]" />
          <span>{coordX}×{coordY}</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span className="text-[#7d725f]">Zoom</span>
          <span>{zoomPercent}%</span>
        </span>
      </div>

      <div className="h-5 w-px bg-[#d8d2c6]" />

      <Button variant="outline" size="sm" onClick={onCreateSection} aria-label="Create Section">
        <LibraryBig className="mr-1 h-4 w-4" />
        Section
      </Button>
      {creatingNote ? (
        <Input
          autoFocus
          value={noteDraft}
          placeholder="Note title…"
          aria-label="Note title"
          className="h-9 w-56 bg-white"
          onChange={(event) => onNoteDraftChange?.(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              event.currentTarget.blur();
            }
            if (event.key === "Escape") onCancelNote?.();
          }}
          onBlur={() => onSubmitNote?.()}
        />
      ) : (
        <Button variant="outline" size="sm" onClick={onCreateNote} aria-label="Create Note">
          <PenSquare className="mr-1 h-4 w-4" />
          Note
        </Button>
      )}
      {creatingQuestion ? (
        <Input
          autoFocus
          value={questionDraft}
          placeholder="Question title…"
          aria-label="Question title"
          className="h-9 w-56 bg-white"
          onChange={(event) => onQuestionDraftChange?.(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              event.currentTarget.blur();
            }
            if (event.key === "Escape") onCancelQuestion?.();
          }}
          onBlur={() => onSubmitQuestion?.()}
        />
      ) : (
        <Button variant="outline" size="sm" onClick={onCreateQuestion} aria-label="Create Question">
          <CircleHelp className="mr-1 h-4 w-4" />
          Question
        </Button>
      )}
      {currentWhiteboardId.startsWith("projects/") && (
        creatingTask ? (
          <Input
            autoFocus
            value={taskDraft}
            placeholder="Task title…"
            aria-label="Task title"
            className="h-9 w-56 bg-white"
            onChange={(event) => onTaskDraftChange?.(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                event.currentTarget.blur();
              }
              if (event.key === "Escape") onCancelTask?.();
            }}
            onBlur={() => onSubmitTask?.()}
          />
        ) : (
          <Button variant="outline" size="sm" onClick={onCreateTask} aria-label="Create Task">
            <CheckSquare className="mr-1 h-4 w-4" />
            Task
          </Button>
        )
      )}
      {currentWhiteboardId === "wb_root" && (
        creatingWhiteboard ? (
          <Input
            autoFocus
            value={whiteboardDraft}
            placeholder="Whiteboard name (= folder)…"
            aria-label="Whiteboard name"
            className="h-9 w-56 bg-white"
            onChange={(event) => onWhiteboardDraftChange?.(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                event.currentTarget.blur();
              }
              if (event.key === "Escape") onCancelWhiteboard?.();
            }}
            onBlur={() => onSubmitWhiteboard?.()}
          />
        ) : (
          <Button
            variant="outline"
            size="sm"
            onClick={onCreateWhiteboard}
            aria-label="Create Whiteboard"
          >
            <FolderPlus className="mr-1 h-4 w-4" />
            Whiteboard
          </Button>
        )
      )}
      <Button
        variant="outline"
        size="icon"
        onClick={onShowOrphans}
        aria-label="Show orphans"
        title="Show orphan cards"
      >
        <Lightbulb className="h-4 w-4" />
      </Button>

      {sections.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger
            render={(props) => (
              <Button
                variant="outline"
                size="sm"
                aria-label="Jump to section"
                {...(props as React.ComponentProps<typeof Button>)}
              >
                <FolderKanban className="mr-1 h-4 w-4" />
                Sections
              </Button>
            )}
          />
          <DropdownMenuContent align="start" className="max-h-96 w-64 overflow-y-auto">
            {sections.map((s) => (
              <DropdownMenuItem
                key={s.id}
                onClick={() => onJumpToSection?.(s.id)}
                className="flex items-center justify-between gap-3"
              >
                <span className="truncate text-sm">{s.title || "(untitled)"}</span>
                <span className="shrink-0 text-xs text-muted-foreground">
                  {s.cardIds.length}
                </span>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}

      {whiteboards.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger
            render={(props) => (
              <Button
                variant="outline"
                size="sm"
                aria-label="Jump to whiteboard"
                title="Jump to whiteboard"
                {...(props as React.ComponentProps<typeof Button>)}
              >
                <LibraryBig className="mr-1 h-4 w-4" />
                Boards
              </Button>
            )}
          />
          <DropdownMenuContent align="start" className="max-h-96 w-64 overflow-y-auto">
            {whiteboards.map((wb) => {
              const isCurrent = wb.whiteboardId === currentWhiteboardId;
              return (
                <DropdownMenuItem
                  key={wb.whiteboardId}
                  onClick={() => onJumpToBoard?.(wb.whiteboardId)}
                  className="flex items-center justify-between gap-3"
                >
                  <span
                    className={
                      isCurrent
                        ? "truncate text-sm font-semibold"
                        : "truncate text-sm"
                    }
                  >
                    {wb.whiteboardId}
                  </span>
                  <span className="shrink-0 text-xs text-muted-foreground">
                    {wb.cards}
                  </span>
                </DropdownMenuItem>
              );
            })}
          </DropdownMenuContent>
        </DropdownMenu>
      )}

      {/* 操作区 */}
      <div className="ml-auto flex items-center gap-1 border-l border-[#d8d2c6] pl-3">
        <Button variant="ghost" size="icon" onClick={onSync} aria-label="Sync" title="Sync">
          <RefreshCw className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={actions.zoomOut}
          aria-label="Zoom out"
          title="Zoom out"
        >
          <Minus className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={actions.zoomIn}
          aria-label="Zoom in"
          title="Zoom in"
        >
          <Plus className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={actions.resetView}
          aria-label="Reset zoom"
          title="Reset zoom"
        >
          <RotateCcw className="h-4 w-4" />
        </Button>
      </div>

    </div>
  );
}
