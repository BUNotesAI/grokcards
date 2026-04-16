//! keysight-core — 业务逻辑 crate
//!
//! 从 super-tauri app 抽出的纯业务模块,无 tauri 依赖。被 super-tauri app
//! 与未来的 keysight-cli binary 共享使用。

pub mod db;
pub mod domain;
pub mod errors;
pub mod id;
pub mod ipc;
pub mod models;
pub mod parser;
pub mod state;
pub mod vault_fs;
