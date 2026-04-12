import { useEffect, useMemo, useState } from "react";
import type { AtomicCard } from "@/bindings";
import { Shuffle, Target } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ReviewViewProps {
  cards: AtomicCard[];
  onFocusCard: (cardId: string) => void;
}

export function ReviewView({ cards, onFocusCard }: ReviewViewProps) {
  const [order, setOrder] = useState<string[]>([]);
  const [index, setIndex] = useState(0);

  useEffect(() => {
    const next = [...cards].sort(() => Math.random() - 0.5).map((card) => card.id);
    setOrder(next);
    setIndex(0);
  }, [cards]);

  const current = useMemo(() => {
    const id = order[index];
    return cards.find((card) => card.id === id) ?? null;
  }, [cards, index, order]);

  return (
    <div className="space-y-4">
      <div className="rounded-3xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-[11px] font-semibold uppercase tracking-[0.16em] text-[#8f846f]">
          Review
        </div>
        <div className="mt-1 text-xs text-[#766a57]">
          Random flashcard drill. Content stays hidden until you jump into graph or detail.
        </div>
      </div>

      {current ? (
        <div className="rounded-[28px] border border-[#d9cfbf] bg-[#fffaf1] p-5 shadow-sm">
          <div className="text-xs font-semibold text-[#8d816b]">
            {index + 1} / {order.length}
          </div>
          <div className="mt-3 text-2xl font-semibold leading-tight text-[#241d15]">
            {current.title}
          </div>
          <div className="mt-4 rounded-2xl bg-[#f4efe3] px-4 py-3 text-sm leading-6 text-[#4f4538]">
            {current.understanding || "No understanding captured yet."}
          </div>
          <div className="mt-4 text-xs text-[#7f735e]">{current.filePath}</div>
          <div className="mt-5 flex gap-2">
            <Button variant="outline" onClick={() => onFocusCard(current.id)}>
              <Target className="size-4" />
              Locate In Graph
            </Button>
            <Button
              variant="secondary"
              onClick={() => setIndex((prev) => (prev + 1) % order.length)}
            >
              Next Card
            </Button>
            <Button
              variant="ghost"
              onClick={() => {
                const next = [...cards].sort(() => Math.random() - 0.5).map((card) => card.id);
                setOrder(next);
                setIndex(0);
              }}
            >
              <Shuffle className="size-4" />
              Shuffle
            </Button>
          </div>
        </div>
      ) : (
        <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-5 text-sm text-[#8d816b]">
          No cards available for review.
        </div>
      )}
    </div>
  );
}
