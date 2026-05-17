import { useEffect, useMemo, useState } from "react";
import type { AtomicCard, CardAlias, CardLinksResponse, GraphNote } from "@/bindings";
import type { GraphSelection } from "@/components/keysight/types";
import { useCardActions } from "@/hooks/useCardActions";
import { useNoteActions } from "@/hooks/useNoteActions";
import { openPath } from "@tauri-apps/plugin-opener";
import { ExternalLink, Focus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

interface InsightCardDetailProps {
  selection: GraphSelection | null;
  card: AtomicCard | null;
  note: GraphNote | null;
  alias: CardAlias | null;
  cardsById: Record<string, AtomicCard>;
  links: CardLinksResponse | null;
  aliasRefs: Array<{ aliasId: string; aliasTitle: string }>;
  onFocusEntity: (entityId: string) => void;
}

function CardReferenceList({
  title,
  ids,
  cardsById,
  onFocusEntity,
}: {
  title: string;
  ids: string[];
  cardsById: Record<string, AtomicCard>;
  onFocusEntity: (entityId: string) => void;
}) {
  if (ids.length === 0) return null;

  return (
    <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
      <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">{title}</div>
      <div className="mt-3 space-y-2">
        {ids.map((id) => (
          <button
            key={id}
            type="button"
            onClick={() => onFocusEntity(id)}
            className="block w-full rounded-xl bg-[#f7f1e6] px-3 py-2 text-left text-sm text-[#362d21] hover:bg-[#efe7d8]"
          >
            {cardsById[id]?.title ?? id}
          </button>
        ))}
      </div>
    </div>
  );
}

export function InsightCardDetail({
  selection,
  card,
  note,
  alias,
  cardsById,
  links,
  aliasRefs,
  onFocusEntity,
}: InsightCardDetailProps) {
  // sidebar 不知具体 wb,用 broadcast invalidate(hook wb 省略)
  const cards = useCardActions();
  const notes = useNoteActions();
  const [title, setTitle] = useState("");
  const [understanding, setUnderstanding] = useState("");
  const [content, setContent] = useState("");

  useEffect(() => {
    setTitle(card?.title ?? note?.title ?? "");
    setUnderstanding(card?.understanding ?? "");
    setContent(card?.content ?? note?.content ?? "");
  }, [card, note]);

  const titleLabel = useMemo(() => {
    if (!selection) return "Details";
    if (selection.kind === "alias") return "Alias Detail";
    if (selection.kind === "note") return "Note Detail";
    return "Entity Detail";
  }, [selection]);

  if (!selection) {
    return (
      <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-5 text-sm text-[#8d816b]">
        Select a card or note in the graph to inspect it here.
      </div>
    );
  }

  if (note && selection.kind === "note") {
    return (
      <div className="space-y-4">
        <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
          <div className="text-sm font-semibold text-[#281f15]">{titleLabel}</div>
          <div className="mt-2 text-lg font-semibold text-[#241d15]">{note.title}</div>
          <div className="mt-2 text-xs text-[#7d715e]">{note.id}</div>
        </div>
        <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
          <Input value={title} onChange={(event) => setTitle(event.target.value)} className="mb-3" />
          <Textarea value={content} onChange={(event) => setContent(event.target.value)} className="min-h-64" />
          <div className="mt-3 flex gap-2">
            <Button
              onClick={async () => {
                await notes.update(note.id, title, content, null);
              }}
            >
              Save
            </Button>
            <Button variant="outline" onClick={() => onFocusEntity(note.id)}>
              <Focus className="size-4" />
              Center In Graph
            </Button>
          </div>
        </div>
      </div>
    );
  }

  const detailCard = card;
  if (!detailCard) {
    return (
      <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-5 text-sm text-[#8d816b]">
        Detailed editing is currently available for cards and notes.
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-sm font-semibold text-[#281f15]">{titleLabel}</div>
        <div className="mt-2 text-xs text-[#7d715e]">
          {selection.kind} · {selection.id}
          {alias ? ` · target ${alias.cardId}` : ""}
        </div>
        <div className="mt-4 flex gap-2">
          <Button variant="outline" onClick={() => onFocusEntity(selection.id)}>
            <Focus className="size-4" />
            Center In Graph
          </Button>
          <Button variant="outline" onClick={() => openPath(detailCard.filePath)}>
            <ExternalLink className="size-4" />
            Jump To Source
          </Button>
        </div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="space-y-3">
          <Input value={title} onChange={(event) => setTitle(event.target.value)} />
          <Textarea
            value={understanding}
            onChange={(event) => setUnderstanding(event.target.value)}
            className="min-h-24"
            placeholder="Understanding..."
          />
          <Textarea
            value={content}
            onChange={(event) => setContent(event.target.value)}
            className="min-h-64"
            placeholder="Content..."
          />
          <div className="flex gap-2">
            <Button
              onClick={async () => {
                await cards.editTitle(detailCard.id, title);
                await cards.updateUnderstanding(detailCard.id, understanding);
                await cards.editBody(detailCard.id, content);
              }}
            >
              Save Card
            </Button>
          </div>
        </div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Tags</div>
        <div className="mt-3 flex flex-wrap gap-2">
          {detailCard.tags.length > 0 ? detailCard.tags.map((tag) => (
            <span key={tag} className="rounded-full bg-[#eef7f2] px-3 py-1 text-xs font-medium text-[#1d7058]">
              #{tag}
            </span>
          )) : <span className="text-sm text-[#8d816b]">No tags</span>}
        </div>
      </div>

      <CardReferenceList title="Linked" ids={links?.linkTo ?? []} cardsById={cardsById} onFocusEntity={onFocusEntity} />
      <CardReferenceList title="Related" ids={links?.related ?? []} cardsById={cardsById} onFocusEntity={onFocusEntity} />
      <CardReferenceList title="Incoming Links" ids={links?.linkedFrom ?? []} cardsById={cardsById} onFocusEntity={onFocusEntity} />
      <CardReferenceList title="Related From" ids={links?.relatedFrom ?? []} cardsById={cardsById} onFocusEntity={onFocusEntity} />

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Aliases & See Also</div>
        <div className="mt-3 space-y-2">
          {aliasRefs.map((item) => (
            <button
              key={item.aliasId}
              type="button"
              onClick={() => onFocusEntity(item.aliasId)}
              className="block w-full rounded-xl bg-[#f7f1e6] px-3 py-2 text-left text-sm text-[#362d21] hover:bg-[#efe7d8]"
            >
              Alias in {item.aliasTitle}
            </button>
          ))}
          {detailCard.seeAlso.map((entry) => (
            <div key={entry} className="rounded-xl bg-[#f7f1e6] px-3 py-2 text-sm text-[#362d21]">
              {entry}
            </div>
          ))}
          {aliasRefs.length === 0 && detailCard.seeAlso.length === 0 && (
            <div className="text-sm text-[#8d816b]">No aliases or see-also entries.</div>
          )}
        </div>
      </div>
    </div>
  );
}
