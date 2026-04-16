//! CLI endpoint 文件加载 —— 三级优先级。
//!
//! Phase 6.2a:仅写/flush 命令读此文件(查询命令不依赖 Tauri 在线,直连 SQLite)。
//! 文件由 Tauri 启动时 atomic write(见 `src-tauri/src/modules/keysight/endpoint_file.rs`),
//! 关闭时删除 —— 所以 "文件缺失" = "Tauri 离线" 的强信号(spec L156)。

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config;
use crate::errors::CliError;

pub const ENDPOINT_FILENAME: &str = "cli-endpoint.toml";
pub const ENV_VAR: &str = "KEYSIGHT_CLI_ENDPOINT";

/// `cli-endpoint.toml` 反序列化结构。和 `src-tauri/.../endpoint_file.rs` 的写侧
/// `EndpointFileContents` 镜像 —— server 写什么,CLI 读什么。
///
/// **只读侧不包含 `started_at`**:server 侧有 rfc3339 时间戳,CLI 不用于 gate,
/// 也不展示。toml 反序列化对未知字段默认忽略,所以不定义也兼容。
#[derive(Debug, Deserialize)]
pub struct EndpointFileContents {
    /// HTTP endpoint,形如 `http://127.0.0.1:51234`(loopback + OS 分配端口)
    pub http_endpoint: String,
    /// 64-char lowercase hex token(server 启动时生成 256-bit random)
    pub token: String,
    /// Server 声明的 RPC 协议 major 版本(CLI 前置 gate 比对)
    pub rpc_protocol_version: u32,
}

/// 三级优先级 endpoint 文件发现:
///
/// 1. `--endpoint-file` flag(最高优先级)
/// 2. `KEYSIGHT_CLI_ENDPOINT` 环境变量
/// 3. `{app_data_override 或 app_data_dir}/cli-endpoint.toml`(最低优先级)
///
/// `app_data_override` 是测试专用入口(避免读 production 的 app_data_dir);production 传 `None`。
///
/// **文件缺失时返 `CliError::Endpoint`**,Display 会包含 "Tauri is not running"(spec L159)。
pub fn load_endpoint(
    flag: Option<&Path>,
    app_data_override: Option<&Path>,
) -> Result<EndpointFileContents, CliError> {
    let endpoint_path = if let Some(f) = flag {
        f.to_path_buf()
    } else if let Ok(env_path) = std::env::var(ENV_VAR) {
        PathBuf::from(env_path)
    } else {
        let base = config::resolve_app_data_dir(app_data_override)?;
        base.join(ENDPOINT_FILENAME)
    };

    let content = std::fs::read_to_string(&endpoint_path).map_err(|e| {
        CliError::Endpoint(format!("read {}: {}", endpoint_path.display(), e))
    })?;
    toml::from_str(&content)
        .map_err(|e| CliError::Endpoint(format!("parse {}: {}", endpoint_path.display(), e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "keysight_cli_endpoint_test_{}_{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_ep(path: &Path, http_endpoint: &str) {
        fs::write(
            path,
            format!(
                "http_endpoint = \"{}\"\ntoken = \"dummy-token\"\nrpc_protocol_version = 1\n",
                http_endpoint
            ),
        )
        .unwrap();
    }

    /// Phase 6.2a spec scenario(spec.md L290):
    /// 三份 cli-endpoint.toml 同时存在于 Pf/Pe/Pd,KEYSIGHT_CLI_ENDPOINT=Pe,flag=--endpoint-file Pf。
    /// 期望:CLI 用 Pf 读 endpoint,`http_endpoint` = Pf 中声明值。
    ///
    /// 注:Rust 2024 `set_var` 需 unsafe。此测单线程下运行不会竞争(cargo test parallel,
    /// 各测试用独立 env prefix;Red 阶段 todo!() panic 发生在 env 访问之前)。
    #[test]
    fn test_endpoint_file_flag_takes_precedence_over_env_and_default() {
        let flag_dir = temp_root("ep_flag");
        let env_dir = temp_root("ep_env");
        let default_dir = temp_root("ep_default");

        let flag_ep = flag_dir.join(ENDPOINT_FILENAME);
        let env_ep = env_dir.join(ENDPOINT_FILENAME);
        let default_ep = default_dir.join(ENDPOINT_FILENAME);

        write_ep(&flag_ep, "http://flag.local:1111");
        write_ep(&env_ep, "http://env.local:2222");
        write_ep(&default_ep, "http://default.local:3333");

        unsafe {
            std::env::set_var(ENV_VAR, &env_ep);
        }

        let ep = load_endpoint(Some(&flag_ep), Some(&default_dir)).unwrap();

        unsafe {
            std::env::remove_var(ENV_VAR);
        }

        assert_eq!(
            ep.http_endpoint, "http://flag.local:1111",
            "flag should win over env and default"
        );
    }
}

