---
area: workflow-v4-migration
last_updated: 2026-04-15T18:20:00+08:00
session_id: v4-design-session
status: ready-to-resume
stale_check: ls ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs 2>/dev/null | head -10; echo "---"; git status --short docs/
---

# Handoff: Workflow v4 迁移执行

## 正在做的 Task

把 super-tauri 项目从 V3 四关注点模型 **迁移到 v4 task-centric 对话式工作流**。设计阶段(本 session)已完成,**下一个 session 执行实际迁移**。

## 前置阅读(按顺序,必须全部读完再开始)

1. **v4 完整设计文档**:`docs/2026-04-15-complete-workflow-v4-cc.md`(本 repo)
   - 方案架构 + frontmatter schema + slash command 规格 + tradeoff
2. **V3 slipbox 最终方案 note**:`~/Documents/obsidian_workspace/agent-slipbox-v3/projects/keysight/2026-04-15 1815 cc 项目管理体系 v4 — task-centric 对话式工作流.md`
   - v4 方案的精炼版 + Bootstrap 清单 + CLAUDE.md 模板段落
3. **V3 原版 note**:`~/Documents/obsidian_workspace/agent-slipbox-v3/projects/keysight/2026-04-04 2029 cc 项目管理体系 — 从混乱到四关注点.md`
   - v3 四关注点模型 + L0-L4 质量规则 + TS/Rust 双栈约束(v4 全部继承)
4. **现有 CLAUDE.md**:`CLAUDE.md`(super-tauri 根目录)
   - 当前的质量规则段落(L0 建模优先 / 防火墙 / TDD 等),迁移后这部分 **保留**,只改项目管理体系段落

## 已完成步骤(本 session)

- [x] 问题分析:V3 的 Progress → Quality 断层 + docs 散落 + 无强制沉淀闭环
- [x] v1 草案(agent-spec 融入):`docs/collaboration/2026-04-15-agent-spec-quality-integration-v1-cc.md`
- [x] v2 codex review:`docs/collaboration/2026-04-15-agent-spec-quality-integration-v2-codex.md`
- [x] v3 修订方案(补 project.spec + 修正 lifecycle 边界):`docs/collaboration/2026-04-15-agent-spec-quality-integration-v3-cc.md`
- [x] v4 完整方案:`docs/2026-04-15-complete-workflow-v4-cc.md`(task-centric + 对话式)
- [x] 与用户讨论收敛:5 阶段 + numbered slash commands(01-05)+ 横向 review/view + 删除 bug-fix
- [x] V3 slipbox 最终 note:`~/Documents/obsidian_workspace/agent-slipbox-v3/projects/keysight/2026-04-15 1815 cc 项目管理体系 v4 — task-centric 对话式工作流.md`
- [ ] **卡在这里**:设计完成,下一步是实际执行迁移

## 下一步具体动作(按 Phase 严格顺序执行)

### Phase 0 — 环境准备(预计 5 分钟)

**目的**:确认执行前提条件具备。

- [ ] 0.1 检查 vault 路径存在:`ls ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/`
- [ ] 0.2 检查是否已装 agent-spec CLI:`which agent-spec`
  - 装了 → 继续
  - 没装 → **先跳过**,迁移不强依赖 CLI,Phase 1-6 都能跑,CLI 可以后补
- [ ] 0.3 `git status` 确认 working tree 干净(或只有预期的 unstaged 改动)
- [ ] 0.4 建一个新的 git branch 做迁移:`git checkout -b workflow-v4-migration`
  - 迁移过程中有反悔机会,不污染 main

### Phase 1 — Vault 目录初始化(预计 2 分钟)

**目的**:在 vault 里创建 v4 需要的目录骨架。

- [ ] 1.1 创建目录结构(一条命令):
  ```bash
  mkdir -p ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs/{progress,devlog,handoff,tasks,lessons/modules}
  ```
