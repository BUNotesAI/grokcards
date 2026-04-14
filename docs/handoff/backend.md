---
area: backend
last_updated: 2026-04-14T12:06:00+08:00
session_id: eca60756
status: ready-to-resume
stale_check: cargo test --manifest-path src-tauri/Cargo.toml --workspace 2>&1 | grep "test result" | head -1 && pnpm test -- --run 2>&1 | tail -5
---

# Handoff: backend

## 正在做的 Task 集合(4 条,按 P0→P3 顺序)

**Task feature 端到端可用化** — 对应 `docs/progress/backend.md` > Next > "Task 前端 UI 入口 + edit_title inline editor" + 新增 3 条(whiteboard 枚举 / kanban view / list_whiteboards 修复)。

本 session 刚刚完成的工作:
- Task 菜单接入(5 commit)
- Phase B2 task CRUD + color 贯穿 + 嵌套 whiteboard sync(7 commit)
- Phase B2 post-review P0/P1 fixes(1 commit,`874ae95`)

当前 DB / 文件层已经**数据安全干净**,但**UI 集成有 4 个缺口**挡住真实使用。这 4 条是下次 session 的核心任务。

## 已完成步骤(前置上下文 — 不是这 4 条的步骤)

- [x] 所有 Rust 层:`domain::task::{create,update,delete,get,query_all}` 齐全,测试 245 绿(task 25 / parser 8 / sync 8 / card set_color 3 / question color 3 / 其他无变化)
- [x] 所有 IPC 命令:`task_create / task_update / task_delete / task_set_color / card_set_color / question_update(+color)` 已注册 + bindings.ts 可用 — commit `d6c6f18` + `133e2b9`
- [x] 所有 TS 菜单:Task 菜单 5 项能力(copy / move / remove / set_color / delete)可用,前提 task 已渲染 — commit `bf96491`
- [x] `ProjectName::new` 严格校验(不 silent trim / 禁 NUL / 控制字符 / dots-only / Windows reserved / 长度 ≤ 100)— commit `874ae95`
- [x] `parser::write_frontmatter` / `write_color_frontmatter` 失败时 loud 报错(之前会静默销毁 frontmatter)— commit `874ae95`
- [x] `task::update` 识别 `"default"` sentinel 清空 color + 严格解析 legacy status — commit `874ae95`
- [ ] **卡在这里**:下面 4 条是 UI 集成,尚未开始

## 下一步具体动作(4 条,P0 先做)

### P0 — Task 1:`list_whiteboards` 递归 `projects/*`(纯 Rust)

**问题**:`src-tauri/src/modules/keysight/domain/overview.rs:130` 的 `list_whiteboards` 只调 `fs.list_first_level_dirs("whiteboard")`(`src-tauri/src/modules/keysight/vault_fs.rs:112`),只扫一级子目录 → 返回 `"projects"` 作为一个假 whiteboard,真实的 `projects/super-tauri` 只有在 DB 已经有 task 时才通过 SQL GROUP BY 副路径出现。空 project whiteboard 根本不会显示。

**具体动作**:

1. 读 `src-tauri/src/modules/keysight/vault_fs.rs:12-22` 看 `VaultFs` trait 方法列表,决定是扩 `list_first_level_dirs` 还是新加 `list_project_whiteboards(&self) -> Result<Vec<String>, KeysightError>`(推荐新加独立方法,语义清晰)
2. 在 `VaultFs` trait 加方法签名 + `RealVaultFs`(`vault_fs.rs:112-133` 附近)+ `MockVaultFs`(`vault_fs.rs:201-250` 附近)各自实现:
   - Real:`std::fs::read_dir("{vault}/whiteboard/projects")` → 过滤出 dir entries → 返回名字 list(不含 `projects/` 前缀,只返回 project name 如 `"super-tauri"`)
   - Mock:基于 dirs HashSet 或 files map 推导(同时要更新 `with_file` 或加 `with_dir` 方法)
