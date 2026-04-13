---
area: backend
last_updated: 2026-04-14T03:15:00+08:00
session_id: 32ed1a4a
status: ready-to-resume
stale_check: pnpm test -- --run 2>&1 | tail -5
---

# Handoff: backend

## 正在做的 Task

**Keysight Task 节点菜单接入** — 对应 `docs/progress/backend.md` > Next > "Keysight Task 节点菜单接入"

把 Task 作为第 6 个 NodeKind 加入 Phase B1 的 catalog。**纯 TS 增量扩张**,不动 Rust。scope 受限于"现有 Rust command 能支撑什么",上一 session 的预读已经查清具体能力清单(见下面「能力清单」段)。

## 已完成步骤(本 task 内部)

Task 菜单接入**还没开工**,代码零改动。前置依赖已全部就绪:

- [x] Phase B1 落地 — commits `6a37f25`(catalog type sketch)+ `29d1fb8`(catalog 驱动渲染)+ `2a79d21`(文档收尾)
- [x] **Task 预留路径已在 Phase B1 中设计好** — `NodeCapabilityCatalog.ts` 文件头注释(第 23-30 行段落)说明了后续扩张的 4 步:(1) NodeKind union 加 "task" (2) 更新相应 capability 的 applies_to (3) 新增 TaskNodeHandlers + NodeMenuConfig.task 变体 (4) EntityNode.tsx 的 case "task" 装配 handlers
- [x] **GraphView.menuHandlers.onCopyEntityUuidTitle 的 `case "task"` 已预留 return 占位** — 在 `src/components/keysight/GraphView.tsx` 里 `onCopyEntityUuidTitle` 函数里(搜索 `case "task":` 关键字),目前是 `return;` 占位。接入时要改为 `title = data.tasks.find((t) => t.id === entityId)?.title`
- [x] **上一 session 已完成能力边界预读**,避免下一 session 走子阶段 1 弯路(见「能力清单」+「开放问题」段)
- [ ] **卡在这里**:下一 session 从「下一步具体动作」子阶段 2 直接开始(子阶段 1 的能力分析已在上一 session 提前做完)

## 能力清单(上一 session 预读结论)

Task 菜单在 Phase B1 scope(纯 TS,不动 Rust)下能做 **3 个 capability**,其余全部留给后续 task:

| Capability | 给 Task? | 为什么 |
|---|:-:|---|
| `copy_uuid_title` | ✓ | Task 有 `title`,TS 层 `data.tasks.find(...)` + 现成 normalize pipeline |
| `move_to_section` | ✓ | `section_members` 表 schema 是通用 `entity_id`(`db.rs:58`),接受任意 entity kind,包括 Task。`section_add_member` command 现成 |
| `remove_from_group` | ✓ | 同上,`section_remove_member` command 现成 |
| `draw_connection` | ✗ | Phase A Edge 判别联合**编译期禁止** Task 作 from(`edge.rs:101-102` 有注释 `不存在 TaskLink 变体,task 不主动发边`),**不能**纳入 |
| `edit_title` | ✗ | 后端**无 `task_update` command**(grep `src-tauri/src/modules/keysight/` 只找到 `gen_task_id` / `TaskEntity` 模型 / parser,无 domain/commands 层写函数)。加这个能力要先扩 Rust,违反 B1-like 纯 TS scope,留给独立 task |
| `delete` | ✗ | 同上,无 `task_delete` command |
| `set_color` | ✗ | Task 无 `color` 字段(`models.rs:269-279` 只有 `id/title/content/whiteboardId/status/area/project`),留 Phase B2(Rust schema 变更) |
| `related` / `create_alias` / `jump_to_source_card` | ✗ | Card/Alias 专属,不适用 Task |

**结论**:Task 菜单在本 task 的 scope 下只有 3 项能力。这让工作量比原计划小很多 —— 整个 task 可能一个 session 半天就能落完。

**额外发现**(留给后续独立 task):
- **"Task edit title / delete 能力接入"** 值得开独立 task,要先在 Rust 补 `task_update` / `task_delete` domain 函数 + commands + 副作用矩阵登记,然后在 catalog 扩 applies_to,纯增量。可能合并到 Phase B2 一起做(反正 B2 就是 Rust schema + commands 扩张)。

## 下一步具体动作

### 子阶段 1 — 能力清单验证(跳过,已在上一 session 完成)

