import { useEffect, useMemo, useState } from "react";
import type { AtomicCard } from "@/bindings";
import { commands } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { useQuery } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, Search, Target } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";

interface CardsListProps {
  cards: AtomicCard[];
  selectedCardId?: string | null;
  onFocusCard: (cardId: string) => void;
}

function topFolder(path: string): string {
  const [segment] = path.split("/");
  return segment || "root";
}

export function CardsList({ cards, selectedCardId = null, onFocusCard }: CardsListProps) {
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [collapsedFolders, setCollapsedFolders] = useState<Set<string>>(new Set());

  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedQuery(query.trim()), 300);
    return () => window.clearTimeout(timer);
  }, [query]);

  const searchQuery = useQuery({
    queryKey: ["sidebar", "cards-search", debouncedQuery],
    enabled: debouncedQuery.length > 0,
    queryFn: () => unwrapCommand(commands.cardSearch(debouncedQuery)),
  });

  const visibleCards = debouncedQuery.length > 0 ? (searchQuery.data ?? []) : cards;

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

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white px-3 py-3 shadow-sm">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2 top-1/2 size-4 -translate-y-1/2 text-[#8a7f6b]" />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search cards..."
            className="pl-8"
          />
        </div>
        <div className="mt-2 text-xs text-[#817564]">
          {debouncedQuery.length > 0 ? `${visibleCards.length} search hits` : `${cards.length} cards`}
        </div>
      </div>

      <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-3 py-2 text-xs text-[#8f846f]">
        Batch actions hook reserved for Phase 6 follow-up.
      </div>

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
                    <button
                      key={card.id}
                      type="button"
                      onClick={() => onFocusCard(card.id)}
                      className={`mb-1 flex w-full items-start gap-2 rounded-xl px-2 py-2 text-left transition-colors ${
                        selectedCardId === card.id
                          ? "bg-[#1f2937] text-white"
                          : "hover:bg-[#f5efe3]"
                      }`}
                    >
                      <Target className="mt-0.5 size-3.5 shrink-0" />
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-sm font-medium">{card.title}</div>
                        <div className={`truncate text-xs ${selectedCardId === card.id ? "text-white/70" : "text-[#7d715e]"}`}>
                          {card.filePath}
                        </div>
                        {card.tags.length > 0 && (
                          <div className="mt-1 flex flex-wrap gap-1">
                            {card.tags.slice(0, 4).map((tag) => (
                              <span
                                key={tag}
                                className={`rounded-full px-2 py-0.5 text-[10px] ${
                                  selectedCardId === card.id
                                    ? "bg-white/15 text-white"
                                    : "bg-[#f3ede2] text-[#6f6351]"
                                }`}
                              >
                                #{tag}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {searchQuery.isLoading && (
        <div className="text-xs text-[#8c806b]">Searching...</div>
      )}
    </div>
  );
}
