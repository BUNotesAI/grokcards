pub mod commands;

// Phase 3 新增基础设施模块(`pub(super)`,限 keysight 模块内;Phase 4 http_server.rs
// 作为兄弟能访问)
pub(super) mod config_file;
pub(super) mod endpoint_file;
pub(super) mod http_server;
pub(crate) mod server_state;
// Phase 5 新增:file watcher 三路分派 + self-write suppression 双队列
// Phase 6.2c:suppression 提到 pub(crate) 让 lib.rs setup / maintenance script 能构造 Arc
pub(crate) mod suppression;
pub(super) mod watcher;
// Phase 6.2b 新增:写命令分派器(handle_rpc mutate 分支的 core logic)
pub(super) mod dispatcher;
// Phase 5b wiring(6.2c):记录 self-write fingerprint 的 VaultFs 装饰器
pub(super) mod recording_vault_fs;
// Phase 5b wiring(6.2c):Tauri 运行时 state wrapper(core + suppression)
pub(crate) mod runtime_state;
// Phase 5b wiring(6.2c):production WatcherHandlers impl + notify watcher loop spawn
pub(crate) mod watcher_prod;

// 从 keysight-core re-export,保持 src-tauri 内部 use 路径不变:
// - `crate::modules::keysight::domain::X` → 解析到 `keysight_core::domain::X`
// - `crate::modules::keysight::{models, state, ...}` 同理
// 原 src-tauri 里 `models` / `state` 是 `pub mod`,`db / domain / errors / id / parser / vault_fs`
// 是 private mod;搬到 keysight-core 后全部变成 `pub mod`(跨 crate 可见),
// 这里的 re-export 只是向后兼容 src-tauri 内部的已有 use 路径,不额外放宽 src-tauri 对外的 surface。
// 只 re-export src-tauri 内部实际引用的 5 个 mod(mod.rs hook fn + commands.rs 用到)。
// `errors` / `id` / `parser` 是 keysight-core 内部细节,src-tauri 不需要感知。
pub use keysight_core::{db, domain, models, state};

use std::sync::Mutex;

use crate::perf::lock_db;

/// 初始化 keysight 模块的数据库表。
pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    db::init_db(conn)
}

/// 启动 HTTP IPC server + 发布 cli-config.toml / cli-endpoint.toml 两文件。
///
/// 供 lib.rs setup 阶段调用(`tauri::async_runtime::block_on` 包装)。
/// 返回的 `ServerState` 要被 `app.manage(Mutex::new(Some(server_state)))` 以
/// 支持 `shutdown(app_handle)` 的 take-by-value 顺序契约。
///
/// `app_handle` 用来构造 `TauriEmitter`,给 `handle_rpc` mutate / flush 分支
/// emit `"entity:changed"` / `"vault:flush"` 事件到前端。
///
/// `suppression` 与 Tauri commands 共享同一 `Arc<SelfWriteSuppression>`;IPC server
/// 的 mutate 分支(`handle_rpc`)构造 `RecordingVaultFs` 时注入,让 CLI 写的文件
/// 也能被 watcher 识别为自写 skip(Phase 5b wiring)。
pub(crate) async fn start_http_server(
    data_dir: &std::path::Path,
    db_path: &std::path::Path,
    vault_path: &std::path::Path,
    app_handle: tauri::AppHandle,
    suppression: std::sync::Arc<suppression::SelfWriteSuppression>,
) -> std::io::Result<server_state::ServerState> {
    let emitter: std::sync::Arc<dyn dispatcher::EventEmitter> =
        std::sync::Arc::new(dispatcher::TauriEmitter { app: app_handle });
    let server_state =
        http_server::start(data_dir, db_path, vault_path, emitter, suppression).await?;
    eprintln!(
        "[keysight] HTTP IPC server listening on {}",
        server_state.local_addr
    );
    Ok(server_state)
}

