//! Phase 6.2a / 6.3 写路径 e2e integration tests。
//!
//! spec.md 覆盖:
//! - L153 `test_write_fails_fast_when_endpoint_file_missing`(Phase 6.2a,任意 mutate)
//! - L267 `test_flush_fails_fast_when_endpoint_file_missing`(Phase 6.3,flush 命令)
//!
//! 用 `env!("CARGO_BIN_EXE_keysight-cli")` 拿真实 binary,`std::process::Command` spawn 子进程,
//! 断言 exit code + stderr。同 `tests/cli_integration.rs` pattern。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_keysight-cli");

fn temp_root(prefix: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "keysight_cli_write_integration_{}_{}",
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
            vault_path.display()
        ),
    )
    .unwrap();
    cfg
}

/// Phase 6.2a spec scenario(spec.md L153):
/// 假设 `cli-config.toml` 存在但 `cli-endpoint.toml` 不存在 → 运行任何写命令。
/// 期望:
/// - CLI exit code 为 1
/// - stderr 包含 "Tauri is not running"
/// - 未发起任何 HTTP 请求(本 test 用"endpoint 路径指向不存在文件"作证据:load_endpoint fail 即未触发 HttpClient 构造,自然未发请求)
#[test]
fn test_write_fails_fast_when_endpoint_file_missing() {
    let dir = temp_root("offline_fail_fast");
    let db_path = dir.join("keysight.db");
    let vault_path = dir.join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    seed_db_with_schema(&db_path); // query path 不会开;但统一 precondition

    let config_path = write_config(&dir, &db_path, &vault_path);

    // 故意指向不存在的 endpoint 文件
    let missing_endpoint = dir.join("cli-endpoint.toml");
    assert!(
        !missing_endpoint.exists(),
        "test precondition: endpoint file must be absent"
    );

    let output = Command::new(BIN)
        .arg("--config-file")
        .arg(&config_path)
        .arg("--endpoint-file")
        .arg(&missing_endpoint)
        .arg("graph")
        .arg("note-create")
        .arg("dummy-title")
        .arg("--wb")
        .arg("wb_root")
        .output()
        .expect("spawn keysight-cli");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for missing endpoint file. stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Tauri is not running"),
        "stderr should contain 'Tauri is not running', got: {}",
        stderr
    );
}

/// Phase 6.3 spec scenario(spec.md L267):
/// 假设 `cli-config.toml` 存在但 `cli-endpoint.toml` 不存在 → 运行 `keysight-cli flush`。
/// 期望:
/// - CLI exit code 为 1
/// - stderr 包含 "Tauri is not running"
/// - 未发起任何 HTTP 请求(endpoint 文件不存在 → load_endpoint fail 即未触发 HttpClient 构造)
///
/// 与 L153 `test_write_fails_fast_when_endpoint_file_missing` 形状一致,锁 flush 命令走
/// 同一条 offline fail-fast 路径,而不是独立一套错误处理。
#[test]
fn test_flush_fails_fast_when_endpoint_file_missing() {
    let dir = temp_root("flush_offline_fail_fast");
    let db_path = dir.join("keysight.db");
    let vault_path = dir.join("vault");
    fs::create_dir_all(&vault_path).unwrap();
    seed_db_with_schema(&db_path); // flush 路径不开 DB;保持与 L153 相同 precondition

    let config_path = write_config(&dir, &db_path, &vault_path);

    // 故意指向不存在的 endpoint 文件
    let missing_endpoint = dir.join("cli-endpoint.toml");
    assert!(
        !missing_endpoint.exists(),
        "test precondition: endpoint file must be absent"
    );

    let output = Command::new(BIN)
        .arg("--config-file")
        .arg(&config_path)
        .arg("--endpoint-file")
        .arg(&missing_endpoint)
        .arg("flush")
        .output()
        .expect("spawn keysight-cli");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for missing endpoint file. stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Tauri is not running"),
        "stderr should contain 'Tauri is not running', got: {}",
        stderr
    );
}
