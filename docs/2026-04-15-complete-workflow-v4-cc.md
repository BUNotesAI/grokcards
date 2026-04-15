---
date: 2026-04-15
topic: Complete Task-Centric Development Workflow (v4)
status: 设计草案 v1
authors: [cc]
supersedes:
  - docs/collaboration/2026-04-15-agent-spec-quality-integration-v1-cc.md
  - docs/collaboration/2026-04-15-agent-spec-quality-integration-v2-codex.md
  - docs/collaboration/2026-04-15-agent-spec-quality-integration-v3-cc.md
references:
  - "V3 笔记: agent-slipbox-v3/projects/keysight/2026-04-04 2029 cc 项目管理体系 — 从混乱到四关注点.md"
  - https://github.com/ZhangHanDong/agent-spec
---

# Complete Task-Centric Development Workflow (v4)

> **性质**:本文档是 **新建项目的主要启动依据**,同时是现有项目迁移到 v4 的 reference。它整合了:
>
> 1. V3 "四关注点模型"(设计/进度/质量/日志)的基础架构
> 2. v3 agent-spec 融入质量体系方案(Task Contract + lifecycle 验证)
> 3. **新增** V1.1 实战经验推出的 task-centric 文档组织
> 4. **新增** 知识沉淀闭环(lessons + mempal KG triples)
> 5. **新增** skills 显式声明 + rules 任务级提醒
>
> **核心转变**:从 "docs/ 平铺 + task 是 checkbox" 变成 **"task 是 entity + task.md 的 frontmatter 是该 task 所有 artifact 的 index"**。Task 不再是一行文字,是一个 **任务总入口**。

---

## 一、背景:为什么需要 v4

### V3 + agent-spec v3 的剩余问题

V3 四关注点模型解决了"docs 怎么组织"(Design / Progress / Quality / Records),但在 **Progress → Quality → Records 的协作**上还有两个断层:

1. **Artifact 散落**:一个 task 的 spec / design / plan / review cycles 分散在 `docs/design.md` + `docs/progress/{area}.md` + `docs/collaboration/*.md` + `docs/devlog/*.md` + `docs/handoff/{area}.md` + git log,没有 **task 级的 index**。开发者/AI 要串联一个 task 的完整上下文,需要跨多个目录手动拼接
2. **知识沉淀无闭环**:task 完成后,踩坑经验依赖 `LESSONS.md`(模块级,不和 task 绑定),知识图谱依赖 `mempal` 外部服务(内容不回流到项目),**没有强制的沉淀步骤**让 task 的学习变成未来可用的资产

V1.1 Kanban Subtask 实战完美暴露了这两点:一个 V1.1 task 跨 2 轮 codex review + 2 轮 CC 响应 = 5 份 collaboration doc,另有 11 个 commit + 1 个 devlog 段,想串联起来只能靠 commit message 里的文字引用。

### v4 的核心转变

**task.md 本身就是 task 的 index**:

- 每个 task 在 vault 里已经有一个 markdown 文件(V1.1 用户业务需求决定的存储)
- 把 task.md 的 **frontmatter 扩展成结构化索引**:spec / design / plan / review / commits / lessons / mempal / skills / rules / handoff 全部作为字段
- 所有 task 相关的 artifact 都在 **`whiteboard/projects/{project}/docs/tasks/{task_id}/`** 子目录里,和 task.md 同一个 project 层,用相对路径引用
- 项目级 artifact(design.md / progress/{area}.md / devlog/)同样进 vault 的 `docs/` 子目录

**agent-spec 无缝融入**:
- Task Contract(`.spec.md`)就是 `docs/tasks/{task_id}/spec.md`,由 task frontmatter 的 `spec:` 字段指向
- `agent-spec lifecycle` 在质量关卡时验证(不变)
- `project.spec` 留在 repo(项目级强制约束,给 agent-spec CLI 用)

**开发闭环两端加强**:
- **task 开始前**:`rules` + `skills` + `spec` 作为显式 context 加载,杜绝"默认知识漂移"
- **task 结束后**:`commits` + `lessons` + `mempal` 作为显式沉淀,杜绝"做完就忘"

---

## 二、核心理念

| 理念 | 含义 | 为什么 |
|---|---|---|
| **Task 作为 entity,不是 checkbox** | task.md 是第一等公民,frontmatter 结构化索引所有相关 artifact | 从"工程师凭记忆串联"升级到"打开 task.md 就看到全景" |
| **Vault-centric docs** | 所有工作文档(design / progress / devlog / handoff / review)都进 vault | Obsidian 原生能力(反向链接 / 图谱视图 / 搜索)被用起来,用超级工具的本来用法管理超级工具的开发 |
| **per-task artifact 子目录** | `docs/tasks/{task_id}/` 下放该 task 的 spec / design / plan / reviews | 强 locality:一个 task 的所有文档物理上 co-located,cross-task 独立,便于 archive/move |
| **显式 context 加载(开始前)** | `rules` / `skills` / `spec` 写进 frontmatter,task 启动时 agent 显式读取 | 对抗"默认加载漂移":claude code 虽然自动加载 skills,但显式写进 task 让 review 时有据可查 |
| **显式知识沉淀(结束后)** | `commits` / `lessons` / `mempal` 强制在 done 时填充 | 对抗"做完就忘":每个 task 完成必须沉淀经验到 LESSONS 和 KG,冗余存储在 task 内方便回顾 |
| **冗余是有意的** | `rules` 和 CLAUDE.md 部分重复,`mempal` 三元组和 MCP 服务重复 | 简洁的任务级提醒 > 长远的单一来源,task 上下文内能看到的就是 in-context 强化 |
| **agent-spec 机械验证** | Task Contract + `lifecycle` + `guard` 给出机械 gate | Contract 是测试的结构化前置,`lifecycle` 验证声明的 Scenario 确实被测试覆盖 |

---

## 三、架构概览

```
┌──────────────────────────────────────────────────────────────────┐
│                      Task-Centric Workflow v4                    │
│                                                                  │
│                        ┌──────────────────┐                      │
│                        │    task.md       │  ←── 总入口          │
│                        │   (frontmatter)  │                      │
│                        └────────┬─────────┘                      │
│                                 │                                │
│          ┌──────────────────────┼──────────────────────┐         │
│          │                      │                      │         │
│   ┌──────▼──────┐        ┌──────▼──────┐        ┌──────▼──────┐  │
│   │  开始前 CTX │        │   开发中    │        │  结束后沉淀 │  │
│   │             │        │             │        │             │  │
│   │ • spec      │        │ • plan      │        │ • commits   │  │
│   │ • design    │        │ • handoff   │        │ • lessons   │  │
│   │ • rules     │        │ • review    │        │ • mempal KG │  │
│   │ • skills    │        │   (多轮)    │        │             │  │
│   └─────────────┘        └─────────────┘        └─────────────┘  │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                         Quality Gates                            │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐      │
│  │ agent-spec     │  │ Code Review    │  │ pre-commit     │      │
│  │ lifecycle      │  │ (6 项)         │  │ guard + test   │      │
│  └────────────────┘  └────────────────┘  └────────────────┘      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 核心原则

1. **Task 是唯一的 index**:打开 task.md 能看到这个 task 的一切(spec / design / plan / reviews / commits / lessons / KG triples / handoff / skills / rules)
2. **Vault 是 doc 的家**:所有 working doc(不是 repo source code)在 vault 的 `whiteboard/projects/{project}/docs/` 下
3. **Per-task locality**:task 级 artifact 在 `docs/tasks/{task_id}/` 子目录,跨 task 的索引通过 task frontmatter 相互链接
4. **Project.spec 留在 repo**:agent-spec 的项目级强制约束(禁止路径 / 质量分数)属于 repo 语义,不进 vault
5. **Frontmatter 是约定,不是强制**:`/task-view` / `/task-done` 等命令依赖 frontmatter 字段,但 lint 只在 slash command 运行时做健康检查

---

## 四、文件结构

### Vault 端(知识 / 任务文档)

```
~/Documents/obsidian_workspace/agent-slipbox-v3/
└── whiteboard/projects/{project}/
    ├── task_xxx 【TASK】task title.md        ← task entity(V1.1 已有)
    ├── task_yyy 【TASK】task title.md
    ├── ...
    └── docs/                                 ← 所有 working doc
        ├── design.md                         ← 项目级 design(从 repo 迁入)
        ├── progress/                         ← 按功能域拆文件(v3 不变)
        │   ├── backend.md
        │   ├── frontend.md
        │   └── ipc.md
        ├── devlog/                           ← 按日期拆(v3 不变)
        │   ├── 2026-04-14.md
        │   └── 2026-04-15.md
        ├── handoff/                          ← 功能域级 handoff(v3 不变)
        │   └── backend.md
        ├── tasks/                            ← task 级 artifact 子目录(新增)
        │   └── {task_id}/
        │       ├── spec.md                   ← agent-spec Task Contract
        │       ├── design-v1-cc.md           ← 设计 v1
        │       ├── design-v2-cc.md           ← 设计 v2 (current)
        │       ├── plan.md                   ← 实施计划
        │       ├── reviews/
        │       │   ├── r1-codex.md           ← round 1 codex 评审
        │       │   ├── r1-cc.md              ← round 1 cc 响应
        │       │   ├── r2-codex.md           ← round 2 codex 评审
        │       │   └── r2-cc-final.md        ← round 2 cc 最终(current)
        │       └── handoff.md                ← task 级 handoff(session 切换用)
        └── lessons/                          ← 项目级 lessons(跨 task 可复用的)
            ├── modules/
            │   └── {module_name}.md          ← 模块级踩坑集合
            └── cross-cutting.md              ← 跨模块通用教训