- [ ] 1.2 验证目录已建:
  ```bash
  tree -L 3 ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs
  ```

### Phase 2 — Docs 从 repo 搬到 vault(预计 10 分钟)

**目的**:把现有 repo `docs/` 下的 working doc 搬到 vault,repo 只留 meta doc。

**重要**:用 `git mv` 不能直接跨 repo(vault 是独立 git),所以用 **cp + 后续 `git rm`** 的方式。先搬(确认没事),再删 repo 里的。

- [ ] 2.1 搬 `docs/progress/`:
  ```bash
  cp -r docs/progress/* ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs/progress/
  ```
  验证:对比文件内容 md5,确保搬过去的完整
- [ ] 2.2 搬 `docs/devlog/`(包括 2026-04-14.md 和 2026-04-15.md):
  ```bash
  cp docs/devlog/*.md ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs/devlog/
  ```
- [ ] 2.3 搬 `docs/handoff/`(legacy 功能域级,过渡期保留):
  ```bash
  cp docs/handoff/*.md ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs/handoff/
  ```
  注意:**本 handoff 文件(workflow-v4-migration.md)不搬**,它是迁移本身的 handoff,完成后会被取代
- [ ] 2.4 repo 的 `docs/` 只留 meta:
  - **保留**:`docs/2026-04-15-complete-workflow-v4-cc.md`(v4 设计文档)
  - **保留**:`docs/handoff/workflow-v4-migration.md`(本文件,完成后删)
  - **删除**:其他所有 `docs/progress/`、`docs/devlog/`、`docs/handoff/*.md`(除本文件)、`docs/collaboration/`(Phase 3 会按 task 分拣后再处理)
- [ ] 2.5 验证:
  ```bash
  # vault 里有内容
  ls ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs/progress/
  # repo 里已清理(除本 handoff 和 v4 设计文档)
  find docs -type f -name "*.md"
  ```

### Phase 3 — V1.1 Kanban Subtask 作为 dogfood 示例(预计 30 分钟)

**目的**:拿 V1.1 task 完整走一遍 v4 格式,作为未来模板。这是最重要的一步 —— 没有真实示例,后续 task 不知道该怎么填 frontmatter。

#### 3.1 定位 V1.1 的 task.md

- [ ] 3.1.1 找到 V1.1 task 的 id 和文件路径:
  ```bash
  ls ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/*.md
  ```
  V1.1 task 应该是标题含 "kanban" / "subtask" / "V1.1" 的 task.md。记录 `task_id`(例如 `task_3a3f9a64`)

#### 3.2 建 per-task 子目录

- [ ] 3.2.1 创建:
  ```bash
  TASK_ID={实际task_id}
  mkdir -p ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs/tasks/$TASK_ID/reviews
  ```

#### 3.3 迁入 V1.1 的 5 份 collab docs(按 v4 命名规范重命名)

原文件 → 目标路径 对应关系(在 repo `docs/collaboration/` 下):

| 源文件 | 目标路径(vault,相对 task.md)|
|---|---|
| `2026-04-15-kanban-v1-1-subtask-design-via-cc.md` | `docs/tasks/{task_id}/design-v1-cc.md` |
| `2026-04-15-kanban-v1-1-subtask-design-via-codex.md` | `docs/tasks/{task_id}/reviews/r1-codex.md` |
| `2026-04-15-kanban-v1-1-subtask-design-review-via-cc.md` | `docs/tasks/{task_id}/reviews/r1-cc.md` |
| `2026-04-15-kanban-v1-1-subtask-design-review-via-codex.md` | `docs/tasks/{task_id}/reviews/r2-codex.md` |
| `2026-04-15-kanban-v1-1-subtask-design-v2-via-cc.md` | `docs/tasks/{task_id}/design-v2-cc.md`(**final**) |

