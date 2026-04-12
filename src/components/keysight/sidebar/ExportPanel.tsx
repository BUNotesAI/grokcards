import { useMemo, useState } from "react";
import type { AtomicCard } from "@/bindings";
import { Download } from "lucide-react";
import { Button } from "@/components/ui/button";

type ExportRange = "current" | "all" | "selected";
type ExportFormat = "markdown" | "json";

interface ExportPanelProps {
  cards: AtomicCard[];
  currentWhiteboardId: string;
  currentWhiteboardCardIds: Set<string>;
  selectedCardIds: string[];
}

function generateMarkdown(cards: AtomicCard[]): string {
  return cards.map((card) => {
    const tags = card.tags.length > 0 ? `Tags: ${card.tags.map((tag) => `#${tag}`).join(" ")}` : "";
    const understanding = card.understanding ? `\n> ${card.understanding}\n` : "";
    return [
      `# ${card.title}`,
      "",
      `Source: ${card.filePath}`,
      tags,
      understanding.trimEnd(),
      "",
      card.content || "_No content_",
      "",
      "---",
    ].filter(Boolean).join("\n");
  }).join("\n");
}

function downloadFile(filename: string, content: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function ExportPanel({
  cards,
  currentWhiteboardId,
  currentWhiteboardCardIds,
  selectedCardIds,
}: ExportPanelProps) {
  const [range, setRange] = useState<ExportRange>("current");
  const [format, setFormat] = useState<ExportFormat>("markdown");

  const exportCards = useMemo(() => {
    if (range === "all") return cards;
    if (range === "selected") {
      const selected = new Set(selectedCardIds);
      return cards.filter((card) => selected.has(card.id));
    }
    return cards.filter((card) => currentWhiteboardCardIds.has(card.id));
  }, [cards, currentWhiteboardCardIds, range, selectedCardIds]);

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-sm font-semibold text-[#281f15]">Export Cards</div>
        <div className="mt-1 text-xs text-[#7b6f5d]">
          Save the current slice of cards as Markdown or JSON. Current whiteboard: {currentWhiteboardId}
        </div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Range</div>
        <div className="mt-3 flex flex-wrap gap-2">
          {[
            { id: "current", label: "Current Whiteboard" },
            { id: "all", label: "All Cards" },
            { id: "selected", label: "Selected Cards" },
          ].map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => setRange(item.id as ExportRange)}
              className={`rounded-full px-3 py-1.5 text-xs font-medium ${
                range === item.id ? "bg-[#1f2937] text-white" : "bg-[#f2ece0] text-[#665a49]"
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Format</div>
        <div className="mt-3 flex gap-2">
          {[
            { id: "markdown", label: "Markdown" },
            { id: "json", label: "JSON" },
          ].map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => setFormat(item.id as ExportFormat)}
              className={`rounded-full px-3 py-1.5 text-xs font-medium ${
                format === item.id ? "bg-[#1d9e75] text-white" : "bg-[#eef7f2] text-[#1d7058]"
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      <div className="rounded-2xl border border-dashed border-[#ddd3c2] bg-[#fbf8f1] px-4 py-4 text-sm text-[#6f6351]">
        {exportCards.length} cards ready for export.
      </div>

      <Button
        disabled={exportCards.length === 0}
        onClick={() => {
          const stamp = new Date().toISOString().slice(0, 10);
          if (format === "json") {
            downloadFile(`keysight-export-${range}-${stamp}.json`, JSON.stringify(exportCards, null, 2), "application/json");
          } else {
            downloadFile(`keysight-export-${range}-${stamp}.md`, generateMarkdown(exportCards), "text/markdown");
          }
        }}
      >
        <Download className="size-4" />
        Export {format === "json" ? "JSON" : "Markdown"}
      </Button>
    </div>
  );
}
