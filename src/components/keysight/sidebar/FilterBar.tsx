import { useMemo } from "react";
import type { AtomicCard } from "@/bindings";
import { Button } from "@/components/ui/button";

interface FilterBarProps {
  cards: AtomicCard[];
  selectedFolder: string | null;
  selectedTags: string[];
  onFolderChange: (folder: string | null) => void;
  onTagsChange: (tags: string[]) => void;
}

function topFolder(path: string): string {
  const [segment] = path.split("/");
  return segment || "root";
}

export function FilterBar({
  cards,
  selectedFolder,
  selectedTags,
  onFolderChange,
  onTagsChange,
}: FilterBarProps) {
  const folders = useMemo(
    () => Array.from(new Set(cards.map((card) => topFolder(card.filePath)))).sort(),
    [cards],
  );
  const tags = useMemo(
    () => Array.from(new Set(cards.flatMap((card) => card.tags))).sort(),
    [cards],
  );

  const matchCount = useMemo(() => {
    return cards.filter((card) => {
      const folderMatch = !selectedFolder || topFolder(card.filePath) === selectedFolder;
      const tagMatch = selectedTags.every((tag) => card.tags.includes(tag));
      return folderMatch && tagMatch;
    }).length;
  }, [cards, selectedFolder, selectedTags]);

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-sm font-semibold text-[#281f15]">Graph Highlight Filters</div>
        <div className="mt-1 text-xs text-[#7b6f5d]">
          Folder is single-select. Tags are AND-combined and reflected in graph node highlights.
        </div>
        <div className="mt-3 text-xs font-medium text-[#8a7f6b]">{matchCount} matching cards</div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Folders</div>
        <div className="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => onFolderChange(null)}
            className={`rounded-full px-3 py-1.5 text-xs font-medium ${
              selectedFolder === null
                ? "bg-[#1f2937] text-white"
                : "bg-[#f2ece0] text-[#665a49]"
            }`}
          >
            All
          </button>
          {folders.map((folder) => (
            <button
              key={folder}
              type="button"
              onClick={() => onFolderChange(selectedFolder === folder ? null : folder)}
              className={`rounded-full px-3 py-1.5 text-xs font-medium ${
                selectedFolder === folder
                  ? "bg-[#1f2937] text-white"
                  : "bg-[#f2ece0] text-[#665a49]"
              }`}
            >
              {folder}
            </button>
          ))}
        </div>
      </div>

      <div className="rounded-2xl border border-[#e2dacb] bg-white p-4 shadow-sm">
        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-[#8f846f]">Tags</div>
        <div className="mt-3 flex flex-wrap gap-2">
          {tags.map((tag) => {
            const active = selectedTags.includes(tag);
            return (
              <button
                key={tag}
                type="button"
                onClick={() => {
                  if (active) {
                    onTagsChange(selectedTags.filter((item) => item !== tag));
                  } else {
                    onTagsChange([...selectedTags, tag]);
                  }
                }}
                className={`rounded-full px-3 py-1.5 text-xs font-medium ${
                  active ? "bg-[#1d9e75] text-white" : "bg-[#eef7f2] text-[#1d7058]"
                }`}
              >
                #{tag}
              </button>
            );
          })}
        </div>
      </div>

      <Button
        variant="outline"
        onClick={() => {
          onFolderChange(null);
          onTagsChange([]);
        }}
      >
        Clear Filters
      </Button>
    </div>
  );
}