/// 4 步 shutdown 顺序契约(design-v4 D2-a):
///
/// 1. `endpoint_guard.invalidate_atomic()` —— 文件系统级原子删除 cli-endpoint.toml
/// 2. `shutdown_tx.send(())` —— axum graceful shutdown signal
/// 3. `timeout(5s, join_handle).await` —— 等 in-flight handler 完成
/// 4. `drop(endpoint_guard)` —— RAII 兜底(step 1 正常完成后为 noop)
///
/// 在 Tauri `RunEvent::ExitRequested` 回调里调用。本 fn 把 `ServerState` 字段的
/// destructure 封装在 keysight 模块内,保持 `pub(super)` 字段窄接口不外泄到 crate root。
pub(crate) fn shutdown(app_handle: &tauri::AppHandle) {
    use tauri::Manager;

    let state = app_handle.state::<Mutex<Option<server_state::ServerState>>>();
    let mut guard = match state.lock() {
        Ok(g) => g,
        // 例外: Mutex poisoning 不可恢复,shutdown 是 best-effort 路径
        Err(poison) => poison.into_inner(),
    };
    let Some(srv) = guard.take() else {
        return; // 已经 shutdown 过或从未启动
    };

    let server_state::ServerState {
        shutdown_tx,
        join_handle,
        endpoint_guard,
        local_addr: _,
    } = srv;

    // Step 1: 文件系统级原子删除 cli-endpoint.toml
    if let Err(e) = endpoint_guard.invalidate_atomic() {
        eprintln!("[keysight] shutdown step 1 invalidate_atomic failed: {e}");
    }

    // Step 2: 向 axum server task 发 graceful shutdown signal
    let _ = shutdown_tx.send(());

    // Step 3: 等现有 in-flight handler 完成(5s timeout)
    let _ = tauri::async_runtime::block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), join_handle).await
    });

    // Step 4: 显式 drop —— step 1 正常完成后 Drop 是幂等 noop;crash path 兜底
    drop(endpoint_guard);
}

/// 启动时同步 vault 到 DB。
///
/// 供 lib.rs setup 阶段调用。内部委托 domain::sync::sync_vault。
///
/// 接 `&KeysightRuntimeState`(Phase 6.2c)—— 直接从 runtime state 拿 core + suppression,
/// 不需调用方多传一个参数。
pub fn startup_sync(
    state: &runtime_state::KeysightRuntimeState,
) -> Result<models::SyncVaultReport, String> {
    let conn = lock_db(&state.core.db, "startup_sync");
    if let Err(err) = domain::note::migrate_db_notes_to_files(
        &conn,
        &state.core.db_path,
        &state.core.vault_path,
    ) {
        return Err(format!("note migration failed: {err}"));
    }
    let fs = recording_vault_fs::RecordingVaultFs::wrap_real(
        state.core.vault_path.to_string_lossy().to_string(),
        std::sync::Arc::clone(&state.suppression),
    );
    domain::sync::sync_vault(&conn, &fs).map_err(|e| e.to_string())
}

/// # 清理卡片标题历史转义
///
/// ## 前置条件
/// - `conn` 指向目标 keysight SQLite
/// - `vault_path` 指向与该 SQLite 对应的 vault 根目录
///
/// ## 执行效果
/// 1. 遍历所有 `kind='card'` 的实体
/// 2. 只清理 card title 中历史遗留的安全标点转义
/// 3. 同步写回 markdown 文件、`entities.title` 和 `entities_fts.title`
///
/// ## 不做的事
/// - 不修改正文和其他 frontmatter 字段
/// - 不修改非 card 实体
///
/// ## 幂等性
/// 干净数据重复调用不会产生额外写入。
///
/// ## 关联操作
/// - [`startup_sync`] — 启动时从 vault 同步到 DB
pub fn cleanup_card_title_escapes(
    conn: &rusqlite::Connection,
    vault_path: &std::path::Path,
    suppression: &std::sync::Arc<suppression::SelfWriteSuppression>,
) -> Result<usize, String> {
    let fs = recording_vault_fs::RecordingVaultFs::wrap_real(
        vault_path.to_string_lossy().to_string(),
        std::sync::Arc::clone(suppression),
    );
    domain::card::cleanup_dirty_card_title_escapes(conn, &fs).map_err(|e| e.to_string())
}

/// # 迁移 legacy toggle 语法
///
/// ## 前置条件
/// - `conn` 指向目标 keysight SQLite
/// - `vault_path` 指向与该 SQLite 对应的 vault 根目录
///
/// ## 执行效果
/// 1. 扫描 `whiteboard/` 下全部 markdown 文件
/// 2. 把 legacy `<details>/<summary>` 改写成 `?>> / ?<<`
/// 3. 同步更新 DB 和 FTS
/// 4. 同时清理 DB 内部的非文件型 note
///
/// ## 不做的事
/// - 不修改未使用 legacy 语法的内容
///
/// ## 幂等性
/// 幂等:重复执行不会产生额外修改。
pub fn migrate_toggle_syntax(
    conn: &rusqlite::Connection,
    vault_path: &std::path::Path,
    suppression: &std::sync::Arc<suppression::SelfWriteSuppression>,
) -> Result<models::ToggleSyntaxMigrationReport, String> {
    let fs = recording_vault_fs::RecordingVaultFs::wrap_real(
        vault_path.to_string_lossy().to_string(),
        std::sync::Arc::clone(suppression),
    );
    domain::migration::migrate_legacy_toggle_syntax(conn, &fs).map_err(|e| e.to_string())
}
