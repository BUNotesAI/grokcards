//! Self-write suppression(design-v4 D6-b)—— 双队列记录 Tauri 自身写 / 删,
//! watcher 收到 fs event 时通过 fingerprint 比对 decide 是否 skip。
//!
//! ## Phase 5 scope
//! 本模块公 API 仅被 `watcher.rs` + 测试引用;production wiring(`http_server.rs`
//! 写路径 + `commands.rs` 写命令完成后调 `record_write` / `record_delete`)在 Phase
//! 6+ 接入,所以 Phase 5 用 file-level `#![allow(dead_code)]` 对齐 Phase 4 的
//! "wiring TODO" 先例(参考 `endpoint_file.rs`)。

#![allow(dead_code)]
//!
//! ## 队列语义
//!
//! - **writes**:`(path, size, mtime_ns)` 指纹。Tauri 每次 `write_atomic` 成功后立即
//!   `record_write`。watcher 收到 `Create/Modify` 事件时 stat 路径得到当前 `(size,
//!   mtime_ns)`,逐一与队列比对;命中即 consume(remove entry)并 skip 事件。
//! - **deletes**:只记 `path`(文件已不存在,stat 不可用)。Tauri 每次
//!   `vault_fs.delete_file` 成功后立即 `record_delete`。watcher 收到 `Remove` 事件
//!   时按路径比对;命中即 consume + skip。
//! - **TTL**:记录后 2s 未被 consume 视为过期,下次 check 时先 drain;防止 fs event
//!   永远不来导致队列泄漏。
//!
//! ## 不变式
//!
//! - `check_and_consume_*` 永远先 drain 过期再匹配;命中必须同时 remove entry(避免
//!   再次命中导致误 suppress 真外部事件)
//! - 空队列 + 任何事件 → return `false`(未命中,交 watcher 走 sync_file / remove_file 路径)

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use notify::Event;

/// 指纹 TTL。2s 足以覆盖常见 fs event 延迟(inotify ~100ms / fsevents ~1s)。
const TTL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
struct WriteFingerprint {
    path: PathBuf,
    size: u64,
    mtime_ns: i128,
    recorded_at: Instant,
}

#[derive(Debug, Clone)]
struct DeleteFingerprint {
    path: PathBuf,
    recorded_at: Instant,
}

/// 双队列 self-write suppression(design-v4 D6-b)。
///
/// 所有方法 `&self`(内部 Mutex),允许多 handler 并发调用。
pub(super) struct SelfWriteSuppression {
    writes: Mutex<VecDeque<WriteFingerprint>>,
    deletes: Mutex<VecDeque<DeleteFingerprint>>,
}

impl SelfWriteSuppression {
    pub(super) fn new() -> Self {
        Self {
            writes: Mutex::new(VecDeque::new()),
            deletes: Mutex::new(VecDeque::new()),
        }
    }

    /// 记录一次 Tauri 自身 `write_atomic` 成功。
    ///
    /// `path` 应是 write_atomic 的目标(不是 `.tmp`);`size` 与 `mtime_ns` 来自
    /// rename 成功后对 `path` 的 stat —— 必须与 watcher 将要看到的事件 fingerprint
    /// 完全一致才能命中。
    pub(super) fn record_write(&self, path: &Path, size: u64, mtime_ns: i128) {
        // 例外: Mutex poisoning 不可恢复,suppression 是 fire-and-forget best-effort,不影响正确性
        let mut writes = self.writes.lock().unwrap();
        writes.push_back(WriteFingerprint {
            path: path.to_path_buf(),
            size,
            mtime_ns,
            recorded_at: Instant::now(),
        });
    }

    /// 记录一次 Tauri 自身 `vault_fs.delete_file` 成功。
    pub(super) fn record_delete(&self, path: &Path) {
        // 例外: Mutex poisoning 不可恢复,suppression 是 fire-and-forget best-effort
        let mut deletes = self.deletes.lock().unwrap();
        deletes.push_back(DeleteFingerprint {
            path: path.to_path_buf(),
            recorded_at: Instant::now(),
        });
    }

    /// 检查并(若命中)consume `writes` 队列中与 `event` 匹配的指纹。
    ///
    /// 返回 `true` ↔ 该 event 是 Tauri 自写,watcher 应 skip。
    ///
    /// 实现:对 `event.paths.first()` 做 `fs::metadata` 得到当前 `(size, mtime_ns)`;
    /// drain 过期后在队列中线性查找同 `(path, size, mtime_ns)`,找到即 `remove` 并
    /// 返回 `true`。stat 失败(文件已被改名 / 删除)视为 cache miss 返回 `false`。
    pub(super) fn check_and_consume_write(&self, event: &Event) -> bool {
        let Some(event_path) = event.paths.first() else {
            return false;
        };
        let Some((size, mtime_ns)) = stat_fingerprint(event_path) else {
            return false; // stat 失败 → 当外部事件走 sync_file
        };
        // 例外: Mutex poisoning 不可恢复,best-effort
        let mut writes = self.writes.lock().unwrap();
        drain_expired_writes(&mut writes, Instant::now());
        if let Some(pos) = writes
            .iter()
            .position(|fp| fp.path == *event_path && fp.size == size && fp.mtime_ns == mtime_ns)
        {
            writes.remove(pos);
            return true;
        }
        false
    }

    /// 检查并(若命中)consume `deletes` 队列中与 `event` 匹配的指纹。
    ///
    /// 返回 `true` ↔ 该 event 是 Tauri 自删,watcher 应 skip。
    ///
    /// 实现:文件已被删,没法 stat,只按 `path` 匹配。
    pub(super) fn check_and_consume_delete(&self, event: &Event) -> bool {
        let Some(event_path) = event.paths.first() else {
            return false;
        };
        // 例外: Mutex poisoning 不可恢复,best-effort
        let mut deletes = self.deletes.lock().unwrap();
        drain_expired_deletes(&mut deletes, Instant::now());
        if let Some(pos) = deletes.iter().position(|fp| fp.path == *event_path) {
            deletes.remove(pos);
            return true;
        }
        false
    }
}

/// 从队列 front 丢弃所有过期(`recorded_at + TTL < now`)的 write 指纹。
///
/// 队列 FIFO 且 `recorded_at` 单调递增,从 front 停在第一个未过期项即可。
fn drain_expired_writes(queue: &mut VecDeque<WriteFingerprint>, now: Instant) {
    while let Some(front) = queue.front() {
        if now.duration_since(front.recorded_at) > TTL {
            queue.pop_front();
        } else {
            break;
        }
    }
}

/// 同 [`drain_expired_writes`],针对 delete 队列。
fn drain_expired_deletes(queue: &mut VecDeque<DeleteFingerprint>, now: Instant) {
    while let Some(front) = queue.front() {
        if now.duration_since(front.recorded_at) > TTL {
            queue.pop_front();
        } else {
            break;
        }
    }
}

/// Stat `path` 以取 `(size, mtime_ns)`。失败(NotFound 等)返回 `None`。
fn stat_fingerprint(path: &Path) -> Option<(u64, i128)> {
    let meta = fs::metadata(path).ok()?;
    let size = meta.len();
    let mtime_ns = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos() as i128;
    Some((size, mtime_ns))
}
