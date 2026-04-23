//! Config 模块的本地错误类型 —— 通过 `Into<AppError>` 转换为顶层 `AppError::Config`。
//!
//! 所有 IO/Parse 错误都在 module 内 wrap,保证 commands.rs 只跟 typed variant 打交道。

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub(super) enum ConfigError {
    #[error("config 文件读取失败({path}): {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("config 文件 JSON 解析失败({path}): {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("config 文件写入失败({path}): {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("config 目录创建失败({path}): {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("路径不存在或不是目录: {0}")]
    VaultPathInvalid(PathBuf),

    #[error("选择的目录不是 Obsidian vault(缺少 .obsidian/ 子目录): {0}")]
    NotObsidianVault(PathBuf),
}

impl From<ConfigError> for crate::app_error::AppError {
    fn from(e: ConfigError) -> Self {
        crate::app_error::AppError::Config {
            message: e.to_string(),
        }
    }
}
