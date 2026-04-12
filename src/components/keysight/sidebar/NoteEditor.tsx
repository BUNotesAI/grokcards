import { useEffect, useState } from "react";
import type { GraphNote } from "@/bindings";
import { commands } from "@/bindings";
import { unwrapCommand } from "@/lib/commandResult";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

interface NoteEditorProps {
  note: GraphNote | null;
}

export function NoteEditor({ note }: NoteEditorProps) {
  const queryClient = useQueryClient();
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");

  useEffect(() => {
    setTitle(note?.title ?? "");
    setBody(note?.content ?? "");
  }, [note]);

  if (!note) {
    return (
      <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-5 text-sm text-[#8d816b]">
        Select a note to edit it in the sidebar.
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-sm font-semibold text-[#281f15]">Note Editor</div>
        <div className="mt-1 text-xs text-[#7b6f5d]">Large editing surface for note title and body.</div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="space-y-3">
          <Input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="Note title" />
          <Textarea
            value={body}
            onChange={(event) => setBody(event.target.value)}
            className="min-h-64"
            placeholder="Write note body..."
          />
          <div className="flex gap-2">
            <Button
              onClick={async () => {
                await unwrapCommand(commands.noteUpdate(note.id, title, body, null));
                await queryClient.invalidateQueries({ queryKey: ["notes"] });
              }}
            >
              Save Note
            </Button>
          </div>
        </div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm text-sm text-[#5f5548]">
        <div>Linked cards: {(note.linkedCardIds ?? []).length}</div>
        <div className="mt-1">Linked notes: {(note.linkedNoteIds ?? []).length}</div>
        <div className="mt-1">Linked sections: {(note.linkedSectionIds ?? []).length}</div>
      </div>
    </div>
  );
}
