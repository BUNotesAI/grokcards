import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { CardsList } from "@/components/keysight/sidebar/CardsList";
import type { AtomicCard } from "@/bindings";
import { vi } from "vitest";

const mockCardSearch = vi.fn();

vi.mock("@/bindings", () => ({
  commands: {
    cardSearch: (...args: unknown[]) => mockCardSearch(...args),
  },
}));

const cards: AtomicCard[] = [
  {
    id: "card_rust",
    filePath: "whiteboard/rust.md",
    title: "**Rust** Trait",
    content: "Rust body content",
    tags: ["rust", "trait"],
    linkTo: [],
    related: [],
    understanding: "trait object",
    source: "",
    seeAlso: [],
  },
  {
    id: "card_tauri",
    filePath: "whiteboard/tauri.md",
    title: "Tauri Command",
    content: "Tauri body content",
    tags: ["tauri"],
    linkTo: [],
    related: [],
    understanding: "invoke",
    source: "",
    seeAlso: [],
  },
];

function renderCardsList(inputCards: AtomicCard[] = cards) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  function Wrapper() {
    const [query, setQuery] = useState("");
    return (
      <CardsList
        cards={inputCards}
        sections={[]}
        aliasRefsByTargetId={{}}
        selectedCard={null}
        selectedAlias={null}
        query={query}
        mode="all"
        onQueryChange={setQuery}
        onSelectCard={vi.fn()}
        onSelectAlias={vi.fn()}
        onRevealSelection={vi.fn()}
        onSelectTag={vi.fn()}
      />
    );
  }

  return render(
    <QueryClientProvider client={queryClient}>
      <Wrapper />
    </QueryClientProvider>,
  );
}

function renderCardsListWithSelectedCard(inputCards: AtomicCard[] = cards) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  function Wrapper() {
    const [query, setQuery] = useState("");
    return (
      <CardsList
        cards={inputCards}
        sections={[]}
        aliasRefsByTargetId={{}}
        selectedCard={inputCards[0] ?? null}
        selectedAlias={null}
        previewRequestKey={1}
        query={query}
          mode="all"
          onQueryChange={setQuery}
          onSelectCard={vi.fn()}
          onSelectAlias={vi.fn()}
          onRevealSelection={vi.fn()}
        onSelectTag={vi.fn()}
      />
    );
  }

  return render(
    <QueryClientProvider client={queryClient}>
      <Wrapper />
    </QueryClientProvider>,
  );
}

function renderCardsListWithPreviewTrigger(inputCards: AtomicCard[] = cards) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  function Wrapper() {
    const [query, setQuery] = useState("");
    const [previewRequestKey, setPreviewRequestKey] = useState(1);
    return (
      <>
        <button type="button" onClick={() => setPreviewRequestKey((prev) => prev + 1)}>
          reopen preview
        </button>
        <CardsList
          cards={inputCards}
          sections={[]}
          aliasRefsByTargetId={{}}
          selectedCard={inputCards[0] ?? null}
          selectedAlias={null}
          previewRequestKey={previewRequestKey}
          query={query}
          mode="all"
          onQueryChange={setQuery}
          onSelectCard={vi.fn()}
          onSelectAlias={vi.fn()}
          onRevealSelection={vi.fn()}
          onSelectTag={vi.fn()}
        />
      </>
    );
  }

  return render(
    <QueryClientProvider client={queryClient}>
      <Wrapper />
    </QueryClientProvider>,
  );
}