- [ ] 3.3.1 依次 cp + 重命名
- [ ] 3.3.2 每个目标文件开头补 frontmatter(如原来没有):
  ```yaml
  ---
  type: design | review
  task_id: task_xxx
  version: v1 | v2             # design 才有
  round: 1 | 2                  # review 才有
  author: cc | codex
  role: author | reviewer       # review 才有
  status: superseded | final    # 最后一个是 final,其他 superseded
  ---
  ```
- [ ] 3.3.3 验证相对路径引用 —— 如果原文件里有 `docs/collaboration/xxx.md` 的链接,改成新路径

#### 3.4 回溯填 task.md frontmatter

打开 V1.1 task.md,在现有 frontmatter 基础上追加:

- [ ] 3.4.1 `design:` 字段(指向 final):
  ```yaml
  design: "docs/tasks/{task_id}/design-v2-cc.md"
  ```
- [ ] 3.4.2 `review:` 列表(append-only,按时间顺序):
  ```yaml
  review:
    - docs/tasks/{task_id}/reviews/r1-codex.md
    - docs/tasks/{task_id}/reviews/r1-cc.md
    - docs/tasks/{task_id}/reviews/r2-codex.md
    - docs/tasks/{task_id}/design-v2-cc.md
  ```
  **注意**:最后一项是 design-v2(因为它是 CC 对 r2 codex review 的最终响应,既是新 design 也是 review response,不单独建 r2-cc-final)
- [ ] 3.4.3 `commits:` 列表(11 个 V1.1 commit,git log 能查到):
  ```yaml
  commits:
    - hash: 771e30c
      subject: "refactor(app_error): struct variants + MultiBlockChecklist variant (V1.1 Phase 6.0)"
    - hash: 41674b3
      subject: "feat(keysight): parse_task_checklist + Subtask struct (V1.1 Phase 6.1)"
    - hash: 726ae25
      subject: "feat(keysight): TaskEntity.subtasks + 三 reader 填充 (V1.1 Phase 6.2)"
    - hash: 8a60691
      subject: "feat(keysight): task_update_with_subtasks + render_subtasks_into_body (V1.1 Phase 6.3)"
    - hash: a14e128
      subject: "feat(kanban): KanbanCard progress 徽章 (V1.1 Phase 6.4)"
    - hash: be9d2ac
      subject: "feat(kanban): TaskEditModal + ChecklistEditor (V1.1 Phase 6.5)"
    - hash: 844f634
      subject: "feat(kanban): 双击 KanbanCard 打开 TaskEditModal + PointerSensor 防 dnd 冲突 (V1.1 Phase 6.6)"
    - hash: e9d3ff4
      subject: "fix(kanban): 单击打开 modal + color swatch picker (V1.1 Phase 6.6 follow-up)"
    - hash: c2419c4
      subject: "fix(kanban): TaskEditModal clear color 发送 'default' sentinel (V1.1 bug fix)"
    - hash: b2dae84
      subject: "fix(keysight): TaskNode 在 canvas 里结构化渲染 subtasks"
    - hash: e16a179
      subject: "docs(backend): P3 Kanban V1.1 Subtask + 单击编辑 modal 完成收口 (V1.1 Phase 6.7)"
  ```
- [ ] 3.4.4 `lessons:` 字段(从 V1.1 devlog + review cycles 提炼):
  ```yaml
  lessons:
    - "rusqlite 不走 serde,r.get::<_, TaskStatus>() 需要手动 impl FromSql/ToSql,非法字符串走 FromSqlError::Other(Box<Error>)"
    - "task::update 的 color 参数 None 语义是'保留',Some('default') 才是'清空',和 note/card/question 项目约定一致"
    - "@dnd-kit PointerSensor activationConstraint {distance:8} 是 click/drag 互斥的标准做法,不需要 drag handle 或 isDragging check"
    - "AppError 用 struct variants + #[serde(tag='kind')] 让每个 variant 字段在顶层展开,比 tuple+content 清晰"
    - "Subtask 的 line_index 是 parser 内部细节,不应泄漏到 IPC 写契约(v2 codex review 硬伤)"
    - "render_subtasks_into_body 算法要单 block only,多 block 用 typed error fail-closed,不硬做 per-line patch"
    - "TaskEditModal 无 content textarea 是 V1.1 的 scope 收敛,避免滑向半结构化 markdown 编辑器"
  ```