**注**:上一 session 已经做完子阶段 1 预读,结论在「能力清单」段。新 session 不需要重跑,但 **建议扫一眼** `src-tauri/src/modules/keysight/domain/edge.rs:101-102` 和 `src-tauri/src/modules/keysight/models.rs:266-279` 做 10 秒核对,防止代码已被其他 session 改动。

### 子阶段 2 — Type sketch 扩张(单 commit,无渲染改动)

1. **Edit `src/components/keysight/nodes/NodeCapabilityCatalog.ts`**:
   - `NodeKind` union 从 5 种扩到 6 种:
     ```ts
     export type NodeKind = "card" | "alias" | "note" | "question" | "section" | "task";
     ```
   - `NODE_CAPABILITIES` 里这 3 个 entry 的 `applies_to` 加 `"task"`:
     - `copy_uuid_title`(现是 `["card","alias","note","question","section"]`,加到 6 项)
     - `move_to_section`(现是 `["card","alias","note","question"]`,加到 5 项)
     - `remove_from_group`(同 `move_to_section`)
   - 新增 `TaskNodeHandlers`:
     ```ts
     export type TaskNodeHandlers = Pick<
       NodeCapabilityHandlerMap,
       "copy_uuid_title" | "move_to_section" | "remove_from_group"
     >;
     ```
   - `NodeMenuConfig` 判别联合加 task 变体:
     ```ts
     | { readonly kind: "task"; readonly handlers: TaskNodeHandlers }
     ```
   - 新增 type alias:
     ```ts
     export type TaskMenuConfig = Extract<NodeMenuConfig, { readonly kind: "task" }>;
     ```
   - 更新文件头注释(第 23-30 行),把 "Task 预留" 段改成 "Task 已接入(3 项能力:copy_uuid_title / move_to_section / remove_from_group;edit_title / delete / draw_connection 留给后续,见 progress/backend.md Next)"

2. **Edit `src/components/keysight/nodes/NodeContextMenu.tsx`**:
   - Re-export 列表加 `TaskMenuConfig`(找到现有 `export type { NodeMenuConfig, CardMenuConfig, ... }` 段,加一行)
   - **渲染逻辑不需要改**(catalog 驱动,新 NodeKind 自动 work)

3. 跑 `pnpm build`:期望类型扩张编译通过,渲染逻辑无改动。如果有 TS 错误,大概率是 `NodeMenuConfig` 判别联合新增变体后 EntityNode 的 `cardMenu` / `noteMenu` / etc 的 `Extract<NodeMenuConfig, { kind: "xxx" }>` 类型仍然正确 —— 这是应该的,Extract 只会匹配对应 kind,不受新 variant 影响。

4. Commit:`feat(keysight): Task 节点菜单 type sketch — catalog 扩张 NodeKind 含 task`

### 子阶段 3 — TaskNode 补 contextMenu prop + EntityNode 装配 taskMenu

5. **Read `src/components/keysight/nodes/TaskNode.tsx`** 现有结构,参照 `QuestionNode.tsx`(最接近的 pattern)看它是怎么渲染 NodeContextMenu 的:
   - QuestionNode.tsx 有 `contextMenu?: QuestionMenuConfig | null` prop,`menuSections?: SectionListItem[]`,`currentSectionId?: string | null` 三个 prop,然后 JSX 里 `{contextMenu && <NodeContextMenu menu={contextMenu} sections={menuSections ?? []} currentSectionId={currentSectionId ?? null} />}`。TaskNode 依样画葫芦。

6. **Edit `src/components/keysight/nodes/TaskNode.tsx`**:
   - 加 `contextMenu?: TaskMenuConfig | null` / `menuSections?: SectionListItem[]` / `currentSectionId?: string | null` 三个 prop
   - import `NodeContextMenu` + `TaskMenuConfig` + `SectionListItem` from "./NodeContextMenu"
   - 在 JSX 里加 `{contextMenu && <NodeContextMenu menu={contextMenu} sections={menuSections ?? []} currentSectionId={currentSectionId ?? null} />}`(放在合适的位置,参考 QuestionNode 的 `position: absolute` ⋯ 按钮布局)

