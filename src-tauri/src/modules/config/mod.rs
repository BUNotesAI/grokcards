//! App 级配置模块 —— 持久化 `{app_data_dir}/config.json`
//!
//! 当前唯一配置项是 `vault_path`(KeySight 的 Obsidian vault 根目录)。
//! 未来其他 app 级配置(主题 / 默认语言 / 备份间隔等)都挂在 `AppConfig` 上。
//!
//! ## 窄接口
//!
//! - `commands` 模块:三个 Tauri commands(`get_vault_config` / `set_vault_path` / `exit_app`)。
//! - `models` 模块:`AppConfig`(内部持久化结构) + `VaultConfigResponse`(跨 IPC 类型)。
//!
//! ## 内部实现
//!
//! - `domain` 模块:文件读写 + `.obsidian/` 校验(纯函数 + unit tests);
//! - `errors` 模块:本地 `ConfigError` + `Into<AppError>` 转换。

pub mod commands;
pub mod models;

pub(super) mod domain;
pub(super) mod errors;

/// 启动路径读 config 的 thin wrapper —— 给 `lib.rs::resolve_startup_vault_path` 用。
///
/// 把 `ConfigError` 转成 String,`lib.rs` 只做 best-effort 读取(读不到就走 first-run)。
pub fn load_startup_config(
    app_data_dir: &std::path::Path,
) -> Result<models::AppConfig, String> {
    domain::load_config(app_data_dir).map_err(|e| e.to_string())
}

/// Vault 路径合法性校验的跨模块入口 —— 给 `keysight::bootstrap_runtime` 用。
///
/// 要求路径存在、是目录,且包含 `.obsidian/` 子目录(Obsidian vault 特征)。
/// 错误 flatten 为 String,让 bootstrap 路径的 `Result<(), String>` 能 `?` 直传。
pub fn ensure_vault_path_valid(vault_path: &std::path::Path) -> Result<(), String> {
    domain::validate_vault_path(vault_path).map_err(|e| e.to_string())
}