```

### Repo 端(source code + 项目指令)

```
super-tauri/
├── CLAUDE.md                                 ← 项目指令(保留,给 Claude Code 读)
├── specs/
│   └── project.spec                          ← agent-spec 项目级强制约束(不进 vault)
├── src-tauri/                                ← Rust source
├── src/                                      ← TS source
├── .claude/                                  ← local Claude Code config(不提交)
└── docs/                                     ← **只留 meta 文档**
    ├── 2026-04-15-complete-workflow-v4-cc.md ← 本方案(workflow spec 本身)
    └── README.md                             ← 可选(如果需要)
```

**关键规则**:
- `docs/design.md`、`docs/progress/`、`docs/devlog/`、`docs/handoff/`、`docs/collaboration/` **全部搬迁到 vault** `whiteboard/projects/{project}/docs/`
- 迁移后 repo 的 `docs/` 只保留 **meta 文档**(workflow spec 本身、README)
- `CLAUDE.md` 和 `specs/project.spec` 留在 repo,因为它们是**项目配置**,不是 working doc

---

## 五、Task Frontmatter Schema

### 完整字段(按语义分组)

```yaml
---
# === 基础字段(V1.1 已有,不变) ===
type: project-task
id: task_3a3f9a64
status: inbox | next | active | blocked | done
project: super-tauri
color: "#bdb2ff"            # 可选,Kanban 卡片背景色
area: backend                # 可选,功能域
subtasks:                    # V1.1 body checklist 的 frontmatter 映射(可选,body 是真源)
  - "子步骤 1 描述"
  - "子步骤 2 描述"

# === 开始前 context(在 active 前填充) ===
spec: "docs/tasks/task_3a3f9a64/spec.md"           # agent-spec Task Contract(单值,指向 current)
design: "docs/tasks/task_3a3f9a64/design-v2-cc.md" # 设计文档(单值,指向 current/final)
plan: "docs/tasks/task_3a3f9a64/plan.md"           # 实施计划(单值,可能不单独写,见 §7.3)

rules:                                              # 任务级 rule 提醒(和 CLAUDE.md 重复有意)
  - "Subtask 写路径必须走 Rust,TS 不得 parse checklist"
  - "MultiBlockChecklist 用 typed error,禁止字符串匹配"
  - "TDD Red → 用户确认 → Green,本 session 已授权跳过"

skills:                                             # Claude Code skills 显式声明
  - rust-errors                                     # 错误处理
  - rust-modeling                                   # 领域建模
  - react-hook-form-zod                             # 表单校验

# === 开发中 artifact ===
review:                                             # 多轮评审,append-only 列表
  - docs/tasks/task_3a3f9a64/reviews/r1-codex.md
  - docs/tasks/task_3a3f9a64/reviews/r1-cc.md
  - docs/tasks/task_3a3f9a64/reviews/r2-codex.md
  - docs/tasks/task_3a3f9a64/reviews/r2-cc-final.md # 最后一项 = current

handoff: "docs/tasks/task_3a3f9a64/handoff.md"     # task 级 handoff(切 session 时填)

# === 结束后沉淀(done 时填充) ===
commits:                                            # git log 快照(含 message)
  - hash: 771e30c
    subject: "refactor(app_error): struct variants + MultiBlockChecklist variant (V1.1 Phase 6.0)"
  - hash: 41674b3
    subject: "feat(keysight): parse_task_checklist + Subtask struct (V1.1 Phase 6.1)"
  # ...

lessons:                                            # 踩坑经验(短句)
  - "rusqlite 不走 serde,FromSql 必须手工 impl"
  - "task::update 的 None 语义是'保留原值',清空要发 'default' sentinel"
  - "PointerSensor activationConstraint 天然解决 click/drag 冲突,不需要 drag handle"

mempal:                                             # KG 三元组(冗余存一份,实际在 mempal MCP)
  - subject: "rusqlite"
    predicate: "反序列化机制"
    object: "自有 FromSql trait,非 serde"
  - subject: "V1.1 Kanban Subtask"
    predicate: "触发的 refactor"
    object: "AppError 从 tuple variants 改为 struct variants"
  - subject: "@dnd-kit PointerSensor"
    predicate: "解决的问题"
    object: "double-click/drag 事件冲突(activationConstraint 8px)"
---

# 【TASK】task title

(body: 可自由写,V1.1 subtasks = body 的 GFM checklist)

