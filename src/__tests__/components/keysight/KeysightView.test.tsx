import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { KeysightView } from "@/components/keysight/KeysightView";

vi.mock("@/bindings", () => ({
  commands: {
    cardSearch: vi.fn(),
  },
}));

vi.mock("@/components/keysight/GraphView", () => ({
  ROOT_WHITEBOARD: "root",
  GraphView: ({
    currentWhiteboardId,
    onSearchTag,
    onOpenCard,
    onOpenAlias,
    onSelectEntity,
    selectedEntityId,
  }: {
    currentWhiteboardId?: string;
    onSearchTag?: (tag: string) => void;
    onOpenCard?: (cardId: string) => void;
    onOpenAlias?: (aliasId: string) => void;
    onSelectEntity?: (selection: { id: string; kind: "card" | "alias" | "note" | "task" | "question" | "section" } | null) => void;
    selectedEntityId?: string | null;
  }) => (
    <div>
      <div>whiteboard:{currentWhiteboardId ?? "none"}</div>
      <div>selected:{selectedEntityId ?? "none"}</div>
      <button type="button" onClick={() => onSelectEntity?.({ id: "card_1", kind: "card" })}>
        trigger select
      </button>
      <button type="button" onClick={() => onSearchTag?.("rust")}>
        trigger tag
      </button>
      <button type="button" onClick={() => onOpenCard?.("card_1")}>
        trigger related
      </button>
      <button type="button" onClick={() => onOpenAlias?.("alias_1")}>
        trigger alias
      </button>
    </div>
  ),
}));

vi.mock("@/components/keysight/sidebar/CardsList", () => ({
  CardsList: ({
    query,
    selectedCard,
    selectedAlias,
    onRevealSelection,
  }: {
    query: string;
    selectedCard?: { id: string; content: string } | null;
    selectedAlias?: { aliasId: string; aliasTitle: string; sectionTitle?: string | null } | null;
    onRevealSelection: (selection: { id: string; kind: "card" | "alias" | "note" | "task" | "question" | "section" }) => void;
  }) => (
    <div>
      <input placeholder="Search cards..." readOnly value={query} />
      {selectedCard && <div>{selectedCard.content}</div>}
      {selectedAlias && <div>{`Alias in ${selectedAlias.sectionTitle ?? selectedAlias.aliasTitle}`}</div>}
      {selectedCard && <button aria-label="Back to results">Back to results</button>}
      {(selectedCard || selectedAlias) && (
        <button
          aria-label="Reveal in graph"
          onClick={() =>
            onRevealSelection(
              selectedAlias
                ? { id: selectedAlias.aliasId, kind: "alias" }
                : { id: selectedCard!.id, kind: "card" },
            )
          }
        >
          Reveal
        </button>
      )}
    </div>
  ),
}));

vi.mock("@/components/keysight/hooks/useWhiteboardData", () => ({
  useWhiteboardData: () => ({
    cards: [
      {
        id: "card_1",
        filePath: "whiteboard/rust/card-one.md",
        title: "Card One",
        content: "Card body",
        tags: ["rust"],
        linkTo: [],
        related: [],
        understanding: "",
        source: "",
        seeAlso: [],
      },
    ],
    notes: [],
    tasks: [],
    questions: [],
    sections: [
      {
        id: "section_1",
        title: "Trait Basics",
        cardIds: ["alias_1"],
      },
    ],
    aliases: [
      {
        aliasId: "alias_1",
        cardId: "card_1",
      },
    ],
  }),
}));

function renderKeysightView() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <KeysightView />
    </QueryClientProvider>,
  );
}

describe("KeysightView", () => {
  beforeEach(() => {
    localStorage.clear();
    localStorage.setItem("keysight:sidebar:collapsed", "true");
    localStorage.setItem("keysight:sidebar:query", "");
    localStorage.setItem("keysight:sidebar:mode", "all");
  });

  it("点击 tag 时会自动展开 sidebar", async () => {
    renderKeysightView();

    expect(screen.getByRole("button", { name: /open sidebar/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /trigger tag/i }));

    await waitFor(() => {
      expect(screen.getByPlaceholderText("Search cards...")).toHaveValue("#rust");
    });
  });

  it("点击画布里的 card 时会自动展开 sidebar 并显示 preview", async () => {
    renderKeysightView();

    expect(screen.getByRole("button", { name: /open sidebar/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /trigger select/i }));

    await waitFor(() => {
      expect(screen.getByText("Card body")).toBeInTheDocument();
    });
  });

  it("点击 related 时会自动展开 sidebar 并显示 preview", async () => {
    renderKeysightView();

    expect(screen.getByRole("button", { name: /open sidebar/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /trigger related/i }));

    await waitFor(() => {
      expect(screen.getByText("Card body")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /back to results/i })).toBeInTheDocument();
    });
  });

  it("alias preview 的 Reveal 会对准并激活 alias 本身", async () => {
    renderKeysightView();

    fireEvent.click(screen.getByRole("button", { name: /trigger alias/i }));

    await waitFor(() => {
      expect(
        screen.getByText((_, node) => node?.textContent === "selected:alias_1"),
      ).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /reveal in graph/i }));

    await waitFor(() => {
      expect(
        screen.getByText((_, node) => node?.textContent === "selected:alias_1"),
      ).toBeInTheDocument();
    });
  });

  it("card preview 的 Reveal 会切到卡片所属 whiteboard 并激活卡片", async () => {
    renderKeysightView();

    expect(
      screen.getByText((_, node) => node?.textContent === "whiteboard:root"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /trigger select/i }));

    await waitFor(() => {
      expect(screen.getByText("Card body")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /reveal in graph/i }));

    await waitFor(() => {
      expect(
        screen.getByText((_, node) => node?.textContent === "whiteboard:rust"),
      ).toBeInTheDocument();
      expect(
        screen.getByText((_, node) => node?.textContent === "selected:card_1"),
      ).toBeInTheDocument();
    });
  });
});
