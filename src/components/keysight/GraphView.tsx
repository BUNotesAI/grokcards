import { GraphCanvas } from "@/components/keysight/GraphCanvas";
import { useViewport } from "@/components/keysight/useViewport";

/**
 * KeySight 知识图谱主视图
 *
 * 5b 阶段临时状态 — Task 8 会完整重写为数据枢纽。
 */
export function GraphView() {
  const viewport = useViewport();

  return (
    <div className="h-full w-full">
      <GraphCanvas viewport={viewport}>
        {/* 占位 — Task 8 替换为真实实体 */}
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
      </GraphCanvas>
    </div>
  );
}
