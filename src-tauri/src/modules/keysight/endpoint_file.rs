//! `cli-endpoint.toml` 短生命周期 endpoint 发布(design-v4 D3b)+ 配套的
//! [`EndpointFileGuard`] RAII 清理(D2-a)。
//!
//! Tauri app HTTP server listen 成功后 [`write_atomic`] 发布 endpoint;shutdown
//! 顺序契约(见 `server_state::ServerState` doc):
//! 1. `endpoint_guard.invalidate_atomic()` — 原子删除 cli-endpoint.toml,跨进程立即可见
//! 2. `shutdown_tx.send(())` — axum graceful shutdown
//! 3. `timeout(5s, join_handle).await` — drain in-flight handlers
//! 4. `drop(endpoint_guard)` — crash path 兜底(正常 shutdown 后 noop)
//!
//! 跨进程语义:文件存在 ↔ server ready;文件不存在 ↔ server 不可用。CLI 通过
//! HTTP ping connection refused 判定离线 fail-fast,不做 pid 探活。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

/// cli-endpoint.toml 的内容 schema。
///
/// Phase 3 只定义 struct + write_atomic,Phase 4 启 server 时 instantiate。
#[allow(dead_code)] // Phase 4 wiring
#[derive(Debug, Serialize)]
pub(super) struct EndpointFileContents {
    pub(super) http_endpoint: String,
    pub(super) token: String,
    pub(super) started_at: String,
    pub(super) rpc_protocol_version: u32,
}

/// RAII guard —— 持有 `cli-endpoint.toml` 路径,在 [`invalidate_atomic`](Self::invalidate_atomic)
/// 或 [`Drop`] 时原子删除该文件。
///
/// ## Invariants
/// - **幂等**:`AtomicBool::swap` 保证首次 `invalidate_atomic` 成功后,再次调用是 noop
/// - **Drop 兜底**:正常 shutdown 路径手动调 `invalidate_atomic`;panic / 异常退出时,
///   Drop 仍会尝试删除(`let _ = invalidate_atomic()` 吞错)
/// - **文件系统级跨进程语义**:文件存在/不存在是 CLI 唯一的"server 是否在线"信号
pub(super) struct EndpointFileGuard {
    path: PathBuf,
    invalidated: AtomicBool,
}

impl EndpointFileGuard {
    #[allow(dead_code)] // Phase 4 wiring:setup hook 构造 instance
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            invalidated: AtomicBool::new(false),
        }
    }

    /// 原子删除 cli-endpoint.toml。幂等。
    ///
    /// ## 规则
    /// - 首次调用成功删除 → `Ok(())`
    /// - 再次调用 → `AtomicBool::swap` 命中 invalidated flag → 直接 `Ok(())`,不再碰 fs
    /// - 文件已被外部删除 → `io::ErrorKind::NotFound` 视为成功(crash 后重启场景)
    /// - 其他 IO 错误 → 向上传播
    pub(super) fn invalidate_atomic(&self) -> io::Result<()> {
        // Load 而非 swap:transient error(PermissionDenied / EBUSY 等)时**不** commit
        // invalidated,让调用方或 Drop 能 retry。只在 remove 成功(或 NotFound 视为等价)
        // 时才 store true,保证 "commit flag 与磁盘状态一致" 的 invariant —— flag=true
        // ↔ 文件确实已不存在。
        if self.invalidated.load(Ordering::SeqCst) {
            return Ok(());
        }
        match fs::remove_file(&self.path) {
            Ok(()) => {
                self.invalidated.store(true, Ordering::SeqCst);
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.invalidated.store(true, Ordering::SeqCst);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl Drop for EndpointFileGuard {
    fn drop(&mut self) {
        // 例外: Drop 里无法向外传播错误,只能 swallow。shutdown 正常路径会先手动
        // invalidate_atomic(error 沿 Result 传播);异常路径(panic / crash)依赖本
        // Drop 做最后一次清理,删除失败没有接收方可以汇报。
        let _ = self.invalidate_atomic();
    }
}

/// Atomic write `cli-endpoint.toml`:
///
/// 写到同目录下的 `.toml.tmp` 文件 → `rename` 到目标路径,保证读取方要么看到完整
/// 文件要么看不到文件,不会读到半个 TOML。
#[allow(dead_code)] // Phase 4 wiring
pub(super) fn write_atomic(path: &Path, contents: &EndpointFileContents) -> io::Result<()> {
    let tmp = path.with_extension("toml.tmp");
    let rendered = toml::to_string(contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&tmp, rendered)?;
    // `fs::rename` 在当前 Rust std(1.28+)两平台都 replace existing file(Unix 用 rename()
    // atomic replace;Windows 走 MoveFileExW / FileRenameInfoEx,file→file 替换 OK)。
    // 见 std::fs::rename doc:"Renames a file or directory to a new name, replacing the
    // original file if `to` already exists."
    fs::rename(tmp, path)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario: `invalidate_atomic` 是幂等操作
    /// (spec.md / design-v4 D2-a)
    #[test]
    fn test_endpoint_file_guard_invalidate_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cli-endpoint.toml");
        fs::write(&path, "dummy").unwrap();

        let guard = EndpointFileGuard::new(path.clone());
        assert!(path.exists(), "fixture 文件应就位");

        // 首次 invalidate — 删除文件
        guard.invalidate_atomic().unwrap();
        assert!(!path.exists(), "首次 invalidate 后文件应被删除");

        // 再次 invalidate — 幂等 noop
        guard.invalidate_atomic().unwrap();
        assert!(!path.exists(), "重复 invalidate 不应错误");
    }

    /// Scenario: `EndpointFileGuard` 的 Drop 兜底清理 crash 场景
    /// (spec.md / design-v4 D2-a R8 契约的兜底保证)
    #[test]
    fn test_endpoint_file_guard_drop_cleans_on_crash_path() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cli-endpoint.toml");
        fs::write(&path, "dummy").unwrap();

        {
            let _guard = EndpointFileGuard::new(path.clone());
            assert!(path.exists());
            // guard 在块尾被 drop,模拟未手动调用 invalidate_atomic 的异常退出
        }

        assert!(
            !path.exists(),
            "Drop 应兜底清理未 invalidate 的 cli-endpoint.toml"
        );
    }
}