3. 改 `src-tauri/src/modules/keysight/domain/overview.rs:134`:
   - `fs.list_first_level_dirs("whiteboard")` 过滤掉 `"projects"` 字面(不再进列表)
   - 新增 `fs.list_project_whiteboards()` → 每个 `name` 转成 wb_id `format!("projects/{name}")`
   - 两个集合 union 后再和 DB 查询结果合并
4. 读现有的 overview 测试(`overview.rs` 内的 `#[cfg(test)]` 或 `tests/` 下)看测试结构 → 加 3 个新测试:(a) project whiteboards 枚举 (b) 排除裸 `projects` (c) 空 project dir 也出现
5. 跑 `cargo test --manifest-path src-tauri/Cargo.toml --workspace` 全绿

**风险**:可能发现 `WhiteboardSummary` 的 `whiteboard_id` 字段在前端某处被假设为扁平字符串(无 `/`)— 需要 grep TS 侧 `whiteboardId` 使用点,确认 URL 编码、路由、sidebar 树渲染都能处理 `projects/xxx`。

### P0 — Task 2:新建 Task 的 UI 入口(纯 TS)

**问题**:前端没有按钮调用 `commands.taskCreate`。用户只能手写 md 文件或通过 devtools 调命令。

**具体动作**:

1. 读 `src/components/keysight/GraphToolbar.tsx` 全文,找现有的 "新建 Note" / "新建 Question" 按钮作为 template(Phase B1 之前的 session 可能已经建立这种 pattern,看 `2026-04-13` devlog 里的 `Keysight toolbar 补 Question 创建入口`)
2. 按相同 pattern 加 "新建 Task" 按钮:
   - 参数需要:`project` + `title` + 默认 `status = "next"`
   - 最简模式:toolbar 按钮点击后弹一个 modal / popover,两个 input(project + title)+ 一个 submit
   - 进阶:project 字段做成带 autocomplete 的 input,datalist 填现有 project 名(从 `list_whiteboards` 结果里过滤 `projects/*` 的 suffix)
3. Submit handler:`await commands.taskCreate(project, title, null, "next", null, null)` → 成功后 `queryClient.invalidateQueries()` + 关闭 modal + 可选:把新 task 位置设到 viewport 中心(调 `layoutSetPosition`)
4. 可能需要切换当前 whiteboard 到对应 project whiteboard(`projects/{project}`)—— 依赖 Task 1 的 `list_whiteboards` 返回结果,否则新建的 task 看不见
5. 组件测试:`src/__tests__/components/keysight/GraphToolbar.test.tsx`(若存在)加 case:点按钮 → mock commands.taskCreate 被调用 with 正确参数

**依赖**:**Task 1 必须先做完**,否则新建的 task 没有对应 whiteboard 可显示,测试不了端到端。

### P1 — Task 4:Task edit_title inline editor(纯 TS)

**问题**:TaskNode.tsx 没有 inline editing 支持,所以 Task 菜单无法提供 Edit title 能力。`NodeCapabilityCatalog.ts` 的 `edit_title.applies_to` 故意不含 task,防火墙锁死。

**具体动作**(按依赖顺序):

1. 读 `src/components/keysight/nodes/QuestionNode.tsx` 全文作为 template —— 它有完整的 inline editor 模式(`editingField` prop + `draftTitle` state + `onCommitEdit` / `onCancelEdit` props + 输入时阻止拖拽的 `onMouseDown={e => e.stopPropagation()}`)
2. 读 `src/components/keysight/nodes/TaskNode.tsx` 全文,对照 QuestionNode 的形状 → 新增:
   - prop:`editingField?: "task-title" \| null`(body 留到后续 session,本次只做 title)
   - prop:`onStartEdit?`, `onCommitEdit?`, `onCancelEdit?`
   - state:`draftTitle` useState + useEffect 同步 `task.title`
   - 替换标题 `<h3>` 为 editing 时的 `<input>`,Enter 提交 / Escape 取消 / onBlur 提交