- [ ] subtask 1
- [x] subtask 2
```

### 字段详细说明

| 字段 | 类型 | 必需 | 填充时机 | 说明 |
|---|---|---|---|---|
| `type` | string | ✓ | 新建 | 固定 `project-task` |
| `id` | string | ✓ | 新建 | `task_{8位hex}`,V1.1 已有 |
| `status` | enum | ✓ | 变化中 | `inbox / next / active / blocked / done` |
| `project` | string | ✓ | 新建 | V1.1 已有 |
| `color` | hex string | ✗ | 任意 | V1.1 Kanban 卡片色 |
| `area` | string | ✗ | 新建 | 功能域,新增(V1.1 已支持) |
| `subtasks` | list[string] | ✗ | 新建 / 实施中 | 可选,和 body checklist 重复。**body 是真源**(V1.1 parser),frontmatter 只是备份索引 |
| `spec` | path | 推荐 | spec 阶段 | 指向当前版本的 spec.md(agent-spec Task Contract) |
| `design` | path | 推荐 | 设计阶段 | 指向**当前/最终**版本的 design doc |
| `plan` | path | ✗ | 设计阶段 | 实施计划;简单 task 可省 |
| `rules` | list[string] | ✗ | 新建 / active 前 | 本 task 特别提醒的规则,短句,和 CLAUDE.md 有意重复 |
| `skills` | list[string] | ✗ | 新建 / active 前 | 需要的 Claude Code skill 名称,显式声明 |
| `review` | list[path] | ✗ | 评审中 | append-only 列表,每轮 review 追加,**最后一项 = current** |
| `handoff` | path | ✗ | session 切换 | task 级 handoff 文件,切 session 时填写 |
| `commits` | list[{hash, subject}] | 推荐 | done 时 | git log 快照,从 task_id 相关 commit 提取 |
| `lessons` | list[string] | 推荐 | done 时 | 踩坑经验短句,同时沉淀到 `docs/lessons/` |
| `mempal` | list[{subject, predicate, object}] | 推荐 | done 时 | KG 三元组,同时 ingest 到 mempal MCP |

### 版本管理约定

**单值字段**(`spec / design / plan / handoff`):
- 始终指向**当前 / 最终**版本
- 历史版本不从 frontmatter 引用,但物理文件保留(命名含 `-v1 / -v2` 后缀)
- 升版本时改 frontmatter 路径 + 保留旧文件(不 rm)

**列表字段**(`review`):
- append-only,新评审追加到末尾
- **最后一项** = 当前 final
- 每个 review doc 自己的 frontmatter 带 `status: superseded | current | final`,作为机器可读的状态(`/task-view` 用)

**路径规范**:
- 所有路径相对 **task.md 所在目录**(即 `whiteboard/projects/{project}/`)
- `docs/tasks/{task_id}/...` 是规范路径
- 不用绝对路径 / URL

---

## 六、Task 生命周期

### 状态机

```
         ┌─────────┐
         │  inbox  │  ← /task-new (默认)
         └────┬────┘
              │  优先级确定
         ┌────▼────┐
         │  next   │
         └────┬────┘
              │  开始开发前:/task-kickoff(填 spec / design / rules / skills)
         ┌────▼────┐
         │ active  │  ← 实际开发(TDD Red → Green → Refactor)
         └────┬────┘
              │
    ┌─────────┼──────────┐
    │         │          │
┌───▼───┐ ┌───▼───┐ ┌────▼────┐
│blocked│ │ active│ │  done   │  ← /task-done(commits/lessons/mempal 沉淀)
└───┬───┘ │(多轮  │ └─────────┘
    │     │review)│
    └──→──┘
```

### 阶段详解

#### 阶段 1: 新建(inbox/next)

**触发**:Kanban UI "+ New task" 或 `/task-new {title}`

**动作**:
- 生成 task_id
- 创建 `{task_id} 【TASK】{title}.md` 基础 frontmatter
- **同时创建** `docs/tasks/{task_id}/` 子目录(空)
- 默认 status = `inbox`(未排期) 或 `next`(已排期)

**frontmatter 初始状态**:
```yaml
---
type: project-task
id: task_xxx
status: inbox
project: {project}
---
# 【TASK】{title}
```

#### 阶段 2: Kickoff(active 前)

**触发**:用户把 task 从 next 拖到 active,或 `/task-kickoff {task_id}`

**动作**(可半自动):
1. **写 spec**:调 `/task-spec {task_id}` → 创建 `docs/tasks/{task_id}/spec.md`(agent-spec Task Contract 格式) → 填 frontmatter `spec:`
2. **写 design**(复杂 task):调 `/task-design {task_id}` → 创建 `docs/tasks/{task_id}/design-v1-cc.md` → 填 frontmatter `design:`
3. **声明 skills**:agent 根据 task 性质推荐,用户确认后写入 `skills:` 字段
4. **抽 rules**:从 CLAUDE.md / spec 里提炼本 task 特别要注意的 3-5 条,写入 `rules:` 字段
5. 状态切 `active`

**Rules 从哪来**(故意冗余的理由):
- CLAUDE.md 是完整规则库(几千行),task 启动时不可能全记住
- `rules:` 是**本 task 最关键的 3-5 条**,短句,一眼看到
- 冗余不是 bug —— in-context 强化比"单一来源"更重要(人读 task.md 是高频,读 CLAUDE.md 是低频)
- 写法:动词开头,短句 + 可选的 "Why"

**Skills 显式的理由**:
- Claude Code 有自动加载,但加载的结果对用户不可见
- `skills:` 字段让用户知道"这个 task 我期望 agent 用什么 skill",Code Review 时可以质疑"这项没用 skill X 是不是漏了"
- 也让 session resume 时能快速 reload context

#### 阶段 3: 设计 + 评审

**触发**:`/task-review-request {task_id} --by codex` 调 Oracle/Codex 做 review

**动作**:
1. Codex 读 spec + design → 输出 review 到 `docs/tasks/{task_id}/reviews/r1-codex.md`
2. 追加到 task frontmatter 的 `review:` 列表
3. CC 写响应 → `docs/tasks/{task_id}/reviews/r1-cc.md`,同样追加
4. 如果需要多轮 → r2-codex.md / r2-cc-final.md,追加
5. 定稿时把最后一版的 design 文件设为 `design: docs/tasks/{task_id}/design-v2-cc.md`(指向 final)

**版本标记**:
- 每个 review doc 自己 frontmatter 带 `status: superseded | current | final`
- 最新的 review doc 也是最终 design,frontmatter `status: final`
- `/task-view` 解析列表并按 `status` 标记

#### 阶段 4: 实施(active)

**触发**:spec 和 design 定稿后开始实际编码

**动作**(TDD):
- 按 spec 的 Completion Criteria 写 Red 测试桩(函数名已在 spec 里声明)
- 用户确认 Red
- 写实现
- 用户确认 Green(本 session 可跳过关卡)
- Refactor
- 循环直到 task 完成

**Session 切换中途**:
- `/task-handoff {task_id}` → 写快照到 `docs/tasks/{task_id}/handoff.md`
- frontmatter `handoff:` 字段指向 handoff 文件
- 新 session `/task-resume {task_id}` → 读 handoff + 验证 + 汇报

#### 阶段 5: 完成(done)

**触发**:所有 Completion Criteria 的测试 Green + Code Review 通过 + 用户手动 walkthrough 通过

**强制沉淀步骤**(`/task-done {task_id}` 自动化):

1. **收集 commits**:
   - 扫 git log 中 subject 含 `{task_id}` 或 V1.1 Phase 号(约定)
   - 或扫 commit message 中的 task 引用(`Refs: task_xxx`)
   - 写入 frontmatter `commits:` 列表

2. **Lessons 捕捉**(交互式):
   - Agent 根据本 task 的 review cycles + handoff 历史 + commit diff 提炼"踩坑摘要"
   - 用户确认/编辑/追加,写入 frontmatter `lessons:`
   - **同时**追加到 `docs/lessons/modules/{module}.md`(跨 task 可搜)

3. **Mempal KG triples 沉淀**:
   - 调 `mcp__mempal__mempal_ingest` 提交三元组到 mempal 服务
   - 调 `/backfill-kg` 按 slot-specific 规则抽取
   - 把三元组列表**同时**写入 task frontmatter `mempal:` 字段
   - 冗余的理由:task 内能看到的三元组让人知道"这个 task 学到了什么",mempal 服务是全局检索用

4. **状态切 done**:
   - frontmatter `status: done`
   - Kanban UI 自动把卡片移到 Done 列

### /task-done 的自动化流程伪代码

```
input: task_id

step 1: git log --grep "{task_id}" --pretty='%h %s' → 写入 commits 字段
step 2: 读 task.md + review cycles + handoff → agent 提炼 lessons
        → 用户交互式确认 → 写入 lessons 字段 + append 到 docs/lessons/
step 3: 调 /backfill-kg 从 task 相关内容提取 KG 三元组
        → mcp__mempal__mempal_ingest 提交
        → 把提取的三元组列表回填 task frontmatter mempal 字段
step 4: 最后把 status 改为 done,追加 changelog + devlog
```

---

## 七、子模块详解

### 7.1 Spec(agent-spec Task Contract)

**位置**:`docs/tasks/{task_id}/spec.md`

**格式**(不变,来自 v3):
```markdown
spec: task
name: "task-name-slug"
task_id: task_xxx
tags: [area, module]
---

