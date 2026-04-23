//! Tauri commands —— 薄壳,每个 command 把参数转交给 domain 或 keysight bootstrap。
//!
//! 所有 `#[tauri::command]` 标注的函数必须 `pub`(不是 `pub(super)`):tauri-specta
//! proc macro 生成的隐藏符号需要和函数同等可见性,否则 `collect_commands![]`
//! 无法找到符号。

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use super::domain::{save_config, validate_vault_path};
use super::models::{AppConfig, VaultConfigResponse};
use crate::app_error::AppError;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;

/// 读取当前 vault 配置的**运行时真实状态** —— 前端启动 gate 的唯一信号源。
///
/// ## 前置条件
/// - 无
///
/// ## 执行效果(纯读)
/// 1. 通过 `KeysightRuntimeState::resolved()` 探测 runtime 是否已 install:
///    - Installed → `{ vaultPath: Some(inner.core.vault_path), ready: true }`;
///    - Uninstalled(未配置 / bootstrap 失败)→ `{ vaultPath: None, ready: false }`。
/// 2. `vaultPath` 反映 **effective vault**(debug env override 或 config.json
///    都最终经 bootstrap 流入 inner.core.vault_path,所以两条路径统一为同一信号);
/// 3. `ready` 由 runtime availability 决定,不依赖 `config.json` 是否存在 —— 这样
///    env override(config.json 空)也能正确进入主 UI,bootstrap 半失败也不会误入主 UI。
///
/// ## 不做的事
/// - 不读 `config.json`(config 仅为"下次启动的 suggested path",不反映当前运行态);
/// - 不校验 vault 目录仍然合法(vault 可能被外部移除,由 watcher 或下次 bootstrap 发现)。
///
/// ## 幂等性
/// 纯读,幂等。
#[tauri::command]
#[specta::specta]
pub fn get_vault_config(
    state: State<'_, KeysightRuntimeState>,
) -> Result<VaultConfigResponse, AppError> {
    match state.resolved() {
        Ok(inner) => Ok(VaultConfigResponse {
            vault_path: Some(inner.core.vault_path.to_string_lossy().into_owned()),
            ready: true,
        }),
        Err(_) => Ok(VaultConfigResponse {
            vault_path: None,
            ready: false,
        }),
    }
}

/// 首次启动时设置 vault 路径 —— bootstrap runtime 成功后才持久化 config。
///
/// ## 前置条件
/// - `vault_path` 对应的目录存在且包含 `.obsidian/` 子目录(`validate_vault_path`);
/// - keysight runtime **尚未**初始化(`state.resolved()` 返 `VaultNotConfigured`);
///   重复调用 bootstrap 不幂等(HTTP 端口冲突),当前不支持运行中切换 vault。
///
/// ## 执行效果(transactional:bootstrap 失败则零持久化)
/// 1. 校验 `.obsidian/` 存在;
/// 2. 调 `keysight::bootstrap_runtime` —— 开 DB、建表、启动 HTTP server、install state、
///    spawn watcher、startup sync;**任何 fallible 步骤失败 → 早 return,零副作用**;
/// 3. bootstrap 成功后才 `save_config` 把 `vault_path` 写入 `config.json`。
///
/// ## 不做的事
/// - 不重启 app;
/// - 不关闭已有 HTTP server / watcher;
/// - `save_config` 失败时 **不**回滚 runtime install(罕见 IO 错误;运行时仍可用,
///   仅下次启动记不住 vault path —— 用户会看到 save error,可选 retry)。
///
/// ## 幂等性
/// 非幂等 —— 见 `bootstrap_runtime`。VaultSetup UI 在 onConfigured 后卸载避免重复。
#[tauri::command]
#[specta::specta]
pub fn set_vault_path(
    app: AppHandle,
    state: State<'_, KeysightRuntimeState>,
    vault_path: String,
) -> Result<(), AppError> {
    let path = PathBuf::from(&vault_path);
    validate_vault_path(&path).map_err(Into::<AppError>::into)?;

    // 先 bootstrap —— 失败则早 return,config.json 不写,ready 保持 false。
    crate::modules::keysight::bootstrap_runtime(&app, &state, &path)
        .map_err(|e| AppError::Config { message: e })?;

    // bootstrap 成功后才持久化:后续启动能复用此 path。
    let data_dir = app_data_dir(&app)?;
    save_config(
        &data_dir,
        &AppConfig {
            vault_path: Some(vault_path),
        },
    )
    .map_err(Into::<AppError>::into)?;

    Ok(())
}

/// 退出应用 —— 用户在 VaultSetup 对话框取消 / 关闭时调用。
///
/// ## 前置条件
/// - 无
///
/// ## 执行效果
/// 1. 调 `AppHandle::exit(0)` 触发 Tauri 的正常退出流程,`RunEvent::ExitRequested`
///    回调会运行,keysight shutdown 钩子被执行。
///
/// ## 幂等性
/// 单次退出,后续调用无意义。
#[tauri::command]
#[specta::specta]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path().app_data_dir().map_err(|e| AppError::Config {
        message: format!("无法获取 app_data_dir: {e}"),
    })
}
