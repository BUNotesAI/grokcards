import { useEffect, useRef, useState, type CSSProperties } from "react";
import type { QuestionEntity } from "@/bindings";
import type { LodLevel } from "@/components/keysight/types";
import {
  NodeContextMenu,
  type QuestionMenuConfig,
  type SectionListItem,
} from "./NodeContextMenu";

type QuestionEditField = "question-title" | "question-body";

interface QuestionNodeProps {
  question: QuestionEntity;
  style: CSSProperties;
  lodLevel?: LodLevel;
  selected?: boolean;
  highlighted?: boolean;
  dimmed?: boolean;
  editingField?: QuestionEditField | null;
  onStartEdit?: (id: string, field: QuestionEditField) => void;
  onCommitEdit?: (id: string, field: QuestionEditField, value: string) => void;
  onCancelEdit?: () => void;
  /** ⋯ 菜单配置；null/undefined 时不渲染菜单 */
  contextMenu?: QuestionMenuConfig | null;
  /** 可选择加入的 sections（当前白板） */
  menuSections?: SectionListItem[];
  /** 当前 question 所属 section；null 表示未在任何 section */
  currentSectionId?: string | null;
}

const STATUS_STYLES: Record<string, string> = {
  pending: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300",
  doing: "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300",
  done: "bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400",
  understood: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300",
};

export function QuestionNode({
  question,
  style,
  lodLevel = 0,
  selected = false,
  highlighted = false,
  dimmed = false,
  editingField = null,
  onStartEdit,
  onCommitEdit,
  onCancelEdit,
  contextMenu = null,
  menuSections = [],
  currentSectionId = null,
}: QuestionNodeProps) {
  const badgeClass = STATUS_STYLES[question.status] ?? STATUS_STYLES.pending;
  const ring = selected
    ? "0 0 0 2px rgba(245, 158, 11, 0.8)"
    : highlighted
      ? "0 0 0 2px rgba(16, 185, 129, 0.7)"
      : undefined;
  const opacity = dimmed ? 0.35 : 1;
  const backgroundColor = question.color ?? "#ffffff";
  const isEditingTitle = editingField === "question-title";
  const isEditingBody = editingField === "question-body";

  const [draftTitle, setDraftTitle] = useState(question.title);
  const [draftBody, setDraftBody] = useState(question.content);
  const titleInputRef = useRef<HTMLInputElement>(null);
  const bodyTextareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    setDraftTitle(question.title);
  }, [question.title]);

  useEffect(() => {
    setDraftBody(question.content);
  }, [question.content]);

  useEffect(() => {
    if (isEditingTitle) {
      titleInputRef.current?.focus();
      titleInputRef.current?.select();
    }
  }, [isEditingTitle]);

  useEffect(() => {
    if (isEditingBody) {
      bodyTextareaRef.current?.focus();
      const len = bodyTextareaRef.current?.value.length ?? 0;
      bodyTextareaRef.current?.setSelectionRange(len, len);
    }
  }, [isEditingBody]);

  if (lodLevel === 2) {
    return (
      <div
        data-entity-id={question.id}
        className="select-none rounded-md border border-amber-200 bg-amber-200/70"
        style={{ ...style, width: 320, height: 18, opacity: dimmed ? 0.2 : 0.7, boxShadow: ring }}
      />
    );
  }

  return (
      <div
        data-entity-id={question.id}
        className="select-none rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)]"
        style={{ ...style, width: 320, opacity, boxShadow: ring, backgroundColor }}
      >
      <div className="flex items-center gap-2 px-4 pt-3 pb-1">
        <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-amber-400 to-orange-500 text-[10px] font-bold text-white">
          Q
        </span>
        {isEditingTitle ? (
          <input
            ref={titleInputRef}
            value={draftTitle}
            onChange={(event) => setDraftTitle(event.target.value)}
            onBlur={(event) =>
              onCommitEdit?.(question.id, "question-title", event.currentTarget.value)
            }
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                onCommitEdit?.(question.id, "question-title", draftTitle);
              } else if (event.key === "Escape") {
                event.preventDefault();
                onCancelEdit?.();
              }
            }}
            className="h-7 flex-1 rounded-md border border-[#d9c8a5] bg-white px-2 text-sm font-semibold text-foreground outline-none"
          />
        ) : (
          <h3
            className="flex-1 truncate text-sm font-semibold text-foreground"
            onDoubleClick={() => onStartEdit?.(question.id, "question-title")}
          >
            {question.title}
          </h3>
        )}
        <span className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${badgeClass}`}>
          {question.status}
        </span>
        {contextMenu && (
          <NodeContextMenu
            menu={contextMenu}
            sections={menuSections}
            currentSectionId={currentSectionId}
          />
        )}
      </div>
      {lodLevel === 0 && (
        isEditingBody ? (
          <textarea
            ref={bodyTextareaRef}
            value={draftBody}
            onChange={(event) => setDraftBody(event.target.value)}
            onBlur={(event) =>
              onCommitEdit?.(question.id, "question-body", event.currentTarget.value)
            }
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                onCancelEdit?.();
              }
            }}
            className="mx-4 mb-3 min-h-24 w-[calc(100%-2rem)] resize-none rounded-lg border border-[#eadfc8] bg-white px-3 py-2 text-xs leading-relaxed text-[#54493c] outline-none"
          />
        ) : question.content ? (
          <p
            className="px-4 pb-3 text-xs leading-relaxed text-muted-foreground"
            onDoubleClick={() => onStartEdit?.(question.id, "question-body")}
          >
            {question.content.length > 100
              ? question.content.slice(0, 100) + "..."
              : question.content}
          </p>
        ) : (
          <div
            className="mx-4 mb-3 rounded-lg border border-dashed border-[#eadfc8] px-3 py-2 text-xs text-[#8b816f]"
            onDoubleClick={() => onStartEdit?.(question.id, "question-body")}
          >
            Double-click to add question details
          </div>
        )
      )}
    </div>
  );
}
