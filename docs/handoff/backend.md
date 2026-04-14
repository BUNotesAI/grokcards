---
area: backend
last_updated: 2026-04-14T13:30:00+08:00
session_id: eca60756
status: ready-to-resume
stale_check: cargo test --manifest-path src-tauri/Cargo.toml --workspace 2>&1 | grep "test result" | head -1 && pnpm test -- --run 2>&1 | tail -5
---

# Handoff: backend

## 正在做的 Task

**🚨 P0 — Project 白板上 `set_color` / `draw_connection` 在 Note/Question/Task 三种节点上全失效**

发现于本 session 末尾 dev server 验证 Task 4 时,**不是本 session 引入**(理由见下文)。
全部上下文 + 6 个开放假设 + 推荐排查顺序 + 关键代码路径 + 不要做的事都在:

📄 **`docs/collaboration/2026-04-14-project-whiteboard-set-color-and-draw-connection-broken.md`**

下次 session **第一件事就是读这个文档**,不要凭印象推测。

### 一句话症状

`projects/super-tauri` 白板上 Note/Question/Task 三种节点:
- ❌ Set color: 色块菜单**有出现** + 点击**背景色不变**
- ❌ Draw connection (Note/Question): 菜单**有出现** + 点击**完全没反应**(无 source 高亮 / 无 cursor / 第二次点击不画线)
- ✅ Edit title: 三种节点全部正常(本 session P1 Task 4 接入)
- ✅ wb_root 上同操作完全正常(用户已确认)

### 一句话定位指引

**指向数据写回路径里 wb_id 派生的某个分支在 `/` 出现时退化或失效**。优先看 H1
(`domain::note` / `domain::question` 的文件路径派生),其次 H4(`entity_connect`
的 enum 解析)。**不要先看前端**,前端已经穷举过了。

## 已完成步骤(本 session 的 3 个 task,bug 跟它们没关系)

- [x] **P0 Task 1 — `list_whiteboards` 递归 `projects/*` 嵌套白板** — commit `c91f7f7`
  - VaultFs 加 default trait method `list_project_whiteboards`(委托给 `list_first_level_dirs("whiteboard/projects")`)
  - `domain/overview.rs::list_whiteboards` 过滤裸 `"projects"` + chain 嵌套 project 白板(带 `projects/` 前缀)
  - 修 2 个缺口:裸 "projects" 假白板 + 空 project 目录不显示
  - Rust 245→**248**(+3)
- [x] **P0 Task 2 — GraphToolbar 加 `Task` 创建按钮(project 白板专属)** — commit `c59cf7e`
  - GraphToolbar 加 6 个 task-* props + 条件渲染:仅在 `currentWhiteboardId.startsWith("projects/")` 时显示(对齐 Whiteboard-on-root 先例)
  - GraphView `handleSubmitCreateTask` 派生 project + 防御 re-check + `commands.taskCreate(project, title, null, "next", null, null)` → `layoutSetPosition` → `onSelectEntity` → invalidate
  - 互斥 state:Note/Question/Whiteboard 创建态切换时同步 `setCreatingTask(false)`
  - TS 230→**234**(+4)
  - **端到端 dev 验证通过**(用户截图)
- [x] **P1 Task 4 — TaskNode 接入 `edit_title` inline editor** — commit `7f3e24d`
  - NodeCapabilityCatalog `edit_title.applies_to` 加 `task` + `TaskNodeHandlers` Pick 加 `edit_title`(5→6 项)
  - TaskNode mirror QuestionNode 的 inline editor 模式
  - EntityNode 加 `onEditTaskTitle` + `EditingField` union 加 `task-title` + taskMenu 加 `edit_title` handler + case task render 传 4 prop
  - GraphView 本地 `EditingField` union 同步 + `handleCommitEdit` 加 case + invalidate `["tasks", currentWhiteboardId]` + menuHandlers 加 `onEditTaskTitle`
  - TS 234→**238**(+4)
  - **端到端 dev 验证通过**(用户在 dev 测试时**顺带发现**了上面的 P0 bug)
- [x] **写 P0 bug 的 collab 调查报告** — `docs/collaboration/2026-04-14-project-whiteboard-set-color-and-draw-connection-broken.md`
  - 12 节:症状/触发上下文/已排除 6 假设/开放 6 假设/推荐排查顺序/代码路径/相关 commit/硬约束/ground truth/不要做的事/完成标准/一句话总结

## 下一步具体动作 (P0 bug 调查)

**Phase 0 — 先读** `docs/collaboration/2026-04-14-project-whiteboard-set-color-and-draw-connection-broken.md`(全文),特别是第 4 节"开放假设"和第 5 节"推荐排查顺序"。**不要跳过**,这个文档密度比 handoff 高。

**Phase 1 — 最容易验证的事先做**(按 collab 文档第 5 节顺序):

1. 用户在 dev server 打开 DevTools console(Cmd+Opt+I),在 `projects/super-tauri` 重复操作:
   - 点色块 → 看 console 有没有 `更新 note 颜色失败:` / `更新 task 颜色失败:` / `更新 question 颜色失败:` 之类的 error log
   - 点 Draw connection → 点第二个节点 → 看有没有 `建立连线失败:` log
   - 如果有 error → 直接拿 stack trace 定位 → 跳到 Phase 3
   - 如果没 error → 进入 Phase 2(说明命令"成功"但效果没出来)

2. 检查磁盘和 DB:
   - 在 project 白板上设一次 color
   - vault 里对应的 .md 文件 frontmatter `color` 字段变了没?
   - SQLite 的 entities 表对应行 color 列变了没?
   - 路径:`~/Library/Application Support/co.bunotes.super-tauri/keysight.db`

