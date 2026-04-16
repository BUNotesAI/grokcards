//! keysight-cli — 查询 CLI 的 library crate。
//!
//! main.rs 用 `use keysight_cli::*;` 消费这里的模块;
//! 集成测试(`tests/`)也通过此 crate 访问 `commands::*` / `config::load_config` 等。

pub mod client;
pub mod commands;
pub mod config;
pub mod endpoint;
pub mod errors;
