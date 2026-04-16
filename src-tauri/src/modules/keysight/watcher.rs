//! File watcher 事件分派(design-v4 D6-a)—— 三路分派 + self-write suppression 协同。
//!
//! ## 路由表
//!
//! | 事件 | 路由 | debounce |
//! |---|---|---|
//! | `need_rescan()` 或 `EventKind::Any` / `Other` | [`WatcherHandlers::sync_vault`](全量) | 1s |
//! | `EventKind::Create(_)` / `Modify(_)` | [`SelfWriteSuppression::check_and_consume_write`] 命中 skip;否则 [`WatcherHandlers::sync_file`] | 300ms |
//! | `EventKind::Remove(_)` | [`check_and_consume_delete`](SelfWriteSuppression::check_and_consume_delete) 命中 skip;否则 [`WatcherHandlers::remove_file`] | 300ms |
//! | `EventKind::Access(_)` 等其他 | 忽略 | — |
//!
//! ## Phase 5 scope
//!
//! 实现 `handle_event` 纯分派函数 + `WatcherHandlers` trait 契约。实际 notify watcher
//! 进程 spawn / tokio channel wiring 在 Phase 6+ 接入(lib.rs setup hook),因此 Phase
//! 5 用 file-level `#![allow(dead_code)]` 对齐 Phase 4 的 "wiring TODO" 先例。

#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use notify::{Event, EventKind};

use super::suppression::SelfWriteSuppression;

/// 默认 debounce(production):单文件 300ms,rescan 1s。
pub(super) const DEFAULT_DEBOUNCE_FILE: Duration = Duration::from_millis(300);
pub(super) const DEFAULT_DEBOUNCE_RESCAN: Duration = Duration::from_millis(1000);

/// 可配置 debounce —— production 用 `Default`;测试传 `WatcherDebounce::ZERO`
/// 避免 suite 挂 sleep。
#[derive(Debug, Clone, Copy)]
pub(super) struct WatcherDebounce {
    pub(super) file: Duration,
    pub(super) rescan: Duration,
}

impl Default for WatcherDebounce {
    fn default() -> Self {
        Self {
            file: DEFAULT_DEBOUNCE_FILE,
            rescan: DEFAULT_DEBOUNCE_RESCAN,
        }
    }
}

#[cfg(test)]
impl WatcherDebounce {
    const ZERO: Self = Self {
        file: Duration::ZERO,
        rescan: Duration::ZERO,
    };
}

/// Watcher handler 契约 —— `handle_event` 不关心具体实现,可注入。
///
/// Production 实现会 clone `Arc<AppHandle>` / `Arc<Connection>` 持有,调用 keysight-core
/// domain fn(sync_file / sync::remove_file / sync_vault)。测试注入 mock 记录 call。
pub(super) trait WatcherHandlers {
    fn sync_file(&self, path: &Path);
    fn remove_file(&self, path: &Path);
    fn sync_vault(&self);
}

