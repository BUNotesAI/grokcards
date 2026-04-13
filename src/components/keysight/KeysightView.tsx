import { useCallback, useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";
import { GraphView, ROOT_WHITEBOARD, type GraphFocusTarget } from "@/components/keysight/GraphView";
import { useWhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { AliasReference, GraphSelection } from "@/components/keysight/types";
import { Sidebar } from "@/components/keysight/sidebar/Sidebar";
import { CardsList } from "@/components/keysight/sidebar/CardsList";

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

function deriveWhiteboardIdFromFilePath(filePath: string): string {
  const normalized = filePath.replace(/\\/g, "/");
  if (!normalized.startsWith("whiteboard/")) {
    return ROOT_WHITEBOARD;
  }

  const rest = normalized.slice("whiteboard/".length);
  const [first, second] = rest.split("/", 2);
  return second ? first : ROOT_WHITEBOARD;
}

export function KeysightView() {
  const [currentWhiteboardId, setCurrentWhiteboardId] = useState(ROOT_WHITEBOARD);
  const [selected, setSelected] = useState<GraphSelection | null>(null);
  const [previewRequestKey, setPreviewRequestKey] = useState(0);
  const [focusTarget, setFocusTarget] = useState<GraphFocusTarget | null>(null);
  const [sidebarWidth, setSidebarWidth] = useState(() => loadNumber("width", DEFAULT_WIDTH));
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => loadBoolean("collapsed", false));
  const [sidebarQuery, setSidebarQuery] = useState(() => loadString("query", ""));
  const [sidebarMode, setSidebarMode] = useState<"all" | "orphans">(
    () => (loadString("mode", "all") === "orphans" ? "orphans" : "all"),
  );
  const boardData = useWhiteboardData(currentWhiteboardId);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "width", String(sidebarWidth));
  }, [sidebarWidth]);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "collapsed", String(sidebarCollapsed));
  }, [sidebarCollapsed]);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "query", sidebarQuery);
  }, [sidebarQuery]);

  useEffect(() => {
    localStorage.setItem(STORAGE_PREFIX + "mode", sidebarMode);
  }, [sidebarMode]);

  const cardsById = useMemo(
    () => Object.fromEntries(boardData.cards.map((card) => [card.id, card])),
    [boardData.cards],
  );
  const aliasRefsByTargetId = useMemo(() => {
    const map: Record<string, AliasReference[]> = {};
    for (const alias of boardData.aliases) {
      if (!map[alias.cardId]) {
        map[alias.cardId] = [];
      }
      const containingSection = boardData.sections.find((section) => section.cardIds.includes(alias.aliasId));
      map[alias.cardId].push({
        aliasId: alias.aliasId,
        aliasTitle: containingSection?.title ?? alias.aliasId,
        cardId: alias.cardId,
        sectionId: containingSection?.id ?? null,
        sectionTitle: containingSection?.title ?? null,
      });
    }
    return map;
  }, [boardData.aliases, boardData.sections]);
  const aliasesById = useMemo(() => {
    const map: Record<string, AliasReference> = {};
    for (const refs of Object.values(aliasRefsByTargetId)) {
      for (const ref of refs) {
        map[ref.aliasId] = ref;
      }
    }
    return map;
  }, [aliasRefsByTargetId]);

  const selectedAlias = useMemo(
    () => (selected?.kind === "alias" ? aliasesById[selected.id] ?? null : null),
    [aliasesById, selected],
  );

  const selectedCard = useMemo(() => {
    if (!selected) return null;
    if (selected.kind === "card") return cardsById[selected.id] ?? null;
    if (selected.kind === "alias") return selectedAlias?.cardId ? cardsById[selectedAlias.cardId] ?? null : null;
    return null;
  }, [cardsById, selected, selectedAlias]);

  const handleWhiteboardChange = useCallback((whiteboardId: string) => {
    setCurrentWhiteboardId(whiteboardId);
    setSelected(null);
  }, []);

  const handleFocusEntity = useCallback((entityId: string, kind: GraphSelection["kind"] = "card") => {
    setSelected({ id: entityId, kind });
    setFocusTarget({ id: entityId, nonce: Date.now() });
  }, []);

  const handleRevealSelection = useCallback((selection: GraphSelection) => {
    if (selection.kind === "card") {
      const card = cardsById[selection.id];
      if (card) {
        const targetWhiteboardId = deriveWhiteboardIdFromFilePath(card.filePath);
        if (targetWhiteboardId !== currentWhiteboardId) {
          setCurrentWhiteboardId(targetWhiteboardId);
        }
      }
      handleFocusEntity(selection.id, "card");
      return;
    }

    if (selection.kind === "alias") {
      handleFocusEntity(selection.id, "alias");
      return;
    }

    handleFocusEntity(selection.id, selection.kind);
  }, [cardsById, currentWhiteboardId, handleFocusEntity]);

  const handlePreviewEntity = useCallback((entityId: string, kind: GraphSelection["kind"] = "card") => {
    setSidebarCollapsed(false);
    setSelected({ id: entityId, kind });
    setPreviewRequestKey((prev) => prev + 1);
  }, []);

  const handleSelectEntity = useCallback((selection: GraphSelection | null) => {
    if (selection) {
      setSidebarCollapsed(false);
    }
    setSelected(selection);
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

  const selectedEntityExists = useMemo(() => {
    if (!selected) return false;
    if (selected.kind === "card") return Boolean(cardsById[selected.id]);
    if (selected.kind === "alias") return Boolean(selectedAlias);
    if (selected.kind === "note") return boardData.notes.some((note) => note.id === selected.id);
    if (selected.kind === "task") return boardData.tasks.some((task) => task.id === selected.id);
    if (selected.kind === "question") return boardData.questions.some((question) => question.id === selected.id);
    if (selected.kind === "section") return boardData.sections.some((section) => section.id === selected.id);
    return false;
  }, [boardData.notes, boardData.questions, boardData.sections, boardData.tasks, cardsById, selected, selectedAlias]);

  useEffect(() => {
    if (selected && !selectedEntityExists) {
      setSelected(null);
    }
  }, [selected, selectedEntityExists]);

  const handleSelectTag = useCallback((tag: string) => {
    setSidebarCollapsed(false);
    setSidebarMode("all");
    setSidebarQuery(`#${tag}`);
  }, []);

  const handleShowOrphans = useCallback(() => {
    setSidebarCollapsed(false);
    setSidebarMode("orphans");
    setSidebarQuery("");
    setSelected(null);
  }, []);

  return (
    <div className="flex h-full w-full overflow-hidden bg-[#f0eadf]">
      <div className="min-w-0 flex-1">
        <GraphView
          currentWhiteboardId={currentWhiteboardId}
          onWhiteboardChange={handleWhiteboardChange}
          selectedEntityId={selected?.id ?? null}
          onSelectEntity={handleSelectEntity}
          focusTarget={focusTarget}
          onShowOrphans={handleShowOrphans}
          onSearchTag={handleSelectTag}
          onOpenCard={(cardId) => handlePreviewEntity(cardId, "card")}
          onOpenAlias={(aliasId) => handlePreviewEntity(aliasId, "alias")}
        />
      </div>

      <Sidebar
        collapsed={sidebarCollapsed}
        width={sidebarWidth}
        title={currentWhiteboardId === ROOT_WHITEBOARD ? "Library" : currentWhiteboardId}
        onToggleCollapse={() => setSidebarCollapsed((prev) => !prev)}
        onResizeStart={handleResizeStart}
      >
        <CardsList
          cards={boardData.cards}
          sections={boardData.sections}
          aliasRefsByTargetId={aliasRefsByTargetId}
          selectedCard={selectedCard}
          selectedAlias={selectedAlias}
          previewRequestKey={previewRequestKey}
          query={sidebarQuery}
          mode={sidebarMode}
          onQueryChange={setSidebarQuery}
          onSelectCard={(cardId) => handlePreviewEntity(cardId, "card")}
          onSelectAlias={(aliasId) => handlePreviewEntity(aliasId, "alias")}
          onRevealSelection={handleRevealSelection}
          onSelectTag={handleSelectTag}
        />
      </Sidebar>
    </div>
  );
}
