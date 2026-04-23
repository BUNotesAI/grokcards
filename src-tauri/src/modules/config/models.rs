//! App 级配置 schema —— 持久化在 `app_data_dir/config.json`
//!
//! 当前唯一的配置项是 `vault_path`(KeySight 的 Obsidian vault 根目录)。
//! 未来可扩展(主题、默认语言、自动备份间隔等)而不破坏 schema 兼容。

use serde::{Deserialize, Serialize};

/// 持久化到磁盘的应用配置。
///
/// `vault_path = None` 表示首次启动 / 未配置状态。Rust 不会 panic,
/// keysight commands 会返 `AppError::VaultNotConfigured`,前端据此弹 setup dialog。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Obsidian vault 根目录绝对路径(校验过 `.obsidian/` 子目录存在)。
    #[serde(default)]
    pub vault_path: Option<String>,
}

/// `get_vault_config` command 的返回值 —— 暴露给 TS 侧的运行时状态视图。
///
/// **Ready 的语义是 runtime availability,不是 config 持久化**:
/// - 只有 `ready: true` 时前端才渲染主 UI;
/// - 持久化 config.json 存在但 bootstrap 失败 → `ready: false`、`vaultPath: None`,
///   前端正确地重新弹 VaultSetup;
/// - `vault_path` 反映 **effective vault**(不管来自 debug env override 还是 config.json,
///   最终都经 bootstrap 写入 runtime inner —— 统一信号源)。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VaultConfigResponse {
    /// 当前 runtime installed 的 vault 路径;`ready: false` 时永远为 `None`。
    pub vault_path: Option<String>,
    /// keysight runtime 是否已成功 install —— 前端启动 gate 的真源。
    pub ready: bool,
}