7. **Edit `src/components/keysight/nodes/EntityNode.tsx`**:
   - 新增 `taskMenu` useMemo(放在现有 5 个 useMemo 之后),类型 `Extract<NodeMenuConfig, { kind: "task" }> | null`:
     ```ts
     const taskMenu = useMemo<Extract<NodeMenuConfig, { kind: "task" }> | null>(() => {
       if (!menuHandlers || entity.kind !== "task") return null;
       return {
         kind: "task",
         handlers: {
           copy_uuid_title: () => menuHandlers.onCopyEntityUuidTitle(entity.id, "task"),
           move_to_section: (sid) => menuHandlers.onMoveToSection(entity.id, sid),
           remove_from_group: () => menuHandlers.onRemoveFromGroup(entity.id),
         },
       };
     }, [menuHandlers, entity.id, entity.kind]);
     ```
   - `case "task":` 渲染分支里给 TaskNode 传 `contextMenu={taskMenu}` / `menuSections={menuSections}` / `currentSectionId={currentSectionId}`(现有 `case "task":` 在 `EntityNode.tsx:275-287` 附近,只传了基础 props,要补三个菜单 prop)

8. **Edit `src/components/keysight/GraphView.tsx`**:
   - `onCopyEntityUuidTitle` 的 `case "task":` 从 `return` 改为 `title = data.tasks.find((t) => t.id === entityId)?.title; break;`
   - deps 数组在 `[data.cards, data.notes, data.questions, data.aliases, data.sections, ...]` 里加 `data.tasks`
   - **不需要新增 handler 字段** — 本 task 的 3 个能力都复用已有 handler(`onCopyEntityUuidTitle` / `onMoveToSection` / `onRemoveFromGroup`)

9. 跑 `pnpm test -- --run` + `pnpm build`,期望全绿。如果 Task 菜单现有测试(若有)因 prop 变化失败,调整 TaskNode.test.tsx 构造函数。

10. Commit:`feat(keysight): Task 节点菜单接入 — copy_uuid_title / move_to_section / remove_from_group`

### 子阶段 4 — 测试扩充

11. **Edit `src/__tests__/components/keysight/nodes/NodeContextMenu.test.tsx`**:新增 `describe("variant='task'")` 套,按 section variant 的模板写:
    - 构造 `taskConfig: Extract<NodeMenuConfig, { kind: "task" }>` with 3 handlers vi.fn()
    - 测试:打开菜单显示 Copy UUID + title / Move to Section / (可能的)Remove from group
    - 测试:sections 为空时不显示 Move to Section
    - 测试:在 section 时显示 Remove from group,不在时不显示
    - 测试:点击每个项调用对应 handler
    - **边界断言**:Task 菜单**不含** Draw connection / Edit title / Delete / Set color / Related / Create alias / Jump to source card(白名单正确性 —— 这是最重要的断言,锁死能力边界)

12. **可选:Edit/新建 `src/__tests__/components/keysight/nodes/TaskNode.test.tsx`** — 加 "传入 contextMenu prop → 渲染 ⋯ 按钮" 的简单 case,参考 QuestionNode.test.tsx。如果 TaskNode 目前无 test file,看要不要为这一个 case 新建(可以先跳过,留到 Task 有更多专属测试需求时再建)。

13. 跑 `pnpm test` + `pnpm build` 全绿。

### 子阶段 5 — 收尾

14. `/harness-check-tests` 自查(本 task 纯 TS,只看前端测试覆盖)

15. **手动 Code Review 6 项**(L0 要求):
    - 测试覆盖:每个新 capability 有 presence + click + 白名单边界
    - 逻辑正确性:catalog 扩张未破坏现有 filter/sort/dispatch 逻辑,EntityNode 装配 pattern 一致
    - 回归风险:原 215 测试 + Task 新 case 全绿
    - I/O 正确性:N/A(无 Rust 改动)
    - IPC 类型安全:N/A(无新 command)
    - 建模强度:Task 纳入 NodeKind union 后仍保持防火墙(applies_to 精确到 3 项能力,不含不适用的 draw_connection / edit_title / delete)

16. 更新 `docs/progress/backend.md`:把 "Keysight Task 节点菜单接入" 从 Next 移到 Done 2026-04-{日期},新增一条 "Task edit_title / delete 能力接入"(或并入 Phase B2 描述)到 Next
17. 更新 `docs/handoff/backend.md` 为 `status: done`(本 task 完成后)
18. 追加 devlog + changelog
19. Commit:`docs(backend): Task 节点菜单接入完成 — progress/handoff/devlog 同步`

## 关键上下文(/new 之后会丢的东西)

