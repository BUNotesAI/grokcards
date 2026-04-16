//! CLI config 文件加载 —— 三级优先级。

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::errors::CliError;

pub const CONFIG_FILENAME: &str = "cli-config.toml";
pub const ENV_VAR: &str = "KEYSIGHT_CLI_CONFIG";

#[derive(Debug, Deserialize)]
pub struct CliConfig {
    pub cli_config_version: u32,
    pub db_path: PathBuf,
    pub vault_path: PathBuf,
}

/// Phase 6.1 — 三级优先级 config 文件发现:
///
/// 1. `--config-file` flag(最高优先级)
/// 2. `KEYSIGHT_CLI_CONFIG` 环境变量
/// 3. `{app_data_override 或 app_data_dir}/cli-config.toml`(最低优先级)
///
/// `app_data_override` 是测试专用入口(avoid 读 production 的 app_data_dir);production 传 `None`。
pub fn load_config(
    flag: Option<&Path>,
    app_data_override: Option<&Path>,
) -> Result<CliConfig, CliError> {
    let config_path = if let Some(f) = flag {
        f.to_path_buf()
    } else if let Ok(env_path) = std::env::var(ENV_VAR) {
        PathBuf::from(env_path)
    } else {
        let base = resolve_app_data_dir(app_data_override)?;
        base.join(CONFIG_FILENAME)
    };

    let content = std::fs::read_to_string(&config_path).map_err(|e| {
        CliError::Config(format!("read {}: {}", config_path.display(), e))
    })?;
    toml::from_str(&content)
        .map_err(|e| CliError::Config(format!("parse {}: {}", config_path.display(), e)))
}

/// Tauri app bundle identifier(来自 `src-tauri/tauri.conf.json` `identifier` 字段)。
/// **必须与 Tauri 保持一致**—— server 用 `app.path().app_data_dir()` 发布 `cli-config.toml`,
/// macOS 解析为 `~/Library/Application Support/{identifier}`,CLI 默认路径必须撞上。
///
/// Phase 6.2a 提升到 `pub(crate)` —— `endpoint.rs` 复用,避免两处硬编码漂移。
pub(crate) const APP_IDENTIFIER: &str = "co.bunotes.super-tauri";

/// Phase 6.2a 提升到 `pub(crate)` —— `endpoint.rs::load_endpoint` 复用同一解析逻辑。
pub(crate) fn resolve_app_data_dir(override_path: Option<&Path>) -> Result<PathBuf, CliError> {
    if let Some(p) = override_path {
        return Ok(p.to_path_buf());
    }
    // macOS production:~/Library/Application Support/{identifier}
    // (Linux/Windows 后续扩展;spec 测试走 override 路径,production 目前仅 macOS)
    let home = std::env::var("HOME")
        .map_err(|_| CliError::Config("cannot resolve app_data_dir (HOME unset)".to_string()))?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support")
        .join(APP_IDENTIFIER))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "keysight_cli_config_test_{}_{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_cfg(path: &Path, db: &str, vault: &str) {
        fs::write(
            path,
            format!(
                "cli_config_version = 1\ndb_path = \"{}\"\nvault_path = \"{}\"\n",
                db, vault
            ),
        )
        .unwrap();
    }

    /// Phase 6.1 spec scenario(spec.md L280):
    /// 三份 cli-config.toml 同时存在于 Pf/Pe/Pd,KEYSIGHT_CLI_CONFIG=Pe,flag=--config-file Pf。
    /// 期望:CLI 用 Pf 读 config,db_path = Pf 中声明值。
    #[test]
    fn test_config_file_flag_takes_precedence_over_env_and_default() {
        let flag_dir = temp_root("flag");
        let env_dir = temp_root("env");
        let default_dir = temp_root("default");

        let flag_cfg = flag_dir.join(CONFIG_FILENAME);
        let env_cfg = env_dir.join(CONFIG_FILENAME);
        let default_cfg = default_dir.join(CONFIG_FILENAME);

        write_cfg(&flag_cfg, "/path/from/flag", "/vault/flag");
        write_cfg(&env_cfg, "/path/from/env", "/vault/env");
        write_cfg(&default_cfg, "/path/from/default", "/vault/default");

        // Rust 2024 `set_var` 需 unsafe。此测单线程下运行不会竞争
        // (cargo test 默认 parallel,但各测试用独立 env prefix;Green 实施时若出现
        // 测试交叉可能需 serial_test,目前 Red 阶段 todo!() panic 发生在 env 访问之前)
        unsafe {
            std::env::set_var(ENV_VAR, &env_cfg);
        }

        let config = load_config(Some(&flag_cfg), Some(&default_dir)).unwrap();

        unsafe {
            std::env::remove_var(ENV_VAR);
        }

        assert_eq!(
            config.db_path,
            PathBuf::from("/path/from/flag"),
            "flag should win over env and default"
        );
    }
}
