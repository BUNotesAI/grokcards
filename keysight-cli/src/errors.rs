//! CLI 错误类型。
//!
//! 手写 `Display` + `std::error::Error`(allowlist 不含 `thiserror`)。
//! `NotImplemented` 专门给 Phase 6.1 deferred 命令用,dispatcher 映 exit 2;
//! 其他 variant 都走 exit 1。

use std::fmt;
use std::path::PathBuf;

use keysight_core::errors::KeysightError;

#[derive(Debug)]
pub enum CliError {
    /// DB 文件不存在(ReadClient 开 READ_ONLY 无 CREATE,DB 不在即 fail-fast)
    DatabaseNotFound { path: PathBuf },

    /// Phase 6.1 deferred 命令统一错误出口(weak-list / graph-get-bounds)
    NotImplemented {
        command: &'static str,
        reason: &'static str,
    },

    /// Config 文件解析 / IO 错误
    Config(String),

    /// SQLite 开库或查询错误
    Sqlite(String),

    /// 底层 keysight-core domain 查询错误
    Query(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseNotFound { path } => {
                write!(f, "database not found: {}", path.display())
            }
            Self::NotImplemented { command, reason } => {
                write!(f, "{}: not yet implemented ({})", command, reason)
            }
            Self::Config(msg) => write!(f, "config error: {}", msg),
            Self::Sqlite(msg) => write!(f, "sqlite error: {}", msg),
            Self::Query(msg) => write!(f, "query error: {}", msg),
        }
    }
}

impl std::error::Error for CliError {}

impl From<KeysightError> for CliError {
    fn from(e: KeysightError) -> Self {
        Self::Query(e.to_string())
    }
}
