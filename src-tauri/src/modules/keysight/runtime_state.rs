//! Tauri 运行时 state wrapper —— 把 keysight-core 的纯 `KeysightState`
//! 和 src-tauri 独有的 `Arc<SelfWriteSuppression>` 捏在一起,作为 Tauri
//! managed state 注入。
//!
//! ## 为什么不直接把 suppression 加到 `KeysightState`
//!
//! `KeysightState` 在 `keysight-core`(纯 domain,给 CLI 与 Tauri 共享)。
//! `SelfWriteSuppression` 是 Tauri-specific 概念(自写事件抑制,CLI 不需要)。
//! 加字段会让 keysight-core 反向依赖 suppression,违反 deep module 边界。
//!
//! 方案 B1(本文件):在 src-tauri 本地加 wrapper,运行时依赖收进来,keysight-core 零感知。
//!
//! ## 首次启动 / 未配置状态
//!
//! vault 路径由用户在 VaultSetup 对话框里选定后才能初始化 keysight runtime。
//! 因此 `KeysightRuntimeState` 本身必须支持"未配置"态:外壳是
//! `RwLock<Option<Arc<KeysightRuntimeInner>>>`,`inner()` 未 install 时返
//! `AppError::VaultNotConfigured`。
//!
//! ## 访问模式
//!
//! - Tauri command:`state: State<'_, KeysightRuntimeState>`,
//!   body 第一行 `let state = state.resolved()?;` **shadow** 成
//!   `Arc<KeysightRuntimeInner>`;之后 `state.core.xxx` / `state.suppression`
//!   的访问(通过 Arc 的 Deref)原样不变;
//! - Helper fn(非 command):接 `&KeysightRuntimeInner`(已解开)。

use std::sync::{Arc, RwLock};

use super::suppression::SelfWriteSuppression;
use crate::app_error::AppError;
use keysight_core::state::KeysightState;

/// 已 install 的 keysight 运行时状态 —— core + suppression 捆绑。
///
/// 通过 `Arc<KeysightRuntimeInner>` 从 `KeysightRuntimeState::inner()` 返回,
/// commands 可以 shadow `state` 变量,保持 `state.core.xxx` / `state.suppression`
/// 的访问形态不变(Arc 透明 Deref)。
pub(crate) struct KeysightRuntimeInner {
    /// 纯 domain state —— 跨 crate 共享(CLI 直连 SQLite 也用它)。
    pub core: KeysightState,
    /// Tauri 自写事件抑制器 —— 与 IPC server 的 ServerContext 共享同一 `Arc`。
    pub suppression: Arc<SelfWriteSuppression>,
}

/// Tauri 运行时 keysight state —— `app.manage(...)` 注入,
/// 通过 `State<'_, KeysightRuntimeState>` 在 `#[tauri::command]` 里取。
pub(crate) struct KeysightRuntimeState {
    inner: RwLock<Option<Arc<KeysightRuntimeInner>>>,
}

impl KeysightRuntimeState {
    /// 未配置的 runtime state(首次启动前的占位)。
    pub fn empty() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    /// 安装已初始化的 keysight runtime —— 由 `keysight::bootstrap_runtime` 调用。
    ///
    /// 当前不保证幂等:重复 install 会覆盖已有 inner。调用方负责保证只在
    /// 未 install 时调用(VaultSetup 成功一次即 unmount)。
    pub fn install(&self, rt: KeysightRuntimeInner) {
        // 例外: RwLock poisoning 不可恢复
        let mut guard = self.inner.write().unwrap();
        *guard = Some(Arc::new(rt));
    }

    /// 解开已 install 的 inner —— 未配置时返 `VaultNotConfigured`。
    ///
    /// 每个 keysight command 第一行 `let state = state.resolved()?;` 就是调这个。
    /// 返回 `Arc` 保证 clone 后立即释放 RwLock guard,不 hold 到 command 结束。
    ///
    /// 命名上避免 `inner()` 以防与 Tauri `State::inner()` 方法冲突(Deref 链路
    /// 会先命中 `State` 上的同名方法,类型不对且返回非 Result)。
    pub fn resolved(&self) -> Result<Arc<KeysightRuntimeInner>, AppError> {
        // 例外: RwLock poisoning 不可恢复
        let guard = self.inner.read().unwrap();
        guard.clone().ok_or(AppError::VaultNotConfigured)
    }
}
