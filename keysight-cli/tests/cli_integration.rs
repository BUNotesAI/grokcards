//! CLI 集成测试(Phase 6.1 spec scenarios — integration 层级)。
//!
//! spec.md:
//! - L110 `test_query_works_when_tauri_offline`
//! - L121 `test_query_fails_fast_when_db_missing_without_creating`
//! - L186 `test_query_ignores_rpc_version_mismatch`
//!
//! 用 `env!("CARGO_BIN_EXE_keysight-cli")` 拿到真实 binary 路径,
//! `std::process::Command` spawn 子进程,断言 exit code + stderr。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_keysight-cli");

fn temp_root(prefix: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "keysight_cli_integration_{}_{}",
        prefix,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn seed_db_with_schema(db_path: &Path) {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    keysight_core::db::init_db(&conn).unwrap();
    drop(conn);
}

fn write_config(dir: &Path, db_path: &Path, vault_path: &Path) -> PathBuf {
    let cfg = dir.join("cli-config.toml");
    fs::write(
        &cfg,
        format!(
            "cli_config_version = 1\ndb_path = \"{}\"\nvault_path = \"{}\"\n",
            db_path.display(),
            vault_path.display(),
        ),
    )
    .unwrap();
    cfg
}

/// Spec L110 — 查询命令在 Tauri 离线(无 cli-endpoint.toml)时仍能返回数据。
#[test]
fn test_query_works_when_tauri_offline() {
    let dir = temp_root("offline");
    let db_path = dir.join("keysight.db");
    let vault_path = dir.join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    seed_db_with_schema(&db_path);
    let cfg = write_config(&dir, &db_path, &vault_path);

    // 故意 NOT 创建 cli-endpoint.toml —— 模拟 Tauri 离线

    let output = Command::new(BIN)
        .arg("--config-file")
        .arg(&cfg)
        .arg("list")
        .output()
        .expect("failed to execute keysight-cli");

    assert_eq!(
        output.status.code(),
        Some(0),
        "exit code should be 0 (success)\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Spec L121 — DB 不存在时 CLI fail-fast,**且不** 悄悄建空库。
#[test]
fn test_query_fails_fast_when_db_missing_without_creating() {
    let dir = temp_root("db_missing");
    let vault_path = dir.join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    let missing_db = dir.join("does_not_exist.db");

    let cfg = dir.join("cli-config.toml");
    fs::write(
        &cfg,
        format!(
            "cli_config_version = 1\ndb_path = \"{}\"\nvault_path = \"{}\"\n",
            missing_db.display(),
            vault_path.display(),
        ),
    )
    .unwrap();

    let output = Command::new(BIN)
        .arg("--config-file")
        .arg(&cfg)
        .arg("list")
        .output()
        .expect("failed to execute keysight-cli");

    assert_eq!(
        output.status.code(),
        Some(1),
        "exit code should be 1 (runtime error, not 2 which is NotImplemented)\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("database not found"),
        "stderr should contain 'database not found', got: {}",
        stderr,
    );
    assert!(
        stderr.contains(&missing_db.display().to_string()),
        "stderr should contain the missing DB path, got: {}",
        stderr,
    );

    assert!(
        !missing_db.exists(),
        "DB file must NOT be silently created by a failed READ_ONLY open"
    );
}

/// Spec L186 — 查询命令对 rpc_protocol_version 不做 gate(version 只影响写/flush)。
#[test]
fn test_query_ignores_rpc_version_mismatch() {
    let dir = temp_root("version_mismatch");
    let db_path = dir.join("keysight.db");
    let vault_path = dir.join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    seed_db_with_schema(&db_path);
    let cfg = write_config(&dir, &db_path, &vault_path);

    // 存一个 version 远超 CLI 预期的 endpoint 文件;查询路径应忽略
    let endpoint = dir.join("cli-endpoint.toml");
    fs::write(
        &endpoint,
        "rpc_protocol_version = 99\nhttp_endpoint = \"http://127.0.0.1:0\"\ntoken = \"bogus\"\n",
    )
    .unwrap();

    let output = Command::new(BIN)
        .arg("--config-file")
        .arg(&cfg)
        .arg("--endpoint-file")
        .arg(&endpoint)
        .arg("list")
        .output()
        .expect("failed to execute keysight-cli");

    assert_eq!(
        output.status.code(),
        Some(0),
        "exit code should be 0 (version gate is write-path only; query path ignores)\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
