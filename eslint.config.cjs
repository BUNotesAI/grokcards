// task_ca5ece70 A1 — 机械守门:文件行数 / 函数行数 / 圈复杂度
// 只挂 ESLint built-in 三规则,不引入其他 plugin / preset

const tseslint = require('typescript-eslint');

const limits = {
  'max-lines': ['error', { max: 600, skipBlankLines: true, skipComments: true }],
  'max-lines-per-function': ['error', { max: 100, skipBlankLines: true, IIFEs: true }],
  complexity: ['error', 15],
};

module.exports = [
  {
    // 排除生成产物、Rust crate、vault 符号链接、bindings 生成文件
    ignores: [
      'dist/**',
      'node_modules/**',
      'src-tauri/**',
      'keysight-core/**',
      'keysight-cli/**',
      'vault/**',
      'src/bindings.ts',
      '**/*.d.ts',
    ],
  },
  {
    files: ['src/**/*.{ts,tsx}', 'scripts/**/*.{ts,tsx}'],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: { jsx: true },
      },
    },
    linterOptions: {
      // 禁止 inline `/* eslint-disable */` 散乱豁免;
      // 豁免统一走下方 file-level overrides(精确单文件路径),与 task_ca5ece70 "禁止目录广义豁免" 精神一致
      noInlineConfig: true,
    },
    rules: limits,
  },

  // ---------- file-level overrides (task_ca5ece70 起点 41 个现有超标文件) ----------
  //
  // 这些文件在本 task 落地时已经超标(test 13 + 业务 28),容忍现状不重构,
  // 三规则全关。**新写文件禁止 add 到此列表**:任何新文件超标都应该拆解,
  // 而不是加进豁免名单。重构由后续 task 推进(参 task_cb66f158 C Theme GraphView 拆解)。
  {
    files: [
      // ---- 测试文件(13) ----
      'src/__tests__/components/kanban/KanbanToolbar.test.tsx',
      'src/__tests__/components/kanban/KanbanView.test.tsx',
      'src/__tests__/components/kanban/TaskEditModal.test.tsx',
      'src/__tests__/components/keysight/GraphToolbar.test.tsx',
      'src/__tests__/components/keysight/GraphView.test.tsx',
      'src/__tests__/components/keysight/nodes/CardNode.test.tsx',
      'src/__tests__/components/keysight/nodes/EntityNode.test.tsx',
      'src/__tests__/components/keysight/nodes/NodeContextMenu.test.tsx',
      'src/__tests__/components/keysight/nodes/NoteNode.test.tsx',
      'src/__tests__/components/keysight/nodes/SectionNode.test.tsx',
      'src/__tests__/components/keysight/nodes/TaskNode.test.tsx',
      'src/__tests__/components/keysight/sidebar/CardsList.test.tsx',
      'src/__tests__/components/keysight/useViewport.test.ts',
      // ---- 业务文件(28) ----
      'src/components/kanban/CreateTaskModal.tsx',
      'src/components/kanban/KanbanView.tsx',
      'src/components/kanban/TaskEditModal.tsx',
      'src/components/keysight/GraphToolbar.tsx',
      'src/components/keysight/GraphView.tsx',
      'src/components/keysight/KeysightView.tsx',
      'src/components/keysight/hooks/useCreationModals.ts',
      'src/components/keysight/hooks/useDragInteraction.ts',
      'src/components/keysight/hooks/useEntityContextMenu.ts',
      'src/components/keysight/hooks/useEntityMenuHandlers.ts',
      'src/components/keysight/hooks/useViewportRecovery.ts',
      'src/components/keysight/hooks/useWhiteboardData.ts',
      'src/components/keysight/lib/buildEdges.ts',
      'src/components/keysight/lib/clipToRect.ts',
      'src/components/keysight/lib/graphPositioning.ts',
      'src/components/keysight/nodes/AliasNode.tsx',
      'src/components/keysight/nodes/CardNode.tsx',
      'src/components/keysight/nodes/EntityNode.tsx',
      'src/components/keysight/nodes/NoteNode.tsx',
      'src/components/keysight/nodes/QuestionNode.tsx',
      'src/components/keysight/nodes/RenderedMarkdown.tsx',
      'src/components/keysight/nodes/TaskNode.tsx',
      'src/components/keysight/sidebar/CardsList.tsx',
      'src/components/keysight/sidebar/InsightCardDetail.tsx',
      'src/components/keysight/useViewport.ts',
      'src/components/ui/calendar.tsx',
      'src/components/ui/chart.tsx',
      'src/components/ui/sidebar.tsx',
      'src/pages/TodoPage.tsx',
    ],
    rules: {
      'max-lines': 'off',
      'max-lines-per-function': 'off',
      complexity: 'off',
    },
  },
];