**Phase 2 — git bisect 定位引入 commit**:
- 从 Phase B2 之前的 commit (比如 `2026-04-13` 那批) 开始往后切
- 二分查找首个表现 bug 的 commit
- 关键嫌疑 commit (来自 collab 文档第 7.1 节):`a4290e5` / `16d91df` / `133e2b9` / `bf96491` / `874ae95`

**Phase 3 — TDD 修复**:
- 写一个 Rust domain unit test 精确复现 bug(比如 `note::create("projects/super-tauri", ...)` → 文件应该落在嵌套子目录 + 后续 `note::update` 应该能找到文件 + color 应该写入 frontmatter)
- 测试 Red → 用户确认 → 修代码 → Green → 用户确认
- 修复完之后 dev server 手动验证三种 entity 全部 OK

**关键约束**(来自 collab 文档第 8 节):
- ⚠️ TDD Red→Green 人工确认关卡 **不要跳过**,不要 batch 跑测试就提交
- 不要为修 bug 破坏 deep module 边界
- 如果根因是 wb_id stringly typed,**顺手做 `WhiteboardId` newtype**(handoff 第 12 节列在 Phase B3 P2 推迟项里 → 现在可以提前)
- bug 复现测试要保留为永久回归防护
- 必须 dev server 手动验证才算完成

## 关键上下文(/new 之后会丢的东西)

### 本 session 的关键事实

- 本 session 一共 3 个 task commit (`c91f7f7` / `c59cf7e` / `7f3e24d`),**全部不触碰 set_color / draw_connection 代码路径**,所以 bug 不是本 session 引入的
- 用户 ground truth(可信前提,不需要重新验证):
  - Task 1: `projects/super-tauri` 出现在 Boards 下拉 ✅
  - Task 2: 在 project 白板上能创建 task,文件落到正确路径,卡片渲染正确 ✅
  - Task 4: Note/Question/Task 的 edit_title 在 project 白板上正常工作 ✅
  - wb_root 上 set_color / draw_connection 完全正常 ✅
- bug 是用户测 Task 4 时**顺手点其他菜单项**才发现的,所以可能在 Phase B2 阶段就引入了
- 截图(用户提供):Task `haha sandbox 2` 的 ⋯ 菜单展开,显示 `Copy UUID + title` / `Edit title` / 7 色块 / `Delete` —— **菜单渲染完全正常**,色块都在,只是点了没用

### 试过但不行的方案 / 已排除的假设

详见 collab 文档第 3 节,简列:
- 菜单渲染层有 wb_id 守卫(no — NodeContextMenu 完全 wb-agnostic)
- applies_to 漏配 task/note/question(no — 三种 kind 都在 set_color / draw_connection 的 applies_to)
- menuHandlers 在 GraphView 漏 callback(no — 6 个 set color callback 全在)
- TaskNode 不渲染 color(独立缺口,但解释不了 Note/Question 失效 → 不是根因)
- handleSelectEntity 第二次点击逻辑有 wb_id 参与(no — 完全 entity id 派发)
- drawingState 第一次 set 不被 wb_id 影响(no)

### 开放问题(等待 dev 验证)

- 用户测试时 DevTools console 里有没有 error log?(若有,Rust 命令失败 → 看 stack trace)
- 在 project 白板上建的 Note 文件**实际落到哪里**?平铺在 `whiteboard/` 下,还是嵌套在 `whiteboard/projects/super-tauri/` 下?
- 设 color 后 .md 文件 frontmatter 里 `color` 字段实际有没有更新?(检查磁盘)
- 设 color 后 entities 表 color 列实际有没有更新?(检查 SQLite)

### 已知 P2 / P3 推迟项

- **P2 — TaskNode 不渲染 `task.color`** —— 独立缺口,可单独修。即便修了 P0 bug 仍然存在
- **P3 — Kanban view** —— 大 feature,需独立 session brainstorm,详见上一份 handoff 的 P3 Task 3 段落:`git show 7e2f828:docs/handoff/backend.md`
- **Phase B3 P2** 推迟项(来自上一份 handoff,可能这次 P0 bug 修复时顺带做):
  - `enum ColorUpdate { Keep, Clear, Set(_) }` 替换 `"default"` sentinel
  - `HexColor` newtype
  - **`WhiteboardId` newtype + `WhiteboardId::for_project(&ProjectName)`** ← 如果 P0 根因是 wb_id stringly typed,优先做这个
  - `TaskEntity.status: String → TaskStatus`
  - IPC 边界 `status: String → TaskStatus`
  - `InvalidProjectName` 变体细分

## Resume 检查清单

- [ ] 读 `docs/collaboration/2026-04-14-project-whiteboard-set-color-and-draw-connection-broken.md` **全文**(密度高,15 分钟左右)
- [ ] 读 `docs/progress/backend.md` 确认 Next 里 P0 bug 还在(没被别人挑走)
- [ ] 跑 `stale_check`:Rust **248 passed** / 0 failed / 1 ignored;TS **238 passed** / 29 files
- [ ] `git status` 干净;`git log --oneline -3` 应是 `7f3e24d`(Task 4) → `c59cf7e`(Task 2) → `c91f7f7`(Task 1)
- [ ] 准备好 dev server 跑起来:`pnpm tauri dev`(因为修复必须 dev 手动验证)
- [ ] 准备好 SQLite 客户端(`sqlite3 ~/Library/Application\ Support/co.bunotes.super-tauri/keysight.db`),Phase 1 step 2 要查表
- [ ] 准备好 vault 的 file explorer,Phase 1 step 2 要看磁盘 .md 文件
- [ ] 如果状态不匹配 — **不要盲目继续**,先 ping 用户确认
