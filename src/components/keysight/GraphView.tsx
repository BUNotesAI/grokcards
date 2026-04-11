import { GraphCanvas } from "@/components/keysight/GraphCanvas";

/**
 * KeySight 知识图谱主视图
 *
 * 在 GraphCanvas 内放置占位卡片，用于验证 pan/zoom 工作正常。
 * 后续阶段会替换为真实的卡片节点。
 */
export function GraphView() {
  return (
    <div className="h-full w-full">
      <GraphCanvas>
        {/* 5a 阶段：调试用占位块，验证 pan/zoom 工作 */}
        <div
          style={{
            position: "absolute",
            left: 100,
            top: 100,
            width: 200,
            height: 100,
            backgroundColor: "hsl(var(--primary) / 0.15)",
            border: "1px solid hsl(var(--border))",
            borderRadius: 8,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 14,
            color: "hsl(var(--foreground))",
          }}
        >
          占位卡片 (100, 100)
        </div>
        <div
          style={{
            position: "absolute",
            left: 400,
            top: 250,
            width: 200,
            height: 100,
            backgroundColor: "hsl(var(--accent) / 0.15)",
            border: "1px solid hsl(var(--border))",
            borderRadius: 8,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 14,
            color: "hsl(var(--foreground))",
          }}
        >
          占位卡片 (400, 250)
        </div>
      </GraphCanvas>
    </div>
  );
}
