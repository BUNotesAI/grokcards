//! Production `WatcherHandlers` impl + notify watcher loop spawn(Phase 5b wiring)。
//!
//! ## 职责
//!
//! - 接 `notify::RecommendedWatcher` 监 vault 递归,把 event 从 sync callback thread
//!   forward 到 tokio async channel
//! - 在 async loop 里调 `watcher::handle_event`(走 suppression 判自写 + 三路分派 +
//!   debounce),命中三路后调下面的 production handler
//! - Production handler:
//!   - `sync_file`:读文件 → `domain::sync::sync_file` → emit `entity:changed`
//!   - `remove_file`:`domain::sync::remove_file` → emit `entity:changed`
//!   - `sync_vault`:`domain::sync::sync_vault(&conn, &RecordingVaultFs)` → emit `entity:changed`
//!
//! ## 生命周期
//!
//! `spawn_watcher_loop` 内部 `tauri::async_runtime::spawn` 的 task 持有 watcher,
//! task 运行期间 watcher 持续推 event。app 退出 → runtime shutdown → task cancel →
//! watcher drop → notify 停监听。无需额外 manage。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use keysight_core::domain;
use notify::{RecursiveMode, Watcher};
use rusqlite::Connection;
use tauri::{AppHandle, Emitter};

use super::recording_vault_fs::RecordingVaultFs;
use super::suppression::SelfWriteSuppression;
use super::watcher::{handle_event, WatcherDebounce, WatcherHandlers};

/// Production `WatcherHandlers` —— 调真 `domain::sync` + emit `entity:changed` event。
///
/// 与 Tauri command 侧共享同一 `Arc<Mutex<Connection>>`(通过 `KeysightRuntimeState.core.db`
/// 的 inner 给我们传),watcher 路径和 command 路径写同一 SQLite handle(短事务串行)。
struct ProdHandlers {
    db: Arc<Mutex<Connection>>,
    vault_path: PathBuf,
    suppression: Arc<SelfWriteSuppression>,
    app: AppHandle,
}

impl ProdHandlers {
    /// 把绝对 path 转成 vault 相对 path string。
    fn relative(&self, path: &Path) -> Option<String> {
        path.strip_prefix(&self.vault_path)
            .ok()
            .map(|rel| rel.to_string_lossy().to_string())
    }

    /// Lock Connection;poisoned 视为可恢复(fire-and-forget watcher,best-effort)。
    fn lock_db(&self) -> std::sync::MutexGuard<'_, Connection> {
        // 例外: Mutex poisoning 不可恢复,watcher 是 fire-and-forget
        self.db.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 读 path 当前 mtime(epoch ms,f64),失败返 0.0 —— sync_file 签名要 f64。
    fn mtime_ms(path: &Path) -> f64 {
        match std::fs::metadata(path).and_then(|m| m.modified()) {
            Ok(t) => {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
                    * 1000.0
            }
            Err(_) => 0.0,
        }
    }
}

impl WatcherHandlers for ProdHandlers {
    fn sync_file(&self, path: &Path) {
        let Some(rel) = self.relative(path) else {
            eprintln!(
                "[keysight] watcher sync_file: path not under vault: {}",
                path.display()
            );
            return;
        };
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[keysight] watcher sync_file: read failed: {} {e}",
                    path.display()
                );
                return;
            }
        };
        let mtime = Self::mtime_ms(path);
        let conn = self.lock_db();
        match domain::sync::sync_file(&conn, &rel, &content, mtime) {
            Ok(_) => {
                let _ = self.app.emit(
                    "entity:changed",
                    serde_json::json!({ "reason": "watcher-sync", "path": rel }),
                );
            }
            Err(e) => eprintln!("[keysight] watcher sync_file error: {rel}: {e}"),
        }
    }

    fn remove_file(&self, path: &Path) {
        let Some(rel) = self.relative(path) else {
            return;
        };
        let conn = self.lock_db();
        match domain::sync::remove_file(&conn, &rel) {
            Ok(()) => {
                let _ = self.app.emit(
                    "entity:changed",
                    serde_json::json!({ "reason": "watcher-remove", "path": rel }),
                );
            }
            Err(e) => eprintln!("[keysight] watcher remove_file error: {rel}: {e}"),
        }
    }

    fn sync_vault(&self) {
        let fs = RecordingVaultFs::wrap_real(
            self.vault_path.to_string_lossy().to_string(),
            Arc::clone(&self.suppression),
        );
        let conn = self.lock_db();
        match domain::sync::sync_vault(&conn, &fs) {
            Ok(_) => {
                let _ = self.app.emit(
                    "entity:changed",
                    serde_json::json!({ "reason": "watcher-rescan" }),
                );
            }
            Err(e) => eprintln!("[keysight] watcher sync_vault error: {e}"),
        }
    }
}

