import { describe, it, expect } from "vitest";
import graphPositioningSrc from "@/components/keysight/lib/graphPositioning.ts?raw";
import sectionLayoutSrc from "@/components/keysight/lib/sectionLayout.ts?raw";
import taskLayoutSrc from "@/components/keysight/lib/taskLayout.ts?raw";
import mergeEntitiesWithPositionsSrc from "@/components/keysight/lib/mergeEntitiesWithPositions.ts?raw";
import graphViewSrc from "@/components/keysight/GraphView.tsx?raw";

// 静态扫描:vite ?raw 把源文件当字符串 import,正则断言 export / 不再含定义。

describe("C Theme W0 — 模块顶部纯函数 → lib/", () => {
  it("c_theme_w0_lib_files_exist_with_exports", () => {
    // graphPositioning.ts exports
    expect(graphPositioningSrc).toMatch(/export\s+function\s+viewportCenterWorld\b/);
    expect(graphPositioningSrc).toMatch(/export\s+function\s+visibleViewportSize\b/);

    // sectionLayout.ts exports
    expect(sectionLayoutSrc).toMatch(/export\s+(interface|type)\s+Rect\b/);
    expect(sectionLayoutSrc).toMatch(/export\s+function\s+rectsOverlap\b/);
    expect(sectionLayoutSrc).toMatch(/export\s+function\s+avoidSectionOverlap\b/);

    // taskLayout.ts exports
    expect(taskLayoutSrc).toMatch(/export\s+function\s+taskPositionAboveTopmost\b/);
    expect(taskLayoutSrc).toMatch(/export\s+function\s+packTaskPositions\b/);
    expect(taskLayoutSrc).toMatch(/export\s+const\s+TASK_NODE_WIDTH\b/);
    expect(taskLayoutSrc).toMatch(/export\s+const\s+TASK_NODE_HEIGHT\b/);
    expect(taskLayoutSrc).toMatch(/export\s+const\s+TASK_JUMP_MIN_ZOOM\b/);
    expect(taskLayoutSrc).toMatch(/export\s+const\s+TASK_TOP_INSERT_OFFSET_Y\b/);
    expect(taskLayoutSrc).toMatch(/export\s+const\s+TASK_PACK_COLUMN_GAP\b/);
    expect(taskLayoutSrc).toMatch(/export\s+const\s+TASK_PACK_ROW_GAP\b/);
    expect(taskLayoutSrc).toMatch(/export\s+const\s+TASK_PACK_TOP_OFFSET_Y\b/);

    // mergeEntitiesWithPositions.ts exports
    expect(mergeEntitiesWithPositionsSrc).toMatch(
      /export\s+function\s+mergeEntitiesWithPositions\b/,
    );

    // GraphView.tsx 不再含上述函数 / 常量的顶层定义(仅 import 它们)
    expect(graphViewSrc).not.toMatch(/^function\s+viewportCenterWorld\b/m);
    expect(graphViewSrc).not.toMatch(/^function\s+visibleViewportSize\b/m);
    expect(graphViewSrc).not.toMatch(/^function\s+rectsOverlap\b/m);
    expect(graphViewSrc).not.toMatch(/^function\s+avoidSectionOverlap\b/m);
    expect(graphViewSrc).not.toMatch(/^function\s+taskPositionAboveTopmost\b/m);
    expect(graphViewSrc).not.toMatch(/^function\s+packTaskPositions\b/m);
    expect(graphViewSrc).not.toMatch(/^function\s+mergeEntitiesWithPositions\b/m);
    expect(graphViewSrc).not.toMatch(/^const\s+TASK_NODE_WIDTH\b/m);
    expect(graphViewSrc).not.toMatch(/^const\s+TASK_NODE_HEIGHT\b/m);
    expect(graphViewSrc).not.toMatch(/^const\s+TASK_JUMP_MIN_ZOOM\b/m);
    expect(graphViewSrc).not.toMatch(/^const\s+TASK_TOP_INSERT_OFFSET_Y\b/m);
    expect(graphViewSrc).not.toMatch(/^const\s+TASK_PACK_COLUMN_GAP\b/m);
    expect(graphViewSrc).not.toMatch(/^const\s+TASK_PACK_ROW_GAP\b/m);
    expect(graphViewSrc).not.toMatch(/^const\s+TASK_PACK_TOP_OFFSET_Y\b/m);

    // 抽走后引用是否断裂由 pnpm build / 编译期保证,无需 import-assertion;
    // W1+ 后某些函数被 useCreationModals 等 hook 直接 import,GraphView 不再持有 import,
    // 留 import-assertion 会随实施波次 stale。
  });
});
