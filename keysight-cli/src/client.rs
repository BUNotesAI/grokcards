//! CLI 客户端 —— 查询走 [`SqliteReadClient`](READ_ONLY SQLite),
//! 写/flush 走 [`HttpClient`](POST /rpc)。

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::endpoint::EndpointFileContents;
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

// -----------------------------------------------------------------------------
// Phase 6.2a:写/flush 客户端 —— POST /rpc
// -----------------------------------------------------------------------------

/// CLI 内置期望的 RPC 协议 major 版本。必须和 server 侧
/// `src-tauri/src/modules/keysight/http_server.rs::RPC_PROTOCOL_VERSION`(当前 1)一致。
pub const CLI_EXPECTED_RPC_MAJOR: u32 = 1;

/// 写/flush 客户端契约。
///
/// Phase 6.2a 约束:
/// - **不做 version gate** —— gate 在 [`HttpClient::new`] 构造时已完成,构造成功
///   意味着"可发写请求"的前置条件全部满足
/// - **不做 CLI-side 业务校验** —— CLI 是薄交互层,所有业务规则由 server 端
///   `MutateParams` dispatcher 判定(Phase 6.2b)
pub trait WriteClient {
    /// POST `{ "method": method, "params": params }` + header `X-Keysight-Token`。
    /// 返 response body 的 `result` 字段(未 typed — 由调用方按 method 解析)。
    fn post_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, CliError>;
}

/// HttpClient —— reqwest blocking + rustls-tls + redirect::Policy::none。
///
/// 构造成功 = version 匹配 + HTTP client 就位 + redirect 禁用。
#[derive(Debug)]
pub struct HttpClient {
    endpoint: String, // "http://127.0.0.1:51234"
    token: String,
    http: reqwest::blocking::Client,
}

impl HttpClient {
    /// 从 [`EndpointFileContents`] 构造。
    ///
    /// # 前置 gate(Phase 6.2a 契约)
    /// - `contents.rpc_protocol_version != cli_expected_major` → `Err(VersionMismatch)`,**不发 HTTP**,不构造 reqwest client
    ///
    /// # reqwest::blocking::Client 配置
    /// - `redirect::Policy::none()` —— 禁 follow redirect(security hardening;CLI 只访问 loopback,任何 redirect 都可疑)
    /// - `timeout(Duration::from_secs(30))` —— 避免 Tauri 进程卡死时 CLI hang
    /// - 默认 rustls-tls(Cargo.toml allowlist 严格)
    ///
    /// # 关联操作
    /// - [`WriteClient::post_rpc`] — 发 POST,header 带 `X-Keysight-Token`
    pub fn new(
        contents: &EndpointFileContents,
        cli_expected_major: u32,
    ) -> Result<Self, CliError> {
        // 前置 version gate —— 在构造 reqwest client 之前,不做任何网络动作
        if contents.rpc_protocol_version != cli_expected_major {
            return Err(CliError::VersionMismatch {
                cli_major: cli_expected_major,
                server_major: contents.rpc_protocol_version,
            });
        }

        // reqwest::blocking::Client:禁 redirect follow + 30s timeout
        let http = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| CliError::Http(format!("build reqwest client: {}", e)))?;

        Ok(Self {
            endpoint: contents.http_endpoint.clone(),
            token: contents.token.clone(),
            http,
        })
    }
}

impl WriteClient for HttpClient {
    fn post_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, CliError> {
        let url = format!("{}/rpc", self.endpoint);
        let body = serde_json::json!({
            "method": method,
            "params": params,
        });

        let response = self
            .http
            .post(&url)
            .header("X-Keysight-Token", &self.token)
            .json(&body)
            .send()
            .map_err(|e| CliError::Http(format!("POST {} failed: {}", url, e)))?;

        let status = response.status();
        if !status.is_success() {
            // 非 2xx(含 401)→ fail-fast。格式含 status code + canonical reason,
            // 满足 non-spec 401 test 的 "'401' 或 'unauthorized'" 断言。
            return Err(CliError::Http(format!(
                "POST {} returned HTTP {} {}",
                url,
                status.as_u16(),
                status.canonical_reason().unwrap_or("unknown"),
            )));
        }

        let parsed: serde_json::Value = response
            .json()
            .map_err(|e| CliError::Http(format!("parse response json: {}", e)))?;

        // response body 形状:`{ "result": <json value> }`(见 src-tauri http_server::RpcResponse)
        Ok(parsed
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }
}

// -----------------------------------------------------------------------------
// Tests —— Phase 6.1 ReadClient + Phase 6.2a WriteClient
// -----------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // Phase 6.2a —— WriteClient / HttpClient tests
    // -------------------------------------------------------------------------

    /// Phase 6.2a spec scenario(spec.md L174):
    /// `cli-endpoint.toml` 的 `rpc_protocol_version = 2`,CLI 内置期望 major = 1。
    /// 期望:`HttpClient::new` 前置 gate 触发 → `Err(VersionMismatch{cli:1, server:2})`,
    /// **不发 HTTP 请求**,不构造 reqwest client。
    #[test]
    fn test_write_fails_fast_on_rpc_version_major_mismatch() {
        let contents = EndpointFileContents {
            // 故意坏 endpoint URL:如果 gate 失效反而去发请求,test 会挂在连接上
            // (或更快的 ConnectionRefused)。但正确行为是根本不碰 URL,gate 前置触发
            http_endpoint: "http://127.0.0.1:1".to_string(),
            token: "placeholder-token".to_string(),
            rpc_protocol_version: 2, // 不匹配 CLI_EXPECTED_RPC_MAJOR (=1)
        };

        let result = HttpClient::new(&contents, 1);

        match result {
            Err(CliError::VersionMismatch {
                cli_major: 1,
                server_major: 2,
            }) => {}
            other => panic!(
                "expected Err(VersionMismatch {{ cli_major: 1, server_major: 2 }}), got {:?}",
                other
            ),
        }
    }

    /// Phase 6.2a non-spec 契约锁(补 spec L164 的 CLI-half):
    /// Phase 4 `test_write_rejects_with_401_when_token_mismatch` 已 green server-side;
    /// 此 test 锁 CLI-side:HttpClient 收到 401 response → `Err(CliError::Http)` 含 401 信号。
    ///
    /// 做法:起 tiny local TcpListener,返 HTTP/1.1 401;HttpClient 对同一 addr POST → 解析 401 → Err(Http)。
    #[test]
    fn test_http_client_fails_fast_on_401_response() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind tiny mock server");
        let addr = listener.local_addr().expect("local_addr");

        std::thread::spawn(move || {
            // 只接一个请求就退出;drop(listener) 关闭
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });

        let contents = EndpointFileContents {
            http_endpoint: format!("http://{}", addr),
            token: "wrong-token".to_string(),
            rpc_protocol_version: 1, // 匹配,version gate 应过
        };
        let client = HttpClient::new(&contents, 1)
            .expect("version 匹配时 HttpClient::new 必须成功");

        let result = client.post_rpc(
            "mutate",
            serde_json::json!({ "kind": "section-create", "wb": "wb_root", "title": "x" }),
        );

        match result {
            Err(CliError::Http(msg)) => {
                let lower = msg.to_lowercase();
                assert!(
                    msg.contains("401") || lower.contains("unauthorized"),
                    "CLI-side 401 handling: expected Err(Http) containing '401' or 'unauthorized', got: {}",
                    msg
                );
            }
            other => panic!(
                "expected Err(CliError::Http(..)) on 401 response, got {:?}",
                other
            ),
        }
    }
}
