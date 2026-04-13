import { fireEvent, render, screen } from "@testing-library/react";
import { RenderedMarkdown } from "@/components/keysight/nodes/RenderedMarkdown";

describe("RenderedMarkdown toggle list", () => {
  it("?>> 标记渲染为默认折叠的 toggle block，点击后展开内容", () => {
    render(
      <RenderedMarkdown
        markdown={"?>> 折叠标题\n这里是详细内容\n?<<"}
        variant="body"
      />,
    );

    expect(screen.getByText("折叠标题")).toBeInTheDocument();
    expect(screen.queryByText("这里是详细内容")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /折叠标题/i }));

    expect(screen.getByText("这里是详细内容")).toBeInTheDocument();
  });

  it("无序列表保留 bullet 样式和正常缩进", () => {
    const { container } = render(
      <RenderedMarkdown
        markdown={"- 第一项\n- 第二项"}
        variant="body"
      />,
    );

    const list = container.querySelector("ul");
    expect(list).not.toBeNull();
    expect(list).toHaveStyle({
      listStyleType: "disc",
      listStylePosition: "outside",
    });
    expect(screen.getByText("第一项")).toBeInTheDocument();
    expect(screen.getByText("第二项")).toBeInTheDocument();
  });

  it("markdown table 渲染为表格结构", () => {
    const { container } = render(
      <RenderedMarkdown
        markdown={"| 名称 | 含义 |\n| --- | --- |\n| 函数 | 运行时 |\n| 宏 | 编译时 |"}
        variant="body"
      />,
    );

    const table = container.querySelector("table");
    expect(table).not.toBeNull();
    expect(screen.getByRole("columnheader", { name: "名称" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "含义" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "函数" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "编译时" })).toBeInTheDocument();
  });
});
