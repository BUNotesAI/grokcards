import { useMemo } from "react";
import type { AtomicCard, GraphNote } from "@/bindings";
import type { GraphSelection } from "@/components/keysight/types";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { Copy, ExternalLink, Focus, FolderSearch } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ContextPanelProps {
  selection: GraphSelection | null;
  card: AtomicCard | null;
  note: GraphNote | null;
  onFocusEntity: (entityId: string) => void;
}

export function ContextPanel({ selection, card, note, onFocusEntity }: ContextPanelProps) {
  const summary = useMemo(() => {
    if (!selection) return null;
    if (card) {
      return {
        title: card.title,
        path: card.filePath,
        meta: [`tags ${card.tags.length}`, `links ${card.linkTo.length}`],
      };
    }
    if (note) {
      return {
        title: note.title,
        path: null,
        meta: [`linked cards ${(note.linkedCardIds ?? []).length}`, `linked notes ${(note.linkedNoteIds ?? []).length}`],
      };
    }
    return {
      title: selection.id,
      path: null,
      meta: [selection.kind],
    };
  }, [card, note, selection]);

  if (!selection || !summary) {
    return (
      <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-5 text-sm text-[#8d816b]">
        Select an entity to open the context panel.
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Context</div>
        <div className="mt-2 text-lg font-semibold text-[#241d15]">{summary.title}</div>
        <div className="mt-2 text-xs text-[#7d715e]">{selection.kind} · {selection.id}</div>
        <div className="mt-3 flex flex-wrap gap-2 text-xs text-[#7d715e]">
          {summary.meta.map((item) => (
            <span key={item} className="rounded-full bg-[#f3ede2] px-2 py-1">{item}</span>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <Button variant="outline" onClick={() => onFocusEntity(selection.id)}>
          <Focus className="size-4" />
          Center
        </Button>
        <Button
          variant="outline"
          onClick={async () => {
            await navigator.clipboard.writeText(selection.id);
          }}
        >
          <Copy className="size-4" />
          Copy UUID
        </Button>
        <Button
          variant="outline"
          disabled={!summary.path}
          onClick={() => summary.path ? openPath(summary.path) : undefined}
        >
          <ExternalLink className="size-4" />
          Jump To Source
        </Button>
        <Button
          variant="outline"
          disabled={!summary.path}
          onClick={() => summary.path ? revealItemInDir(summary.path) : undefined}
        >
          <FolderSearch className="size-4" />
          Reveal File
        </Button>
      </div>

      <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-4 text-sm text-[#6f6351]">
        Draw connection / delete / move to section are intentionally deferred until the sidebar action model is stabilized.
      </div>
    </div>
  );
}
