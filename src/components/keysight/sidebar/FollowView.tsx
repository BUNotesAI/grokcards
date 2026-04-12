import { useEffect, useState } from "react";
import type { AtomicCard } from "@/bindings";
import { commands } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { useQuery } from "@tanstack/react-query";
import { Pin, Target } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface FollowViewProps {
  pinnedFilePath: string;
  onPinnedFilePathChange: (path: string) => void;
  selectedCard: AtomicCard | null;
  onFocusCard: (cardId: string) => void;
}

export function FollowView({
  pinnedFilePath,
  onPinnedFilePathChange,
  selectedCard,
  onFocusCard,
}: FollowViewProps) {
  const [draft, setDraft] = useState(pinnedFilePath);

  useEffect(() => {
    setDraft(pinnedFilePath);
  }, [pinnedFilePath]);

  const followQuery = useQuery({
    queryKey: ["sidebar", "follow", pinnedFilePath],
    enabled: pinnedFilePath.trim().length > 0,
    queryFn: () => unwrapCommand(commands.cardQueryByFile(pinnedFilePath)),
  });

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-sm font-semibold text-[#281f15]">Follow File</div>
        <div className="mt-1 text-xs text-[#7b6f5d]">
          Temporary explicit pinning until a vault watcher-driven active file model lands.
        </div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="space-y-2">
          <Input
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="Paste an .md file path..."
          />
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onPinnedFilePathChange(draft.trim())}>
              <Pin className="size-4" />
              Pin To File
            </Button>
            <Button
              variant="ghost"
              disabled={!selectedCard}
              onClick={() => {
                if (!selectedCard) return;
                setDraft(selectedCard.filePath);
                onPinnedFilePathChange(selectedCard.filePath);
              }}
            >
              Use Selected Card
            </Button>
          </div>
        </div>
      </div>

      {pinnedFilePath.trim().length === 0 ? (
        <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-5 text-sm text-[#8d816b]">
          No pinned file yet.
        </div>
      ) : (
        <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
          <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Pinned Path</div>
          <div className="mt-2 break-all text-sm text-[#3f372d]">{pinnedFilePath}</div>

          <div className="mt-4 space-y-2">
            {(followQuery.data ?? []).map((card) => (
              <button
                key={card.id}
                type="button"
                onClick={() => onFocusCard(card.id)}
                className="flex w-full items-start gap-2 rounded-xl bg-[#f7f1e6] px-3 py-2 text-left hover:bg-[#efe7d8]"
              >
                <Target className="mt-0.5 size-4 shrink-0 text-[#6f6351]" />
                <div>
                  <div className="text-sm font-medium text-[#2b241a]">{card.title}</div>
                  <div className="text-xs text-[#7d715e]">{card.filePath}</div>
                </div>
              </button>
            ))}
            {followQuery.data?.length === 0 && !followQuery.isLoading && (
              <div className="text-sm text-[#8d816b]">No cards mapped to this file.</div>
            )}
            {followQuery.isLoading && <div className="text-sm text-[#8d816b]">Looking up cards...</div>}
          </div>
        </div>
      )}
    </div>
  );
}
