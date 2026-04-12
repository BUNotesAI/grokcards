import type { CSSProperties } from "react";
import type { WhiteboardSummary } from "@/bindings";

interface WhiteboardNodeProps {
  summary: WhiteboardSummary;
  onNavigate: (whiteboardId: string) => void;
  style: CSSProperties;
}

/** 非零统计项格式化，零值字段不出现在结果中 */
function formatStats(s: WhiteboardSummary): string {
  const parts: string[] = [];
  if (s.cards > 0) parts.push(`${s.cards} card${s.cards !== 1 ? "s" : ""}`);
  if (s.notes > 0) parts.push(`${s.notes} note${s.notes !== 1 ? "s" : ""}`);
  if (s.sections > 0) parts.push(`${s.sections} section${s.sections !== 1 ? "s" : ""}`);
  if (s.aliases > 0) parts.push(`${s.aliases} alias${s.aliases !== 1 ? "es" : ""}`);
  if (s.tasks > 0) parts.push(`${s.tasks} task${s.tasks !== 1 ? "s" : ""}`);
  if (s.questions > 0) parts.push(`${s.questions} question${s.questions !== 1 ? "s" : ""}`);
  return parts.join(" · ");
}

/**
 * 子白板预览卡 — Clean Elevated 风格
 *
 * 白底圆角卡片 + 紫色渐变竖条 + 统计行 + 进入提示。
 * 宽度 320px，点击整卡导航进入子白板。
 */
export function WhiteboardNode({ summary, onNavigate, style }: WhiteboardNodeProps) {
  const stats = formatStats(summary);
  const isEmpty = stats.length === 0;

  return (
    <div
      data-whiteboard-id={summary.whiteboardId}
      className="flex cursor-pointer select-none overflow-hidden rounded-xl border border-border/50 bg-card shadow-[0_1px_3px_rgba(0,0,0,0.06),0_4px_12px_rgba(0,0,0,0.04)] transition-shadow hover:shadow-[0_2px_8px_rgba(0,0,0,0.1),0_8px_24px_rgba(0,0,0,0.08)]"
      style={{ ...style, width: 320 }}
      onClick={() => onNavigate(summary.whiteboardId)}
    >
      {/* 左侧紫色渐变竖条 */}
      <div className="w-1.5 shrink-0 bg-gradient-to-b from-violet-400 to-purple-600" />

      <div className="flex flex-1 flex-col px-4 py-3">
        {/* 标题行 */}
        <div className="flex items-center gap-2">
          <span className="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-violet-400 to-purple-600 text-[10px] font-bold text-white">
            W
          </span>
          <h3 className="flex-1 truncate text-sm font-semibold text-foreground">
            {summary.whiteboardId}
          </h3>
        </div>

        {/* 统计行：非零字段 · 分隔；全零时显示 empty */}
        <p className="mt-1.5 text-xs text-muted-foreground">
          {isEmpty ? "empty" : stats}
        </p>

        {/* 进入提示 */}
        <p className="mt-2 text-[10px] text-muted-foreground/50">
          Click to enter →
        </p>
      </div>
    </div>
  );
}
