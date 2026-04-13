import { useEffect, useMemo, useState, type ReactNode } from "react";
import type { AtomicCard, GraphSection } from "@/bindings";
import { commands } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { useQuery } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, Search, Target, ArrowLeft, PanelRightOpen } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { RenderedMarkdown } from "@/components/keysight/nodes/RenderedMarkdown";
import type { AliasReference, GraphSelection } from "@/components/keysight/types";

interface CardsListProps {
  cards: AtomicCard[];
  sections: GraphSection[];
  aliasRefsByTargetId?: Record<string, AliasReference[]>;
  selectedCard?: AtomicCard | null;
  selectedAlias?: AliasReference | null;
  previewRequestKey?: number;
  query: string;
  mode: "all" | "orphans";
  onQueryChange: (query: string) => void;
  onSelectCard: (cardId: string) => void;
  onSelectAlias: (aliasId: string) => void;
  onRevealSelection: (selection: GraphSelection) => void;
  onSelectTag: (tag: string) => void;
}

function topFolder(path: string): string {
  const [segment] = path.split("/");
  return segment || "root";
}

function buildOrphanSet(sections: GraphSection[]): Set<string> {
  return new Set(sections.flatMap((section) => section.cardIds));
}

function ResultItem({
  card,
  active,
  onClick,
}: {
  card: AtomicCard;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      key={card.id}
      type="button"
      onClick={onClick}
      className={`mb-1 flex w-full items-start gap-2 rounded-xl px-2 py-2 text-left transition-colors ${
        active ? "bg-[#1f2937] text-white" : "hover:bg-[#f5efe3]"
      }`}
    >
      <Target className="mt-0.5 size-3.5 shrink-0" />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">
          <RenderedMarkdown markdown={card.title} variant="title" />
        </div>
        <div className={`truncate text-xs ${active ? "text-white/70" : "text-[#7d715e]"}`}>
          {card.filePath}
        </div>
        {card.tags.length > 0 && (
          <div className="mt-1 flex flex-wrap gap-1">
            {card.tags.slice(0, 4).map((tag) => (
              <span
                key={tag}
                className={`rounded-full px-2 py-0.5 text-[10px] ${
                  active ? "bg-white/15 text-white" : "bg-[#f3ede2] text-[#6f6351]"
                }`}
              >
                #{tag}
              </span>
            ))}
          </div>
        )}
      </div>
    </button>
  );
}

function PreviewSectionHeading({ label, count }: { label: string; count: number }) {
  return (
    <div className="pt-1 text-xs font-semibold uppercase tracking-[0.16em] text-[#8f846f]">
      {label} ({count})
    </div>
  );
}

function PreviewActionRow({
  children,
  onClick,
}: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="block w-full border-t border-[#ece4d5] bg-transparent px-0 py-2 text-left text-sm text-[#362d21]"
    >
      {children}
    </button>
  );
}