/// 启动 notify watcher loop —— 供 `lib.rs setup` 调用。
///
/// 内部 `tauri::async_runtime::spawn` 一个长跑 task:
/// 1. 构 unbounded mpsc channel + `notify::recommended_watcher` 用 closure forward event
/// 2. `watcher.watch(vault_path, Recursive)` 开启监听
/// 3. `while let Some(res) = rx.recv().await` 循环,每个 event 调 `handle_event`(走
///    suppression 双队列 + 三路分派 + debounce)
///
/// watcher 所有权被 spawned task 持有,task 存活期间 watcher 持续运作;app 退出 →
/// runtime shutdown → task cancel → watcher drop。
///
/// watcher 内部开独立 `Connection`(对齐 Phase 6.2b `http_server::start` 做法):
/// SQLite WAL 支持同进程多 Connection 写串行,避免把 `KeysightState.db` 拆成 `Arc<Mutex<_>>`
/// 导致 20+ call sites churn。短事务(sync_file / remove_file)各自独立 commit,
/// Tauri command 侧的 `KeysightState.db` 拿到 WAL commit 后的快照。
///
/// 返回 `()` —— 不需要 caller 管理 watcher 生命周期。
pub(crate) fn spawn_watcher_loop(
    app: AppHandle,
    vault_path: PathBuf,
    db_path: PathBuf,
    suppression: Arc<SelfWriteSuppression>,
) {
    let db = match Connection::open(&db_path) {
        Ok(conn) => Arc::new(Mutex::new(conn)),
        Err(e) => {
            eprintln!(
                "[keysight] watcher_prod: open Connection failed for {}: {e}",
                db_path.display()
            );
            return;
        }
    };

    let handlers = Arc::new(ProdHandlers {
        db,
        vault_path: vault_path.clone(),
        suppression: Arc::clone(&suppression),
        app,
    });

    tauri::async_runtime::spawn(async move {
        // channel:notify callback thread → async consumer
        let (tx, mut rx) =
            tokio::sync::mpsc::unbounded_channel::<notify::Result<notify::Event>>();

        // 初始化 watcher;失败只 log 不上报(watcher 失败 ≠ app 启动失败)
        let mut watcher = match notify::recommended_watcher(
            move |res: notify::Result<notify::Event>| {
                // closure 是 FnMut + Send + 'static;失败(receiver dropped)说明
                // async task 已退出,无所谓
                let _ = tx.send(res);
            },
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[keysight] watcher init failed: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&vault_path, RecursiveMode::Recursive) {
            eprintln!(
                "[keysight] watcher.watch failed for {}: {e}",
                vault_path.display()
            );
            return;
        }

        eprintln!("[keysight] watcher monitoring {}", vault_path.display());

        // 循环消费 event —— handle_event 内部走 suppression + 三路分派 + debounce
        while let Some(res) = rx.recv().await {
            let event = match res {
                Ok(ev) => ev,
                Err(e) => {
                    eprintln!("[keysight] watcher event error: {e}");
                    continue;
                }
            };
            handle_event(
                event,
                suppression.as_ref(),
                handlers.as_ref(),
                WatcherDebounce::default(),
            )
            .await;
        }
        // rx.recv 返 None → 所有 sender drop → watcher 已 drop → 退出 task
    });
}
