#![allow(dead_code)] // domain 模块逐步实现后自然消除

use crate::app_error::AppError;

/// KeySight 模块内部错误类型。
#[derive(Debug, thiserror::Error)]
pub(super) enum KeysightError {
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

    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
}

impl From<KeysightError> for AppError {
    fn from(e: KeysightError) -> Self {
        AppError::Keysight(e.to_string())
    }
}
