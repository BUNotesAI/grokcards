//! 只读查询客户端 —— READ_ONLY SQLite 连接包装。

use std::path::Path;

use rusqlite::Connection;

use crate::errors::CliError;

/// Phase 6.1 —— 只读查询契约。Bootstrap 阶段只要求 `open`;
/// 后续若需 mock 实现可扩此 trait。
pub trait ReadClient {
    // 只读契约:Bootstrap 阶段空
}

/// SQLite 实现。`conn` 暴露给同 crate 的 command handlers 调 keysight-core domain fn。
pub struct SqliteReadClient {
    pub(crate) conn: Connection,
}

impl SqliteReadClient {
    /// Phase 6.1 契约(spec scenario test_read_client_open_does_not_set_write_pragmas):
    ///
    /// - 必须用 `OpenFlags::SQLITE_OPEN_READ_ONLY`
    /// - **禁带** `SQLITE_OPEN_CREATE`(DB 不在时 fail-fast,不悄悄建空库)
    /// - **禁调** `pragma_update` 设置 `journal_mode` / `busy_timeout`
    pub fn open(db_path: &Path) -> Result<Self, CliError> {
        // 显式 existence check:READ_ONLY 对不存在 DB 会返 rusqlite 通用 SqliteFailure(code 14),
        // 我们提前 fail-fast 到 `CliError::DatabaseNotFound` 以满足 spec 契约
        // (stderr 消息 + 路径,且不创建空库)
        if !db_path.exists() {
            return Err(CliError::DatabaseNotFound {
                path: db_path.to_path_buf(),
            });
        }

        let conn = Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| CliError::Sqlite(e.to_string()))?;

        // 契约:**禁调** pragma_update 设置 journal_mode / busy_timeout
        // (保持 SQLite 默认值,测试会验证)
        Ok(Self { conn })
    }
}

impl ReadClient for SqliteReadClient {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "keysight_cli_client_test_{}_{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// Phase 6.1 spec scenario(spec.md L132):
    /// 契约 = 我方代码不调 pragma_update 改 journal_mode / busy_timeout,
    /// 且 open_with_flags 不带 SQLITE_OPEN_CREATE。
    ///
    /// 混合断言(runtime + 源码):
    /// 1. 运行时:journal_mode ≠ WAL(rusqlite 默认非 WAL,调 pragma_update 改 WAL 是最明显的写 pragma)
    /// 2. 源码:client.rs 的 impl 区不含 `pragma_update` 调用和 `SQLITE_OPEN_CREATE` 常量
    ///
    /// 注:busy_timeout 不断言具体值 —— rusqlite 0.33 `open_with_flags` 默认 5000ms,
    /// 这不是"我方代码设置的 write pragma",不归本契约管。
    #[test]
    fn test_read_client_open_does_not_set_write_pragmas() {
        // ===== runtime part =====
        let dir = temp_root("read_pragma");
        let db_path = dir.join("keysight.db");
        Connection::open(&db_path).unwrap();

        let client = SqliteReadClient::open(&db_path).unwrap();

        let journal_mode: String = client
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_ne!(
            journal_mode.to_lowercase(),
            "wal",
            "ReadClient::open must NOT set journal_mode=WAL"
        );

        // ===== source-level part =====
        // 只扫非 test 区;匹配"使用模式"而非裸字符串,避免匹配 doc comment 里
        // 的 `` `pragma_update` `` / `` `SQLITE_OPEN_CREATE` `` 提示文本。
        let source = include_str!("client.rs");
        let impl_part = source.split("#[cfg(test)]").next().unwrap_or("");
        assert!(
            !impl_part.contains(".pragma_update("),
            "ReadClient impl must NOT call `.pragma_update(...)`"
        );
        assert!(
            !impl_part.contains("OpenFlags::SQLITE_OPEN_CREATE"),
            "ReadClient impl must NOT use `OpenFlags::SQLITE_OPEN_CREATE`"
        );
    }
}
