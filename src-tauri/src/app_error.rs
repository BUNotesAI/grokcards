use serde::Serialize;

/// 顶层统一错误类型，所有 Tauri command 的返回错误。
/// 各模块错误通过 `Into<AppError>` 转换。
#[derive(Debug, Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("{0}")]
    Todo(String),

    #[error("{0}")]
    Keysight(String),
}
