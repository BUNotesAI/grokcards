//! `ServerState` —— axum HTTP IPC server 的生命周期状态管理(design-v4 D2-a)。
//!
//! 本 Phase 3 只定义 struct;Phase 4 在 `app.setup()` 里构造 instance、启动 axum
//! server、实施 4 步 shutdown 顺序契约。

use std::net::SocketAddr;

use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::endpoint_file::EndpointFileGuard;

/// keysight-cli HTTP IPC server 的运行时状态。
///
/// ## Shutdown 顺序(Phase 4 实施,本 struct 承载契约)
/// 1. `endpoint_guard.invalidate_atomic()` —— 文件系统级原子 remove,跨进程立即可见;
///    新 CLI 请求读不到 cli-endpoint.toml 立即 fail-fast "Tauri is not running"
/// 2. `shutdown_tx.send(())` —— axum graceful shutdown,停止 accept 新连接
/// 3. `timeout(5s, join_handle).await` —— 等现有 in-flight handler 完成
/// 4. `drop(endpoint_guard)` —— RAII 兜底(step 1 正常完成后为 noop;panic / crash
///    path 生效)
#[allow(dead_code)] // Phase 4 wiring
pub(super) struct ServerState {
    pub(super) shutdown_tx: oneshot::Sender<()>,
    pub(super) join_handle: JoinHandle<Result<(), ServerError>>,
    pub(super) local_addr: SocketAddr,
    pub(super) endpoint_guard: EndpointFileGuard,
}

/// axum server 运行时错误类型。Phase 4 补齐 variant(bind fail / hyper error 等)。
#[allow(dead_code)] // Phase 4 wiring
#[derive(Debug, thiserror::Error)]
pub(super) enum ServerError {
    #[error("server IO error: {0}")]
    Io(#[from] std::io::Error),
}
