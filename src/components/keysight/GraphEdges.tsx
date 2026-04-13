import { memo } from "react";
import type { Position } from "@/bindings";
import {
  buildEdgePath,
  type EdgeBox,
} from "@/components/keysight/lib/edgePath";
import type { RenderEdge } from "@/components/keysight/lib/buildEdges";

interface GraphEdgesProps {
  edges: RenderEdge[];
  positions: Record<string, Position>;
  /** 实体 id → 真实渲染尺寸（计算 clipToRect 用） */
  dimensions: Record<string, { width: number; height: number }>;
  opacity?: number;
}

/** link_to 边颜色 — Heptabase coral，对齐旧 Obsidian */
const COLOR_LINK = "rgb(250, 119, 107)";
/** note_link / alias_link 边颜色 — 青色，对齐旧 Obsidian */
const COLOR_NOTE_LINK = "#5DCAA5";

/**
 * 知识图谱连线层 — SVG overlay。
 *
 * 渲染所有 edge 的二次贝塞尔曲线 + 箭头。位于实体节点**之下**作为背景层，
 * 必须在 GraphCanvas 的 transform 容器内部以跟随 pan/zoom。
 *
 * - link_to: 橙色实线 + 橙色箭头
 * - note_link / alias_link: 青色虚线 + 青色箭头
 *
 * 不可点击（pointerEvents: none），edge 删除等交互在后续 phase 再加。
 */
export const GraphEdges = memo(function GraphEdges({
  edges,
  positions,
  dimensions,
  opacity = 1,
}: GraphEdgesProps) {
  return (
    <svg
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        width: 1,
        height: 1,
        overflow: "visible",
        pointerEvents: "none",
        opacity,
      }}
    >
      {edges.map((edge, i) => {
        const fromPos = positions[edge.from];
        const toPos = positions[edge.to];
        if (!fromPos || !toPos) return null;
        const fromDim = dimensions[edge.from];
        const toDim = dimensions[edge.to];
        if (!fromDim || !toDim) return null;

        const fromBox: EdgeBox = {
          x: fromPos.x,
          y: fromPos.y,
          width: fromDim.width,
          height: fromDim.height,
        };
        const toBox: EdgeBox = {
          x: toPos.x,
          y: toPos.y,
          width: toDim.width,
          height: toDim.height,
        };
        const result = buildEdgePath(fromBox, toBox);
        if (!result) return null;

        const isLink = edge.kind === "link_to";
        const color = isLink ? COLOR_LINK : COLOR_NOTE_LINK;
        const markerId = `${isLink ? "ks-edge-arrow-link" : "ks-edge-arrow-note"}-${edge.from}-${edge.to}-${i}`;
        const markerSize = isLink ? 8 : 10;
        const refX = isLink ? 7.5 : 9;
        const refY = isLink ? 4 : 5;

        return (
          <g key={`${edge.from}->${edge.to}-${edge.kind}-${i}`}>
            <defs>
              <marker
                id={markerId}
                viewBox={`0 0 ${markerSize} ${markerSize}`}
                markerWidth={markerSize}
                markerHeight={markerSize}
                refX={refX}
                refY={refY}
                orient="auto-start-reverse"
                markerUnits="userSpaceOnUse"
              >
                <path
                  d={
                    isLink ? "M 0 0 L 8 4 L 0 8 Z" : "M 0 0 L 10 5 L 0 10 Z"
                  }
                  fill={color}
                />
              </marker>
            </defs>
            <path
              data-edge-kind={edge.kind}
              d={result.path}
              stroke={color}
              strokeWidth={2}
              fill="none"
              strokeDasharray={isLink ? undefined : "8 4"}
              markerEnd={`url(#${markerId})`}
            />
          </g>
        );
      })}
    </svg>
  );
});
