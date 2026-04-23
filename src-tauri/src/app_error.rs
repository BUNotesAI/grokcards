use serde::Serialize;

/// 顶层统一错误类型，所有 Tauri command 的返回错误。
/// 各模块错误通过 `Into<AppError>` 转换。
///
/// ## 序列化形态(TS 侧看到的 tagged union)
///
/// 使用 `#[serde(tag = "kind")]`(V1.1 Phase 6.0 改动,原来是 `tag + content`):
/// 每个 variant 的字段在顶层展开,不嵌套 `content` 字段。
///
/// - `Todo { message }`        → `{ kind: "Todo", message: "..." }`
/// - `Keysight { message }`    → `{ kind: "Keysight", message: "..." }`
/// - `MultiBlockChecklist {..}`→ `{ kind: "MultiBlockChecklist", taskId, blockCount }`
///
/// TS 侧可通过 `err.kind === "MultiBlockChecklist"` 穷尽 match,拿到结构化字段,
/// 不依赖 message 字符串 parse(守 L0 硬约束:TS 不解析 Rust error message 做控制流)。
#[derive(Debug, Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "kind")]
pub enum AppError {
    #[error("{message}")]
    Todo { message: String },

    #[error("{message}")]
    Keysight { message: String },

    /// V1.1 新增 —— Kanban subtask 结构化编辑遇到多 block checklist 时抛出。
    ///
    /// 读路径(parse_task_checklist / reader 填充 subtasks)不受影响;只在
    /// 写路径 `task_update_with_subtasks` 的 `render_subtasks_into_body` 检测
    /// 到 checklist 跨多个不连续 block 时 fail-closed。TS 侧 TaskEditModal submit
    /// catch 此 variant,显示友好提示"请直接编辑 markdown 文件"。
    ///
    /// 结构化字段(TS 侧看到 `taskId` / `blockCount` camelCase,对齐其他 IPC 类型惯例)
    /// 允许 TS 侧不依赖字符串匹配定位问题 task。
    #[error("task {task_id} 的 checklist 有 {block_count} 个不连续 block,无法结构化编辑")]
    #[serde(rename_all = "camelCase")]
    MultiBlockChecklist {
        task_id: String,
        block_count: usize,
    },

    /// App 级配置错误(读 / 写 / 解析 / 校验失败)—— 见 `modules::config::errors::ConfigError`
    /// 的 `Into<AppError>` 实现。
    #[error("{message}")]
    Config { message: String },

    /// Vault 未配置 —— keysight commands 在 runtime state 尚未 install 时抛出。
    ///
    /// TS 侧 `err.kind === "VaultNotConfigured"` 可穷尽 match,首次启动时
    /// 正常触发(此时前端应展示 VaultSetup 对话框)。
    #[error("vault 未配置,请先选择 Obsidian vault 根目录")]
    VaultNotConfigured,
}

impl From<keysight_core::errors::KeysightError> for AppError {
    fn from(e: keysight_core::errors::KeysightError) -> Self {
        // MultiBlockChecklist 走独立的 AppError variant 保留结构化字段;
        // 其他所有 KeysightError 变体 flatten 为 AppError::Keysight { message } 字符串
        // (V1.2 若把 KeysightError 全面 typed through IPC,这一段可以整体换成嵌套传递)。
        match e {
            keysight_core::errors::KeysightError::MultiBlockChecklist {
                task_id,
                block_count,
            } => AppError::MultiBlockChecklist {
                task_id,
                block_count,
            },
            other => AppError::Keysight {
                message: other.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V1.1 Phase 6.0 回归:AppError::Todo 保持 `{kind, message}` 形态,
    /// 现存 TS consumer(CreateTaskModal `err.message` 显示)不受破坏。
    #[test]
    fn todo_variant_serializes_with_message_field() {
        let err = AppError::Todo {
            message: "boom".to_string(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "Todo");
        assert_eq!(json["message"], "boom");
    }

    /// V1.1 Phase 6.0 回归:AppError::Keysight 保持 `{kind, message}` 形态。
    #[test]
    fn keysight_variant_serializes_with_message_field() {
        let err = AppError::Keysight {
            message: "db error".to_string(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "Keysight");
        assert_eq!(json["message"], "db error");
    }

    /// V1.1 Phase 6.0 核心:AppError::MultiBlockChecklist 序列化成带结构化字段的
    /// tagged union,TS 侧可按 kind 穷尽 match 并拿到 taskId + blockCount。
    ///
    /// camelCase 字段名对齐项目其他 IPC 类型惯例(见 `TaskEntity` / `Subtask` 等),
    /// TS 侧可以写 `err.taskId` / `err.blockCount` 而不是 `err.task_id` / `err.block_count`。
    #[test]
    fn multi_block_checklist_serializes_with_structured_fields() {
        let err = AppError::MultiBlockChecklist {
            task_id: "task_abc123".to_string(),
            block_count: 2,
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "MultiBlockChecklist");
        assert_eq!(json["taskId"], "task_abc123");
        assert_eq!(json["blockCount"], 2);
    }
}
