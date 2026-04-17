//! 记录 self-write fingerprint 的 `VaultFs` 装饰器(design-v4 D6-b)。
//!
//! 每次 write/delete 成功后登记到 [`SelfWriteSuppression`] 队列,watcher 后续
//! 通过 `check_and_consume_*` 识别自写 event 并 skip,避免 Tauri 自己触发 sync。
//!
//! ## Phase 5b scope
//!
//! Red 阶段只落地 skeleton —— struct 定义、pass-through impl、3 个 scenario 测试;
//! `write_file` / `delete_file` 故意**不**调用 suppression,让 test 1/2 Red。Green
//! 阶段再补 record 调用,让 3 条测试全绿。
//!
use std::path::PathBuf;
use std::sync::Arc;

use keysight_core::errors::KeysightError;
use keysight_core::vault_fs::{RealVaultFs, VaultFs};

use super::suppression::SelfWriteSuppression;

/// 在真实 `VaultFs` 上叠加 self-write 记录。
///
/// 泛型 `V` 便于测试注入不同 inner;prod 统一用 [`Self::wrap_real`] 便利构造。
pub(super) struct RecordingVaultFs<V: VaultFs = RealVaultFs> {
    inner: V,
    vault_root: PathBuf,
    suppression: Arc<SelfWriteSuppression>,
}

impl<V: VaultFs> RecordingVaultFs<V> {
    /// 通用构造器 —— 测试可注入自定义 inner。
    pub(super) fn new(
        inner: V,
        vault_root: PathBuf,
        suppression: Arc<SelfWriteSuppression>,
    ) -> Self {
        Self {
            inner,
            vault_root,
            suppression,
        }
    }

    /// 把相对路径解析为 vault 下的绝对路径 —— 与 watcher 事件路径对齐。
    fn abs_path(&self, relative: &str) -> PathBuf {
        self.vault_root.join(relative)
    }
}

impl RecordingVaultFs<RealVaultFs> {
    /// prod 便利构造器 —— `vault_path` 同时构 inner `RealVaultFs` 和 `vault_root`。
    pub(super) fn wrap_real(
        vault_path: String,
        suppression: Arc<SelfWriteSuppression>,
    ) -> Self {
        let vault_root = PathBuf::from(&vault_path);
        let inner = RealVaultFs::new(vault_path);
        Self::new(inner, vault_root, suppression)
    }
}

impl<V: VaultFs> VaultFs for RecordingVaultFs<V> {
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError> {
        self.inner.read_file(relative_path)
    }

    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError> {
        self.inner.write_file(relative_path, content)?;
        // 成功写入后立即登记 fingerprint —— watcher 收到此 path 的 Create/Modify event 时
        // 调 check_and_consume_write 命中即 skip(不二次触发 sync_file)。
        // stat 失败(极端:别的进程瞬删)时 no-op;watcher 若收到事件会走 sync_file,DB 幂等无害。
        self.suppression
            .record_write_from_stat(&self.abs_path(relative_path));
        Ok(())
    }

    fn delete_file(&self, relative_path: &str) -> Result<(), KeysightError> {
        self.inner.delete_file(relative_path)?;
        // 成功删除后登记 path —— watcher 收到此 path 的 Remove event 时
        // 调 check_and_consume_delete 命中即 skip(不二次触发 remove_file)。
        self.suppression.record_delete(&self.abs_path(relative_path));
        Ok(())
    }

    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError> {
        self.inner.list_md_files(subdir)
    }

    fn list_first_level_dirs(&self, subdir: &str) -> Result<Vec<String>, KeysightError> {
        self.inner.list_first_level_dirs(subdir)
    }

    // list_project_whiteboards 用 VaultFs 默认 impl(委托给 list_first_level_dirs),不重写
}

// -----------------------------------------------------------------------------
// Tests —— Phase 5b scenarios(模块内 #[cfg(test)],保 pub(super) 窄接口)
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    use notify::{Event, EventKind};

    /// Scenario 1:wrapper `write_file` 成功后 suppression.writes 应记录 fingerprint,
    /// 同 path + 同 (size, mtime_ns) 的 Modify event 应被命中 → watcher 会 skip。
    #[test]
    fn test_record_write_fingerprint_after_write_file() {
        let tmp = tempfile::tempdir().unwrap();
        let suppression = Arc::new(SelfWriteSuppression::new());
        let fs = RecordingVaultFs::wrap_real(
            tmp.path().to_string_lossy().to_string(),
            Arc::clone(&suppression),
        );

        fs.write_file("cards/x.md", "# hi\n").unwrap();

        let abs = tmp.path().join("cards/x.md");
        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(abs);

        assert!(
            suppression.check_and_consume_write(&event),
            "write_file 成功后 suppression 应记录 (path, size, mtime_ns) 使相同事件命中"
        );
    }

    /// Scenario 2:wrapper `delete_file` 成功后 suppression.deletes 应记录 path,
    /// 同 path 的 Remove event 应被命中 → watcher 会 skip。
    /// 先用 wrapper 写再用 wrapper 删,锁 decorator 契约而非手工落盘。
    #[test]
    fn test_record_delete_fingerprint_after_delete_file() {
        let tmp = tempfile::tempdir().unwrap();
        let suppression = Arc::new(SelfWriteSuppression::new());
        let fs = RecordingVaultFs::wrap_real(
            tmp.path().to_string_lossy().to_string(),
            Arc::clone(&suppression),
        );

        // 先通过 wrapper 写入 —— 会往 writes 队列记录(不影响本 test 对 deletes 的断言)
        fs.write_file("cards/x.md", "seed").unwrap();
        // 再通过 wrapper 删除 —— decorator 契约要求 record_delete
        fs.delete_file("cards/x.md").unwrap();

        let abs = tmp.path().join("cards/x.md");
        let event = Event::new(EventKind::Remove(RemoveKind::File)).add_path(abs);

        assert!(
            suppression.check_and_consume_delete(&event),
            "delete_file 成功后 suppression 应记录 path 使相同 Remove 事件命中"
        );
    }

    /// Scenario 3:read/list 不触任何 suppression 副作用 ——
    /// write 队列 + delete 队列都保持为空(双断言)。
    #[test]
    fn test_read_and_list_are_pass_through_with_no_suppression_effect() {
        let tmp = tempfile::tempdir().unwrap();
        // 手工落盘 fixture —— 绕过 wrapper 避免 write_file 污染待测队列
        std::fs::create_dir_all(tmp.path().join("cards")).unwrap();
        std::fs::write(tmp.path().join("cards/x.md"), "# content\n").unwrap();

        let suppression = Arc::new(SelfWriteSuppression::new());
        let fs = RecordingVaultFs::wrap_real(
            tmp.path().to_string_lossy().to_string(),
            Arc::clone(&suppression),
        );

        // 读 + 列表 纯查询,应不触 suppression
        let content = fs.read_file("cards/x.md").unwrap();
        assert_eq!(content, "# content\n");
        let files = fs.list_md_files("cards").unwrap();
        assert_eq!(files.len(), 1);

        // 探测:两队列应双双 miss
        let abs = tmp.path().join("cards/x.md");
        let write_event = Event::new(EventKind::Create(CreateKind::File)).add_path(abs.clone());
        let delete_event = Event::new(EventKind::Remove(RemoveKind::File)).add_path(abs);

        assert!(
            !suppression.check_and_consume_write(&write_event),
            "read/list 后 writes 队列应仍为空 —— write event 不该命中"
        );
        assert!(
            !suppression.check_and_consume_delete(&delete_event),
            "read/list 后 deletes 队列应仍为空 —— delete event 不该命中"
        );
    }
}
