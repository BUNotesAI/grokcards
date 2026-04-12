import { memo, type CSSProperties } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface Props {
  markdown: string;
  /** 风格变体：understanding 用绿色边框+琥珀色背景，title 单行紧凑，body 适合卡片正文 */
  variant?: "title" | "understanding" | "body";
}

/**
 * Markdown 渲染组件 — 用 react-markdown + remark-gfm
 *
 * 颜色和字号对齐旧 Obsidian 插件的 keysight-understanding / ks-graph-card-title 样式。
 * **加粗** 固定为 rgb(229, 84, 3)（vivid orange），强调重点。
 * 代码块：inline 浅灰底，block 深色 #1e1e1e 背景 + #d4d4d4 文字。
 */
export const RenderedMarkdown = memo(function RenderedMarkdown({
  markdown,
  variant = "body",
}: Props) {
  if (!markdown) return null;

  // 每个 variant 的基础容器样式
  const containerStyle: CSSProperties = (() => {
    switch (variant) {
      case "title":
        return { fontSize: 14, fontWeight: 600, color: "#2C2C2A", lineHeight: 1.45, letterSpacing: -0.1 };
      case "understanding":
        return { fontSize: 13.5, color: "#5F5E5A", lineHeight: 1.7 };
      default:
        return { fontSize: 13, color: "#4a4a47", lineHeight: 1.6 };
    }
  })();

  return (
    <div className="ks-rendered" style={containerStyle}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          p({ children }) {
            return <p style={{ margin: variant === "title" ? 0 : "2px 0" }}>{children}</p>;
          },
          strong({ children }) {
            return (
              <strong style={{ color: "rgb(229, 84, 3)", fontWeight: 700 }}>{children}</strong>
            );
          },
          em({ children }) {
            return <em style={{ color: "#5F5E5A" }}>{children}</em>;
          },
          ul({ children }) {
            return (
              <ul style={{ margin: "4px 0", paddingLeft: 0, listStylePosition: "inside" }}>
                {children}
              </ul>
            );
          },
          ol({ children }) {
            return (
              <ol style={{ margin: "4px 0", paddingLeft: 0, listStylePosition: "inside" }}>
                {children}
              </ol>
            );
          },
          li({ children }) {
            return <li style={{ margin: "1px 0" }}>{children}</li>;
          },
          code({ className, children, ...props }) {
            const isBlock = className?.startsWith("language-");
            if (isBlock) {
              return (
                <code
                  style={{
                    display: "block",
                    background: "transparent",
                    color: "#d4d4d4",
                    padding: 0,
                    fontSize: 12.5,
                    lineHeight: 1.7,
                    fontFamily:
                      "SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace",
                  }}
                  {...props}
                >
                  {children}
                </code>
              );
            }
            return (
              <code
                style={{
                  background: "rgba(0, 0, 0, 0.055)",
                  padding: "1px 4px",
                  borderRadius: 3,
                  fontSize: 12,
                  fontFamily:
                    "SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace",
                }}
                {...props}
              >
                {children}
              </code>
            );
          },
          pre({ children }) {
            return (
              <pre
                style={{
                  margin: "4px 0",
                  padding: "16px 18px",
                  borderRadius: 10,
                  background: "#1e1e1e",
                  overflowX: "auto",
                  fontSize: 12.5,
                  lineHeight: 1.7,
                }}
              >
                {children}
              </pre>
            );
          },
          h1({ children }) {
            return <div style={{ margin: "4px 0", fontWeight: 600, fontSize: 14 }}>{children}</div>;
          },
          h2({ children }) {
            return <div style={{ margin: "4px 0", fontWeight: 600, fontSize: 13 }}>{children}</div>;
          },
          h3({ children }) {
            return <div style={{ margin: "4px 0", fontWeight: 600, fontSize: 12 }}>{children}</div>;
          },
          a({ children, href }) {
            return (
              <a
                href={href}
                style={{ color: "#1D9E75", textDecoration: "none" }}
                onClick={(e) => e.preventDefault()}
              >
                {children}
              </a>
            );
          },
        }}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
});