/// 处理一个 fs event —— 三路分派,见文件头路由表。
///
/// 对每个 single-file 路径会先查 `suppression` 双队列;命中即 skip(返回 early),
/// 否则 await `debounce.file` 后调 handler。对 rescan / `Any` / `Other` await
/// `debounce.rescan` 后调 `sync_vault`。
pub(super) async fn handle_event<H: WatcherHandlers>(
    event: Event,
    suppression: &SelfWriteSuppression,
    handlers: &H,
    debounce: WatcherDebounce,
) {
    // 路由 1:rescan 或模糊事件 → sync_vault 全量兜底。
    // 注意:必须先 cover Any / Other / need_rescan,再 fall through 到 kind 分派;
    // 如果 Modify 事件带 Flag::Rescan,也要走 sync_vault,不能走 sync_file。
    if event.need_rescan() || matches!(event.kind, EventKind::Any | EventKind::Other) {
        tokio::time::sleep(debounce.rescan).await;
        handlers.sync_vault();
        return;
    }

    match event.kind {
        // 路由 2:单文件写 / 创建 —— Tauri 自写命中 skip,否则 debounce 后 sync_file
        EventKind::Create(_) | EventKind::Modify(_) => {
            if suppression.check_and_consume_write(&event) {
                return;
            }
            let Some(path) = event.paths.first().cloned() else {
                return; // 无路径的 Create/Modify 事件忽略
            };
            tokio::time::sleep(debounce.file).await;
            handlers.sync_file(&path);
        }
        // 路由 3:单文件删除 —— Tauri 自删命中 skip,否则 debounce 后 remove_file。
        // 硬约束(task rules):Remove 路由**不得** collapse 到 sync_file。
        EventKind::Remove(_) => {
            if suppression.check_and_consume_delete(&event) {
                return;
            }
            let Some(path) = event.paths.first().cloned() else {
                return;
            };
            tokio::time::sleep(debounce.file).await;
            handlers.remove_file(&path);
        }
        // Access / 其他非变更事件 —— 忽略。Any / Other 已在顶上 rescan 分支 cover。
        _ => {}
    }
}

