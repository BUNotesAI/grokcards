import { useMemo, type CSSProperties } from "react";
import type { GraphSection, Position } from "@/bindings";
import {
  ENTITY_DIMENSIONS,
  type EntityKind,
} from "@/components/keysight/types";
import {
  computeSectionBounds,
  type SectionMember,
} from "@/components/keysight/lib/sectionBounds";

interface SectionNodeProps {
  section: GraphSection;
  memberPositions: Record<string, Position>;
  /** member id → kind 映射，用于查询每个 member 的真实渲染尺寸 */
  memberKinds: Record<string, EntityKind>;
  style: CSSProperties;
}

/** Section 颜色映射 — 7 色半透明配色 */
const SECTION_COLORS: Record<
  string,
  { bg: string; border: string; text: string }
> = {
  default: {
    bg: "bg-gray-100/50 dark:bg-gray-800/30",
    border: "border-gray-300/50 dark:border-gray-600/50",
    text: "text-gray-600 dark:text-gray-300",
  },
  purple: {
    bg: "bg-purple-100/50 dark:bg-purple-900/30",
    border: "border-purple-300/50 dark:border-purple-600/50",
    text: "text-purple-600 dark:text-purple-300",
  },
  blue: {
    bg: "bg-blue-100/50 dark:bg-blue-900/30",
    border: "border-blue-300/50 dark:border-blue-600/50",
    text: "text-blue-600 dark:text-blue-300",
  },
  green: {
    bg: "bg-emerald-100/50 dark:bg-emerald-900/30",
    border: "border-emerald-300/50 dark:border-emerald-600/50",
    text: "text-emerald-600 dark:text-emerald-300",
  },
  orange: {
    bg: "bg-orange-100/50 dark:bg-orange-900/30",
    border: "border-orange-300/50 dark:border-orange-600/50",
    text: "text-orange-600 dark:text-orange-300",
  },
  yellow: {
    bg: "bg-yellow-100/50 dark:bg-yellow-900/30",
    border: "border-yellow-300/50 dark:border-yellow-600/50",
    text: "text-yellow-600 dark:text-yellow-300",
  },
  pink: {
    bg: "bg-pink-100/50 dark:bg-pink-900/30",
    border: "border-pink-300/50 dark:border-pink-600/50",
    text: "text-pink-600 dark:text-pink-300",
  },
};

/** 分组容器四边 padding（世界坐标像素） */
const PADDING = 40;

/**
 * 分组容器节点 — 半透明背景矩形
 * 大小由成员位置 + 每个成员的真实 kind 尺寸动态计算。
 * 无成员时使用最小尺寸。7 色配色。
 */
export function SectionNode({
  section,
  memberPositions,
  memberKinds,
  style,
}: SectionNodeProps) {
  const colors =
    SECTION_COLORS[section.color ?? "default"] ?? SECTION_COLORS.default;

  const bounds = useMemo(() => {
    const members: SectionMember[] = [];
    for (const id of section.cardIds) {
      const pos = memberPositions[id];
      if (!pos) continue;
      const kind = memberKinds[id] ?? "card";
      const dim = ENTITY_DIMENSIONS[kind];
      members.push({ position: pos, width: dim.width, height: dim.height });
    }
    return computeSectionBounds(members, PADDING);
  }, [section.cardIds, memberPositions, memberKinds]);

  return (
    <div
      data-entity-id={section.id}
      className={`select-none rounded-2xl border-2 border-dashed ${colors.border} ${colors.bg}`}
      style={{
        ...style,
        width: bounds.width,
        height: bounds.height,
        minWidth: 200,
        minHeight: 100,
      }}
    >
      <div className="px-3 pt-2">
        <h3
          className={`text-xs font-bold uppercase tracking-wide ${colors.text}`}
        >
          {section.title}
        </h3>
      </div>
    </div>
  );
}
