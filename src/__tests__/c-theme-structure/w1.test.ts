import { describe, it, expect } from "vitest";
import useEntityEditingSrc from "@/components/keysight/hooks/useEntityEditing.ts?raw";
import useCreationModalsSrc from "@/components/keysight/hooks/useCreationModals.ts?raw";
import graphViewSrc from "@/components/keysight/GraphView.tsx?raw";

describe("C Theme W1 — useEntityEditing + useCreationModals", () => {
  it("c_theme_w1_hook_files_exist", () => {
    // useEntityEditing.ts exports
    expect(useEntityEditingSrc).toMatch(/export\s+function\s+useEntityEditing\b/);
    expect(useEntityEditingSrc).toMatch(/export\s+(interface|type)\s+UseEntityEditingDeps\b/);
    expect(useEntityEditingSrc).toMatch(/export\s+(interface|type)\s+UseEntityEditingResult\b/);

    // useCreationModals.ts exports
    expect(useCreationModalsSrc).toMatch(/export\s+function\s+useCreationModals\b/);
    expect(useCreationModalsSrc).toMatch(/export\s+(interface|type)\s+UseCreationModalsDeps\b/);
    expect(useCreationModalsSrc).toMatch(/export\s+(interface|type)\s+UseCreationModalsResult\b/);

    // GraphView.tsx 内不再含 editing state + 3 个 edit handler 的顶层定义
    expect(graphViewSrc).not.toMatch(/^\s*const\s+\[editing,\s*setEditing\]\s*=\s*useState/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleStartEdit\s*=\s*useCallback/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleCancelEdit\s*=\s*useCallback/m);
    expect(graphViewSrc).not.toMatch(/^\s*const\s+handleCommitEdit\s*=\s*useCallback/m);

    // GraphView.tsx 内不再含 4 类 creating* + draft state useState 调用
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[creatingNote,\s*setCreatingNote\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[noteDraft,\s*setNoteDraft\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[creatingQuestion,\s*setCreatingQuestion\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[questionDraft,\s*setQuestionDraft\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[creatingTask,\s*setCreatingTask\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[taskDraft,\s*setTaskDraft\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[creatingWhiteboard,\s*setCreatingWhiteboard\]\s*=\s*useState/m,
    );
    expect(graphViewSrc).not.toMatch(
      /^\s*const\s+\[whiteboardDraft,\s*setWhiteboardDraft\]\s*=\s*useState/m,
    );

    // GraphView.tsx 内不再含 13 个 create handler 的定义
    const createHandlerNames = [
      "handleCreateSection",
      "handleCreateNote",
      "handleCancelCreateNote",
      "handleSubmitCreateNote",
      "handleCreateQuestion",
      "handleCancelCreateQuestion",
      "handleSubmitCreateQuestion",
      "handleCreateTask",
      "handleCancelCreateTask",
      "handleSubmitCreateTask",
      "handlePackTasks",
      "handleCreateWhiteboard",
      "handleCancelCreateWhiteboard",
      "handleSubmitCreateWhiteboard",
    ];
    for (const name of createHandlerNames) {
      expect(graphViewSrc).not.toMatch(new RegExp(`^\\s*const\\s+${name}\\s*=\\s*useCallback`, "m"));
    }

    // GraphView.tsx 应该 import 两个新 hook
    expect(graphViewSrc).toMatch(
      /import\s+\{[^}]*\buseEntityEditing\b[^}]*\}\s+from\s+["']@\/components\/keysight\/hooks\/useEntityEditing["']/,
    );
    expect(graphViewSrc).toMatch(
      /import\s+\{[^}]*\buseCreationModals\b[^}]*\}\s+from\s+["']@\/components\/keysight\/hooks\/useCreationModals["']/,
    );
  });
});
