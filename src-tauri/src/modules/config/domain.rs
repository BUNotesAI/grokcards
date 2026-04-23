//! Config 模块的纯函数实现 —— 文件读写 + vault 路径校验
//!
//! 所有函数接收 plain 依赖(&Path),无 Tauri 耦合,可直接 unit test。

use std::path::{Path, PathBuf};

use super::errors::ConfigError;
use super::models::AppConfig;

const CONFIG_FILE_NAME: &str = "config.json";

/// 计算 config.json 的绝对路径(`{app_data_dir}/config.json`)。
pub(super) fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CONFIG_FILE_NAME)
}

/// 加载配置:
///
/// - 文件不存在 → 返回 `AppConfig::default()`(vault_path = None);
/// - 文件存在但解析失败 → 返回 `Parse` 错误(不静默吞掉);
/// - IO 错误 → 返回 `Read` 错误。
pub(super) fn load_config(app_data_dir: &Path) -> Result<AppConfig, ConfigError> {
    let path = config_path(app_data_dir);
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&content).map_err(|source| ConfigError::Parse { path, source })
}

/// 保存配置:确保目录存在 + 写入 pretty JSON(便于人工查看)。
pub(super) fn save_config(
    app_data_dir: &Path,
    config: &AppConfig,
) -> Result<(), ConfigError> {
    std::fs::create_dir_all(app_data_dir).map_err(|source| ConfigError::CreateDir {
        path: app_data_dir.to_path_buf(),
        source,
    })?;
    let path = config_path(app_data_dir);
    let content = serde_json::to_string_pretty(config).map_err(|source| ConfigError::Parse {
        path: path.clone(),
        source,
    })?;
    std::fs::write(&path, content).map_err(|source| ConfigError::Write { path, source })
}

/// 校验 vault 路径合法性:
/// 1. 路径必须存在且是目录;
/// 2. 必须包含 `.obsidian/` 子目录(本项目硬要求,见 task 设计决定 #2)。
pub(super) fn validate_vault_path(vault_path: &Path) -> Result<(), ConfigError> {
    if !vault_path.exists() || !vault_path.is_dir() {
        return Err(ConfigError::VaultPathInvalid(vault_path.to_path_buf()));
    }
    let obsidian_dir = vault_path.join(".obsidian");
    if !obsidian_dir.exists() || !obsidian_dir.is_dir() {
        return Err(ConfigError::NotObsidianVault(vault_path.to_path_buf()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 不存在的 config 文件 → 返回默认值,不报错(首次启动场景)。
    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = load_config(dir.path()).unwrap();
        assert!(cfg.vault_path.is_none());
    }

    /// 写入后再读取应得到相同内容(round-trip)。
    #[test]
    fn save_then_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AppConfig {
            vault_path: Some("/some/vault".to_string()),
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path()).unwrap();
        assert_eq!(loaded.vault_path, Some("/some/vault".to_string()));
    }

    /// app_data_dir 不存在时 save 应自动建目录。
    #[test]
    fn save_creates_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("not_yet_created");
        let cfg = AppConfig {
            vault_path: Some("/v".to_string()),
        };
        save_config(&nested, &cfg).unwrap();
        assert!(nested.exists());
        assert!(config_path(&nested).exists());
    }

    /// 损坏的 JSON → Parse 错误,不静默退化为默认值。
    #[test]
    fn corrupted_json_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(config_path(dir.path()), "{ invalid json").unwrap();
        let err = load_config(dir.path()).unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
    }

    /// 路径不存在 → VaultPathInvalid。
    #[test]
    fn validate_rejects_nonexistent_path() {
        let dir = tempfile::tempdir().unwrap();
        let bogus = dir.path().join("does_not_exist");
        let err = validate_vault_path(&bogus).unwrap_err();
        assert!(matches!(err, ConfigError::VaultPathInvalid(_)));
    }

    /// 路径是文件不是目录 → VaultPathInvalid。
    #[test]
    fn validate_rejects_file_not_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("foo.txt");
        std::fs::write(&file, "x").unwrap();
        let err = validate_vault_path(&file).unwrap_err();
        assert!(matches!(err, ConfigError::VaultPathInvalid(_)));
    }

    /// 目录存在但没有 .obsidian/ → NotObsidianVault。
    #[test]
    fn validate_rejects_dir_without_obsidian() {
        let dir = tempfile::tempdir().unwrap();
        let err = validate_vault_path(dir.path()).unwrap_err();
        assert!(matches!(err, ConfigError::NotObsidianVault(_)));
    }

    /// .obsidian 是文件而不是目录 → NotObsidianVault。
    #[test]
    fn validate_rejects_obsidian_as_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".obsidian"), "x").unwrap();
        let err = validate_vault_path(dir.path()).unwrap_err();
        assert!(matches!(err, ConfigError::NotObsidianVault(_)));
    }

    /// 完整合法的 Obsidian vault 通过校验。
    #[test]
    fn validate_accepts_valid_vault() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".obsidian")).unwrap();
        validate_vault_path(dir.path()).unwrap();
    }
}
