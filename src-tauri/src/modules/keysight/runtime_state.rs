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
//! 未来 Phase 5b step 3 的 watcher handle / 其他 runtime-only 产物也可挂在这里。
//!
//! ## 访问模式
//!
//! - Tauri command:`state: State<'_, KeysightRuntimeState>`,body 用
//!   `state.core.db` / `state.core.vault_path` / `state.suppression.clone()`
//! - Helper fn(非 command):接 `&KeysightRuntimeState`

use std::sync::Arc;

use super::suppression::SelfWriteSuppression;
use keysight_core::state::KeysightState;

/// Tauri 运行时 keysight state —— 被 `app.manage(...)` 注入,
/// 通过 `State<'_, KeysightRuntimeState>` 在 `#[tauri::command]` 里取。
pub(crate) struct KeysightRuntimeState {
    /// 纯 domain state —— 跨 crate 共享(CLI 直连 SQLite 也用它)。
    pub core: KeysightState,
    /// Tauri 自写事件抑制器 —— 与 IPC server 的 ServerContext 共享同一 `Arc`。
    pub suppression: Arc<SelfWriteSuppression>,
}
