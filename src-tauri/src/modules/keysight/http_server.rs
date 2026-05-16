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
//! ## Phase 6.2b scope
//! - ServerContext 扩 db / vault_path / emitter(供 dispatcher 用)
//! - handle_rpc `"mutate"` 分支:deserialize `MutateParams` → 调 `dispatch_mutate`
//!   → 序列化 `MutateResponse` 返;成功后 dispatcher 内部 emit `"entity:changed"`
//! - handle_rpc `"flush"` 分支:直接 emit `"vault:flush"` event + 返 success
//! - handle_rpc `"query"` 分支:保留 Phase 4 Null stub(Phase 7 决策)

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    routing::post,
};
use keysight_core::ipc::{MutateParams, MutateResponse};
use rand::Rng;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tokio::sync::oneshot;

use super::config_file;
use super::dispatcher::{self, EventEmitter};
use super::endpoint_file::{self, EndpointFileContents, EndpointFileGuard};
use super::recording_vault_fs::RecordingVaultFs;
use super::server_state::{ServerError, ServerState};
use super::suppression::SelfWriteSuppression;

/// 当前 RPC 协议 major 版本(design-v4 D7 / spec D7)。
/// CLI 端内置相同期望值,mismatch 时 CLI 在发 HTTP 请求前 fail-fast。
const RPC_PROTOCOL_VERSION: u32 = 1;

/// axum Router state —— 含 auth token + db 连接 + vault_path + emitter + suppression。
///
/// `suppression` 与 Tauri commands 侧的 `KeysightRuntimeState.suppression` 共享
/// 同一 `Arc` —— CLI 通过 IPC 写的文件也会登记进 writes 队列,watcher 收到事件时
/// 能识别为自写并 skip(Phase 5b wiring)。
#[derive(Clone)]
pub(super) struct ServerContext {
    pub(super) token: Arc<String>,
    pub(super) db: Arc<Mutex<Connection>>,
    pub(super) vault_path: PathBuf,
    pub(super) emitter: Arc<dyn EventEmitter>,
    pub(super) suppression: Arc<SelfWriteSuppression>,
}

/// POST /rpc 请求 body。
#[derive(Debug, Deserialize)]
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