export function CardsList({
  cards,
  sections,
  aliasRefsByTargetId = {},
  selectedCard = null,
  selectedAlias = null,
  previewRequestKey = 0,
  query,
  mode,
  onQueryChange,
  onSelectCard,
  onSelectAlias,
  onRevealSelection,
  onSelectTag,
}: CardsListProps) {
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [collapsedFolders, setCollapsedFolders] = useState<Set<string>>(new Set());
  const [previewCardId, setPreviewCardId] = useState<string | null>(null);
  const [previewCard, setPreviewCard] = useState<AtomicCard | null>(null);
  const [previewAliasId, setPreviewAliasId] = useState<string | null>(null);
  const [previewAlias, setPreviewAlias] = useState<AliasReference | null>(null);

  const orphanMembers = useMemo(() => buildOrphanSet(sections), [sections]);
  const scopedCards = useMemo(
    () =>
      mode === "orphans"
        ? cards.filter((card) => !orphanMembers.has(card.id))
        : cards,
    [cards, mode, orphanMembers],
  );
  const cardsById = useMemo(
    () => Object.fromEntries(cards.map((card) => [card.id, card])),
    [cards],
  );
  const normalizedQuery = query.trim();
  const tagQuery = normalizedQuery.startsWith("#") ? normalizedQuery.slice(1).trim().toLowerCase() : "";

  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedQuery(query.trim()), 300);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    setPreviewCardId(selectedCard?.id ?? null);
    setPreviewCard(selectedCard ?? null);
    setPreviewAliasId(selectedAlias?.aliasId ?? null);
    setPreviewAlias(selectedAlias ?? null);
  }, [previewRequestKey, selectedAlias, selectedCard]);

  useEffect(() => {
    setPreviewCardId(null);
    setPreviewCard(null);
    setPreviewAliasId(null);
    setPreviewAlias(null);
  }, [query, mode]);

  const searchQuery = useQuery({
    queryKey: ["sidebar", "cards-search", debouncedQuery],
    enabled: debouncedQuery.length > 0 && tagQuery.length === 0,
    queryFn: () => unwrapCommand(commands.cardSearch(debouncedQuery)),
  });

  const visibleCards = useMemo(() => {
    if (tagQuery.length > 0) {
      return cards.filter((card) =>
        card.tags.some((tag) => tag.toLowerCase().includes(tagQuery)),
      );
    }

    if (debouncedQuery.length > 0) {
      return searchQuery.data ?? [];
    }

    return scopedCards;
  }, [cards, debouncedQuery, scopedCards, searchQuery.data, tagQuery]);

  const grouped = useMemo(() => {
    const map = new Map<string, AtomicCard[]>();
    for (const card of visibleCards) {
      const key = topFolder(card.filePath);
      const list = map.get(key);
      if (list) {
        list.push(card);
      } else {
        map.set(key, [card]);
      }
    }

    return Array.from(map.entries())
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([folder, items]) => ({
        folder,
        items: items.sort((a, b) => a.title.localeCompare(b.title)),
      }));
  }, [visibleCards]);

  const isSearching = normalizedQuery.length > 0;
  const activePreviewAlias = previewAlias ?? (previewAliasId ? selectedAlias : null) ?? null;
  const activePreviewCard = activePreviewAlias
    ? previewCard ??
      selectedCard ??
      (activePreviewAlias.cardId ? cardsById[activePreviewAlias.cardId] ?? null : null)
    : previewCard ??
      (previewCardId ? cardsById[previewCardId] ?? null : null) ??
      (!isSearching && selectedCard && cardsById[selectedCard.id] ? selectedCard : null);
  const relatedCards = activePreviewCard
    ? activePreviewCard.related.map((id) => cardsById[id]).filter((card): card is AtomicCard => card != null)
    : [];
  const aliasRefs = activePreviewCard ? aliasRefsByTargetId[activePreviewCard.id] ?? [] : [];

  if (activePreviewCard) {
    return (
      <div className="space-y-4">
        <div className="rounded-2xl border border-[#e2dacb] bg-white px-3 py-3 shadow-sm">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2 top-1/2 size-4 -translate-y-1/2 text-[#8a7f6b]" />
            <Input
              value={query}
              onChange={(event) => onQueryChange(event.target.value)}
              placeholder="Search cards..."
              className="pl-8"
            />
          </div>
        </div>

        <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
          <div className="flex items-center justify-between gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setPreviewCardId(null);
                setPreviewCard(null);
              }}
              aria-label="Back to results"
            >
              <ArrowLeft className="size-4" />
              Back to results
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                onRevealSelection(
                  activePreviewAlias
                    ? { id: activePreviewAlias.aliasId, kind: "alias" }
                    : { id: activePreviewCard.id, kind: "card" },
                )
              }
              aria-label="Reveal in graph"
            >
              <PanelRightOpen className="size-4" />
              Reveal
            </Button>
          </div>

          <div className="mt-4">
            <div className="text-xs uppercase tracking-[0.16em] text-[#8f846f]">{activePreviewCard.filePath}</div>
            <div className="mt-2 text-lg font-semibold text-[#231f17]">
              <RenderedMarkdown markdown={activePreviewCard.title} variant="title" />
            </div>
            {activePreviewAlias && (
              <div className="mt-2 text-xs uppercase tracking-[0.16em] text-[#8f846f]">
                Alias in {activePreviewAlias.sectionTitle ?? activePreviewAlias.aliasTitle}
              </div>
            )}
            {activePreviewCard.understanding && (
              <div className="mt-3 rounded-xl border border-[#dce7de] bg-[#f7fbf8] px-3 py-3 text-sm text-[#49544b]">
                <RenderedMarkdown markdown={activePreviewCard.understanding} variant="understanding" />
              </div>
            )}
            {activePreviewCard.tags.length > 0 && (
              <div className="mt-4 flex flex-wrap gap-2">
                {activePreviewCard.tags.map((tag) => (
                  <button
                    key={tag}
                    type="button"
                    onClick={() => onSelectTag(tag)}
                    className="rounded-full bg-[#eef7f2] px-3 py-1 text-xs font-medium text-[#1d7058]"
                  >
                    #{tag}
                  </button>
                ))}
              </div>
            )}
            {activePreviewCard.content && (
              <div className="mt-4 rounded-xl border border-[#ece4d5] bg-[#fbf8f1] px-3 py-3 text-sm text-[#2b241a]">
                <RenderedMarkdown markdown={activePreviewCard.content} variant="body" />
              </div>
            )}
            {relatedCards.length > 0 && (
              <div className="mt-4">
                <PreviewSectionHeading label="Related" count={relatedCards.length} />
                {relatedCards.map((card) => (
                  <PreviewActionRow
                    key={card.id}
                    onClick={() => {
                      setPreviewCardId(card.id);
                      setPreviewCard(card);
                      setPreviewAliasId(null);
                      setPreviewAlias(null);
                      onSelectCard(card.id);
                    }}
                  >
                    <RenderedMarkdown markdown={card.title} variant="title" />
                  </PreviewActionRow>
                ))}
              </div>
            )}
            {aliasRefs.length > 0 && (
              <div className="mt-4">
                <PreviewSectionHeading label="Aliases" count={aliasRefs.length} />
                {aliasRefs.map((item) => (
                  <PreviewActionRow
                    key={item.aliasId}
                    onClick={() => {
                      setPreviewAliasId(item.aliasId);
                      setPreviewAlias(item);
                      setPreviewCardId(activePreviewCard.id);
                      setPreviewCard(activePreviewCard);
                      onSelectAlias(item.aliasId);
                    }}
                  >
                    <div className="text-sm text-[#362d21]">
                      {item.sectionTitle ?? item.aliasTitle}
                    </div>
                    <div className="mt-1 text-xs text-[#8f846f]">Alias</div>
                  </PreviewActionRow>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white px-3 py-3 shadow-sm">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2 top-1/2 size-4 -translate-y-1/2 text-[#8a7f6b]" />
          <Input
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="Search cards..."
            className="pl-8"
          />
        </div>
        <div className="mt-2 text-xs text-[#817564]">
          {isSearching
            ? `${visibleCards.length} search hits`
            : mode === "orphans"
              ? `${visibleCards.length} orphan cards`
              : `${scopedCards.length} cards`}
        </div>
      </div>

      {isSearching ? (
        <div className="rounded-2xl border border-[#e2dacb] bg-white px-2 py-2 shadow-sm">
          {visibleCards.map((card) => (
            <ResultItem
              key={card.id}
              card={card}
              active={selectedCard?.id === card.id}
              onClick={() => {
                setPreviewCardId(card.id);
                setPreviewCard(card);
                onSelectCard(card.id);
              }}
            />
          ))}
          {visibleCards.length === 0 && (
            <div className="px-2 py-4 text-sm text-[#8d816b]">No matching cards.</div>
          )}
        </div>
      ) : (
        <div className="space-y-3">
          {grouped.map(({ folder, items }) => {
            const collapsed = collapsedFolders.has(folder);
            return (
              <div key={folder} className="rounded-2xl border border-[#e2dacb] bg-white shadow-sm">
                <button
                  type="button"
                  onClick={() => {
                    setCollapsedFolders((prev) => {
                      const next = new Set(prev);
                      if (next.has(folder)) {
                        next.delete(folder);
                      } else {
                        next.add(folder);
                      }
                      return next;
                    });
                  }}
                  className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm font-semibold text-[#2b241a]"
                >
                  {collapsed ? <ChevronRight className="size-4" /> : <ChevronDown className="size-4" />}
                  <span className="flex-1 truncate">{folder}</span>
                  <Badge variant="outline">{items.length}</Badge>
                </button>
                {!collapsed && (
                  <div className="border-t border-[#efe7d8] px-2 py-2">
                    {items.map((card) => (
                      <ResultItem
                        key={card.id}
                        card={card}
                        active={selectedCard?.id === card.id}
                        onClick={() => {
                          setPreviewCardId(card.id);
                          setPreviewCard(card);
                          onSelectCard(card.id);
                        }}
                      />
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {searchQuery.isLoading && (
        <div className="text-xs text-[#8c806b]">Searching...</div>
      )}
    </div>
  );
}
