//! `cli-config.toml` 持久化配置(design-v4 D3a)—— 记录 Tauri app 当前使用的
//! db_path 和 vault_path。
//!
//! Tauri app 启动时调 [`ensure`] 幂等写该文件;shutdown **不删**,keysight-cli 的
//! 查询命令直接读此文件拿 db_path,实现"Tauri 离线也能做只读 inspection"的 D3a 契约。

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// cli-config.toml 的当前 schema 版本。
/// 未来改 schema(加字段 / 改语义)时 bump,CLI 读时可按 version 做兼容分支。
const CLI_CONFIG_VERSION: u32 = 1;

/// cli-config.toml 文件内容 schema。
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct CliConfig {
    pub(super) cli_config_version: u32,
    pub(super) db_path: String,
    pub(super) vault_path: String,
}

/// 幂等写 `cli-config.toml`:
///
/// - 文件不存在 → atomic write
/// - 文件存在且内容与期望一致 → noop
/// - 文件存在但 db_path / vault_path 与期望不一致 → atomic rewrite
///
/// ## 前置条件
/// - `path` 的父目录已存在(调用方负责 mkdir);否则返回 `io::ErrorKind::NotFound`
///
/// ## 执行效果
/// 1. 读磁盘上可能存在的 cli-config.toml
/// 2. 对比内容 —— 不一致则 atomic rewrite(写 tmp + rename)
/// 3. 一致则 noop
///
/// ## 幂等性
/// 重复调用同一 `(path, db_path, vault_path)` 组合不会产生额外写入。
#[allow(dead_code)] // Phase 4 wiring:setup hook 调用
pub(super) fn ensure(
    path: &Path,
    db_path: &Path,
    vault_path: &Path,
) -> io::Result<()> {
    let expected = CliConfig {
        cli_config_version: CLI_CONFIG_VERSION,
        db_path: db_path.to_string_lossy().into_owned(),
        vault_path: vault_path.to_string_lossy().into_owned(),
    };

    // 现存文件内容一致 → noop
    if let Ok(existing) = fs::read_to_string(path)
        && let Ok(parsed) = toml::from_str::<CliConfig>(&existing)
        && parsed == expected
    {
        return Ok(());
    }

    // 不一致或不存在 → atomic write:tmp + rename。
    // `fs::rename` 在当前 Rust std(1.28+)两平台都 replace existing file — drifted
    // config 直接被覆盖,不需要先 remove。见 std::fs::rename doc + endpoint_file.rs
    // write_atomic 的同类注释。
    let tmp = path.with_extension("toml.tmp");
    let rendered = toml::to_string(&expected)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&tmp, rendered)?;
    fs::rename(tmp, path)
}

// -----------------------------------------------------------------------------
// Tests(模块内 unit,真实 tempdir fixture,保 pub(super) 窄接口)
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario: `config_file::ensure` 在磁盘 config 与期望 db_path 不一致时覆盖更新
    /// (spec.md acceptance criteria,design-v4 R11 一致性兜底)
    #[test]
    fn test_config_file_ensure_updates_drifted_db_path() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("cli-config.toml");

        // 写一个旧版本 config,db_path 指向已过期路径
        let old_toml = r#"cli_config_version = 1
db_path = "/old/path/keysight.db"
vault_path = "/old/vault"
"#;
        fs::write(&config_path, old_toml).unwrap();

        // 期望的新路径
        let p_new = tmp.path().join("new-keysight.db");
        let vault_new = tmp.path().join("new-vault");

        ensure(&config_path, &p_new, &vault_new).unwrap();

        // 文件被 rewrite,db_path / vault_path 更新为期望值
        let contents = fs::read_to_string(&config_path).unwrap();
        let parsed: CliConfig = toml::from_str(&contents).unwrap();
        assert_eq!(parsed.db_path, p_new.to_string_lossy());
        assert_eq!(parsed.vault_path, vault_new.to_string_lossy());
        assert_eq!(parsed.cli_config_version, CLI_CONFIG_VERSION);
    }
}
