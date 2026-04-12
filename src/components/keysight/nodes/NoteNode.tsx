import { memo, type CSSProperties } from "react";
import type { GraphNote } from "@/bindings";
import { RenderedMarkdown } from "./RenderedMarkdown";

interface NoteNodeProps {
  note: GraphNote;
  style: CSSProperties;
}

/**
 * 笔记节点 — 对齐旧 Obsidian `.ks-graph-note` 样式
 *
 * - 宽度 520px（NOTE_W），最小高度 60px
 * - 背景淡绿 `#E1F5EE` 40% 混合（便签风格）
 * - 圆角 12px，无边框
 * - 右上角 "NOTE" badge（#C2E8D8 底，#0F6E56 文字）
 * - 左侧绿色 pen 图标 `#1D9E75`
 * - 标题 + markdown 渲染的正文
 */
export const NoteNode = memo(function NoteNode({ note, style }: NoteNodeProps) {
  return (
    <div
      data-entity-id={note.id}
      style={{
        ...style,
        position: "relative",
        width: 520,
        minHeight: 60,
        padding: "14px 16px",
        borderRadius: 12,
        background: "#ecf7f2", // #E1F5EE 40% mix on white
        userSelect: "none",
        cursor: "grab",
      }}
    >
      {/* 右上角 NOTE badge */}
      <span
        style={{
          position: "absolute",
          top: 8,
          right: 10,
          background: "#C2E8D8",
          color: "#0F6E56",
          fontFamily: "SFMono-Regular, Menlo, Monaco, Consolas, monospace",
          fontSize: 10,
          fontWeight: 600,
          padding: "1px 6px",
          borderRadius: 4,
          letterSpacing: 0.5,
          lineHeight: 1.4,
          pointerEvents: "none",
        }}
      >
        NOTE
      </span>

      {/* 标题栏 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          marginBottom: 6,
          paddingRight: 48, // 给 badge 留空间
        }}
      >
        {/* 绿色 pen 图标 */}
        <svg
          width={16}
          height={16}
          viewBox="0 0 24 24"
          fill="none"
          stroke="#1D9E75"
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{ flexShrink: 0 }}
        >
          <path d="M12 20h9" />
          <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
        </svg>
        <div
          style={{
            flex: 1,
            minWidth: 0,
            fontWeight: 500,
            fontSize: 14,
            color: "#2C2C2A",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {note.title}
        </div>
      </div>

      {/* 正文（markdown） */}
      {note.content && <RenderedMarkdown markdown={note.content} variant="body" />}
    </div>
  );
});
