//! 性能日志 + Mutex 锁竞争监控辅助。
//!
//! 三个工具：
//! - `ScopedTimer` — RAII 计时，drop 时 eprintln 总耗时
//! - `lock_db` — 包装 `state.db.lock()` 测量等锁时间，> 50ms 时打 warn
//! - `lock_db_warn_long` — 一段闭包包住数据库操作，监控总耗时

use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

pub struct ScopedTimer {
    name: &'static str,
    start: Instant,
}

impl ScopedTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let ms = self.start.elapsed().as_secs_f64() * 1000.0;
        if ms >= 100.0 {
            eprintln!("[perf rust] 🐢 SLOW {} = {:.1}ms", self.name, ms);
        } else {
            eprintln!("[perf rust] {} = {:.1}ms", self.name, ms);
        }
    }
}

/// 获取 Mutex 锁的同时记录等锁耗时，> 50ms 打 warn。
/// 给所有 keysight commands 用，让锁竞争（多 cmd 抢同一个 conn）变可见。
pub fn lock_db<'a, T>(mutex: &'a Mutex<T>, name: &'static str) -> MutexGuard<'a, T> {
    let start = Instant::now();
    let guard = mutex.lock().unwrap_or_else(|e| {
        eprintln!("[perf rust] ❌ POISONED LOCK {}: {e}", name);
        e.into_inner()
    });
    let wait_ms = start.elapsed().as_secs_f64() * 1000.0;
    if wait_ms >= 50.0 {
        eprintln!(
            "[perf rust] ⚠️ LOCK WAIT {} = {:.1}ms (Mutex 竞争)",
            name, wait_ms
        );
    }
    guard
}
