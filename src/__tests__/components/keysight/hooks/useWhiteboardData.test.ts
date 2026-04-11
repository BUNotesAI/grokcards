import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useWhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import { commands } from "@/bindings";
import React from "react";

// mock Tauri commands
vi.mock("@/bindings", () => ({
  commands: {
    cardQueryAll: vi.fn(),
    sectionQueryAll: vi.fn(),
    noteQueryAll: vi.fn(),
    aliasQueryAll: vi.fn(),
    taskQueryAll: vi.fn(),
    questionQueryAll: vi.fn(),
    layoutQueryPositions: vi.fn(),
    syncVault: vi.fn(),
  },
}));

/** 创建测试用 QueryClient wrapper */
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

describe("useWhiteboardData", () => {
  beforeEach(() => {
    vi.mocked(commands.cardQueryAll).mockResolvedValue({
      status: "ok",
      data: [
        {
          id: "card_001",
          title: "Test Card",
          content: "body",
          filePath: "",
          tags: [],
          linkTo: [],
          related: [],
          understanding: "",
          source: "",
          seeAlso: [],
        },
      ],
    } as any);
    vi.mocked(commands.sectionQueryAll).mockResolvedValue({
      status: "ok",
      data: [],
    } as any);
    vi.mocked(commands.noteQueryAll).mockResolvedValue({
      status: "ok",
      data: [],
    } as any);
    vi.mocked(commands.aliasQueryAll).mockResolvedValue({
      status: "ok",
      data: [],
    } as any);
    vi.mocked(commands.taskQueryAll).mockResolvedValue({
      status: "ok",
      data: [],
    } as any);
    vi.mocked(commands.questionQueryAll).mockResolvedValue({
      status: "ok",
      data: [],
    } as any);
    vi.mocked(commands.layoutQueryPositions).mockResolvedValue({
      status: "ok",
      data: {},
    } as any);
  });

  it("加载完成后返回 cards 数据", async () => {
    const { result } = renderHook(() => useWhiteboardData("wb_root"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.cards).toHaveLength(1);
    expect(result.current.cards[0].id).toBe("card_001");
  });

  it("isLoading 初始为 true", () => {
    const { result } = renderHook(() => useWhiteboardData("wb_root"), {
      wrapper: createWrapper(),
    });
    expect(result.current.isLoading).toBe(true);
  });

  it("所有查询使用 whiteboardId 作参数", async () => {
    renderHook(() => useWhiteboardData("wb_root"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(commands.sectionQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.noteQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.aliasQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.taskQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.questionQueryAll).toHaveBeenCalledWith("wb_root");
      expect(commands.layoutQueryPositions).toHaveBeenCalledWith("wb_root");
    });
  });
});
