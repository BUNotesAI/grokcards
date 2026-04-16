#![allow(dead_code)] // domain 模块逐步实现后自然消除

/// KeySight 模块内部错误类型。
#[derive(Debug, thiserror::Error)]
pub enum KeysightError {
    #[error("标题不能为空")]
    EmptyTitle,

    #[error("实体未找到: {0}")]
    NotFound(String),

    #[error("无效的实体类型: {0}")]
    InvalidEntityType(String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("文件操作失败: {0}")]
    FileError(String),

    /// 用户尝试从不允许主动发 edge 的 entity kind 画出箭头(当前 section / task)
    #[error("连接不合法: {from_kind} 不能作为 edge 的 from (业务规则)")]
    ConnectionNotAllowed { from_kind: &'static str },

    /// Project 名不合法 —— 空串 / 含路径分隔符 / 含 Windows 禁用字符。
    #[error("project 名不合法: {0}")]
    InvalidProjectName(String),

    /// Task 状态字符串不合法 —— 不在 next/active/blocked/done 枚举内。
    #[error("task 状态不合法: {0}")]
    InvalidTaskStatus(String),

    /// V1.1 Phase 6.0 新增 —— Kanban subtask 结构化编辑遇到多 block checklist。
    ///
    /// 由 `render_subtasks_into_body` 检测 checklist 行不连续时抛出。透传到
    /// `AppError::MultiBlockChecklist { task_id, block_count }`(保留结构化字段,
    /// 不走 `AppError::Keysight(String)` 的 flatten 路径),让 TS 侧可按 typed
    /// variant 做分支而不是字符串匹配。
    #[error("task {task_id} 的 checklist 有 {block_count} 个不连续 block")]
    MultiBlockChecklist {
        task_id: String,
        block_count: usize,
    },

    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
}

// `impl From<KeysightError> for AppError` 搬到 src-tauri/src/app_error.rs
// —— keysight-core 不知道 AppError(防止循环依赖:AppError 属于 Tauri app 的顶层错误类型,
// keysight-core 不应感知上层壳)。按 Rust orphan rule,From impl 必须在 AppError 或
// KeysightError 其中一个的 defining crate,这里选 src-tauri(AppError 那边)。