3. `src/components/keysight/nodes/EntityNode.tsx` 的 `EditingField` 类型 union(line ~66)加 `"task-title"`;`case "task":` render 分支传 `editingField={taskEditingField}` + 3 个 edit prop
4. `src/components/keysight/GraphView.tsx` 的 `NodeContextMenuHandlers` 接口(line ~38)加 `onEditTaskTitle: (id: string) => void`;`menuHandlers` useMemo 实现 `onEditTaskTitle: (id) => setEditing({id, field: "task-title"})`;`onCommitEdit` 分支(已有的 switch)加 case `"task-title": await unwrapCommand(commands.taskUpdate(id, value, null, null, null, null))`
5. `NodeCapabilityCatalog.ts`:`edit_title.applies_to` 从 `["note","question"]` 扩到 `["note","question","task"]`;`TaskNodeHandlers` Pick 加 `"edit_title"`(从 5 项 → 6 项)
6. `EntityNode.tsx` 的 `taskMenu` useMemo 加 `edit_title: () => menuHandlers.onEditTaskTitle(entity.id)`
7. 测试更新:
   - `src/__tests__/components/keysight/nodes/NodeContextMenu.test.tsx` 的 `variant='task'` 块:taskConfig.handlers 加 `edit_title: vi.fn()`;加 click `Edit title → edit_title handler 被调用` 的 case;白名单边界 assertion 移除 `Edit title` 不应出现的断言
   - `src/__tests__/components/keysight/nodes/TaskNode.test.tsx`:加 `editingField='task-title' → 渲染 input` 测试
   - `src/__tests__/components/keysight/nodes/EntityNode.test.tsx`:menuHandlers mock 加 `onEditTaskTitle: vi.fn()`
   - `src/__tests__/components/keysight/GraphView.test.tsx`:如有 menuHandlers mock 也需补
8. 跑 `pnpm test -- --run` + `pnpm build` 全绿

**没有 backend 改动** — 只改 TS 前端。复用已有的 `task_update` command。

### P3 — Task 3:Kanban view(大 feature,需要先 brainstorm)

**问题**:用户的核心业务需求("类似 kanban 的地方,能统计项目的任务进展,点击 task 进去就可以对 task 做 note/alias 连线标注")完全没实装。Phase B2 只做到"Task 可以作为一个 entity 存在于画布"。

**建议开工前先 brainstorm 的决策点**:

1. **Kanban view 的 UI 形态**:
   - (a) 独立页面 / 路由(如 `/projects/{name}/kanban`),和 whiteboard canvas 完全分开的视图模式
   - (b) project whiteboard 的默认渲染就是 kanban layout,canvas 作为另一个视图切换
   - (c) kanban 是 canvas 的一个 overlay / layout strategy,tasks 自动按 status 排列在固定列区域,其他 entity 可以自由摆
2. **Status columns 怎么实现**:
   - (a) 4 个 Section entity(Next / Active / Blocked / Done),task 通过 `section_members` 表归属
   - (b) 固定视觉列(前端纯渲染),不走 section 机制
   - (c) 混合:section 记录归属,视觉列从 section 推导
3. **拖拽改 status**:`@dnd-kit` 还是 HTML5 drag?需要调 `task_update` with new status 但不 rename 文件
4. **点击 task 进入标注模式**:是切换到 canvas 视图并 pan 到该 task 邻居?还是在 kanban 视图里 overlay 一个侧边面板?
5. **Kanban view 和 "新建 task" UI 的关系**:新建的 task 直接出现在当前 kanban 的 Next 列?

**具体动作**(建议独立开一个 session,先走 brainstorm 再实施):

1. 先用 brainstorming skill 和用户澄清上述 5 个决策点
2. 根据决策画一个 mockup / wireframe(可以是 markdown ASCII 或截图 draft)
3. 按 mockup 切分子阶段
4. 实施 + 测试

**不建议和 P0/P1 同 session 做**,规模差一个数量级。

## 关键上下文(/new 之后会丢的东西)

### 本次会话的假设与决策

