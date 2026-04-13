import { render, screen, fireEvent, act, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { vi } from "vitest";
import { GraphView } from "@/components/keysight/GraphView";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";

const mockEntityConnect = vi.fn();
const mockSectionCreate = vi.fn();
const mockLayoutSetPosition = vi.fn();
const mockSectionDelete = vi.fn();

function makeWhiteboardData(): WhiteboardData {
  return {
    cards: [
      {
        id: "card_a",
        title: "Source Card",
        content: "alpha body",
        filePath: "",
        tags: [],
        linkTo: [],
        related: ["card_c"],
        seeAlso: [],
        understanding: "",
        source: "",
        mtime: null,
      },
      {
        id: "card_b",
        title: "Target Card",
        content: "beta body",
        filePath: "",
        tags: [],
        linkTo: [],
        related: [],
        seeAlso: [],
        understanding: "",
        source: "",
        mtime: null,
      },
      {
        id: "card_c",
        title: "Already Related",
        content: "gamma body",
        filePath: "",
        tags: [],
        linkTo: [],
        related: [],
        seeAlso: [],
        understanding: "",
        source: "",
        mtime: null,
      },
      {
        id: "card_d",
        title: "Reverse Related",
        content: "delta body",
        filePath: "",
        tags: [],
        linkTo: [],
        related: ["card_a"],
        seeAlso: [],
        understanding: "",
        source: "",
        mtime: null,
      },
    ],
    sections: [],
    notes: [],
    aliases: [],
    tasks: [],
    questions: [],
    positions: {
      card_a: { x: 120, y: 80 },
      card_b: { x: 760, y: 80 },
      card_c: { x: 120, y: 360 },
      card_d: { x: 760, y: 360 },
    },
    isLoading: false,
    syncVault: { mutate: vi.fn(), isPending: false },
  };
}

const mockState = vi.hoisted(() => ({
  viewport: {
    state: { zoom: 1, panX: 0, panY: 0 },
    lodLevel: 0,
    needsFit: false,
    handlers: {
      onMouseDown: vi.fn(),
      onMouseMove: vi.fn(),
      onMouseUp: vi.fn(),
      onWheel: vi.fn(),
    },
    actions: {
      zoomIn: vi.fn(),
      zoomOut: vi.fn(),
      resetView: vi.fn(),
      fitToContent: vi.fn(),
      centerOn: vi.fn(),
    },
  },
  whiteboardData: makeWhiteboardData(),
}));

vi.mock("@/bindings", () => ({
  commands: {
    entityConnect: (...args: unknown[]) => mockEntityConnect(...args),
    sectionCreate: (...args: unknown[]) => mockSectionCreate(...args),
    layoutSetPosition: (...args: unknown[]) => mockLayoutSetPosition(...args),
    sectionDelete: (...args: unknown[]) => mockSectionDelete(...args),
  },
}));

vi.mock("@/components/keysight/useViewport", () => ({
  useViewport: () => mockState.viewport,
}));

vi.mock("@/components/keysight/hooks/useContainerSize", () => ({
  useContainerSize: () => ({ width: 1280, height: 720 }),
}));

vi.mock("@/components/keysight/hooks/useWhiteboardData", () => ({
  useWhiteboardData: () => mockState.whiteboardData,
  useWhiteboardList: () => ({ data: [], isLoading: false }),
}));

vi.mock("@/components/keysight/hooks/useVisibleEntities", () => ({
  useVisibleEntities: (entities: unknown[]) => entities,
}));

function renderGraphView() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <GraphView
        currentWhiteboardId="wb_root"
        onWhiteboardChange={vi.fn()}
      />
    </QueryClientProvider>,
  );
}