describe("CardsList", () => {
  beforeEach(() => {
    mockCardSearch.mockReset();
    mockCardSearch.mockResolvedValue({
      status: "ok",
      data: [{ ...cards[0], id: "card_search_rust" }],
    });
  });

  it("搜索结果点击后在侧边栏原地弹出卡片详情", async () => {
    renderCardsList();

    fireEvent.change(screen.getByPlaceholderText("Search cards..."), {
      target: { value: "rust" },
    });

    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 350));
    });

    await waitFor(() => {
      expect(mockCardSearch).toHaveBeenCalledWith("rust");
      expect(screen.getByRole("button", { name: /Rust Trait/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /Rust Trait/i }));

    expect(screen.getByText("Rust body content")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /back to results/i })).toBeInTheDocument();
  });

  it("已有 preview 时切到 tag 搜索，应显示 tag 结果而不是继续停留在 preview", async () => {
    renderCardsListWithSelectedCard();

    expect(screen.getByText("Rust body content")).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText("Search cards..."), {
      target: { value: "#tauri" },
    });

    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 350));
    });

    expect(screen.queryByText("Rust body content")).not.toBeInTheDocument();
    expect(screen.getByText("Tauri Command")).toBeInTheDocument();
    expect(screen.getByText("1 search hits")).toBeInTheDocument();
  });

  it("列表标题保留 markdown 渲染", async () => {
    renderCardsList();

    fireEvent.change(screen.getByPlaceholderText("Search cards..."), {
      target: { value: "rust" },
    });

    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 350));
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Rust Trait/i })).toBeInTheDocument();
    });

    expect(document.querySelectorAll("strong").length).toBeGreaterThan(0);
  });

  it("同一张卡在 tag 搜索后再次触发 preview 请求时，会重新打开 related 预览", async () => {
    renderCardsListWithPreviewTrigger();

    expect(screen.getByText("Rust body content")).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText("Search cards..."), {
      target: { value: "#rust" },
    });

    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 350));
    });

    expect(screen.queryByText("Rust body content")).not.toBeInTheDocument();
    expect(screen.getByText("1 search hits")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /reopen preview/i }));

    await waitFor(() => {
      expect(screen.getByText("Rust body content")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /back to results/i })).toBeInTheDocument();
    });
  });

  it("preview 中显示 card 对应 alias 所在的 section 名", () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <CardsList
          cards={cards}
          sections={[]}
          aliasRefsByTargetId={{
            card_rust: [
              {
                aliasId: "alias_1",
                aliasTitle: "Trait Basics",
                cardId: "card_rust",
                sectionId: "section_1",
                sectionTitle: "Trait Basics",
              },
              {
                aliasId: "alias_2",
                aliasTitle: "Advanced Rust",
                cardId: "card_rust",
                sectionId: "section_2",
                sectionTitle: "Advanced Rust",
              },
            ],
          }}
          selectedCard={cards[0]}
          selectedAlias={null}
          previewRequestKey={1}
          query=""
          mode="all"
          onQueryChange={vi.fn()}
          onSelectCard={vi.fn()}
          onSelectAlias={vi.fn()}
          onRevealSelection={vi.fn()}
          onSelectTag={vi.fn()}
        />
      </QueryClientProvider>,
    );

    expect(screen.getByText("Aliases (2)")).toBeInTheDocument();
    expect(screen.getByText("Trait Basics")).toBeInTheDocument();
    expect(screen.getByText("Advanced Rust")).toBeInTheDocument();
  });

  it("点击 alias 行后切到 alias preview，Reveal 走 alias selection", () => {
    const onSelectAlias = vi.fn();
    const onRevealSelection = vi.fn();
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <CardsList
          cards={cards}
          sections={[]}
          aliasRefsByTargetId={{
            card_rust: [
              {
                aliasId: "alias_1",
                aliasTitle: "Trait Basics",
                cardId: "card_rust",
                sectionId: "section_1",
                sectionTitle: "Trait Basics",
              },
            ],
          }}
          selectedCard={cards[0]}
          selectedAlias={null}
          previewRequestKey={1}
          query=""
          mode="all"
          onQueryChange={vi.fn()}
          onSelectCard={vi.fn()}
          onSelectAlias={onSelectAlias}
          onRevealSelection={onRevealSelection}
          onSelectTag={vi.fn()}
        />
      </QueryClientProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: /Trait Basics/i }));

    expect(screen.getByText("Alias in Trait Basics")).toBeInTheDocument();
    expect(onSelectAlias).toHaveBeenCalledWith("alias_1");

    fireEvent.click(screen.getByRole("button", { name: /reveal in graph/i }));

    expect(onRevealSelection).toHaveBeenCalledWith({ id: "alias_1", kind: "alias" });
  });
});
