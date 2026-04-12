import { memo, type CSSProperties } from "react";
import type { GraphNote } from "@/bindings";
import { RenderedMarkdown } from "./RenderedMarkdown";

/** Note 行内编辑可选字段 */
type NoteEditField = "note-title" | "note-body";

interface NoteNodeProps {
  note: GraphNote;
  style: CSSProperties;
  /** 当前正在编辑哪个字段；null 表示未编辑 */
  editingField?: NoteEditField | null;
  /** 双击进入编辑模式回调 */
  onStartEdit?: (id: string, field: NoteEditField) => void;
  /** 提交编辑回调 */
  onCommitEdit?: (id: string, field: NoteEditField, value: string) => void;
  /** 取消编辑回调（Escape） */
  onCancelEdit?: () => void;
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
export const NoteNode = memo(function NoteNode({
  note,
  style,
  editingField = null,
  onStartEdit,
  onCommitEdit,
  onCancelEdit,
}: NoteNodeProps) {
  const isEditingTitle = editingField === "note-title";
  const isEditingBody = editingField === "note-body";
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
          onDoubleClick={(e) => {
            e.stopPropagation();
            onStartEdit?.(note.id, "note-title");
          }}
        >
          {isEditingTitle ? (
            <input
              autoFocus
              defaultValue={note.title}
              onMouseDown={(e) => e.stopPropagation()}
              onClick={(e) => e.stopPropagation()}
              onBlur={(e) => onCommitEdit?.(note.id, "note-title", e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  e.currentTarget.blur();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  onCancelEdit?.();
                }
              }}
              style={{
                width: "100%",
                fontSize: 14,
                fontWeight: 500,
                color: "#2C2C2A",
                border: "1.5px solid #1D9E75",
                borderRadius: 4,
                padding: "2px 6px",
                outline: "none",
                background: "#fff",
                fontFamily: "inherit",
              }}
            />
          ) : (
            note.title
          )}
        </div>
      </div>

      {/* 正文（markdown）— 双击进入编辑模式；空内容也允许编辑 */}
      {(note.content || isEditingBody) && (
        <div
          onDoubleClick={(e) => {
            e.stopPropagation();
            onStartEdit?.(note.id, "note-body");
          }}
        >
          {isEditingBody ? (
            <textarea
              ref={(el) => {
                if (!el) return;
                el.style.height = "auto";
                el.style.height = `${el.scrollHeight}px`;
                el.focus();
                el.setSelectionRange(el.value.length, el.value.length);
              }}
              defaultValue={note.content}
              onInput={(e) => {
                const el = e.currentTarget;
                el.style.height = "auto";
                el.style.height = `${el.scrollHeight}px`;
              }}
              onMouseDown={(e) => e.stopPropagation()}
              onClick={(e) => e.stopPropagation()}
              onBlur={(e) => onCommitEdit?.(note.id, "note-body", e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  onCancelEdit?.();
                } else if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
                  e.preventDefault();
                  e.currentTarget.blur();
                }
              }}
              style={{
                width: "100%",
                minHeight: 60,
                fontSize: 13,
                color: "#4a4a47",
                lineHeight: 1.6,
                border: "1.5px solid #1D9E75",
                borderRadius: 4,
                padding: "6px 8px",
                outline: "none",
                background: "#fff",
                fontFamily: "inherit",
                resize: "none",
                overflow: "hidden",
              }}
            />
          ) : (
            <RenderedMarkdown markdown={note.content} variant="body" />
          )}
        </div>
      )}
    </div>
  );
});