- [ ] 3.4.5 `mempal:` 三元组(提取关键 KG):
  ```yaml
  mempal:
    - subject: "rusqlite"
      predicate: "反序列化机制"
      object: "自有 FromSql trait,非 serde"
    - subject: "task::update color None"
      predicate: "语义"
      object: "保留原值(Some('default') 才是清空)"
    - subject: "@dnd-kit PointerSensor activationConstraint"
      predicate: "解决的问题"
      object: "click/drag 手势冲突,天然互斥,无需 drag handle"
    - subject: "V1.1 Kanban Subtask"
      predicate: "触发的 refactor"
      object: "AppError tuple → struct variants + MultiBlockChecklist variant"
    - subject: "GFM checklist body parser 的 line_index"
      predicate: "暴露规则"
      object: "不跨 IPC,仅 Rust 内部 ParsedItem 私有类型使用"
    - subject: "render_subtasks_into_body 算法"
      predicate: "约束"
      object: "只支持单连续 block,多 block 返 typed error"
  ```
  可选:调 `mcp__mempal__mempal_ingest` 把这些三元组也 ingest 到 mempal 服务
- [ ] 3.4.6 `rules:` 字段(从 V1.1 经验总结):
  ```yaml
  rules:
    - "Subtask parser 必须在 Rust,TS 禁止 import parse 逻辑(L0 TS/Rust 职责边界)"
    - "Multi-block checklist 走 typed error,禁 TS 字符串 match 控制流"
    - "Color clear 发 'default' sentinel,null 语义是'保留原值'"
    - "PointerSensor activationConstraint 是唯一 click/drag 互斥机制"
    - "TaskEntity 跨 IPC 字段升 enum,bindings 自动生成 literal union"
  ```
- [ ] 3.4.7 `skills:` 字段:
  ```yaml
  skills:
    - rust-modeling
    - rust-errors
    - rust-types
    - react-hook-form-zod
    - tanstack-router-core-navigation
  ```
- [ ] 3.4.8 **不填 spec 字段** —— V1.1 没有写正式 spec.md(当时是探索型 task,design 就是事实上的规格)。v4 定义 design v2 是 final,无 spec.md。可以在 task.md 里注明 `spec: null  # V1.1 无正式 spec,design-v2-cc.md 是事实规格`,或干脆省略 `spec:` 字段

#### 3.5 删除 repo 里的 collab 原文件

- [ ] 3.5.1 确认 vault 里所有文件都对,再删 repo:
  ```bash
  git rm docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-*.md
  ```
- [ ] 3.5.2 其他无关 collaboration 文档(`2026-04-15-agent-spec-quality-integration-*.md` 等)**保留**,它们是 v4 设计过程的历史

### Phase 4 — 写 slash commands(预计 45 分钟,最耗时)

**目的**:实现 v4 的核心 5 个 numbered commands + 2 个横向 + 调整保留的 commands。

命令文件位置:`~/.claude/commands/`(全局) 或项目 `.claude/commands/`(本项目限定)。**推荐全局**,以便新项目也能用。

#### 4.1 写 `/harness-01-design`

- [ ] 4.1.1 文件:`~/.claude/commands/harness-01-design.md`
- [ ] 4.1.2 Prompt 要点:
  - 如果当前无 active task,提示用户确认是否新建
  - 调 `superpowers:brainstorming` skill 或自由对话
  - 产出写到 vault 的 `docs/tasks/{task_id}/design-v{N}.md`
  - 更新 task.md frontmatter `design:` 字段
  - 参考 V3 slipbox note §十 的 Design 阶段规则

