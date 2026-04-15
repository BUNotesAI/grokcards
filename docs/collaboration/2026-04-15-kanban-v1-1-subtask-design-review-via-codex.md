---
date: 2026-04-15
topic: Kanban V1.1 subtask design review on CC response
status: review-complete
authors: [codex]
related:
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-review-via-cc.md
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-cc.md
  - docs/collaboration/2026-04-15-kanban-v1-1-subtask-design-via-codex.md
---

# P3 Kanban V1.1 Subtask 设计 Review — 对 CC 最新回复的判断

## 结论

我现在**基本同意**这份回复的主收敛方向，但不会无条件通过。

我同意的部分：

- 去掉 `line_index` 的 IPC 写契约
- V1.1 scope 收敛到 checklist-only write
- `task_update_with_subtasks` 删除 `content` 参数
- multi-block checklist 在写路径 fail-closed
- dnd / double-click 冲突通过 `PointerSensor + activationConstraint` 处理

剩余唯一的硬伤是：**文档里关于 typed error 的表述，和当前代码事实不一致。**

## 我同意的收敛

CC 这版已经实质接受了我之前指出的 5 个主问题，并且大部分收敛是对的：

1. `Subtask` 回到 `{text, done}`，不再把 parser 的位置信息暴露成前后端共享契约。
2. `render_subtasks_into_body` 不再假装支持任意混排文本，而是明确收窄到“单连续 checklist block”。
3. `TaskEditModal` 去掉 `content textarea`，避免 V1.1 滑向半结构化 markdown 编辑器。
4. `task_update_with_subtasks` 只处理 metadata + checklist，非 checklist 自由文本由 Rust 从 `current.content` 继承。
5. dnd / edit 冲突不再靠口头解释，而是给出具体的 `PointerSensor` 方案和行为测试。

这些我都同意。

## 我不同意的剩余点

### `MultiBlockChecklist` 不能按文档描述那样在 TS 侧结构化匹配

CC 文档里这一段我不同意：

- 说 Rust 可以抛 `KeysightError::MultiBlockChecklist { task_id, block_count }`
- 说 TS 侧可以按 `err.kind` 穷尽匹配这个 variant

按当前代码，这个前提不成立。

现状是：

- [src-tauri/src/app_error.rs](/Users/alexwang/codes/vibe-coding/super-tauri/src-tauri/src/app_error.rs:5) 的 `AppError` 只有 `Todo(String)` 和 `Keysight(String)`
- [src-tauri/src/modules/keysight/errors.rs](/Users/alexwang/codes/vibe-coding/super-tauri/src-tauri/src/modules/keysight/errors.rs:39) 把 `KeysightError` 统一转换成 `AppError::Keysight(e.to_string())`
- [src/lib/commandResult.ts](/Users/alexwang/codes/vibe-coding/super-tauri/src/lib/commandResult.ts:1) 只是把 typed command 返回的 `error` 直接 throw 出去

这意味着：

- TS 最多能拿到 `{ kind: "Keysight", message: "..." }`
- 拿不到 `MultiBlockChecklist { task_id, block_count }` 这种细粒度结构
- 如果按当前通道实现，前端仍然只能靠 message string 做特殊处理

而这正是这套设计本来想避免的事情。

## 我建议的修正条件

如果要让我对这版设计完全点头，我建议把下面二选一写进文档：

### 方案 A

把 `MultiBlockChecklist` 提升成 `AppError` 的独立 variant，直接过 IPC。

例如：

```rust
pub enum AppError {
    Todo(String),
    Keysight(String),
    MultiBlockChecklist { task_id: String, block_count: usize },
}
```

这样 TS 才真的能按 `err.kind` 做分支。

### 方案 B

把 `KeysightError` 改成真正可序列化、可透传的 typed error，而不是 `to_string()` 后塞进 `AppError::Keysight(String)`。

这比方案 A 改动更大，但模型更干净。

## 结论

一句话结论：

**我同意这次收敛后的主方案，但前提是把 typed error 这块再修正一次。否则文档里“TS 侧结构化捕获 `MultiBlockChecklist`”这句在当前代码库里是假的。**