// -----------------------------------------------------------------------------
// Tests —— Phase 5 scenarios(模块内 #[cfg(test)],保 pub(super) 窄接口)
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, Flag, ModifyKind, RemoveKind};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex as StdMutex;

    /// Mock `WatcherHandlers` —— 记录每次调用的 label + 路径,测试 snapshot 断言。
    #[derive(Default)]
    struct MockHandlers {
        calls: StdMutex<Vec<String>>,
    }

    impl MockHandlers {
        fn snapshot(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl WatcherHandlers for MockHandlers {
        fn sync_file(&self, path: &Path) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("sync_file:{}", path.display()));
        }
        fn remove_file(&self, path: &Path) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("remove_file:{}", path.display()));
        }
        fn sync_vault(&self) {
            self.calls.lock().unwrap().push("sync_vault".to_string());
        }
    }

    /// 从 fixture 文件取 `(size, mtime_ns)`,用于构造 `record_write` 参数与事件
    /// 指纹必须匹配(check_and_consume_write 会 stat 同路径得到相同值)。
    fn file_fingerprint(path: &Path) -> (u64, i128) {
        let meta = fs::metadata(path).unwrap();
        let size = meta.len();
        let mtime_ns = meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i128;
        (size, mtime_ns)
    }

    /// Scenario: Watcher 收到外部 Modify 事件(writes 队列为空)时触发 `sync_file`
    #[tokio::test]
    async fn test_watcher_modify_event_external_triggers_sync_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("foo.md");
        fs::write(&path, "external edit").unwrap();

        let suppression = SelfWriteSuppression::new();
        let handlers = MockHandlers::default();

        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.clone());

        handle_event(event, &suppression, &handlers, WatcherDebounce::ZERO).await;

        assert_eq!(
            handlers.snapshot(),
            vec![format!("sync_file:{}", path.display())]
        );
    }

    /// Scenario: Watcher 收到外部 Remove 事件(deletes 队列为空)时触发 `remove_file`,**不**触发 `sync_file`
    #[tokio::test]
    async fn test_watcher_remove_event_external_triggers_remove_file() {
        let tmp = tempfile::tempdir().unwrap();
        // Remove event 下文件应该已不存在 —— 不 create
        let path = tmp.path().join("foo.md");

        let suppression = SelfWriteSuppression::new();
        let handlers = MockHandlers::default();

        let event = Event::new(EventKind::Remove(RemoveKind::Any)).add_path(path.clone());

        handle_event(event, &suppression, &handlers, WatcherDebounce::ZERO).await;

        assert_eq!(
            handlers.snapshot(),
            vec![format!("remove_file:{}", path.display())],
            "Remove 路由必须 trigger remove_file 且**不得** collapse 到 sync_file"
        );
    }

    /// Scenario: Watcher 的 Remove 分支在 suppression 命中时 skip
    #[tokio::test]
    async fn test_watcher_remove_event_suppressed_after_record_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("foo.md");

        let suppression = SelfWriteSuppression::new();
        suppression.record_delete(&path);

        let handlers = MockHandlers::default();
        let event = Event::new(EventKind::Remove(RemoveKind::Any)).add_path(path);

        handle_event(event, &suppression, &handlers, WatcherDebounce::ZERO).await;

        assert!(
            handlers.snapshot().is_empty(),
            "suppressed remove 不应触发任何 handler"
        );
    }

    /// Scenario: Watcher 的 Modify 分支在 write suppression 命中时 skip
    #[tokio::test]
    async fn test_watcher_modify_event_suppressed_after_record_write() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("foo.md");
        fs::write(&path, "self-written").unwrap();

        let (size, mtime_ns) = file_fingerprint(&path);

        let suppression = SelfWriteSuppression::new();
        suppression.record_write(&path, size, mtime_ns);

        let handlers = MockHandlers::default();
        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path);

        handle_event(event, &suppression, &handlers, WatcherDebounce::ZERO).await;

        assert!(
            handlers.snapshot().is_empty(),
            "suppressed modify 不应触发 sync_file"
        );
    }

    /// Scenario: Watcher 收到 rescan / 模糊事件时 fallback 到 `sync_vault`,不走单文件路径
    #[tokio::test]
    async fn test_watcher_rescan_triggers_sync_vault() {
        let suppression = SelfWriteSuppression::new();

        // Case 1: EventKind::Any(模糊事件)
        let handlers_a = MockHandlers::default();
        let event_a = Event::new(EventKind::Any);
        handle_event(event_a, &suppression, &handlers_a, WatcherDebounce::ZERO).await;
        assert_eq!(
            handlers_a.snapshot(),
            vec!["sync_vault"],
            "EventKind::Any 应 fallback 到 sync_vault"
        );

        // Case 2: Modify + Flag::Rescan —— event.need_rescan() == true
        let handlers_b = MockHandlers::default();
        let event_b = Event::new(EventKind::Modify(ModifyKind::Any))
            .add_path(PathBuf::from("/vault/foo.md"))
            .set_flag(Flag::Rescan);
        handle_event(event_b, &suppression, &handlers_b, WatcherDebounce::ZERO).await;
        assert_eq!(
            handlers_b.snapshot(),
            vec!["sync_vault"],
            "need_rescan() == true 即使 kind 是 Modify 也应走 sync_vault"
        );
    }

    /// Scenario: Watcher 收到外部 Create 事件(writes 队列为空)时触发 `sync_file`
    #[tokio::test]
    async fn test_watcher_create_event_external_triggers_sync_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("foo.md");
        fs::write(&path, "new external file").unwrap();

        let suppression = SelfWriteSuppression::new();
        let handlers = MockHandlers::default();

        let event = Event::new(EventKind::Create(CreateKind::File)).add_path(path.clone());

        handle_event(event, &suppression, &handlers, WatcherDebounce::ZERO).await;

        assert_eq!(
            handlers.snapshot(),
            vec![format!("sync_file:{}", path.display())]
        );
    }

    /// Scenario: Watcher 的 Create 分支在 write suppression 命中时 skip(底层 FS 将自写 event 报为 Create 的场景)
    #[tokio::test]
    async fn test_watcher_create_event_suppressed_after_record_write() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("foo.md");
        fs::write(&path, "write surfaced as Create").unwrap();

        let (size, mtime_ns) = file_fingerprint(&path);

        let suppression = SelfWriteSuppression::new();
        suppression.record_write(&path, size, mtime_ns);

        let handlers = MockHandlers::default();
        let event = Event::new(EventKind::Create(CreateKind::File)).add_path(path);

        handle_event(event, &suppression, &handlers, WatcherDebounce::ZERO).await;

        assert!(
            handlers.snapshot().is_empty(),
            "suppressed create 不应触发 sync_file"
        );
    }
}