#### 4.2 写 `/harness-02-spec`

- [ ] 4.2.1 文件:`~/.claude/commands/harness-02-spec.md`
- [ ] 4.2.2 Prompt 要点:
  - 前置检查:task 必须有 design(或机械型 task 直接跳)
  - 从 design 提炼 Completion Criteria(BDD Scenario + `Test: fn_name` 绑定)
  - 写到 `docs/tasks/{task_id}/spec.md`
  - 更新 frontmatter `spec:` 字段
  - 引导用户填 `rules:` 和 `skills:`(从 CLAUDE.md + spec 内容提炼)

#### 4.3 写 `/harness-03-plan`

- [ ] 4.3.1 文件:`~/.claude/commands/harness-03-plan.md`
- [ ] 4.3.2 Prompt 要点(标注 optional):
  - 明确说明"简单 task 可跳过,直接 /harness-04-execute"
  - 拆 design + spec 为分步 phases
  - 每个 phase 对应若干 Scenario + Test: fn_name
  - 写到 `docs/tasks/{task_id}/plan.md`

#### 4.4 写 `/harness-04-execute`

- [ ] 4.4.1 文件:`~/.claude/commands/harness-04-execute.md`
- [ ] 4.4.2 Prompt 要点:
  - 前置检查:task 有 spec(或 bug fix 准备 Red 测试)
  - 无 plan 时直接用 spec 的 Test: 列表顺序驱动
  - `status → active`
  - 调 `superpowers:test-driven-development` skill
  - 循环 Red → 用户确认 → Green → Refactor

#### 4.5 写 `/harness-05-close`

- [ ] 4.5.1 文件:`~/.claude/commands/harness-05-close.md`
- [ ] 4.5.2 Prompt 要点(顺序,不可少步):
  1. 运行 `/harness-check-tests`(spec completeness 自查)
  2. 运行 `agent-spec lifecycle`(如果 CLI 装了)
  3. 运行 `/harness-type-safety-check`
  4. Code Review 6 项检查
  5. 提示用户手动 walkthrough(如果有 UI 改动)
  6. 收集 commits(交互式从 git log 过滤)
  7. 提炼 lessons(从 review + handoff + commits 总结,用户确认)
  8. 提取 mempal 三元组 + ingest(调 mempal MCP 或 `/backfill-kg`)
  9. 写入 task.md frontmatter(commits / lessons / mempal 三处)
  10. 同步 lessons 到 `docs/lessons/modules/{name}.md`
  11. `status → done`

#### 4.6 写 `/harness-review`(横向)

- [ ] 4.6.1 文件:`~/.claude/commands/harness-review.md`
- [ ] 4.6.2 Prompt 要点:
  - 识别当前 task 最新 artifact(design / spec / plan / 实施代码 diff)
  - spawn oracle 或 codex 做 review(通过 `codex:rescue` skill 或直接调 `Agent`)
  - 输出到 `docs/tasks/{task_id}/reviews/r{N}-{reviewer}.md`(自动选 N)
  - 追加 task frontmatter `review:` 列表

#### 4.7 写 `/harness-view`(横向读)

- [ ] 4.7.1 文件:`~/.claude/commands/harness-view.md`
- [ ] 4.7.2 Prompt 要点:
  - 读 task.md,解析 frontmatter
  - 渲染 markdown summary:Intent(从 spec 或 design 第一段读)/ status / artifacts 树 / 时间线(reviews + commits 按时间排序) / lessons / mempal triples
  - 纯读,无副作用

#### 4.8 更新 `/harness-check-tests` 的 prompt(降级版)

- [ ] 4.8.1 找到现有文件:`~/.claude/commands/harness-check-tests.md`
- [ ] 4.8.2 Prompt 从"在代码里找缺失测试"改成"审查 Completion Criteria 是否足够完整(spec completeness 自查)"
- [ ] 4.8.3 具体新 prompt 见 V3 slipbox note §十 或 v3 agent-spec doc §四