/// POST /rpc handler —— Phase 6.2b 实分派(mutate + flush);query 保留 Phase 4 Null。
async fn handle_rpc(
    State(ctx): State<ServerContext>,
    Json(req): Json<RpcRequest>,
) -> Result<Json<RpcResponse>, StatusCode> {
    match req.method.as_str() {
        "mutate" => {
            let params: MutateParams = serde_json::from_value(req.params).map_err(|e| {
                eprintln!("[keysight] mutate params parse 失败: {e}");
                StatusCode::BAD_REQUEST
            })?;
            // lock DB connection —— 与 Tauri commands 的 KeysightState.db 是不同
            // handle,WAL 层面写串行,各自 transaction 看到 commit 后的快照。
            let conn = ctx.db.lock().map_err(|e| {
                eprintln!("[keysight] mutate db lock poisoned: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            // Phase 5b wiring:IPC mutate 路径走 RecordingVaultFs,让 watcher 识别自写
            let vault_fs = RecordingVaultFs::wrap_real(
                ctx.vault_path.to_string_lossy().to_string(),
                Arc::clone(&ctx.suppression),
            );
            let resp = dispatcher::dispatch_mutate(params, &conn, &vault_fs, ctx.emitter.as_ref())
                .map_err(|e| {
                    eprintln!("[keysight] dispatch_mutate 失败: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            Ok(Json(RpcResponse {
                result: serde_json::to_value(&resp).map_err(|e| {
                    eprintln!("[keysight] mutate response serialize 失败: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?,
            }))
        }
        "flush" => {
            ctx.emitter.emit("vault:flush", &serde_json::Value::Null);
            // flush 目前只发事件让 Tauri 侧 UI 重新 sync,没有结构化返回值
            let resp = MutateResponse {
                success: true,
                entity_id: None,
                message: None,
            };
            Ok(Json(RpcResponse {
                result: serde_json::to_value(&resp).unwrap_or(serde_json::Value::Null),
            }))
        }
        "query" => {
            // Phase 6.2b 保留 Phase 4 stub:CLI 查询走直连 SQLite 路径,不走 /rpc。
            // Phase 7 如决定 server-side query 再实装。
            Ok(Json(RpcResponse {
                result: serde_json::Value::Null,
            }))
        }
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
/// 3. 开一个独立 `Connection` 给 axum dispatcher 用(WAL 支持同进程多 Connection)
/// 4. `TcpListener::bind("127.0.0.1:0")` → `local_addr`
/// 5. `tokio::spawn(axum::serve(listener, router).with_graceful_shutdown(shutdown_rx))` → `join_handle`
/// 6. `endpoint_file::write_atomic(data_dir/cli-endpoint.toml, EndpointFileContents { ... })`
/// 7. 构造并返回 `ServerState { shutdown_tx, join_handle, local_addr, endpoint_guard }`
pub(super) async fn start(
    data_dir: &Path,
    db_path: &Path,
    vault_path: &Path,
    emitter: Arc<dyn EventEmitter>,
    suppression: Arc<SelfWriteSuppression>,
) -> io::Result<ServerState> {
    // Step 1: 幂等写 cli-config.toml
    let config_path = data_dir.join("cli-config.toml");
    config_file::ensure(&config_path, db_path, vault_path)?;

    // Step 2: 生成 256-bit token
    let token = generate_token();

    // Step 3: 开独立 Connection(与 KeysightState.db 不同 handle,WAL 文件级串行)
    let ipc_conn = Connection::open(db_path).map_err(|e| io::Error::other(e.to_string()))?;
    let db = Arc::new(Mutex::new(ipc_conn));

    let ctx = ServerContext {
        token: Arc::new(token.clone()),
        db,
        vault_path: vault_path.to_path_buf(),
        emitter,
        suppression,
    };

    // Step 4: bind 127.0.0.1:0(OS 分配随机端口)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;

    // Step 5: spawn axum::serve + graceful shutdown
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

    // Step 6: atomic write cli-endpoint.toml(发布给 keysight-cli)
    let endpoint_path = data_dir.join("cli-endpoint.toml");
    let started_at = chrono::Utc::now().to_rfc3339();
    let contents = EndpointFileContents {
        http_endpoint: format!("http://{}", local_addr),
        token,
        started_at,
        rpc_protocol_version: RPC_PROTOCOL_VERSION,
    };
    endpoint_file::write_atomic(&endpoint_path, &contents)?;

    // Step 7: 构造 ServerState(Phase 4 setup hook 里 app.manage 到 Tauri state)
    Ok(ServerState {
        shutdown_tx,
        join_handle,
        local_addr,
        endpoint_guard: EndpointFileGuard::new(endpoint_path),
    })
}

// -----------------------------------------------------------------------------
// Tests —— Phase 4(lifecycle / 401)+ Phase 6.2b(tower oneshot 集成)
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keysight::dispatcher::RecordingEmitter;
    use keysight_core::db::init_db;
    use std::time::Duration;
    use tower::ServiceExt;

    /// Helper:起一个带 schema + wb_root seed 的 in-memory Connection,包 Arc<Mutex>
    fn seeded_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('wb_root', 'whiteboard', 'Root', 'wb_root')",
            [],
        )
        .unwrap();
        Arc::new(Mutex::new(conn))
    }

    fn test_emitter() -> Arc<RecordingEmitter> {
        Arc::new(RecordingEmitter::new())
    }

    /// Helper:构造 test ServerContext(in-memory DB + tempdir vault + recording emitter
    /// + 全新 SelfWriteSuppression —— test 不校验 suppression 命中,用空队列占位即可)
    fn test_ctx(
        db: Arc<Mutex<Connection>>,
        emitter: Arc<dyn EventEmitter>,
    ) -> (ServerContext, tempfile::TempDir) {
        let vault = tempfile::tempdir().unwrap();
        let ctx = ServerContext {
            token: Arc::new("test-token".to_string()),
            db,
            vault_path: vault.path().to_path_buf(),
            emitter,
            suppression: Arc::new(SelfWriteSuppression::new()),
        };
        (ctx, vault)
    }

    /// Helper: start() 用的 no-op emitter(lifecycle tests 不 care emit)
    fn noop_emitter() -> Arc<dyn EventEmitter> {
        Arc::new(RecordingEmitter::new())
    }

    /// Helper:lifecycle tests 不 care suppression 命中,直接给独立实例
    fn test_suppression() -> Arc<SelfWriteSuppression> {
        Arc::new(SelfWriteSuppression::new())
    }

    /// Helper:查找 keysight-cli binary 路径。
    ///
    /// Workspace target 目录可由 `~/.cargo/config.toml [build] target-dir` 重定向
    /// (例:用户设了 target-shared 跨项目共享),不能假设是项目本地 `target/`。
    /// 用 `cargo metadata` 取真实 target dir,target-dir-agnostic。
    fn cli_bin_path() -> std::path::PathBuf {
        let output = std::process::Command::new(env!("CARGO"))
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .output()
            .expect("cargo metadata failed");
        let meta: serde_json::Value = serde_json::from_slice(&output.stdout)
            .expect("cargo metadata JSON parse failed");
        let target_dir = meta["target_directory"]
            .as_str()
            .expect("metadata 缺 target_directory");
        std::path::PathBuf::from(target_dir)
            .join("debug")
            .join("keysight-cli")
    }

    /// Scenario: Tauri 启动后 config 与 endpoint 两文件同时就位
    #[tokio::test]
    async fn test_startup_writes_both_config_and_endpoint_files() {
        let data_dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let db_path = data_dir.path().join("keysight.db");
        // 预建空 DB 使得 Connection::open 不 fail(init_db 不在此 test 运行路径)
        {
            let conn = Connection::open(&db_path).unwrap();
            init_db(&conn).unwrap();
        }

        let server_state = start(
            data_dir.path(),
            &db_path,
            vault.path(),
            noop_emitter(),
            test_suppression(),
        )
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
        {
            let conn = Connection::open(&db_path).unwrap();
            init_db(&conn).unwrap();
        }

        let server_state = start(
            data_dir.path(),
            &db_path,
            vault.path(),
            noop_emitter(),
            test_suppression(),
        )
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
        let (ctx, _vault) = test_ctx(seeded_db(), noop_emitter());
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

    /// Phase 6.2b Red scenario(spec L141 in-process 变体):
    /// POST /rpc `{method: "mutate", params: SectionCreate}` → server 应返 200,
    /// body 可反序列化为 `MutateResponse { success: true, entity_id: Some(sec_*) }`,
    /// DB 应新增 section 行,emitter 应录到 1 条 "entity:changed" event。
    ///
    /// Red 阶段:`dispatch_mutate` stub 返 Err → handle_rpc 返 500 → 断言 200 **失败**。
    /// Green 阶段:dispatch_mutate 实装 → 全部断言通过。
    #[tokio::test]
    async fn test_mutate_via_http_emits_entity_changed_and_writes_db() {
        let db = seeded_db();
        let emitter = test_emitter();
        let (ctx, _vault) = test_ctx(Arc::clone(&db), Arc::clone(&emitter) as Arc<dyn EventEmitter>);
        let app = router(ctx);

        let body = serde_json::json!({
            "method": "mutate",
            "params": {
                "kind": "section-create",
                "wb": "wb_root",
                "title": "HttpSec",
                "color": null,
            },
        });

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/rpc")
            .header("X-Keysight-Token", "test-token")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "期望 200,实际 {status};body={}",
            String::from_utf8_lossy(&body_bytes)
        );

        // 解析 RpcResponse.result 为 MutateResponse
        let rpc: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let mutate: MutateResponse = serde_json::from_value(rpc["result"].clone())
            .expect("result 应可反序列化为 MutateResponse");
        assert!(mutate.success);
        let id = mutate.entity_id.expect("SectionCreate 应返 entity_id");
        assert!(id.starts_with("sec_"));

        // DB 断言
        let count: i64 = db
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE kind = 'section' AND title = 'HttpSec'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // emit 断言
        let events = emitter.events();
        assert!(
            events.iter().any(|(name, _)| name == "entity:changed"),
            "应 emit entity:changed,实际 events={events:?}"
        );
    }

    /// Phase 6.2c spec L141(subprocess 真补):启真 TCP server + spawn keysight-cli
    /// 子进程走 HTTP → 断言 exit 0 + DB 新增 section + emitter 录 entity:changed。
    ///
    /// 与 `test_mutate_via_http_emits_entity_changed_and_writes_db`(tower oneshot
    /// in-process)互补:本 test 验证"真 CLI → 真 HTTP → 真 dispatcher → 真 DB → 真 emit"
    /// 的端到端链路,锁 endpoint/config 文件协议 + X-Keysight-Token header 接线。
    ///
    /// 前置条件:workspace 里 `keysight-cli` binary 已 build(`cargo test --workspace`
    /// 会自动满足;单 crate test 需先 `cargo build -p keysight-cli`)。
    ///
    /// 用 `multi_thread` flavor —— subprocess `.output()` 会 block 当前线程,
    /// current_thread runtime 会让 axum accept loop 饿死(deadlock)。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_write_command_posts_rpc_when_tauri_online() {
        // 1. 构 tempdir + db + vault + seed wb_root
        let data_dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let db_path = data_dir.path().join("keysight.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            init_db(&conn).unwrap();
            conn.execute(
                "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('wb_root', 'whiteboard', 'Root', 'wb_root')",
                [],
            )
            .unwrap();
        }

        // 2. 启真 TCP server(走 start → bind 127.0.0.1:0 + 发布 cli-endpoint.toml)
        let emitter = test_emitter();
        let suppression = test_suppression();
        let server_state = start(
            data_dir.path(),
            &db_path,
            vault.path(),
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&suppression),
        )
        .await
        .unwrap();

        // 3. 确认 keysight-cli binary 已 build
        let cli_bin = cli_bin_path();
        assert!(
            cli_bin.exists(),
            "keysight-cli binary 未 build: {}\n先跑 `cargo build -p keysight-cli`",
            cli_bin.display()
        );

        // 4. spawn subprocess —— 用 data_dir 的 cli-config.toml + cli-endpoint.toml
        let output = std::process::Command::new(&cli_bin)
            .arg("--config-file")
            .arg(data_dir.path().join("cli-config.toml"))
            .arg("--endpoint-file")
            .arg(data_dir.path().join("cli-endpoint.toml"))
            .args([
                "graph",
                "section-create",
                "HttpSec",
                "--wb",
                "wb_root",
            ])
            .output()
            .expect("spawn keysight-cli failed");

        // 5. 断言前先 shutdown server(避免 flaky test 因 panic 导致 server 泄漏)
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

        // 6. CLI 应 exit 0
        assert!(
            output.status.success(),
            "CLI 应 exit 0;实际 status={:?}\nstdout={}\nstderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );

        // 7. DB 新增 section
        {
            let conn = Connection::open(&db_path).unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM entities WHERE kind = 'section' AND title = 'HttpSec'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "CLI section-create 应在 DB 新增一行 HttpSec section");
        }

        // 8. emitter 应录到 entity:changed
        let events = emitter.events();
        assert!(
            events.iter().any(|(name, _)| name == "entity:changed"),
            "应收到 entity:changed event;实际 events={:?}",
            events
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
        );
    }

    /// Phase 6.3 spec L256(subprocess e2e):启真 TCP server + spawn `keysight-cli flush`
    /// 子进程走 HTTP → 断言 exit 0 + emitter 录到 `vault:flush` event。
    ///
    /// server 侧 flush 分支在 Phase 6.2b 已就位(commit 1e3b413,http_server.rs:144-155
    /// `"flush" => emitter.emit("vault:flush", Null)`);本 test 锁 CLI → /rpc → emit
    /// 的端到端链路,和 Phase 6.2c section-create subprocess test 并列互补。
    ///
    /// 与 mutate 路径区别:flush 不写 DB(无 dispatch_mutate),故无 DB 断言。
    ///
    /// 用 `multi_thread` flavor —— subprocess `.output()` 会 block 当前线程,
    /// current_thread runtime 会让 axum accept loop 饿死(deadlock)。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_flush_command_posts_rpc_and_emits_vault_flush_event() {
        // 1. 构 tempdir + db + vault(flush 不写 DB,但 start 需要有效 db_path)
        let data_dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let db_path = data_dir.path().join("keysight.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            init_db(&conn).unwrap();
        }

        // 2. 启真 TCP server(发布 cli-endpoint.toml)
        let emitter = test_emitter();
        let suppression = test_suppression();
        let server_state = start(
            data_dir.path(),
            &db_path,
            vault.path(),
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&suppression),
        )
        .await
        .unwrap();

        // 3. 确认 keysight-cli binary 已 build
        let cli_bin = cli_bin_path();
        assert!(
            cli_bin.exists(),
            "keysight-cli binary 未 build: {}\n先跑 `cargo build -p keysight-cli`",
            cli_bin.display()
        );

        // 4. spawn subprocess `keysight-cli flush`
        let output = std::process::Command::new(&cli_bin)
            .arg("--config-file")
            .arg(data_dir.path().join("cli-config.toml"))
            .arg("--endpoint-file")
            .arg(data_dir.path().join("cli-endpoint.toml"))
            .arg("flush")
            .output()
            .expect("spawn keysight-cli failed");

        // 5. 断言前先 shutdown server(避免 panic 导致 server 泄漏)
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

        // 6. CLI 应 exit 0
        assert!(
            output.status.success(),
            "CLI 应 exit 0;实际 status={:?}\nstdout={}\nstderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );

        // 7. emitter 应录到 vault:flush
        let events = emitter.events();
        assert!(
            events.iter().any(|(name, _)| name == "vault:flush"),
            "应收到 vault:flush event;实际 events={:?}",
            events
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
        );
    }

    /// Phase 7 spec scenario(spec.md L81):Shutdown step 1 执行后新 CLI 写命令立即 fail-fast。
    ///
    /// 验证 design-v4 D2-a shutdown 顺序契约的跨进程语义:
    /// `endpoint_guard.invalidate_atomic()`(step 1)瞬间删除 `cli-endpoint.toml`,
    /// step 2-4 还没跑完时,新启的 CLI 写命令应通过 endpoint 文件缺失被立即拦截,
    /// 不必等 server 完全关闭、无 race window。
    ///
    /// 与 in-process `test_shutdown_removes_endpoint_keeps_config`(L353)互补:
    /// 那个 verify 4 步全跑完后的文件状态;本 test 锁住 step 1 完成的瞬间,新
    /// CLI 子进程读 `cli-endpoint.toml` 必须 fail-fast。也与 Phase 6.2a offline
    /// test `test_write_fails_fast_when_endpoint_file_missing`(keysight-cli/
    /// tests/cli_write_integration.rs)互补:那个验证 endpoint 文件从未存在的
    /// 场景;本 test 锁住"server 正在 shutdown 中"的并发场景。
    ///
    /// 用 `multi_thread` flavor —— subprocess `.output()` block 当前线程,
    /// current_thread runtime 会让 axum accept loop 饿死(Phase 6.2c 教训)。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_shutdown_step1_atomic_remove_blocks_new_write_cli() {
        // 1. 构 tempdir + db + vault + seed wb_root
        let data_dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let db_path = data_dir.path().join("keysight.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            init_db(&conn).unwrap();
            conn.execute(
                "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('wb_root', 'whiteboard', 'Root', 'wb_root')",
                [],
            )
            .unwrap();
        }

        // 2. 启真 TCP server(发布 cli-endpoint.toml + cli-config.toml)
        let emitter = test_emitter();
        let suppression = test_suppression();
        let server_state = start(
            data_dir.path(),
            &db_path,
            vault.path(),
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&suppression),
        )
        .await
        .unwrap();

        let endpoint_path = data_dir.path().join("cli-endpoint.toml");
        let config_path = data_dir.path().join("cli-config.toml");
        assert!(endpoint_path.exists(), "前置:endpoint 文件应已就位");
        assert!(config_path.exists(), "前置:config 文件应已就位");

        // 3. 拆 ServerState(只取 endpoint_guard,server 仍在跑)
        let ServerState {
            shutdown_tx,
            join_handle,
            endpoint_guard,
            local_addr: _,
        } = server_state;

        // 4. Step 1 only —— 删 endpoint 文件,server 仍在跑(模拟 shutdown 进行中)
        endpoint_guard.invalidate_atomic().unwrap();
        assert!(
            !endpoint_path.exists(),
            "step 1 后 cli-endpoint.toml 应已被删"
        );

        // 5. 确认 keysight-cli binary 已 build
        let cli_bin = cli_bin_path();
        assert!(
            cli_bin.exists(),
            "keysight-cli binary 未 build: {}\n先跑 `cargo build -p keysight-cli`",
            cli_bin.display()
        );

        // 6. spawn 新 CLI 写命令 —— 应 fail-fast(endpoint 文件不存在)
        let output = std::process::Command::new(&cli_bin)
            .arg("--config-file")
            .arg(&config_path)
            .arg("--endpoint-file")
            .arg(&endpoint_path)
            .args([
                "graph",
                "section-create",
                "ShutdownRace",
                "--wb",
                "wb_root",
            ])
            .output()
            .expect("spawn keysight-cli failed");

        // 7. 完成 shutdown step 2-4(避免 server 泄漏到下一个 test)
        let _ = shutdown_tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(5), join_handle).await;
        drop(endpoint_guard);

        // 8. 断言 CLI exit 1 + stderr 包含 "Tauri is not running"
        assert_eq!(
            output.status.code(),
            Some(1),
            "CLI 应 exit 1;实际 status={:?}\nstdout={}\nstderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Tauri is not running"),
            "stderr 应包含 \"Tauri is not running\";实际: {}",
            stderr
        );

        // 9. DB 未新增 entity(CLI 在 endpoint load 阶段 fail-fast,未发起 HTTP)
        {
            let conn = Connection::open(&db_path).unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM entities WHERE kind = 'section' AND title = 'ShutdownRace'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "CLI fail-fast 后不应写入任何 DB 行");
        }
    }
}