## Intent
(1-3 句)这个 task 做什么、为什么做、解决什么问题。

## Decisions
- 本 task 固定的技术决策(从 design.md 局部化抄录)

## Boundaries
### Allowed Changes
- src-tauri/src/modules/{module}/**
### Forbidden
- (project.spec 的 Always Forbidden 自动继承)

## Completion Criteria

Scenario: 场景名(中文)
  Test: test_fn_name_exactly_as_in_code
  Given 前置条件
  When 触发操作
  Then 期望结果
```

**key 字段** `task_id` 新增 —— 让 spec 反向索引到 task.md,形成双向链接。

**验证**:`agent-spec lifecycle docs/tasks/{task_id}/spec.md --code . --project-spec specs/project.spec`

### 7.2 Design / Plan 文档

**Design** 格式(自由,建议):
```markdown
---
type: design
task_id: task_xxx
version: v2
author: cc
status: final
supersedes: docs/tasks/task_xxx/design-v1-cc.md
---

# {Task Title} — Design v2

## 背景
## 方案选择
## 最终架构
## 实施拆解(拆成 plan 的种子)
```

**Plan** 格式(实施层,更细):
```markdown
---
type: plan
task_id: task_xxx
version: v1
author: cc
---

# {Task Title} — Implementation Plan

## Phase 拆解
## 每个 Phase 的 TDD 流程
## 测试矩阵
## 风险清单
```

**Design vs Plan**:
- Design 回答 "做什么 / 怎么设计"(架构决策)
- Plan 回答 "分几步做"(执行拆解)
- 简单 task 可以省 plan,复杂 task 分开写

### 7.3 Review 版本管理

**约定**:

1. 每轮 review 生成 **两个文件**:
   - `reviews/r{N}-{reviewer}.md`:reviewer 的 critique
   - `reviews/r{N}-{author}.md`:author 的 response

2. 每个 review doc 的 frontmatter:
   ```yaml
   ---
   type: review
   task_id: task_xxx
   round: 1          # 1, 2, 3, ...
   author: codex     # 或 cc
   role: reviewer    # reviewer 或 author
   status: superseded | current | final
   responds_to: docs/tasks/{id}/reviews/r1-codex.md  # 可选,CC response 指回 codex critique
   ---
   ```

3. Task frontmatter 的 `review:` 列表:
   ```yaml
   review:
     - docs/tasks/task_xxx/reviews/r1-codex.md       # round 1 codex
     - docs/tasks/task_xxx/reviews/r1-cc.md          # round 1 cc response
     - docs/tasks/task_xxx/reviews/r2-codex.md       # round 2 codex
     - docs/tasks/task_xxx/reviews/r2-cc-final.md    # final,此项 status: final
   ```

4. **"当前版本" 推导规则**:
   - 列表最后一项 + 该文件的 frontmatter `status: final` → 当前 final
   - 如果没有 final 标记,按列表顺序最后一项作为 current
   - `/task-view` 用这个规则渲染 "Current: xxx" 和 "History: [...]"

### 7.4 Rules — 任务级提醒

**格式**:frontmatter `rules:` 字段,字符串列表,每条一句话。

**示例**(从 V1.1 Kanban Subtask 提炼):
```yaml
rules:
  - "Subtask parser 必须在 Rust,TS 不得 import parse 逻辑(L0 TS/Rust 职责边界)"
  - "Multi-block checklist 走 typed error,不用字符串 matching 做控制流"
  - "color clear 发 'default' sentinel,null 语义是'保留原值'"
  - "PointerSensor activationConstraint 是 click/drag 互斥的唯一机制,别加 drag handle"
```

**写作规则**:
- 动词开头,可操作
- 不超过一句话
- 3-5 条最合适,超过 10 条说明 task scope 太大
- 和 CLAUDE.md 的大规则做映射(不是摘抄全文)

**在 task lifecycle 的作用**:
- Kickoff 时提炼(从 spec + design + CLAUDE.md)
- Active 阶段每次 session resume 都重读
- Done 阶段核对:这些 rule 在实现里有没有被破坏(Code Review 第 6 项"建模强度"的细化)

### 7.5 Skills — Claude Code skills 显式声明

**格式**:
```yaml
skills:
  - rust-errors           # skill 名(不带斜线前缀)
  - rust-modeling
  - react-hook-form-zod
  - tanstack-router-core-navigation
```

**用途**:
- Session resume 时让 agent 显式 re-load
- Code Review 时核对 "这个 task 是否真的用上了声明的 skill"
- 为未来的 skill usage analytics 奠基(哪些 skill 实际被 task 用了)

**如何确定**:
- Kickoff 阶段 agent 根据 spec + design 推荐(比如"涉及 SQL SELECT → 推荐 rusqlite / rust-errors")
- 用户确认/删减
- 写入 frontmatter

### 7.6 Commits 归档

**格式**:
```yaml
commits:
  - hash: 771e30c
    subject: "refactor(app_error): struct variants + MultiBlockChecklist variant"
  - hash: 41674b3
    subject: "feat(keysight): parse_task_checklist + Subtask struct"
  # ...
```

**自动化采集规则**(`/task-done` 运行):

1. 扫描 git log,匹配 commit message 中包含:
   - task id (`task_xxx`)
   - 或 task 约定 tag (如 V1.1 Phase 号)
2. 按时间顺序排列
3. 提取 hash + subject 的第一行(去掉 body)
4. 写入 frontmatter

**约定**:每次 commit message 建议包含 `Refs: task_xxx` 或类似标签,让扫描简单。如果早期没做约定,可用时间范围 + 功能域过滤近似。

### 7.7 Lessons 沉淀

**格式**:frontmatter 短句列表 + 同时写入 `docs/lessons/modules/{module}.md`

```yaml
lessons:
  - "rusqlite 用自己的 FromSql trait,不走 serde —— r.get 需要手动 impl"
  - "task::update 的 None 参数语义是'保留原值',清空 color 必须发 'default' sentinel"
  - "@dnd-kit 的 activationConstraint { distance: 8 } 是 click/drag 互斥的标准做法,不需要 drag handle"
```

**触发**:`/task-done` 的第 2 步,交互式

**agent 提炼来源**:
- task 的 review cycles(哪些点 codex 反复指出 → 对应 lesson)
- handoff 里的 "试过但不行的方案"(失败案例 → lesson)
- commit diff 中的 `fix` commit(bug 修复 → lesson)

**用户确认**:提炼后弹出,允许编辑 / 删除 / 追加

**同步写入**:
- task frontmatter(locality,人看 task 能看到)
- `docs/lessons/modules/{module}.md`(跨 task 可搜,LESSONS.md 的进化版)

**与 V3 LESSONS.md 的关系**:
- V3 LESSONS.md 是模块目录下的文件,和代码 co-located
- v4 `docs/lessons/` 在 vault,和 task 一起管理
- **迁移**:V3 LESSONS.md 内容迁入 `docs/lessons/modules/{module}.md`,代码目录不再放 LESSONS.md
- 访问路径变成 vault 搜索(Obsidian 原生)而不是 repo 目录遍历

### 7.8 Mempal KG 三元组沉淀

**格式**:
```yaml
mempal:
  - subject: "rusqlite"
    predicate: "反序列化机制"
    object: "自有 FromSql trait,非 serde"
  - subject: "V1.1 Kanban Subtask"
    predicate: "触发的 refactor"
    object: "AppError 从 tuple variants 改为 struct variants"
```

**触发**:`/task-done` 第 3 步

**流程**:
1. Agent 扫 task 的 spec / design / reviews / lessons,提取可图谱化的 subject-predicate-object 三元组
2. 调 `mcp__mempal__mempal_ingest` 提交到 mempal KG 服务(全局可检索)
3. 把三元组列表**也**写入 task frontmatter(冗余,但 in-task 可见)
4. 调 `/backfill-kg` 做进一步补全

**为什么冗余存一份在 task 里**:
- mempal MCP 是服务,检索快但要发起调用
- task frontmatter 里的列表让开发者读 task.md 时直接看到"这个 task 学了什么知识点"
- 是一种"有意识的显式化",不是数据库设计

**与 /backfill-kg 的关系**:
- `/backfill-kg` 是已有的 slash command,按 slot-specific 规则提取三元组
- `/task-done` 的第 3 步可以直接调 `/backfill-kg --task {task_id}`(如果 command 支持 task scope),然后读取它的输出回填 frontmatter

### 7.9 Handoff — Task-owned

**变化**:V3 的 handoff 是**功能域级**(`docs/handoff/{area}.md`),v4 升级为 **task 级**(`docs/tasks/{task_id}/handoff.md`)

**为什么**:
- 功能域 handoff 适合"我要切回 backend area 做些什么",但当一个 task 跨 session 时,更精准的是"task-xxx 做到哪了"
- Task 级 handoff 能和 task.md frontmatter `handoff:` 字段直接绑定

**格式**(和 V3 handoff 内容相同):
```markdown
---
task_id: task_xxx
last_updated: 2026-04-15T22:00:00+08:00
session_id: {cc_session_id}
status: in-progress | blocked | ready-to-resume
stale_check: cargo test && pnpm test
---

# Handoff: {task title}

## 正在做的 Task
## 已完成步骤
## 下一步具体动作
## 关键上下文(丢了的)
## 试过但不行的方案
## 开放问题
## Resume 检查清单
```

**Flow**:
- 切 session 前:`/task-handoff {task_id}` → 写 handoff → 更新 task frontmatter
- 新 session:`/task-resume {task_id}` → 读 task.md + handoff + 验证 → 汇报

**与功能域级 handoff 的关系**:
- v4 **废弃** `docs/handoff/{area}.md`,全改 task 级
- 如果有"我整个 area 没有 active task,下次过来先看看"这种需求,那本质上是 devlog 职责,不是 handoff

---

## 八、质量关卡

### 功能域完成时的完整流程(v4 版)

```
功能域 Active tasks 全部完成
  │
  ├─ 1. /harness-check-tests(降级为 spec completeness 自查)
  │     ↳ 审查每个 active task spec 的 Completion Criteria 是否完整
  │     ↳ 遗漏场景 → 补 Scenario + Test: 绑定 → 重复
  │
  ├─ 2. agent-spec lifecycle(对每个 active task spec 运行)
  │     ↳ lint: 所有 Scenario 有 Test: 绑定,quality ≥ 0.7
  │     ↳ verify: Test: fn_name 存在且 cargo test 通过
  │     ↳ fail/skip → 修 spec 或补测试 → 重跑
  │
  ├─ 3. /harness-type-safety-check(保留)
  │     ↳ 对照 L0 防火墙清单
  │
  ├─ 4. Code Review(6 项)
  │     1. Contract Acceptance(spec 完整性的人工终审)
  │     2. 逻辑正确性
  │     3. 回归风险
  │     4. I/O 正确性
  │     5. IPC 类型安全
  │     6. 建模强度(含 task 的 rules 字段 diff)
  │
  ├─ 5. 用户手动 walkthrough(如果有 UI 改动)
  │
  ├─ 6. /task-done {task_id}(强制沉淀)
  │     a. 收集 commits → frontmatter
  │     b. 提炼 lessons → frontmatter + docs/lessons/
  │     c. 提取 mempal 三元组 → mempal MCP + frontmatter
  │     d. status → done
  │
  ├─ 7. git commit(pre-commit hook: agent-spec guard + cargo test)
  │
  ▼
标记 Done
```

### Code Review 6 项检查(v4 版)

1. **Contract Acceptance** — `/harness-check-tests` 和 `lifecycle lint` 之后,spec 的 Completion Criteria 是否真的覆盖了所有重要场景?有没有边缘漏网之鱼?task 的 `rules:` 字段是否被所有实现尊重?
2. **逻辑正确性** — 从旧代码移植的逻辑逐行核对
3. **回归风险** — 未来变更可能静默破坏的场景,测试是否锁住
4. **I/O 正确性** — SQLite 读写完整性 / 文件操作正确性
5. **IPC 类型安全** — `#[specta::specta]` / bindings.ts 是否最新
6. **建模强度(防火墙)** — 签名中的 `String/bool/&str` / invariant 保证方式 / 穷尽 match / task `rules:` 字段里声明的约束有无被破坏

### Pre-commit hook(v4 版)

```bash
# .claude/settings.json 里的 hook command
agent-spec guard --spec-dir . --code . --change-scope staged || exit 2
cargo test --workspace --manifest-path src-tauri/Cargo.toml || exit 2
pnpm test -- --run || exit 2
pnpm build || exit 2
```

顺序:guard → cargo test → pnpm test → pnpm build。任一失败阻 commit。

---

## 九、新增 Slash Commands

| 命令 | 用途 | 触发时机 |
|---|---|---|
| `/task-new {title}` | 创建新 task:生成 task.md + `docs/tasks/{id}/` 目录 + 初始 frontmatter | 新需求出现时 |
| `/task-kickoff {task_id}` | Active 前填 spec / design / rules / skills,状态 next → active | 开始开发前 |
| `/task-spec {task_id}` | 生成 / 编辑 `docs/tasks/{task_id}/spec.md`(agent-spec Task Contract) | Kickoff 阶段 |
| `/task-design {task_id} [--version vN]` | 生成 / 编辑 design doc,默认创建新版本 | Kickoff 阶段 or 评审后 |
| `/task-review-request {task_id} --by codex` | 调 Oracle/Codex 做 review,输出到 `reviews/r{N}-codex.md` 并追加 frontmatter `review:` | 设计定稿后 |
| `/task-review-response {task_id} --round N` | CC 写响应,输出到 `reviews/r{N}-cc.md` | Codex 评审完 |
| `/task-handoff {task_id}` | 写 handoff 快照到 `docs/tasks/{id}/handoff.md`,更新 frontmatter | Session 结束前,task 未完成 |
| `/task-resume {task_id}` | 新 session 读 task.md + handoff + 验证 + 汇报 | `/new` 后 |
| `/task-view {task_id}` | 渲染 task 总览(Intent / Status / 所有相关 artifact 的树 / 时间线) | 查看 task 全景时 |
| `/task-done {task_id}` | 强制沉淀:commits / lessons / mempal → frontmatter,status → done | Task 完成 + 用户 walkthrough 通过后 |
| `/harness-check-tests` | **降级版**:spec completeness 自查(审查 Completion Criteria 是否完整) | 功能域完成时,lifecycle 前 |
| `/harness-type-safety-check` | **保留**:对照 L0 防火墙清单 | 功能域完成时 |
| `/harness-save-next-context` | **降级**:退化为 `/task-handoff` 的别名(保留兼容) | - |
| `/harness-resume-context` | **降级**:退化为 `/task-resume` 的别名(保留兼容) | - |

---

## 十、新项目 Bootstrap 流程

```
1. 仓库初始化
   git init; mkdir -p src specs docs
   cargo init; pnpm init (如果 TS/Rust 双栈)

2. 创建 project.spec(agent-spec 全局约束)
   specs/project.spec
   - Intent(项目做什么)
   - Global Decisions(全局硬约束)
   - Global Boundaries / Forbidden
   - Global Completion Rules
   - 最低 quality score

3. 写 CLAUDE.md
   基于本 v4 doc 的附录 B(CLAUDE.md 模板)裁剪,包含:
   - 项目管理体系(四关注点,术语,进度,质量,日志)
   - L0-L4 质量规则
   - TS/Rust 双栈约束(如适用)

4. 初始化 vault
   创建 whiteboard/projects/{project}/docs/{design.md, progress/, devlog/, handoff/, tasks/, lessons/}
   写一份空的 docs/design.md(项目整体架构占位)
   为初始功能域创建 docs/progress/{area}.md

5. 配置 .claude/settings.json
   - build hook:cargo clippy / pnpm build 拦截
   - pre-commit hook:agent-spec guard + cargo test + pnpm test + pnpm build

6. 创建第一篇 devlog
   docs/devlog/{YYYY-MM-DD}.md

7. 创建第一个 task
   /task-new "第一个任务" → task.md + docs/tasks/{id}/

8. Kickoff 开始开发
   /task-kickoff {id} → spec / design / rules / skills
   进入 TDD 循环
```

---

## 十一、现有项目迁移(super-tauri 示例)

现有 super-tauri 项目在 V1.1 已经:
- Task 实体在 `whiteboard/projects/super-tauri/task_xxx.md`
- Docs 在 repo `docs/`(design / progress / devlog / handoff / collaboration)
- `specs/project.spec` 暂时不存在(v3 方案未落地)

### 迁移步骤

**Phase 0: 准备**
1. 装 agent-spec CLI:`cargo install agent-spec`
2. 在 repo 根写 `specs/project.spec`(从 v3 §三抄模板)

**Phase 1: Docs 搬迁到 vault**(最大改动)
1. `docs/design.md` → `whiteboard/projects/super-tauri/docs/design.md`
2. `docs/progress/{area}.md` → `whiteboard/projects/super-tauri/docs/progress/{area}.md`
3. `docs/devlog/*.md` → `whiteboard/projects/super-tauri/docs/devlog/*.md`
4. `docs/handoff/*.md` → `whiteboard/projects/super-tauri/docs/handoff/*.md`(过渡期保留,新 handoff 走 task 级)
5. `docs/collaboration/*.md` **按 task 迁移**:
   - V1.1 Kanban Subtask 的 5 份 collab doc → `whiteboard/projects/super-tauri/docs/tasks/task_3a3f9a64/...`
   - V1 Kanban 的 collab doc → 对应 task id 下
   - 历史无 task 绑定的 collab doc 留在 repo `docs/collaboration/` 作为历史存档

**Phase 2: 现有 task.md 补 frontmatter**(回溯)
对 Active / Next 的 task:
1. 补 `spec:` / `design:` / `plan:` 字段(指向迁移后路径)
2. 补 `rules:` / `skills:` 字段(从 CLAUDE.md 提炼 + agent 建议)
3. Done 的 task **不强制回溯**,但如果有 V1.1 级 task 想 dogfood 可以补 `review:` / `commits:` / `lessons:` / `mempal:` 作为示例

**Phase 3: 切换 slash commands**
1. `/harness-check-tests` prompt 换 v4 新版
2. `/harness-save-next-context` / `/harness-resume-context` 做成 `/task-handoff` / `/task-resume` 的 alias
3. 添加新命令(`/task-new` / `/task-kickoff` / `/task-spec` / `/task-design` / `/task-review-*` / `/task-view` / `/task-done`)

**Phase 4: 更新 CLAUDE.md**
1. "项目管理体系"段落指向 vault 路径
2. 新增 "Task 生命周期" 段落
3. 更新 "质量机制总览" 的触发时机(lifecycle 替代部分 harness-check-tests)
4. 添加 "新增 Slash Commands" 表

**Phase 5: Dogfood 下一个 task**
新建一个 task 完整走 v4 流程(new → kickoff → spec → design → review → impl → done),验证所有命令和 frontmatter 字段,暴露问题快速迭代

### 迁移 Risk

- **最大成本**:Phase 1 的物理文件搬迁 + Phase 2 的 task.md frontmatter 补全。估计 1 session 的手动工作
- **次大成本**:CLAUDE.md 的"项目管理体系"段落重写,以及 slash command 的 prompt 全体更新
- **兼容期**:`docs/handoff/{area}.md` 可以和新 task 级 handoff 并存一段时间,等所有旧 handoff 自然 done 后再删

---

## 十二、完整示例:V1.1 Kanban Subtask 假如走 v4 流程

### task.md(最终完成态)

```yaml
---
type: project-task
id: task_v1_1_kanban_subtask
status: done
project: super-tauri
color: "#bdb2ff"
area: backend

# === kickoff 阶段填写 ===
spec: "docs/tasks/task_v1_1_kanban_subtask/spec.md"
design: "docs/tasks/task_v1_1_kanban_subtask/design-v2-cc.md"
plan: "docs/tasks/task_v1_1_kanban_subtask/plan.md"

rules:
  - "Subtask parser 必须在 Rust,TS 禁止 import parse 逻辑(L0 TS/Rust 职责边界)"
  - "Multi-block checklist 走 typed error,禁 TS 字符串 match 控制流"
  - "Color clear 发 'default' sentinel,null 语义是'保留原值'"
  - "PointerSensor activationConstraint 是唯一 click/drag 互斥机制"
  - "TaskEntity 跨 IPC 字段升 enum,bindings 自动生成 literal union"

skills:
  - rust-modeling
  - rust-errors
  - rust-types
  - react-hook-form-zod
  - tanstack-router-core-navigation

# === review 阶段填写 ===
review:
  - docs/tasks/task_v1_1_kanban_subtask/reviews/r1-codex.md      # 5 项 critique
  - docs/tasks/task_v1_1_kanban_subtask/reviews/r1-cc.md         # CC 全接受 + 2 微调
  - docs/tasks/task_v1_1_kanban_subtask/reviews/r2-codex.md      # 发现 AppError 硬伤
  - docs/tasks/task_v1_1_kanban_subtask/reviews/r2-cc-final.md   # Phase 6.0 新增

handoff: "docs/tasks/task_v1_1_kanban_subtask/handoff.md"        # 可选,如果中间切过 session

# === /task-done 阶段自动填写 ===
commits:
  - { hash: 771e30c, subject: "refactor(app_error): struct variants + MultiBlockChecklist variant" }
  - { hash: 41674b3, subject: "feat(keysight): parse_task_checklist + Subtask struct" }
  - { hash: 726ae25, subject: "feat(keysight): TaskEntity.subtasks + 三 reader 填充" }
  - { hash: 8a60691, subject: "feat(keysight): task_update_with_subtasks + render_subtasks_into_body" }
  - { hash: a14e128, subject: "feat(kanban): KanbanCard progress 徽章" }
  - { hash: be9d2ac, subject: "feat(kanban): TaskEditModal + ChecklistEditor" }
  - { hash: 844f634, subject: "feat(kanban): 双击 KanbanCard 打开 TaskEditModal + PointerSensor" }
  - { hash: e9d3ff4, subject: "fix(kanban): 单击打开 modal + color swatch picker" }
  - { hash: c2419c4, subject: "fix(kanban): TaskEditModal clear color 发送 'default' sentinel" }
  - { hash: b2dae84, subject: "fix(keysight): TaskNode canvas 结构化渲染 subtasks" }

lessons:
  - "rusqlite 不走 serde:r.get::<_, TaskStatus>() 需要手动 impl FromSql/ToSql,非法字符串走 FromSqlError::Other(Box<Error>)"
  - "task::update 的 color 参数 None 语义是'保留',Some('default') 才是'清空' —— 跟 note/card/question 项目约定一致"
  - "@dnd-kit PointerSensor activationConstraint {distance:8} 是 click/drag 互斥的标准做法,不需要 drag handle 或 isDragging check"
  - "AppError 用 struct variants + #[serde(tag='kind')] 让每个 variant 的字段在顶层展开,比 tuple+content 清晰"
  - "`line_index` 是 parser 内部细节,不应泄漏到 IPC 写契约 —— 位置信息 stale 风险会让写路径变脆"
  - "Render checklist 回 body 要单 block only,多 block 用 typed error fail-closed,不硬做 per-line patch"
  - "TaskEditModal 没有 content textarea 是有意的 —— V1.1 不做半结构化 markdown 编辑器,留 V1.2"

mempal:
  - { subject: "rusqlite", predicate: "反序列化机制", object: "自有 FromSql trait,非 serde" }
  - { subject: "task::update color None", predicate: "语义", object: "保留原值(Some('default') 才是清空)" }
  - { subject: "@dnd-kit PointerSensor activationConstraint", predicate: "解决的问题", object: "click/drag 手势冲突,天然互斥,无需 drag handle" }
  - { subject: "V1.1 Kanban Subtask", predicate: "触发的 refactor", object: "AppError tuple → struct variants + MultiBlockChecklist variant" }
  - { subject: "GFM checklist body parser 的 line_index", predicate: "暴露规则", object: "不跨 IPC,仅 Rust 内部 ParsedItem 私有类型使用" }
  - { subject: "render_subtasks_into_body 算法", predicate: "约束", object: "只支持单连续 block,多 block 返 typed error" }
---

# 【TASK】V1.1 Kanban Subtask + 双击编辑 modal

(body)

- [x] 6.0: AppError struct variants + MultiBlockChecklist variant
- [x] 6.1: parse_task_checklist + Subtask struct
- [x] 6.2: TaskEntity.subtasks + 3 reader 填充
- [x] 6.3: task_update_with_subtasks command
- [x] 6.4: KanbanCard progress 徽章
- [x] 6.5: TaskEditModal + ChecklistEditor
- [x] 6.6: 单击接线 + PointerSensor
- [x] 6.7: 收口
```

### `/task-view task_v1_1_kanban_subtask` 的输出

```markdown
# 【TASK】V1.1 Kanban Subtask + 双击编辑 modal

Status: ✅ done | Area: backend | Project: super-tauri

## Intent
(从 spec 读取)

## Current Artifacts
- 📋 Spec:      docs/tasks/.../spec.md
- 🏗️ Design:   docs/tasks/.../design-v2-cc.md (v2, final)
- 📅 Plan:     docs/tasks/.../plan.md

## Review Cycles

| Round | Reviewer | Status      | Path |
|-------|----------|-------------|------|
| 1     | codex    | superseded  | ../reviews/r1-codex.md |
| 1     | cc       | superseded  | ../reviews/r1-cc.md |
| 2     | codex    | superseded  | ../reviews/r2-codex.md |
| 2     | cc       | **final**   | ../reviews/r2-cc-final.md |

## Rules Applied (5)
1. Subtask parser 必须在 Rust...
2. Multi-block checklist 走 typed error...
...

## Skills Used (5)
- rust-modeling, rust-errors, rust-types, react-hook-form-zod, tanstack-router-core-navigation

## Commits (10)
- 771e30c refactor(app_error): struct variants + MultiBlockChecklist
- 41674b3 feat(keysight): parse_task_checklist + Subtask struct
- ...

## Lessons (7)
1. rusqlite 不走 serde...
...

## Mempal KG Triples (6)
- rusqlite --[反序列化机制]--> 自有 FromSql trait,非 serde
- task::update color None --[语义]--> 保留原值
- ...
```

---

## 十三、与 v3 的差异对照

| 维度 | V3 | v4 |
|---|---|---|
| **docs 位置** | repo 的 `docs/` 下 | **vault** `whiteboard/projects/{project}/docs/` 下 |
| **Task 的载体** | progress file 的 checkbox 一行 | **task.md** 作为 entity,frontmatter 是结构化 index |
| **Task 的上下文** | 靠 commit message / devlog 串联 | **task.md frontmatter 直接存**(spec / design / review / commits / lessons / mempal / rules / skills) |
| **Review 版本管理** | collaboration doc 平铺在 `docs/collaboration/` | **per-task 子目录** `docs/tasks/{id}/reviews/`,task.md 的 `review:` 字段是索引 |
| **Handoff** | 功能域级 `docs/handoff/{area}.md` | **task 级** `docs/tasks/{id}/handoff.md` |
| **agent-spec Task Contract** | `specs/{area}/{task}.spec.md` 在 repo | **vault** `docs/tasks/{id}/spec.md`,task.md frontmatter `spec:` 指向 |
| **project.spec** | `specs/project.spec` 在 repo | **同 v3**,在 repo(不进 vault) |
| **lessons** | 模块代码目录下的 `LESSONS.md` | **vault** `docs/lessons/modules/{module}.md` + task.md frontmatter `lessons:` |
| **Skills / Rules 声明** | 无(隐式依赖 claude code 自动加载 + CLAUDE.md) | **task.md frontmatter 显式**(`skills:` / `rules:`) |
| **Commits 归档** | 无(需要靠 git log / devlog 手动找) | **task.md frontmatter `commits:` 列表**,`/task-done` 自动采集 |
| **Mempal 集成** | 无(只有外部 mempal MCP) | **task.md frontmatter `mempal:` 冗余列表** + mempal MCP ingest |
| **新建项目流程** | V3 Bootstrap 清单(8 步) | **扩展版 Bootstrap**(10 步,加 vault 初始化 + project.spec) |

---

## 十四、Tradeoffs / 开放问题

### 1. frontmatter 爆炸的视觉噪声

**问题**:完成态 task.md 的 frontmatter 可能上百行(10+ commits, 7+ lessons, 6+ mempal triples)。YAML 可读性下降。

**缓解**:
- frontmatter 本来就不是给人"日常读"的,是给 `/task-view` 和其他工具机器读
- 人读 task 的入口是 `/task-view {id}` 的渲染输出(markdown summary),不是原始 YAML
- Obsidian 有 frontmatter 折叠视图

**保留问题**:如果 task 规模极大(V1.1 级别,30+ commits),frontmatter 可能超 200 行,需要考虑把 commits 拆到独立文件 `docs/tasks/{id}/commits.md`

### 2. Vault 和 Repo 的同步问题

**问题**:docs 在 vault,code 在 repo,两者不在一个 git repo。vault 自己的版本控制策略是什么?

**V3 的默认**:agent-slipbox-v3 是独立 git repo,自己 commit。跨 repo 改动无原子性。

**v4 的处理**:
- 代码 commit 不强制包含 docs(docs 在 vault 自己的 repo)
- CI 无法同时验证 code + docs 一致性
- 依赖 `/task-done` 作为协调点:完成 task 时,task.md frontmatter 里的 commits 字段是**单向引用** repo 的 commit hash,vault 不 follow code 的分支

**保留问题**:如果 code repo 的 commit 被 rebase / reset,vault 里 frontmatter 的 hash 会失效。需要工具定期校验。

### 3. Rules 和 CLAUDE.md 的重复会漂移

**问题**:task.md 的 `rules:` 和 CLAUDE.md 的内容重复,如果 CLAUDE.md 升级而老 task 的 rules 没跟进 → 两边不一致。

**设计权衡**:**这是有意的冗余,不是 bug**:
- CLAUDE.md 是"当前规则",老 task 的 rules 是"当时的规则"
- 做 task done 回顾时看到老 rules 反而有历史价值
- 新 task 的 rules 应该从最新 CLAUDE.md 提炼(kickoff 阶段)

### 4. Commits 采集的约定问题

**问题**:如果 commit message 没约定包含 `task_id`,`/task-done` 无法精确采集。

**方案**:
- 硬约定:commit message 必须有 `Refs: task_xxx` 末尾 tag
- 或模糊采集:扫描 subject + body,匹配 task_id / phase 号 / 关键词,允许误判 + 交互式确认

v4 默认走**模糊采集 + 交互确认**,commit message 约定不是硬约束。

### 5. Vault 的 per-task 目录和 Obsidian 搜索

**问题**:per-task 子目录会让 Obsidian 的 "文件搜索" 结果略乱(同名文件 spec.md 在多个 task 目录里)

**方案**:
- Obsidian 搜索时用 `path:docs/tasks/task_xyz` 前缀过滤
- 或在文件名加 task_id 前缀 `docs/tasks/task_xyz/task_xyz-spec.md`(牺牲文件名简洁度换搜索明确)

v4 默认走 **简洁文件名 + Obsidian 路径过滤**,如果实际 UX 不满意再加 task_id 前缀。

### 6. Lessons 双存(frontmatter + docs/lessons/)的同步

**问题**:lessons 在 task.md frontmatter 和 `docs/lessons/modules/{module}.md` 两处,任意一处改了另一处不更新 → 漂移。

**设计决策**:
- `/task-done` 写入时两处**同时**写
- 后续修改优先改 `docs/lessons/`(跨 task 的真源)
- task.md frontmatter 的 lessons 是**快照**,不是 live 视图
- 允许漂移是有意的(task.md 是"完成当时的真理")

### 7. Mempal 三元组的质量问题

**问题**:agent 自动提取的三元组可能质量差(宽泛、重复、无价值)。

**缓解**:
- `/task-done` 提取后**强制用户确认**,允许删改
- 提取 prompt 引导 agent 只挑"惊人 / 违反直觉 / 可复用"的知识点,不提取流水账
- 三元组写到 mempal 之后,可以靠 mempal 服务端去重 / 评分

### 8. 过渡期多套规则并存

**问题**:super-tauri 迁移到 v4 过程中,老 task 继续用旧规则,新 task 用 v4,一段时间内两套并存。

**方案**:
- `/task-new` 创建的新 task 默认走 v4
- 旧 active task 不回溯 task.md frontmatter,continue 用旧流程到 done
- `/harness-check-tests` 的 prompt 在所有旧 task 完成前保持"在代码里找缺失测试"的旧职责,完成后统一切到 v4 "spec completeness 自查"版本

---

## 附录 A:完整 Frontmatter Schema(YAML)

```yaml
---
# 基础(V1.1 已有,必需)
type: project-task
id: task_{hex8}
status: inbox | next | active | blocked | done
project: string

# 基础扩展(可选)
color: string                          # hex color "#xxx"
area: string                           # 功能域名
subtasks: string[]                     # 可选,body checklist 的 frontmatter 映射

# 开始前 context
spec: path                             # 指向当前 spec(相对 task.md 的路径)
design: path                           # 指向当前 design
plan: path                             # 指向 plan(可选)
rules: string[]                        # 任务级规则提醒(短句)
skills: string[]                       # Claude Code skill 名称列表

# 开发中 artifact
review: path[]                         # append-only,最后一项 = current
handoff: path                          # task 级 handoff(session 切换时)

# 结束后沉淀
commits:                                # git log 快照
  - hash: string                        # git short hash
    subject: string                     # commit subject 第一行
lessons: string[]                       # 踩坑经验短句(同时写 docs/lessons/)
mempal:                                 # KG 三元组(同时写 mempal MCP)
  - subject: string
    predicate: string
    object: string
---

# 【TASK】{title}

(body: 自由 markdown,V1.1 支持 GFM checklist 作 subtasks)
```

## 附录 B:Slash Commands 一览

| 命令 | 简述 |
|---|---|
| `/task-new {title}` | 新建 task + `docs/tasks/{id}/` 目录 |
| `/task-kickoff {id}` | Active 前填 spec/design/rules/skills,状态 → active |
| `/task-spec {id}` | 生成 / 编辑 agent-spec Task Contract |
| `/task-design {id} [--version vN]` | 生成 / 编辑 design doc |
| `/task-review-request {id} --by codex` | 调 oracle 做 review |
| `/task-review-response {id} --round N` | CC 写响应 |
| `/task-handoff {id}` | 写 task 级 handoff |
| `/task-resume {id}` | 新 session 恢复 task context |
| `/task-view {id}` | 渲染 task 全景 |
| `/task-done {id}` | 强制沉淀 commits / lessons / mempal,状态 → done |
| `/harness-check-tests` | spec completeness 自查(降级版) |
| `/harness-type-safety-check` | L0 防火墙清单对照(保留) |

## 附录 C:CLAUDE.md 需要的更新

1. **四关注点术语表**:加 "Task" 的新定义(entity + frontmatter index,不再是 checkbox)
2. **进度段落**:progress/{area}.md 位置从 `docs/` 改到 vault `whiteboard/projects/{project}/docs/`
3. **质量机制总览**:加 "Task Contract" / "project.spec" 两行;`/harness-check-tests` 触发时机改为"lifecycle 前"
4. **新增 Task 生命周期段落**:状态机 + kickoff / done 阶段的动作
5. **新增 Rules + Skills 显式化说明**:强调"有意的冗余"
6. **功能域完成流程**:加 agent-spec lifecycle + /task-done 两步
7. **Slash command 速查表**:新增 v4 commands,老 harness-* 标"降级 / alias"
8. **Bootstrap 清单**:vault 初始化 + project.spec 创建 + .claude/settings.json hook 配置

---

## 附录 D:与现有 super-tauri 的 V1.1 状态的对比

| 元素 | V1 收口(现状) | v4 目标状态 |
|---|---|---|
| Task 文件 | `whiteboard/projects/super-tauri/task_xxx 【TASK】...md` | 同位置,frontmatter 扩展 |
| V1.1 collab docs(5 份) | `docs/collaboration/2026-04-15-kanban-v1-1-*.md` | 迁到 `whiteboard/projects/super-tauri/docs/tasks/task_v1_1/*.md` |
| V1.1 spec | 无 | 新增 `docs/tasks/task_v1_1/spec.md` |
| V1.1 design | 散在 collab docs | 整合到 `docs/tasks/task_v1_1/design-v2-cc.md` |
| V1.1 reviews | collab docs 平铺 | 进 `docs/tasks/task_v1_1/reviews/r1-codex.md` 等 |
| V1.1 commits | git log(需人工找) | frontmatter `commits:` 字段,11 条 |
| V1.1 lessons | 无(部分散在 CLAUDE.md L0 踩坑样例) | frontmatter `lessons:` + `docs/lessons/modules/keysight.md` |
| V1.1 mempal | 无 | frontmatter `mempal:` + mempal MCP |
| V1.1 handoff | `docs/handoff/backend.md` 功能域级 | `docs/tasks/task_v1_1/handoff.md` task 级 |
| V1.1 rules / skills | CLAUDE.md 隐式 | frontmatter `rules:` + `skills:` |

---

**Document status**: 设计草案 v1,等用户 review。

**未决问题清单**(需用户决定):

1. **vault 初始化时机**:是新建项目时就创建 `whiteboard/projects/{project}/docs/` 所有子目录,还是 lazy 创建(第一次需要时建)?默认 Bootstrap 里一次建齐
2. **per-task 目录的文件名约定**:`spec.md` / `design-v1-cc.md` / `reviews/r1-codex.md` — 可以接受吗?还是要加 task_id 前缀避免同名
3. **Review 字段的格式**:flat path list 还是 typed list(含 round / author / status)?本文用 flat,每个 review doc 自己 frontmatter 带元数据。需要更严格的话可以改成 typed list
4. **Commits 采集方式**:commit message 硬约定 `Refs: task_xxx` / 模糊采集 + 交互确认 / 手动填写 —— 本文走模糊+交互
5. **迁移时机**:super-tauri 下一个 task(V1.2 或其他)开始前一次性迁,还是边做边迁?本文建议"新 task 走 v4,旧 task 不回溯"
6. **agent-spec CLI 是否现在就装**:如果现在不装,Phase 0-3 的一些步骤要 mock。建议 kickoff 一个 task 之前就装好

---

**Next action**:
- 用户 review 本方案,确认/调整方向
- 根据反馈改成 v2(如果需要)
- 开始 super-tauri 迁移(从 Phase 0 开始)或 dogfood 下一个 task