#### 4.9 删除 `/harness-bug-fix`

- [ ] 4.9.1 删除文件:`~/.claude/commands/harness-bug-fix.md`
- [ ] 4.9.2 **用户决策**:bug fix 改为手动调 codex CLI,不封装

#### 4.10 验证 autocomplete

- [ ] 4.10.1 在 Claude Code 里敲 `/harness-0`,确认 5 个 numbered 命令都出现
- [ ] 4.10.2 敲 `/harness-r`,确认 `harness-review` / `harness-resume-context` 两个都出现

### Phase 5 — 更新 CLAUDE.md(预计 30 分钟)

**目的**:把 CLAUDE.md 里"项目管理体系"相关段落升级到 v4。

**不改的部分**(从 V3 继承,保持不变):
- L0 质量规则(建模优先 / 防火墙 / TDD / 测试真实代码路径 / TS/Rust 职责边界等)
- L1-L4 的所有内容
- 现有 CLAUDE.md 的 Tauri 安全质量项 / 构建命令 / 副作用矩阵
- 模块 Bootstrap 清单

**要改的段落**:

- [ ] 5.1 **"项目管理体系"段落**:
  - 术语表:`progress` / `devlog` / `handoff` 的路径指向 vault 新位置
  - **新增** "Task 作为 entity" 定义,说明 task.md 是总入口 + frontmatter 是 index
  - 把功能域级 handoff 说明改为"legacy,过渡期保留"
- [ ] 5.2 **新增段落 "Task 对话式工作流"**:
  - 直接从 V3 slipbox note §十 整段复制
  - 5 阶段触发词 + CC 自动动作
  - 明确说"用户只需要说意图,CC 根据触发词自动调度"
- [ ] 5.3 **新增段落 "Task Frontmatter Schema"**:
  - 完整字段列表(从 V3 slipbox note §四)
  - 按基础 / 上下文 / 开发中 / 完成沉淀分组
- [ ] 5.4 **新增段落 "Task 生命周期"**:
  - 状态机图(inbox → next → active → blocked → done)
  - 5 phase 对应的 slash command
- [ ] 5.5 **更新 "质量机制总览" 表**:
  - 新增 L0 行:"Task Contract — task 级规格"、"project.spec — 项目级全局约束"
  - `/harness-check-tests` 的触发时机改为"功能域完成时,lifecycle 之前"
  - 新增"agent-spec lifecycle — 机械验证声明的 Test: 绑定"行(可选,若装了 CLI)
- [ ] 5.6 **更新 "功能域完成 Code Review" 段落**:
  - 改流程为 v4 版(check-tests → lifecycle → type-safety-check → Code Review → walkthrough → 05-close → commit)
  - Code Review 第 1 项改名为 "Contract Acceptance"
- [ ] 5.7 **新增段落 "Slash Commands 速查表"**:
  - 核心 5 numbered + 2 横向 + 保留 4 个(5 + 2 + 4 = 11)
  - 删除 `/harness-bug-fix` 行
- [ ] 5.8 **"新 Session 开始" 段落** 增加:
  - 读 task.md 和 `/harness-view` 替代一部分 progress 读
  - `/harness-resume-context` 现在读 task 级 handoff 不是功能域级
- [ ] 5.9 **"Session 结束" 段落** 增加:
  - 如果有未完成 task,调 `/harness-save-next-context` 写 task 级 handoff

### Phase 6 — 验证(预计 15 分钟)

**目的**:确认迁移后项目可以正常工作。

