import { render, screen, fireEvent, act } from "@testing-library/react";
import { vi } from "vitest";
import { NodeContextMenu } from "@/components/keysight/nodes/NodeContextMenu";
import type { NodeMenuConfig } from "@/components/keysight/nodes/NodeContextMenu";

/** 打开菜单：点击 ⋯ 按钮 + act flush 动画 / portal */
function openMenu() {
  fireEvent.click(screen.getByRole("button", { name: /open menu/i }));
}

describe("NodeContextMenu", () => {
  const baseSections = [
    { id: "sec_a", title: "Section A" },
    { id: "sec_b", title: "Section B" },
  ];

  describe("variant='card'", () => {
    const cardConfig: Extract<NodeMenuConfig, { kind: "card" }> = {
      kind: "card",
      onCopyTitle: vi.fn(),
      onDrawConnection: vi.fn(),
      onRelated: vi.fn(),
      onCreateAlias: vi.fn(),
      onMoveToSection: vi.fn(),
      onRemoveFromGroup: vi.fn(),
    };

    beforeEach(() => {
      for (const fn of Object.values(cardConfig)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("渲染 ⋯ 按钮（aria-label='Open menu'）", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      expect(screen.getByRole("button", { name: /open menu/i })).toBeInTheDocument();
    });

    it("打开菜单 → 显示 Card 的 5 个基础菜单项", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.getByText("Copy title")).toBeInTheDocument();
      expect(screen.getByText("Draw connection")).toBeInTheDocument();
      expect(screen.getByText("Related")).toBeInTheDocument();
      expect(screen.getByText("Create alias")).toBeInTheDocument();
      expect(screen.getByText("Move to Section")).toBeInTheDocument();
    });

    it("未在任何 section 时不显示 Remove from group", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.queryByText("Remove from group")).not.toBeInTheDocument();
    });

    it("在 section 中时显示 Remove from group", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      expect(screen.getByText("Remove from group")).toBeInTheDocument();
    });

    it("点击 Copy title → onCopyTitle 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Copy title")));
      expect(cardConfig.onCopyTitle).toHaveBeenCalledTimes(1);
    });

    it("点击 Draw connection → onDrawConnection 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Draw connection")));
      expect(cardConfig.onDrawConnection).toHaveBeenCalledTimes(1);
    });

    it("点击 Related → onRelated 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Related")));
      expect(cardConfig.onRelated).toHaveBeenCalledTimes(1);
    });

    it("点击 Create alias → onCreateAlias 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Create alias")));
      expect(cardConfig.onCreateAlias).toHaveBeenCalledTimes(1);
    });

    it("点击 Remove from group → onRemoveFromGroup 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Remove from group")));
      expect(cardConfig.onRemoveFromGroup).toHaveBeenCalledTimes(1);
    });

    it("Move to Section 子菜单包含 Section A / Section B", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Move to Section")));
      expect(screen.getByText("Section A")).toBeInTheDocument();
      expect(screen.getByText("Section B")).toBeInTheDocument();
    });

    it("点击 Section A → onMoveToSection('sec_a') 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Move to Section")));
      act(() => fireEvent.click(screen.getByText("Section A")));
      expect(cardConfig.onMoveToSection).toHaveBeenCalledWith("sec_a");
    });
  });

  describe("variant='note'", () => {
    const noteConfig: Extract<NodeMenuConfig, { kind: "note" }> = {
      kind: "note",
      onCopyUuidTitle: vi.fn(),
      onDrawConnection: vi.fn(),
      onEditTitle: vi.fn(),
      onMoveToSection: vi.fn(),
      onRemoveFromGroup: vi.fn(),
      onDelete: vi.fn(),
      onSetColor: vi.fn(),
    };

    beforeEach(() => {
      for (const fn of Object.values(noteConfig)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("打开菜单 → 显示 Note 菜单项", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.getByText("Copy UUID + title")).toBeInTheDocument();
      expect(screen.getByText("Draw connection")).toBeInTheDocument();
      expect(screen.getByText("Edit title")).toBeInTheDocument();
      expect(screen.getByText("Move to Section")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
    });

    it("点击 Edit title → onEditTitle 被调用", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Edit title")));
      expect(noteConfig.onEditTitle).toHaveBeenCalledTimes(1);
    });

    it("点击 Delete → onDelete 被调用", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Delete")));
      expect(noteConfig.onDelete).toHaveBeenCalledTimes(1);
    });

    it("渲染 7 个颜色块（aria-label='Set color <hex>'）", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      expect(swatches).toHaveLength(7);
    });

    it("点击第一个颜色块 → onSetColor('#fff8b3') 被调用", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      act(() => fireEvent.click(swatches[0]));
      expect(noteConfig.onSetColor).toHaveBeenCalledWith("#fff8b3");
    });
  });

  describe("variant='alias'", () => {
    const aliasConfig: Extract<NodeMenuConfig, { kind: "alias" }> = {
      kind: "alias",
      onJumpToSourceCard: vi.fn(),
      onDrawConnection: vi.fn(),
      onMoveToSection: vi.fn(),
      onRemoveFromGroup: vi.fn(),
      onDelete: vi.fn(),
    };

    beforeEach(() => {
      for (const fn of Object.values(aliasConfig)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("打开菜单 → 显示 Alias 菜单项", () => {
      render(
        <NodeContextMenu menu={aliasConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      expect(screen.getByText("Remove from group")).toBeInTheDocument();
      expect(screen.getByText(/jump to source card/i)).toBeInTheDocument();
      expect(screen.getByText("Draw connection")).toBeInTheDocument();
      expect(screen.getByText("Move to Section")).toBeInTheDocument();
      expect(screen.getByText("Delete alias")).toBeInTheDocument();
    });

    it("点击 Jump to source card → onJumpToSourceCard 被调用", () => {
      render(
        <NodeContextMenu menu={aliasConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText(/jump to source card/i)));
      expect(aliasConfig.onJumpToSourceCard).toHaveBeenCalledTimes(1);
    });

    it("点击 Delete alias → onDelete 被调用", () => {
      render(
        <NodeContextMenu menu={aliasConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Delete alias")));
      expect(aliasConfig.onDelete).toHaveBeenCalledTimes(1);
    });
  });
});