### 本次会话的假设与决策

- **决策 1(scope 大幅收缩)**:Task 菜单接入在 Phase B1 scope 下只能实现 3 项能力(copy_uuid_title + move_to_section + remove_from_group)。原 Phase B1 sub-stage 1 时我曾预想 6 项能力,但预读发现后端无 `task_update` / `task_delete`,且 Edge 判别联合禁止 Task 作 from。**不扩 Rust,留给后续 task** —— 这是保持纯 TS 增量原则。
- **决策 2(不合并入 Phase B2)**:虽然 Task 的 `edit_title` / `delete` / `set_color` 都需要 Rust 变更,但它们分别属于不同维度(edit/delete 是 domain 层,set_color 是 schema 层)。**建议合并处理:Phase B2 扩充为 "Card/Question/Task color schema + Task edit/delete domain"**,一次 session 一并做。这个决策等下次 session 面对 B2 时再拍。
- **决策 3(上一 session 做完 sub-stage 1 预读)**:"能力清单" 段的结论是上一 session(32ed1a4a)在写 handoff 时顺带做的,目的是让下一 session 跳过子阶段 1 直接进 type sketch。如果怀疑代码漂移(比如其他 session 动过 edge.rs 或 models.rs),快速核对 `edge.rs:101-102` 和 `models.rs:266-279` 两处即可。
- **假设**:TaskNode.tsx 目前**不接受** contextMenu prop(EntityNode.tsx 的 `case "task"` 只传了 `task / style / lodLevel / selected / highlighted / dimmed`)。子阶段 3 的 step 5 会验证这一点 —— 如果 TaskNode 已经被其他 session 加了 contextMenu,跳过 step 6 的 prop 新增部分。

### 试过但不行的方案

无 — 本 task 未开工。

### 开放问题

1. **`edit_title` 是否值得在本 task 内补一个 minimal `task_update` Rust command?** — 如果补,Task 菜单能力从 3 项扩到 4 项,UX 更完整(用户可以从菜单里改 Task title,不用双击编辑)。但这违反"纯 TS 增量"原则,且应该和 `task_delete` 一起补而不是单独。**建议:不补,留给 Phase B2 的扩充版一起做。** 下一 session 子阶段 1 验证时再快速问用户一次。
2. **Task 菜单里是否需要 `Copy UUID + title` 之外的复制能力?** — 比如复制 Task 的 `id + status` 或 `id + area + project`?这些是 Task 专属字段。**建议:不加,保持 catalog 的单一 copy 能力语义。** 如果确实需要,另立 capability(如 `copy_task_metadata`,applies_to 只含 task)。
3. **canonical order 里 Task 的 3 项能力在哪里?** — 按现有 `order` 编号(copy=10, move=70, remove=80),Task 菜单会是:Copy UUID + title → [sep 或空白] → Move to Section → Remove from group。这样是 OK 的,但菜单视觉上比较空(3 项 + 中间一大段空白)。可以考虑:要不要在 Task 菜单里**隐藏** order 10-70 之间的空白? 实际上菜单渲染是连续的,不会有空白,只是 3 项紧凑排列 —— 不是问题。

这三个问题都**不是 blocker**,子阶段 2 可以直接开工,问题 1 和 2 在子阶段 1 验证结束时顺带问用户一次,问题 3 纯视觉细节,看做完 UI 再说。

## Resume 检查清单

- [ ] 读 `docs/progress/backend.md` 确认 "Keysight Task 节点菜单接入" 仍在 Next(没被别的 session 挑走)
- [ ] 读 `src/components/keysight/nodes/NodeCapabilityCatalog.ts` 确认 NodeKind union 仍是 5 种(未被其他 session 扩过)
- [ ] 核对 `src-tauri/src/modules/keysight/domain/edge.rs:101-102`(TaskLink 编译期禁止)和 `src-tauri/src/modules/keysight/models.rs:266-279`(TaskEntity 字段),本 handoff 「能力清单」的结论应仍有效
- [ ] 跑 stale_check:`pnpm test -- --run 2>&1 | tail -5`,期望 `Tests  215 passed (215)` 和 `Test Files  29 passed (29)`
- [ ] `git status` 干净;`git log --oneline -1` 应是 `2a79d21 docs(backend): Phase B1 完成 — progress/handoff/devlog 同步`
- [ ] 如果状态不匹配 — **不要盲目继续**,先 ping 用户确认