- [ ] 6.1 `cargo test --workspace --manifest-path src-tauri/Cargo.toml` 全绿(代码未动,应该通过)
- [ ] 6.2 `pnpm test -- --run` 全绿
- [ ] 6.3 `pnpm build` clean
- [ ] 6.4 `cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings` clean
- [ ] 6.5 读 task.md V1.1 dogfood 示例,确认 frontmatter 所有字段都正确
- [ ] 6.6 跑 `/harness-view {task_v1_1_id}`,确认能正确渲染 task 全景
- [ ] 6.7 **Dry run 一个新 task**:
  - 假设新需求 "给 Kanban 加一个筛选框"
  - 跑 `/harness-01-design` → 看是否能进入 brainstorming 对话
  - 跑 `/harness-02-spec` → 看是否能生成 spec.md
  - (不进 execute,只验证 1-2 阶段顺畅)
- [ ] 6.8 验证 slash commands 都在 autocomplete 里

### Phase 7 — 提交 + 收尾(预计 10 分钟)

- [ ] 7.1 `git status` 检查所有改动
- [ ] 7.2 Commit migration:
  ```
  git commit -m "chore: migrate to workflow v4 (task-centric + numbered commands)"
  ```
- [ ] 7.3 可选:合并到 main
- [ ] 7.4 删除本 handoff 文件(迁移完成,不再需要)
- [ ] 7.5 追加 changelog + devlog(在 **vault** 新位置)
- [ ] 7.6 写一条 slipbox permanent note 记录"v4 迁移执行记录 + 实际踩坑"(可选)

---

## 关键上下文(/new 之后会丢的东西)

### 设计决策记录

1. **Spec 在 design 定稿后才写**(不是前置)—— 因为作为开发者,一开始细节不清楚,需要 design 阶段摸索,spec 是 design 的 crystallization
2. **5 个 numbered slash commands + 2 横向 + 删 bug-fix** —— 折中方案,既不是"一堆命令吓人"也不是"纯对话有歧义",用户要记就 7 个
3. **Frontmatter `rules:` 和 CLAUDE.md 部分重复是故意的** —— in-context 强化 > 单一来源,老 task 的 rules 是"当时的真理"有历史价值
4. **Lessons / Mempal 双写是故意的** —— task.md 可见的列表 > 只在外部服务,task 是快照,跨 task 真源在 `docs/lessons/`
5. **V1.1 不补 spec.md** —— 它是探索型 task,design-v2 就是事实规格,不强制回溯
6. **/harness-bug-fix 删除** —— bug fix 改手动调 codex CLI

### 文件位置约定

- Vault 根:`~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/`
- Task.md 在 vault 根(不在 docs/ 下)
- Docs 在 vault 的 `docs/` 子目录:`design.md / progress/ / devlog/ / handoff/ / tasks/ / lessons/modules/`
- Per-task artifact 在 `docs/tasks/{task_id}/{spec.md, design-v{N}.md, plan.md, reviews/r{N}-{reviewer}.md, handoff.md}`
- Repo 的 `docs/` **只留 meta**:v4 设计文档 + README + 本 handoff(完成后删)
- `specs/project.spec` 在 repo(agent-spec CLI 用)

### 相对路径约定

Frontmatter 所有 path **相对 task.md 所在目录**(即 `whiteboard/projects/super-tauri/`),不用绝对路径 / URL。

### 命名约定

- Design:`design-v{N}-cc.md`(或其他 author)
- Plan:`plan.md`(只有一个,不版本化)
- Spec:`spec.md`(只有一个,如果多版本也是 `spec-v{N}.md`)
- Review(reviewer):`reviews/r{N}-{reviewer}.md`
- Review(author response):`reviews/r{N}-{author}.md`
- Handoff:`handoff.md`(task 级,只有一个 current)

---

## 试过但不行的方案(避免重走弯路)

- **docs 留 repo + 只建 task 级 index**:被否掉,失去 Obsidian 原生能力(反向链接 / 图谱 / 搜索)
- **Spec 作为 kickoff 前置**:被否掉,和"边聊边摸索"的现实冲突,spec 必须是 design 定稿后的 crystallization
- **大量 slash commands(12 个)**:被否掉,用户反馈"太多吓到",收敛到 5 numbered + 2 横向
- **纯对话无命令**:被否掉,意图识别有歧义,命令做护栏更可靠
- **把 subtasks 从 body checklist 改到 frontmatter 列表**:被否掉,V1.1 已实现 body checklist,frontmatter 重复会产生同步问题。保持 body 是真源

