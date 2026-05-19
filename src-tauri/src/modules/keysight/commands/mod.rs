//! Keysight Tauri commands —— 按 entity 拆分子模块,每个 `#[tauri::command]` fn 放
//! 对应文件;`pub use {entity}::*;` 让外部路径 `modules::keysight::commands::{cmd}`
//! 保持不变(`lib.rs collect_commands![]` 一行不改、bindings.ts 一字节不变)。
//!
//! 历史:原 `commands.rs` 单文件 1538 行 / 58 cmd,W1-W3 分波拆为 11 个 entity
//! 子文件 + 本 mod.rs(Config 段 1 cmd `get_vault_info` 体量过小,W3 末并入此处)。
//! 详细决策见 task_a60ceca2 design-v1。
//!
//! 11 个 entity 子文件:`card / section / task / entity_graph / note / sync /
//! question / alias / layout / overview / legacy_import`。

mod alias;
mod card;
mod entity_graph;
mod layout;
mod legacy_import;
mod note;
mod overview;
mod question;
mod section;
mod sync;
mod task;

pub use alias::*;
pub use card::*;
pub use entity_graph::*;
pub use layout::*;
pub use legacy_import::*;
pub use note::*;
pub use overview::*;
pub use question::*;
pub use section::*;
pub use sync::*;
pub use task::*;

use tauri::State;

use super::models::VaultInfoResponse;
use super::runtime_state::KeysightRuntimeState;
use crate::app_error::AppError;

// ============================================================
// Config
// ============================================================

/// 返回当前 vault 配置信息。
#[tauri::command]
#[specta::specta]
pub fn get_vault_info(state: State<'_, KeysightRuntimeState>) -> Result<VaultInfoResponse, AppError> {
    let state = state.resolved()?;
    Ok(VaultInfoResponse {
        vault_path: state.core.vault_path.to_string_lossy().into_owned(),
    })
}
