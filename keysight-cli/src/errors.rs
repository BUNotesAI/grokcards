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

    /// Phase 6.1/6.2a deferred 命令统一错误出口:
    /// - Phase 6.1 deferred:`weak-list` / `graph get-bounds`
    /// - Phase 6.2a deferred:`weak-add` / `weak-remove`
    NotImplemented {
        command: &'static str,
        reason: &'static str,
    },

    /// Config 文件解析 / IO 错误
    Config(String),

    /// Endpoint 文件解析 / IO 错误(`cli-endpoint.toml` 缺失/格式错)
    Endpoint(String),

    /// SQLite 开库或查询错误
    Sqlite(String),

    /// 底层 keysight-core domain 查询错误
    Query(String),

    /// HTTP 请求错误(reqwest IO / connect timeout / non-2xx response 等)
    Http(String),

    /// RPC 协议版本 major 不匹配 —— `HttpClient::new` 前置 gate 触发,**不发 HTTP**
    VersionMismatch {
        cli_major: u32,
        server_major: u32,
    },
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
            Self::Endpoint(msg) => write!(f, "endpoint error: Tauri is not running ({})", msg),
            Self::Sqlite(msg) => write!(f, "sqlite error: {}", msg),
            Self::Query(msg) => write!(f, "query error: {}", msg),
            Self::Http(msg) => write!(f, "http error: {}", msg),
            Self::VersionMismatch { cli_major, server_major } => write!(
                f,
                "RPC protocol mismatch: CLI expects major {}, server reports major {}",
                cli_major, server_major
            ),
        }
    }
}

impl std::error::Error for CliError {}

impl From<KeysightError> for CliError {
    fn from(e: KeysightError) -> Self {
        Self::Query(e.to_string())
    }
}