---

## 开放问题(迁移前需决定)

1. **agent-spec CLI 装不装**?
   - 装了:可以跑 lifecycle 验证
   - 不装:迁移完全可以跑,V1.1 dogfood 示例的 spec.md 可以不建(无 CLI 验证意义)
   - **默认**:先跳过,迁移结束后再装,不阻塞

2. **新 branch 做迁移 vs 直接 main**?
   - 新 branch 更安全,有反悔机会
   - 直接 main 更简单
   - **默认**:建新 branch `workflow-v4-migration`,完成验证后合并

3. **V1.1 dogfood 要不要建 spec.md**?
   - 建了可以验证 spec 格式,但需要从 design v2 回溯提炼
   - 不建保留"V1.1 是探索型无正式 spec"的历史真实
   - **默认**:**不建**,在 task.md frontmatter 注明 `spec: null  # 无正式 spec,design-v2 是事实规格`

4. **CLAUDE.md 的 v4 更新范围**?
   - Minimal:只改项目管理体系段落(必须)
   - Full:顺便把所有段落梳理一遍
   - **默认**:Minimal,降低风险,后续再优化

5. **旧 `/harness-bug-fix` 命令现有使用情况**?
   - 如果现有 devlog / commit 里有引用,需要一起更新
   - 可以先 grep 一下:`grep -r "harness-bug-fix" docs/ CLAUDE.md`
   - **动作**:删除命令文件 + 更新所有引用

---

## Resume 检查清单

**新 session `/new` 之后必做**:

1. [ ] `/harness-resume-context` 读本 handoff
2. [ ] 读 **v4 设计文档**:`docs/2026-04-15-complete-workflow-v4-cc.md`(全量,15 节)
3. [ ] 读 **V3 slipbox 最终 note**:`~/Documents/obsidian_workspace/agent-slipbox-v3/projects/keysight/2026-04-15 1815 cc 项目管理体系 v4 — task-centric 对话式工作流.md`
4. [ ] 读 **V3 原 note**:`~/Documents/obsidian_workspace/agent-slipbox-v3/projects/keysight/2026-04-04 2029 cc 项目管理体系 — 从混乱到四关注点.md`(至少 §三 四关注点 + §三附加约束)
5. [ ] 读 **现有 CLAUDE.md**(全量,理解 L0 质量规则 + 项目管理体系段落)
6. [ ] 跑 `stale_check` 命令验证环境:
   ```
   ls ~/Documents/obsidian_workspace/agent-slipbox-v3/whiteboard/projects/super-tauri/docs 2>/dev/null | head -10
   git status --short docs/
   ```
   期望:vault docs/ 目录不存在(未迁移)+ repo docs/ 有预期改动
7. [ ] `git log --oneline -5` 确认最近 commit 是 V1.1 收口(`e16a179`)
8. [ ] **确认理解 5 个 numbered commands + 2 横向 + 删 bug-fix** 的最终方案
9. [ ] **和用户确认 4 个 "开放问题"** 的答案(特别是 branch 策略)
10. [ ] 按 Phase 0 → 7 顺序严格执行,每个 checkbox 做完打勾

**严禁**:
- 跳过 Phase 顺序(比如先改 CLAUDE.md 再迁目录)
- 大 batch 操作(每 Phase 完成先验证再进下一个)
- 破坏现有 L0 质量规则(CLAUDE.md 保留部分)

**鼓励**:
- 每 Phase 完成后展示差异给用户确认,允许调整
- 遇到歧义立即停下 raise,不要猜
- Phase 3(dogfood)是最有价值也最容易踩坑的一步,花时间做细致
