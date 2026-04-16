//! axum HTTP IPC server —— keysight-cli 写/flush 命令的入口(design-v4 D2 / D2-a)。
//!
//! Bind 在 `127.0.0.1:0`(OS 分配随机端口)。通过 HTTP header
//! `X-Keysight-Token`(256-bit random hex)+ `subtle::ConstantTimeEq` 做常数时间
//! 比较认证。POST `/rpc` 接 `{ method, params }` body,dispatch 到 query / mutate /
//! flush 三路。
//!
//! ## Phase 4 scope
//! - server lifecycle(start + 4 步 shutdown 契约的 server 侧)
//! - auth middleware(`test_write_rejects_with_401_when_token_mismatch` 验)
//! - handle_rpc **stub**:三路 method 都返回空 `result: null`
//!
//! 具体 query / mutate / flush 的 domain dispatch 逻辑在 Phase 6.2 / 6.3(CLI 接入
//! 时)填充 handle_rpc body。

use std::fmt::Write as _;
use std::io;
use std::path::Path;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    routing::post,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tokio::sync::oneshot;

use super::config_file;
use super::endpoint_file::{self, EndpointFileContents, EndpointFileGuard};
use super::server_state::{ServerError, ServerState};

/// 当前 RPC 协议 major 版本(design-v4 D7 / spec D7)。
/// CLI 端内置相同期望值,mismatch 时 CLI 在发 HTTP 请求前 fail-fast。
const RPC_PROTOCOL_VERSION: u32 = 1;

/// axum Router state —— 目前只含 auth token;Phase 6.2/6.3 会扩 db handle、
/// vault path、app handle(为 emit Tauri event)等。
#[derive(Clone)]
pub(super) struct ServerContext {
    token: Arc<String>,
}

/// POST /rpc 请求 body。
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Phase 4 stub:params 在 Phase 6.2/6.3 接 domain dispatch 时读
struct RpcRequest {
    method: String,
    params: serde_json::Value,
}

/// POST /rpc 响应 body。
#[derive(Debug, Serialize)]
struct RpcResponse {
    result: serde_json::Value,
}

/// 构建 axum Router:POST /rpc + auth middleware。
pub(super) fn router(ctx: ServerContext) -> Router {
    Router::new()
        .route("/rpc", post(handle_rpc))
        .layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            auth_middleware,
        ))
        .with_state(ctx)
}

