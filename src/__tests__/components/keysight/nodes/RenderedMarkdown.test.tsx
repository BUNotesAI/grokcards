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
});