describe("GraphView", () => {
  beforeEach(() => {
    mockEntityConnect.mockReset();
    mockEntityConnect.mockResolvedValue({ status: "ok", data: null });
    mockSectionCreate.mockReset();
    mockSectionCreate.mockResolvedValue({ status: "ok", data: { id: "sec_new001" } });
    mockLayoutSetPosition.mockReset();
    mockLayoutSetPosition.mockResolvedValue({ status: "ok", data: null });
    mockSectionDelete.mockReset();
    mockSectionDelete.mockResolvedValue({ status: "ok", data: null });
    mockState.whiteboardData = makeWhiteboardData();
  });

  it("点击 Related 打开 picker，排除自己和已关联卡片", () => {
    renderGraphView();

    act(() => {
      fireEvent.click(screen.getAllByRole("button", { name: /open menu/i })[0]!);
    });

    act(() => {
      fireEvent.click(screen.getByText("Related"));
    });

    const picker = screen.getByTestId("related-picker");
    expect(screen.getByPlaceholderText("Search cards...")).toBeInTheDocument();
    expect(within(picker).getByTestId("related-picker-item-card_b")).toBeInTheDocument();
    expect(within(picker).queryByTestId("related-picker-item-card_a")).not.toBeInTheDocument();
    expect(within(picker).queryByTestId("related-picker-item-card_c")).not.toBeInTheDocument();
    expect(within(picker).queryByTestId("related-picker-item-card_d")).not.toBeInTheDocument();
    expect(screen.getByTestId("graph-viewport")).toHaveStyle({ cursor: "grab" });
  });

  it("在 picker 里点击候选项后直接创建 Related 并关闭 picker", async () => {
    renderGraphView();

    act(() => {
      fireEvent.click(screen.getAllByRole("button", { name: /open menu/i })[0]!);
    });
    act(() => {
      fireEvent.click(screen.getByText("Related"));
    });

    act(() => {
      fireEvent.click(screen.getByTestId("related-picker-item-card_b"));
    });

    await waitFor(() => {
      expect(mockEntityConnect).toHaveBeenCalledWith(
        "card_a",
        "card_b",
        "Related",
        null,
        null,
      );
    });
    await waitFor(() => {
      expect(screen.queryByPlaceholderText("Search cards...")).not.toBeInTheDocument();
    });
  });

  it("展开卡片时当前卡片突出，其他卡片和背景暗下来", () => {
    renderGraphView();

    act(() => {
      fireEvent.click(screen.getAllByTitle("Expand")[0]!);
    });

    expect(screen.getByTestId("graph-viewport")).toHaveStyle({
      backgroundColor: "rgb(44, 44, 46)",
    });
    expect(document.querySelector('[data-entity-id="card_a"]')?.parentElement).toHaveStyle({
      opacity: "1",
    });
    expect(document.querySelector('[data-entity-id="card_b"]')?.parentElement).toHaveStyle({
      opacity: "0.15",
    });
  });

  it("展开卡片后按 Escape 恢复到普通未展开模式", () => {
    renderGraphView();

    act(() => {
      fireEvent.click(screen.getAllByTitle("Expand")[0]!);
    });

    expect(screen.getByText(/这是卡片的正文内容|alpha body/)).toBeInTheDocument();
    expect(screen.getByTestId("graph-viewport")).toHaveStyle({
      backgroundColor: "rgb(44, 44, 46)",
    });

    act(() => {
      fireEvent.keyDown(window, { key: "Escape" });
    });

    expect(screen.queryByText(/这是卡片的正文内容|alpha body/)).not.toBeInTheDocument();
    expect(screen.getByTestId("graph-viewport")).not.toHaveStyle({
      backgroundColor: "rgb(44, 44, 46)",
    });
    expect(document.querySelector('[data-entity-id="card_b"]')?.parentElement).toHaveStyle({
      opacity: "1",
    });
  });

  it("创建 section 时应避开当前视口内已有 section，不能直接嵌在里面", async () => {
    const randomSpy = vi.spyOn(Math, "random").mockReturnValue(0.5);
    mockState.whiteboardData = {
      ...makeWhiteboardData(),
      cards: [],
      notes: [{ id: "note_in_sec", title: "Inside", content: "body" }],
      sections: [{ id: "sec_existing", title: "Existing Section", cardIds: ["note_in_sec"] }],
      aliases: [],
      tasks: [],
      questions: [],
      positions: {
        note_in_sec: { x: 200, y: 200 },
      },
    };

    renderGraphView();

    act(() => {
      fireEvent.click(screen.getByRole("button", { name: /create section/i }));
    });

    await waitFor(() => {
      expect(mockLayoutSetPosition).toHaveBeenCalledWith("wb_root", "sec_new001", expect.any(Number), expect.any(Number));
    });

    const lastCall = mockLayoutSetPosition.mock.calls[mockLayoutSetPosition.mock.calls.length - 1]!;
    const [, , x, y] = lastCall;
    const existing = { left: 160, top: 160, right: 760, bottom: 460 };
    const created = { left: x as number, top: y as number, right: (x as number) + 400, bottom: (y as number) + 300 };
    const overlaps =
      created.left < existing.right &&
      created.right > existing.left &&
      created.top < existing.bottom &&
      created.bottom > existing.top;

    expect(overlaps).toBe(false);

    randomSpy.mockRestore();
  });

  it("section 节点应提供菜单并允许删除", async () => {
    mockState.whiteboardData = {
      ...makeWhiteboardData(),
      cards: [],
      notes: [],
      sections: [{ id: "sec_existing", title: "Existing Section", cardIds: [] }],
      aliases: [],
      tasks: [],
      questions: [],
      positions: {
        sec_existing: { x: 320, y: 220 },
      },
    };

    renderGraphView();

    act(() => {
      fireEvent.click(screen.getByRole("button", { name: /open menu/i }));
    });

    act(() => {
      fireEvent.click(screen.getByText(/delete section|delete/i));
    });

    await waitFor(() => {
      expect(mockSectionDelete).toHaveBeenCalledWith("sec_existing");
    });
  });
});