- **方案 A(嵌套 whiteboard)** 确认采纳:一个 project = 一个 kanban whiteboard,`whiteboard_id = "projects/{name}"`,tasks 原生挂在上面。不做方案 B(一个 task 跨多 whiteboard)。
- **ProjectName 是内部 Rust newtype**,IPC 边界仍是 `project: String`,命令层 `task_create` 做 `ProjectName::new` 校验
- **TaskStatus 在 IPC 边界仍是 `String`**(不是 enum)—— Phase B3 会改(参见下方 P2 推迟项)。暂时用 serde_json 做 string→enum 解析
- **Task 的 `edit_title` 故意在 B2 没接** —— TaskNode 缺 inline editor,NodeCapabilityCatalog `edit_title.applies_to` 不含 task。Task 4 就是把这个口子补上
- **"default" sentinel** 是跨模块协议,5 个 domain 模块共享字符串 `"default"` 表示清空 color。Phase B3 想换成 `enum ColorUpdate`,暂时保留字符串约定
- **`list_whiteboards` 没在 B2 改** —— 发现这个问题是在 B2 ship 之后 review 时,推到下次 session。Task 1 就是修这个

### 试过但不行的方案

- **把 Task 接入 B2 一起做 edit_title**:发现 TaskNode 没有 inline editor,接入 edit_title 需要先扩 TaskNode UI(~80 行),超出 B2 "纯 menu 接入 + color schema" scope,推迟
- **`sync::derive_whiteboard_id` 返回 `"projects"` 给 `projects/loose.md`**:Phase B2 初版这样,但 `"projects"` 变成影子 whiteboard,P1-7 修掉,现在 quarantine 到 `wb_root`

### 开放问题

- **Task 3(Kanban view)的 5 个决策点**见上,建议开 session 先走 brainstorm
- **`list_whiteboards` 修复后前端 whiteboard selector 能否正确处理带 `/` 的 wb_id?**(URL 编码?sidebar 树?)需要 grep TS 侧 `whiteboardId` 使用点验证。Task 1 实施时顺带查。
- **新建 Task 的 UX:modal 还是 popover 还是右键 empty canvas 菜单?** 推荐最简 modal(Task 2),等用户用上再优化
- **Project 字段的 autocomplete 是否要做?** 推荐 V1 纯文本,V2 再加 datalist

### 已知 P2 推迟项(Phase B3,下下个 session)

Review 报告里的 P2 建模改进,本 session 没做,列在这里防止遗忘:
- `enum ColorUpdate { Keep, Clear, Set(_) }` 替换 `"default"` sentinel —— 跨 5 模块的 stringly-typed 协议彻底消除
- `HexColor` newtype(`^#[0-9a-fA-F]{6}$` 校验)
- `WhiteboardId` newtype + `WhiteboardId::for_project(&ProjectName)`
- `TaskEntity.status: String → TaskStatus`(DTO 边界 parse once)
- `task_create/task_update` IPC 边界 `status: String → TaskStatus`(tauri-specta 已 derive,让 TS 编译期强制)
- `InvalidProjectName` 变体细分(`Empty / IllegalChar / Missing`)

## Resume 检查清单

- [ ] 读 `docs/progress/backend.md` 确认 Next 里的 "Task 前端 UI 入口..." 条目还在(没被别的 session 挑走)
- [ ] 读 `docs/progress/keysight.md` 看 KeySight 迁移全景,确认本 handoff 的 4 条是否已经被 keysight.md 的更高层 Phase 规划覆盖(如果有冲突先 ping 用户)
- [ ] 跑 `stale_check`:`cargo test --workspace` 应 245 passed / 0 failed / 1 ignored;`pnpm test` 应 230 passed / 29 files
- [ ] `git status` 干净;`git log --oneline -1` 应是 `874ae95 fix(keysight): Phase B2 post-review — P0 数据安全 + P1 沉默错误修复`
- [ ] 读 `src/components/keysight/GraphToolbar.tsx` 全文熟悉现有 toolbar 按钮 pattern(Task 2 依赖)
- [ ] 读 `src/components/keysight/nodes/QuestionNode.tsx` 全文熟悉 inline editor pattern(Task 4 依赖)
- [ ] 读 `src-tauri/src/modules/keysight/domain/overview.rs:125-200` 熟悉 `list_whiteboards` 和 `WhiteboardSummary`(Task 1 依赖)
- [ ] 如果状态不匹配 — **不要盲目继续**,先 ping 用户确认
