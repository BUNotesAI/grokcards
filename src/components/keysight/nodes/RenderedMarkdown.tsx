import { memo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface Props {
  markdown: string;
  className?: string;
}

/**
 * Markdown 渲染组件 — 使用 react-markdown + remark-gfm 支持 GitHub flavored markdown
 *
 * 支持：粗体、斜体、列表、代码块、表格、链接。
 * 不支持：wiki 链接（[[...]]）会原样显示。
 *
 * 节点 memo 化避免 card 重渲染时的 markdown 重计算。
 */
export const RenderedMarkdown = memo(function RenderedMarkdown({ markdown, className }: Props) {
  if (!markdown) return null;
  return (
    <div className={`prose prose-sm max-w-none dark:prose-invert ${className ?? ""}`}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          // 代码块使用等宽字体 + 深色背景
          code({ className, children, ...props }) {
            const isInline = !className?.startsWith("language-");
            if (isInline) {
              return (
                <code className="rounded bg-muted px-1 py-0.5 text-[0.85em] font-mono" {...props}>
                  {children}
                </code>
              );
            }
            return (
              <code
                className="block overflow-x-auto rounded bg-[#1e293b] p-2 text-[11px] leading-relaxed text-slate-100 font-mono"
                {...props}
              >
                {children}
              </code>
            );
          },
          // 列表样式贴近紧凑卡片
          ul({ children }) {
            return <ul className="my-1 list-disc pl-5 space-y-0.5">{children}</ul>;
          },
          ol({ children }) {
            return <ol className="my-1 list-decimal pl-5 space-y-0.5">{children}</ol>;
          },
          p({ children }) {
            return <p className="my-1">{children}</p>;
          },
          h1({ children }) {
            return <h4 className="my-1 font-semibold">{children}</h4>;
          },
          h2({ children }) {
            return <h4 className="my-1 font-semibold">{children}</h4>;
          },
          h3({ children }) {
            return <h5 className="my-1 font-semibold">{children}</h5>;
          },
        }}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
});
