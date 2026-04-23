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
/// 接 `&KeysightRuntimeInner`(resolved 后的 inner)—— 调用方负责先 `state.resolved()?`
/// 解开 Option,本 fn 保证 inner 已初始化。
pub(crate) fn startup_sync(
    inner: &runtime_state::KeysightRuntimeInner,
) -> Result<models::SyncVaultReport, String> {
    let conn = lock_db(&inner.core.db, "startup_sync");
    if let Err(err) = domain::note::migrate_db_notes_to_files(
        &conn,
        &inner.core.db_path,
        &inner.core.vault_path,
    ) {
        return Err(format!("note migration failed: {err}"));
    }
    let fs = recording_vault_fs::RecordingVaultFs::wrap_real(
        inner.core.vault_path.to_string_lossy().to_string(),
        std::sync::Arc::clone(&inner.suppression),
    );
    domain::sync::sync_vault(&conn, &fs).map_err(|e| e.to_string())
}

/// 完整 bootstrap keysight runtime —— 冷启动路径和 `set_vault_path` command
/// 共用同一实现。
///
/// ## 前置条件
/// - Tauri managed state 里已 `app.manage(KeysightRuntimeState::empty())`;
/// - Tauri managed state 里已 `app.manage(Mutex::new(None::<ServerState>))`
///   作为 HTTP server 的容器(shutdown 路径要 take-by-value);
/// - `state` **尚未** install(否则幂等性违例,见下)。
///
/// `vault_path` 的合法性由本函数内部校验(见执行效果 step 1),调用方不必预先
/// 保证 —— 这样 cold-start 从 stale `config.json` 进入也会在 install 之前
/// fail-fast 回退到 first-run。
///
/// ## 执行效果(transactional:所有 fallible 步骤成功后才 commit)
///
/// 前半段(fallible,任何错误都 early-return,runtime 零副作用):
/// 1. **Vault 路径校验** —— 路径存在 + 是目录 + 含 `.obsidian/` 子目录
///    (防止 stale config 进入 ready 状态);
/// 2. 打开 `{app_data_dir}/keysight.db`,运行 schema migration;
/// 3. 构造 `KeysightRuntimeInner`(含新的 `Arc<SelfWriteSuppression>`,但尚未 install);
/// 4. 启动 HTTP IPC server(fail 场景:端口冲突 / endpoint 文件不可写)。
///
/// 后半段(commit,全部 infallible):
/// 4. 写入 managed `Mutex<Option<ServerState>>`;
/// 5. `state.install(inner)` —— runtime state 正式可用,所有 keysight command 解锁;
/// 6. `spawn_watcher_loop` —— notify 文件监听 + 三路分派(后台 task,内部错误自消化);
/// 7. `startup_sync` —— 全量 vault → DB 同步,仅 log,失败不回滚 runtime。
///
/// ## 不做的事
/// - 不写 `config.json`(由 commands 层负责);
/// - 不校验 vault 路径(由调用方或 config 模块负责);
/// - 不卸载已存在的 HTTP server / watcher(不支持运行中切换 vault)。
///
/// ## 幂等性
/// 非幂等 —— 重复调用会试图再开 HTTP server(端口冲突,早 fail)和 spawn 第二个 watcher。
/// 调用方(setup hook + `set_vault_path`)保证只在未 install 时调用。
///
/// ## 失败语义
/// step 1-3 任一失败 → runtime 未 install、HTTP server 未启动、config.json 未写(由
/// `set_vault_path` 的顺序保证)。前端 `get_vault_config` 仍会返 `ready: false`,弹回
/// VaultSetup(附带 error message)。
pub(crate) fn bootstrap_runtime(
    app_handle: &tauri::AppHandle,
    state: &runtime_state::KeysightRuntimeState,
    vault_path: &std::path::Path,
) -> Result<(), String> {
    use tauri::Manager;

    // --- 前半段:fallible steps(失败则零副作用)---

    // Step 1: vault 路径校验(cold-start 从持久化 config 进入也保底)
    crate::modules::config::ensure_vault_path_valid(vault_path)?;

    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取 app_data_dir: {e}"))?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("无法创建 app_data_dir: {e}"))?;
    let db_path = data_dir.join("keysight.db");

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("无法打开 keysight 数据库: {e}"))?;
    init(&conn).map_err(|e| format!("keysight 建表失败: {e}"))?;

    let core = state::KeysightState {
        db: Mutex::new(conn),
        vault_path: vault_path.to_path_buf(),
        db_path: db_path.clone(),
    };
    let suppression = std::sync::Arc::new(suppression::SelfWriteSuppression::new());
    let pending_inner = runtime_state::KeysightRuntimeInner {
        core,
        suppression: std::sync::Arc::clone(&suppression),
    };

    let server_state = tauri::async_runtime::block_on(start_http_server(
        &data_dir,
        &db_path,
        vault_path,
        app_handle.clone(),
        std::sync::Arc::clone(&suppression),
    ))
    .map_err(|e| format!("keysight HTTP IPC server 启动失败: {e}"))?;

    // --- 后半段:commit(infallible)---

    let server_holder =
        app_handle.state::<Mutex<Option<server_state::ServerState>>>();
    // 例外: Mutex poisoning 不可恢复
    *server_holder.lock().unwrap() = Some(server_state);

    state.install(pending_inner);

    watcher_prod::spawn_watcher_loop(
        app_handle.clone(),
        vault_path.to_path_buf(),
        db_path,
        suppression,
    );

    // startup_sync 在 install 之后用 resolved() 拿 inner —— 同步失败只 log,
    // runtime 仍视为 ready(前端能进主 UI,vault 数据由 watcher 后续增量同步)。
    // 例外: install 之后 resolved() 必定成功
    let installed = state.resolved().expect("resolved() after install must be Some");
    match startup_sync(&installed) {
        Ok(report) => eprintln!(
            "[keysight] startup sync: scanned={}, synced={}, removed={}, skipped={}, backfilled={}",
            report.scanned, report.synced, report.removed, report.skipped, report.backfilled
        ),
        Err(e) => eprintln!("[keysight] startup sync failed: {e}"),
    }

    Ok(())
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
