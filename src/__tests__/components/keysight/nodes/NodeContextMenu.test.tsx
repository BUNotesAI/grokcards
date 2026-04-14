import { render, screen, fireEvent, act } from "@testing-library/react";
import { vi } from "vitest";
import { NodeContextMenu } from "@/components/keysight/nodes/NodeContextMenu";
import type { NodeMenuConfig } from "@/components/keysight/nodes/NodeContextMenu";

/** 打开菜单:点击 ⋯ 按钮 */
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
      handlers: {
        copy_uuid_title: vi.fn(),
        draw_connection: vi.fn(),
        related: vi.fn(),
        create_alias: vi.fn(),
        move_to_section: vi.fn(),
        remove_from_group: vi.fn(),
        set_color: vi.fn(),
      },
    };

    beforeEach(() => {
      for (const fn of Object.values(cardConfig.handlers)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("渲染 ⋯ 按钮(aria-label='Open menu')", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      expect(screen.getByRole("button", { name: /open menu/i })).toBeInTheDocument();
    });

    it("点击 ⋯ 按钮不会冒泡到父节点", () => {
      const onParentClick = vi.fn();
      const onParentMouseDown = vi.fn();

      render(
        <div onClick={onParentClick} onMouseDown={onParentMouseDown}>
          <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />
        </div>,
      );

      const trigger = screen.getByRole("button", { name: /open menu/i });
      act(() => {
        fireEvent.mouseDown(trigger);
        fireEvent.click(trigger);
      });

      expect(onParentMouseDown).not.toHaveBeenCalled();
      expect(onParentClick).not.toHaveBeenCalled();
      expect(screen.getByText("Copy UUID + title")).toBeInTheDocument();
    });

    it("打开菜单 → 显示 Card 的基础菜单项(Copy UUID + title / Draw / Related / Create alias / Move)", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.getByText("Copy UUID + title")).toBeInTheDocument();
      expect(screen.getByText("Draw connection")).toBeInTheDocument();
      expect(screen.getByText("Related")).toBeInTheDocument();
      expect(screen.getByText("Create alias")).toBeInTheDocument();
      expect(screen.getByText("Move to Section")).toBeInTheDocument();
    });

    it("Card 菜单不含 Delete(Card 由 CLI/agent 管理生命周期,菜单不暴露 Delete)", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.queryByText("Delete")).not.toBeInTheDocument();
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

    it("点击 Copy UUID + title → copy_uuid_title 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Copy UUID + title")));
      expect(cardConfig.handlers.copy_uuid_title).toHaveBeenCalledTimes(1);
    });

    it("点击 Draw connection → draw_connection 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Draw connection")));
      expect(cardConfig.handlers.draw_connection).toHaveBeenCalledTimes(1);
    });

    it("点击 Related → related 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Related")));
      expect(cardConfig.handlers.related).toHaveBeenCalledTimes(1);
    });

    it("点击 Create alias → create_alias 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Create alias")));
      expect(cardConfig.handlers.create_alias).toHaveBeenCalledTimes(1);
    });

    it("点击 Remove from group → remove_from_group 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Remove from group")));
      expect(cardConfig.handlers.remove_from_group).toHaveBeenCalledTimes(1);
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

    it("点击 Section A → move_to_section('sec_a') 被调用", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Move to Section")));
      act(() => fireEvent.click(screen.getByText("Section A")));
      expect(cardConfig.handlers.move_to_section).toHaveBeenCalledWith("sec_a");
    });

    it("sections 为空时不显示 Move to Section", () => {
      render(
        <NodeContextMenu menu={cardConfig} sections={[]} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.queryByText("Move to Section")).not.toBeInTheDocument();
    });
  });

  describe("variant='note'", () => {
    const noteConfig: Extract<NodeMenuConfig, { kind: "note" }> = {
      kind: "note",
      handlers: {
        copy_uuid_title: vi.fn(),
        draw_connection: vi.fn(),
        edit_title: vi.fn(),
        move_to_section: vi.fn(),
        remove_from_group: vi.fn(),
        set_color: vi.fn(),
        delete: vi.fn(),
      },
    };

    beforeEach(() => {
      for (const fn of Object.values(noteConfig.handlers)) {
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

    it("点击 Edit title → edit_title 被调用", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Edit title")));
      expect(noteConfig.handlers.edit_title).toHaveBeenCalledTimes(1);
    });

    it("点击 Delete → delete 被调用", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Delete")));
      expect(noteConfig.handlers.delete).toHaveBeenCalledTimes(1);
    });

    it("渲染 7 个颜色块(aria-label='Set color <hex>')", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      expect(swatches).toHaveLength(7);
    });

    it("点击第一个颜色块 → set_color('#fff8b3') 被调用", () => {
      render(
        <NodeContextMenu menu={noteConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      act(() => fireEvent.click(swatches[0]));
      expect(noteConfig.handlers.set_color).toHaveBeenCalledWith("#fff8b3");
    });
  });

  describe("variant='alias'", () => {
    const aliasConfig: Extract<NodeMenuConfig, { kind: "alias" }> = {
      kind: "alias",
      handlers: {
        copy_uuid_title: vi.fn(),
        draw_connection: vi.fn(),
        jump_to_source_card: vi.fn(),
        move_to_section: vi.fn(),
        remove_from_group: vi.fn(),
        delete: vi.fn(),
      },
    };

    beforeEach(() => {
      for (const fn of Object.values(aliasConfig.handlers)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("打开菜单 → 显示 Alias 菜单项(新增 Copy UUID + title)", () => {
      render(
        <NodeContextMenu menu={aliasConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      expect(screen.getByText("Copy UUID + title")).toBeInTheDocument();
      expect(screen.getByText("Remove from group")).toBeInTheDocument();
      expect(screen.getByText(/jump to source card/i)).toBeInTheDocument();
      expect(screen.getByText("Draw connection")).toBeInTheDocument();
      expect(screen.getByText("Move to Section")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
    });

    it("点击 Copy UUID + title → copy_uuid_title 被调用(新 UX)", () => {
      render(
        <NodeContextMenu menu={aliasConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Copy UUID + title")));
      expect(aliasConfig.handlers.copy_uuid_title).toHaveBeenCalledTimes(1);
    });

    it("点击 Jump to source card → jump_to_source_card 被调用", () => {
      render(
        <NodeContextMenu menu={aliasConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText(/jump to source card/i)));
      expect(aliasConfig.handlers.jump_to_source_card).toHaveBeenCalledTimes(1);
    });

    it("点击 Delete → delete 被调用", () => {
      render(
        <NodeContextMenu menu={aliasConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Delete")));
      expect(aliasConfig.handlers.delete).toHaveBeenCalledTimes(1);
    });
  });

  describe("variant='question'", () => {
    const questionConfig: Extract<NodeMenuConfig, { kind: "question" }> = {
      kind: "question",
      handlers: {
        copy_uuid_title: vi.fn(),
        draw_connection: vi.fn(),
        edit_title: vi.fn(),
        move_to_section: vi.fn(),
        remove_from_group: vi.fn(),
        delete: vi.fn(),
        set_color: vi.fn(),
      },
    };

    beforeEach(() => {
      for (const fn of Object.values(questionConfig.handlers)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("打开菜单 → 显示 Question 菜单项", () => {
      render(
        <NodeContextMenu menu={questionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.getByText("Copy UUID + title")).toBeInTheDocument();
      expect(screen.getByText("Draw connection")).toBeInTheDocument();
      expect(screen.getByText("Edit title")).toBeInTheDocument();
      expect(screen.getByText("Move to Section")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
    });

    it("未在任何 section 时不显示 Remove from group", () => {
      render(
        <NodeContextMenu menu={questionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.queryByText("Remove from group")).not.toBeInTheDocument();
    });

    it("在 section 中时显示 Remove from group", () => {
      render(
        <NodeContextMenu menu={questionConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      expect(screen.getByText("Remove from group")).toBeInTheDocument();
    });

    it("点击 Copy UUID + title → copy_uuid_title 被调用", () => {
      render(
        <NodeContextMenu menu={questionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Copy UUID + title")));
      expect(questionConfig.handlers.copy_uuid_title).toHaveBeenCalledTimes(1);
    });

    it("点击 Draw connection → draw_connection 被调用", () => {
      render(
        <NodeContextMenu menu={questionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Draw connection")));
      expect(questionConfig.handlers.draw_connection).toHaveBeenCalledTimes(1);
    });

    it("点击 Edit title → edit_title 被调用", () => {
      render(
        <NodeContextMenu menu={questionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Edit title")));
      expect(questionConfig.handlers.edit_title).toHaveBeenCalledTimes(1);
    });

    it("点击 Delete → delete 被调用", () => {
      render(
        <NodeContextMenu menu={questionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Delete")));
      expect(questionConfig.handlers.delete).toHaveBeenCalledTimes(1);
    });
  });

  describe("variant='section'", () => {
    const sectionConfig: Extract<NodeMenuConfig, { kind: "section" }> = {
      kind: "section",
      handlers: {
        copy_uuid_title: vi.fn(),
        set_color: vi.fn(),
        delete: vi.fn(),
      },
    };

    beforeEach(() => {
      for (const fn of Object.values(sectionConfig.handlers)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("打开菜单 → 显示 Section 菜单项(Phase B1 新增 Copy UUID + title / Set color)", () => {
      render(
        <NodeContextMenu menu={sectionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.getByText("Copy UUID + title")).toBeInTheDocument();
      expect(screen.getByText("Delete")).toBeInTheDocument();
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      expect(swatches).toHaveLength(7);
    });

    it("点击 Copy UUID + title → copy_uuid_title 被调用(新 UX)", () => {
      render(
        <NodeContextMenu menu={sectionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Copy UUID + title")));
      expect(sectionConfig.handlers.copy_uuid_title).toHaveBeenCalledTimes(1);
    });

    it("点击颜色块 → set_color 被调用(新 UX)", () => {
      render(
        <NodeContextMenu menu={sectionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      act(() => fireEvent.click(swatches[2]));
      expect(sectionConfig.handlers.set_color).toHaveBeenCalledWith("#ffadad");
    });

    it("点击 Delete → delete 被调用", () => {
      render(
        <NodeContextMenu menu={sectionConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Delete")));
      expect(sectionConfig.handlers.delete).toHaveBeenCalledTimes(1);
    });

    it("Section 不含 Draw connection / Move to Section / Remove from group / Edit title", () => {
      render(
        <NodeContextMenu menu={sectionConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      expect(screen.queryByText("Draw connection")).not.toBeInTheDocument();
      expect(screen.queryByText("Move to Section")).not.toBeInTheDocument();
      expect(screen.queryByText("Remove from group")).not.toBeInTheDocument();
      expect(screen.queryByText("Edit title")).not.toBeInTheDocument();
    });
  });

  describe("variant='task'", () => {
    const taskConfig: Extract<NodeMenuConfig, { kind: "task" }> = {
      kind: "task",
      handlers: {
        copy_uuid_title: vi.fn(),
        move_to_section: vi.fn(),
        remove_from_group: vi.fn(),
        set_color: vi.fn(),
        delete: vi.fn(),
      },
    };

    beforeEach(() => {
      for (const fn of Object.values(taskConfig.handlers)) {
        if (typeof fn === "function") (fn as ReturnType<typeof vi.fn>).mockClear();
      }
    });

    it("打开菜单 → 显示 Task 菜单项(Copy UUID + title / Move to Section)", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.getByText("Copy UUID + title")).toBeInTheDocument();
      expect(screen.getByText("Move to Section")).toBeInTheDocument();
    });

    it("sections 为空时不显示 Move to Section", () => {
      render(<NodeContextMenu menu={taskConfig} sections={[]} currentSectionId={null} />);
      act(() => openMenu());
      expect(screen.queryByText("Move to Section")).not.toBeInTheDocument();
    });

    it("未在任何 section 时不显示 Remove from group", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.queryByText("Remove from group")).not.toBeInTheDocument();
    });

    it("在 section 中时显示 Remove from group", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      expect(screen.getByText("Remove from group")).toBeInTheDocument();
    });

    it("点击 Copy UUID + title → copy_uuid_title 被调用", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Copy UUID + title")));
      expect(taskConfig.handlers.copy_uuid_title).toHaveBeenCalledTimes(1);
    });

    it("点击 Section A → move_to_section('sec_a') 被调用", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Move to Section")));
      act(() => fireEvent.click(screen.getByText("Section A")));
      expect(taskConfig.handlers.move_to_section).toHaveBeenCalledWith("sec_a");
    });

    it("点击 Remove from group → remove_from_group 被调用", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Remove from group")));
      expect(taskConfig.handlers.remove_from_group).toHaveBeenCalledTimes(1);
    });

    it("Task 菜单**显示** Set color + Delete(Phase B2 新加的 3 项能力之 2)", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      expect(screen.getByText("Delete")).toBeInTheDocument();
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      expect(swatches).toHaveLength(7);
    });

    it("点击颜色块 → set_color 被调用", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      const swatches = screen.getAllByRole("button", { name: /set color/i });
      act(() => fireEvent.click(swatches[2]));
      expect(taskConfig.handlers.set_color).toHaveBeenCalledWith("#ffadad");
    });

    it("点击 Delete → delete 被调用", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId={null} />,
      );
      act(() => openMenu());
      act(() => fireEvent.click(screen.getByText("Delete")));
      expect(taskConfig.handlers.delete).toHaveBeenCalledTimes(1);
    });

    it("Task 菜单不含 Draw connection / Edit title / Related / Create alias / Jump to source card(白名单边界锁死,B2 剩余未接入的能力)", () => {
      render(
        <NodeContextMenu menu={taskConfig} sections={baseSections} currentSectionId="sec_a" />,
      );
      act(() => openMenu());
      expect(screen.queryByText("Draw connection")).not.toBeInTheDocument();
      expect(screen.queryByText("Edit title")).not.toBeInTheDocument();
      expect(screen.queryByText("Related")).not.toBeInTheDocument();
      expect(screen.queryByText("Create alias")).not.toBeInTheDocument();
      expect(screen.queryByText("→ Jump to source card")).not.toBeInTheDocument();
    });
  });
});