/// Auth middleware —— 比较 `X-Keysight-Token` header 与 server 启动时生成的 token。
///
/// 使用 `subtle::ConstantTimeEq` 做常数时间比较,防 timing 攻击。
async fn auth_middleware(
    State(ctx): State<ServerContext>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let provided = headers
        .get("X-Keysight-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let eq: subtle::Choice = provided.as_bytes().ct_eq(ctx.token.as_bytes());
    if bool::from(eq) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// POST /rpc handler stub —— Phase 4 只走到 auth;Phase 6.2/6.3 接 domain dispatch。
async fn handle_rpc(
    State(_ctx): State<ServerContext>,
    Json(req): Json<RpcRequest>,
) -> Result<Json<RpcResponse>, StatusCode> {
    match req.method.as_str() {
        "query" | "mutate" | "flush" => Ok(Json(RpcResponse {
            result: serde_json::Value::Null,
        })),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// 生成 256-bit random token → 64-char lowercase hex string。
///
/// 用 `rand::rng()`(ThreadRng)填充 32 字节;ThreadRng 在 rand 0.9 下是 CSPRNG,
/// 对 local-bound IPC token 的安全强度足够。
fn generate_token() -> String {
    let mut buf = [0u8; 32];
    rand::rng().fill(&mut buf[..]);
    let mut s = String::with_capacity(64);
    for byte in &buf {
        // 例外: write! 写 String 不会失败(OOM 前不分配);不上报错误
        let _ = write!(s, "{:02x}", byte);
    }
    s
}

/// 启动 axum server + 发布 endpoint 文件。Integration-test-friendly 入口 —— 测试
/// 直接调用,不走 Tauri AppHandle。setup hook 也用同一入口。
///
/// ## 顺序(对齐 design-v4 §3.5)
/// 1. `config_file::ensure(data_dir/cli-config.toml, db_path, vault_path)`
/// 2. 生成 token
/// 3. `TcpListener::bind("127.0.0.1:0")` → `local_addr`
/// 4. `tokio::spawn(axum::serve(listener, router).with_graceful_shutdown(shutdown_rx))` → `join_handle`
/// 5. `endpoint_file::write_atomic(data_dir/cli-endpoint.toml, EndpointFileContents { ... })`
/// 6. 构造并返回 `ServerState { shutdown_tx, join_handle, local_addr, endpoint_guard }`
pub(super) async fn start(
    data_dir: &Path,
    db_path: &Path,
    vault_path: &Path,
) -> io::Result<ServerState> {
    // Step 1: 幂等写 cli-config.toml
    let config_path = data_dir.join("cli-config.toml");
    config_file::ensure(&config_path, db_path, vault_path)?;

    // Step 2: 生成 256-bit token
    let token = generate_token();
    let ctx = ServerContext {
        token: Arc::new(token.clone()),
    };

    // Step 3: bind 127.0.0.1:0(OS 分配随机端口)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;

    // Step 4: spawn axum::serve + graceful shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let app = router(ctx);
    let join_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .map_err(ServerError::from)
    });

    // Step 5: atomic write cli-endpoint.toml(发布给 keysight-cli)
    let endpoint_path = data_dir.join("cli-endpoint.toml");
    let started_at = chrono::Utc::now().to_rfc3339();
    let contents = EndpointFileContents {
        http_endpoint: format!("http://{}", local_addr),
        token,
        started_at,
        rpc_protocol_version: RPC_PROTOCOL_VERSION,
    };
    endpoint_file::write_atomic(&endpoint_path, &contents)?;

    // Step 6: 构造 ServerState(Phase 4 setup hook 里 app.manage 到 Tauri state)
    Ok(ServerState {
        shutdown_tx,
        join_handle,
        local_addr,
        endpoint_guard: EndpointFileGuard::new(endpoint_path),
    })
}

// -----------------------------------------------------------------------------
// Tests —— Phase 4 scenarios(模块内 #[cfg(test)],保 pub(super) 窄接口)
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tower::ServiceExt;

    /// Scenario: Tauri 启动后 config 与 endpoint 两文件同时就位
    #[tokio::test]
    async fn test_startup_writes_both_config_and_endpoint_files() {
        let data_dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let db_path = data_dir.path().join("keysight.db");

        let server_state = start(data_dir.path(), &db_path, vault.path())
            .await
            .unwrap();

        let config_path = data_dir.path().join("cli-config.toml");
        let endpoint_path = data_dir.path().join("cli-endpoint.toml");

        assert!(config_path.exists(), "cli-config.toml 应已就位");
        assert!(endpoint_path.exists(), "cli-endpoint.toml 应已就位");

        // cleanup:正常 4 步 shutdown
        let ServerState {
            shutdown_tx,
            join_handle,
            endpoint_guard,
            local_addr: _,
        } = server_state;
        let _ = endpoint_guard.invalidate_atomic();
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(5), join_handle).await;
        drop(endpoint_guard);
    }

    /// Scenario: 正常 shutdown 后 endpoint 文件被删而 config 文件保留
    #[tokio::test]
    async fn test_shutdown_removes_endpoint_keeps_config() {
        let data_dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let db_path = data_dir.path().join("keysight.db");

        let server_state = start(data_dir.path(), &db_path, vault.path())
            .await
            .unwrap();

        let config_path = data_dir.path().join("cli-config.toml");
        let endpoint_path = data_dir.path().join("cli-endpoint.toml");
        assert!(config_path.exists());
        assert!(endpoint_path.exists());

        // 4-step shutdown contract(design-v4 D2-a)
        let ServerState {
            shutdown_tx,
            join_handle,
            endpoint_guard,
            local_addr: _,
        } = server_state;
        endpoint_guard.invalidate_atomic().unwrap(); // step 1
        shutdown_tx.send(()).unwrap(); // step 2
        let _ = tokio::time::timeout(Duration::from_secs(5), join_handle).await; // step 3
        drop(endpoint_guard); // step 4(显式)

        assert!(!endpoint_path.exists(), "shutdown 后 cli-endpoint.toml 应被删");
        assert!(config_path.exists(), "shutdown 后 cli-config.toml 应保留");
    }

    /// Scenario: 写命令遇到错误 token 时 server 返回 401 且 CLI fail-fast
    ///
    /// 用 tower::ServiceExt::oneshot 直接调 Router,不 spawn server —— 验证
    /// auth middleware 契约层面的 401 行为。
    #[tokio::test]
    async fn test_write_rejects_with_401_when_token_mismatch() {
        let ctx = ServerContext {
            token: Arc::new("good-token-abcdef".to_string()),
        };
        let app = router(ctx);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/rpc")
            .header("X-Keysight-Token", "bad-token")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(r#"{"method":"query","params":{}}"#))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "错 token 应返 401"
        );
    }
}
