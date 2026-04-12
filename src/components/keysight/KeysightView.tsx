import { useCallback, useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";
import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import type { AtomicCard, GraphNote } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { GraphView, ROOT_WHITEBOARD, type GraphFocusTarget } from "@/components/keysight/GraphView";
import { useWhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { GraphSelection } from "@/components/keysight/types";
import { Sidebar } from "@/components/keysight/sidebar/Sidebar";
import { CardsList } from "@/components/keysight/sidebar/CardsList";
import { ReviewView } from "@/components/keysight/sidebar/ReviewView";
import { FilterBar } from "@/components/keysight/sidebar/FilterBar";
import { ExportPanel } from "@/components/keysight/sidebar/ExportPanel";
import { FollowView } from "@/components/keysight/sidebar/FollowView";
import { InsightCardDetail } from "@/components/keysight/sidebar/InsightCardDetail";
import { ContextPanel } from "@/components/keysight/sidebar/ContextPanel";
import { NoteEditor } from "@/components/keysight/sidebar/NoteEditor";
import type { SidebarTab, SidebarTabItem } from "@/components/keysight/sidebar/types";

const STORAGE_PREFIX = "keysight:sidebar:";
const DEFAULT_WIDTH = 360;
const MIN_WIDTH = 280;
const MAX_WIDTH = 560;

function loadNumber(key: string, fallback: number): number {
  const raw = localStorage.getItem(STORAGE_PREFIX + key);
  if (!raw) return fallback;
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function loadBoolean(key: string, fallback: boolean): boolean {
  const raw = localStorage.getItem(STORAGE_PREFIX + key);
  if (!raw) return fallback;
  return raw === "true";
}

function loadString(key: string, fallback: string): string {
  return localStorage.getItem(STORAGE_PREFIX + key) ?? fallback;
}

function matchesFilters(card: AtomicCard, selectedFolder: string | null, selectedTags: string[]) {
  const folder = card.filePath.split("/")[0] || "root";
  const folderMatch = !selectedFolder || folder === selectedFolder;
  const tagsMatch = selectedTags.every((tag) => card.tags.includes(tag));
  return folderMatch && tagsMatch;
}

export function KeysightView() {
  const [currentWhiteboardId, setCurrentWhiteboardId] = useState(ROOT_WHITEBOARD);
  const [selected, setSelected] = useState<GraphSelection | null>(null);
  const [focusTarget, setFocusTarget] = useState<GraphFocusTarget | null>(null);
  const [sidebarWidth, setSidebarWidth] = useState(() => loadNumber("width", DEFAULT_WIDTH));
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => loadBoolean("collapsed", false));
  const [activeTab, setActiveTab] = useState<SidebarTab>(() => loadString("tab", "cards") as SidebarTab);
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [pinnedFilePath, setPinnedFilePath] = useState(() => loadString("follow-file", ""));
  const boardData = useWhiteboardData(currentWhiteboardId);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "width", String(sidebarWidth));
  }, [sidebarWidth]);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "collapsed", String(sidebarCollapsed));
  }, [sidebarCollapsed]);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "tab", activeTab);
  }, [activeTab]);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "follow-file", pinnedFilePath);
  }, [pinnedFilePath]);

  const cardsById = useMemo(() => {
    const map: Record<string, AtomicCard> = {};
    for (const card of boardData.cards) {
      map[card.id] = card;
    }
    return map;
  }, [boardData.cards]);

  const selectedAlias = useMemo(
    () => selected?.kind === "alias"
      ? boardData.aliases.find((alias) => alias.aliasId === selected.id) ?? null
      : null,
    [boardData.aliases, selected],
  );

  const selectedCard = useMemo(() => {
    if (!selected) return null;
    if (selected.kind === "card") return cardsById[selected.id] ?? null;
    if (selected.kind === "alias") return selectedAlias ? cardsById[selectedAlias.cardId] ?? null : null;
    return null;
  }, [cardsById, selected, selectedAlias]);

  const selectedNote = useMemo<GraphNote | null>(
    () => selected?.kind === "note"
      ? boardData.notes.find((note) => note.id === selected.id) ?? null
      : null,
    [boardData.notes, selected],
  );

  const selectedCardLinks = useQuery({
    queryKey: ["card-links", selectedCard?.id],
    enabled: selectedCard != null,
    queryFn: () => {
      if (!selectedCard) {
        throw new Error("selectedCard is required");
      }
      return unwrapCommand(commands.cardQueryLinks(selectedCard.id));
    },
  });

  const aliasRefs = useMemo(() => {
    if (!selectedCard) return [];
    return boardData.aliases
      .filter((alias) => alias.cardId === selectedCard.id)
      .map((alias) => {
        const containingSection = boardData.sections.find((section) => section.cardIds.includes(alias.aliasId));
        return {
          aliasId: alias.aliasId,
          aliasTitle: containingSection?.title ?? alias.aliasId,
        };
      });
  }, [boardData.aliases, boardData.sections, selectedCard]);

  const highlightedEntityIds = useMemo(() => {
    if (!selectedFolder && selectedTags.length === 0) return undefined;
    const ids = new Set<string>();
    for (const card of boardData.cards) {
      if (matchesFilters(card, selectedFolder, selectedTags)) {
        ids.add(card.id);
      }
    }
    for (const alias of boardData.aliases) {
      if (ids.has(alias.cardId)) {
        ids.add(alias.aliasId);
      }
    }
    return ids;
  }, [boardData.aliases, boardData.cards, selectedFolder, selectedTags]);

  const currentWhiteboardCardIds = useMemo(() => {
    const ids = new Set<string>();
    for (const entityId of Object.keys(boardData.positions)) {
      if (cardsById[entityId]) ids.add(entityId);
      const alias = boardData.aliases.find((item) => item.aliasId === entityId);
      if (alias) ids.add(alias.cardId);
    }
    return ids;
  }, [boardData.aliases, boardData.positions, cardsById]);

  useEffect(() => {
    if (!selected) return;
    if (selected.kind === "note") {
      setActiveTab("note");
    } else {
      setActiveTab("details");
    }
  }, [selected]);

  useEffect(() => {
    if (!selected && (activeTab === "details" || activeTab === "context" || activeTab === "note")) {
      setActiveTab("cards");
    }
  }, [activeTab, selected]);

  const handleWhiteboardChange = useCallback((whiteboardId: string) => {
    setCurrentWhiteboardId(whiteboardId);
    setSelected(null);
  }, []);

  const handleFocusEntity = useCallback((entityId: string) => {
    setFocusTarget({ id: entityId, nonce: Date.now() });
  }, []);

  const handleResizeStart = useCallback((event: ReactMouseEvent<HTMLDivElement>) => {
    const startX = event.clientX;
    const startWidth = sidebarWidth;

    const onMove = (moveEvent: MouseEvent) => {
      const next = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidth - (moveEvent.clientX - startX)));
      setSidebarWidth(next);
    };

    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }, [sidebarWidth]);

  const selectedCardIds = selectedCard ? [selectedCard.id] : [];

  const tabs = useMemo<SidebarTabItem[]>(() => {
    const base: SidebarTabItem[] = [
      { id: "follow", label: "Follow" },
      { id: "cards", label: "Cards" },
      { id: "review", label: "Review" },
      { id: "filter", label: "Filter" },
      { id: "export", label: "Export" },
    ];

    if (selected) {
      base.push({ id: selected.kind === "note" ? "note" : "details", label: selected.kind === "note" ? "Note" : "Details" });
      base.push({ id: "context", label: "Context" });
    }

    return base;
  }, [selected]);

  const selectedEntityExists = useMemo(() => {
    if (!selected) return false;
    if (selected.kind === "card") return Boolean(cardsById[selected.id]);
    if (selected.kind === "alias") return Boolean(selectedAlias);
    if (selected.kind === "note") return Boolean(selectedNote);
    if (selected.kind === "task") return boardData.tasks.some((task) => task.id === selected.id);
    if (selected.kind === "question") return boardData.questions.some((question) => question.id === selected.id);
    if (selected.kind === "section") return boardData.sections.some((section) => section.id === selected.id);
    return false;
  }, [boardData.questions, boardData.sections, boardData.tasks, cardsById, selected, selectedAlias, selectedNote]);

  useEffect(() => {
    if (selected && !selectedEntityExists) {
      setSelected(null);
    }
  }, [selected, selectedEntityExists]);

  return (
    <div className="flex h-full w-full overflow-hidden bg-[#f0eadf]">
      <div className="min-w-0 flex-1">
        <GraphView
          currentWhiteboardId={currentWhiteboardId}
          onWhiteboardChange={handleWhiteboardChange}
          selectedEntityId={selected?.id ?? null}
          onSelectEntity={setSelected}
          highlightedEntityIds={highlightedEntityIds}
          focusTarget={focusTarget}
        />
      </div>

      <Sidebar
        collapsed={sidebarCollapsed}
        width={sidebarWidth}
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={setActiveTab}
        onToggleCollapse={() => setSidebarCollapsed((prev) => !prev)}
        onResizeStart={handleResizeStart}
      >
        {activeTab === "follow" && (
          <FollowView
            pinnedFilePath={pinnedFilePath}
            onPinnedFilePathChange={setPinnedFilePath}
            selectedCard={selectedCard}
            onFocusCard={handleFocusEntity}
          />
        )}
        {activeTab === "cards" && (
          <CardsList cards={boardData.cards} selectedCardId={selectedCard?.id ?? null} onFocusCard={handleFocusEntity} />
        )}
        {activeTab === "review" && (
          <ReviewView cards={boardData.cards} onFocusCard={handleFocusEntity} />
        )}
        {activeTab === "filter" && (
          <FilterBar
            cards={boardData.cards}
            selectedFolder={selectedFolder}
            selectedTags={selectedTags}
            onFolderChange={setSelectedFolder}
            onTagsChange={setSelectedTags}
          />
        )}
        {activeTab === "export" && (
          <ExportPanel
            cards={boardData.cards}
            currentWhiteboardId={currentWhiteboardId}
            currentWhiteboardCardIds={currentWhiteboardCardIds}
            selectedCardIds={selectedCardIds}
          />
        )}
        {activeTab === "details" && (
          <InsightCardDetail
            selection={selected}
            card={selectedCard}
            note={selectedNote}
            alias={selectedAlias}
            cardsById={cardsById}
            links={selectedCardLinks.data ?? null}
            aliasRefs={aliasRefs}
            onFocusEntity={handleFocusEntity}
          />
        )}
        {activeTab === "context" && (
          <ContextPanel
            selection={selected}
            card={selectedCard}
            note={selectedNote}
            onFocusEntity={handleFocusEntity}
          />
        )}
        {activeTab === "note" && <NoteEditor note={selectedNote} />}
      </Sidebar>
    </div>
  );
}
