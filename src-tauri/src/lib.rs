#![warn(clippy::too_many_lines)]

use std::sync::Mutex;

use rusqlite::Connection;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder};

mod app_error;
mod modules;
mod perf;

use perf::ScopedTimer;

/// bindings.ts 导出路径 —— `concat!` 锚定到本 crate 的 `CARGO_MANIFEST_DIR`,
/// 不依赖调用者 cwd(pre-existing D1-a;见 task_dd9e57db plan v2 Phase 2b)。
const BINDINGS_TS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        // todo commands
        modules::todo::commands::list_todos,
        modules::todo::commands::create_todo,
        modules::todo::commands::update_todo,
        modules::todo::commands::toggle_todo,
        modules::todo::commands::delete_todo,
        modules::todo::commands::toggle_all,
        modules::todo::commands::clear_completed,
        // keysight: card
        modules::keysight::commands::card_get,
        modules::keysight::commands::card_query_all,
        modules::keysight::commands::card_query_by_file,
        modules::keysight::commands::card_query_by_ids,
        modules::keysight::commands::card_count,
        modules::keysight::commands::card_search,
        modules::keysight::commands::card_query_links,
        modules::keysight::commands::card_edit_title,
        modules::keysight::commands::card_edit_body,
        modules::keysight::commands::card_update_understanding,
        modules::keysight::commands::card_set_color,
        // keysight: section
        modules::keysight::commands::section_get,
        modules::keysight::commands::section_query_all,
        modules::keysight::commands::section_create,
        modules::keysight::commands::section_delete,
        modules::keysight::commands::section_update,
        modules::keysight::commands::section_add_member,
        modules::keysight::commands::section_remove_member,
        modules::keysight::commands::section_move_to_whiteboard,
        // keysight: task
        modules::keysight::commands::task_query_all,
        modules::keysight::commands::task_query_kanban,
        modules::keysight::commands::task_create,
        modules::keysight::commands::task_update,
        modules::keysight::commands::task_delete,
        modules::keysight::commands::task_set_color,
        modules::keysight::commands::task_update_with_subtasks,
        // keysight: question
        modules::keysight::commands::question_query_all,
        modules::keysight::commands::question_create,
        modules::keysight::commands::question_update,
        modules::keysight::commands::question_delete,
        // keysight: note
        modules::keysight::commands::note_get,
        modules::keysight::commands::note_query_all,
        modules::keysight::commands::note_create,
        modules::keysight::commands::note_delete,
        modules::keysight::commands::note_update,
        modules::keysight::commands::note_migrate_to_files,
        // keysight: alias
        modules::keysight::commands::alias_get,
        modules::keysight::commands::alias_query_all,
        modules::keysight::commands::alias_create,
        modules::keysight::commands::alias_delete,
        // keysight: layout
        modules::keysight::commands::layout_query_positions,
        modules::keysight::commands::layout_set_position,
        modules::keysight::commands::layout_remove_position,
        // keysight: entity graph
        modules::keysight::commands::entity_edges_from,
        modules::keysight::commands::entity_edges_to,
        modules::keysight::commands::entity_connect,
        modules::keysight::commands::entity_relate,
        modules::keysight::commands::entity_disconnect,
        // keysight: sync
        modules::keysight::commands::sync_file,
        modules::keysight::commands::sync_remove_file,
        modules::keysight::commands::sync_all_file_mtimes,
        modules::keysight::commands::sync_vault,
        // keysight: overview
        modules::keysight::commands::overview_stats,
        modules::keysight::commands::overview_graph,
        modules::keysight::commands::whiteboard_list,
        modules::keysight::commands::whiteboard_create,
        // keysight: config
        modules::keysight::commands::get_vault_info,
        // keysight: legacy import
        modules::keysight::commands::import_legacy_db,
        // app config: first-run vault setup
        modules::config::commands::get_vault_config,
        modules::config::commands::set_vault_path,
        modules::config::commands::exit_app,
    ])
}

/// 初始化 todo 的 SQLite 连接并建表。
///
/// 使用 `app_data_dir/super_tauri.db` 而非 CWD 下的相对路径 —— 避免 Finder
/// 双击 `.app` 启动时 CWD=`/` 导致无法写入 panic。
fn init_todo_database(data_dir: &std::path::Path) -> Connection {
    std::fs::create_dir_all(data_dir).expect("无法创建 app_data_dir");
    let db_path = data_dir.join("super_tauri.db");
    let conn = Connection::open(&db_path).expect("无法打开 todo 数据库");
    modules::init_all(&conn).expect("todo 建表失败");
    conn
}

/// 解析启动时的 vault 路径:
/// 1. **仅 debug build**:环境变量 `KEYSIGHT_VAULT_PATH` 作为 override(dev 友好);
/// 2. 读 `{app_data_dir}/config.json` 的 `vault_path` 字段;
/// 3. 都没有 → 返回 `None`(进入 first-run flow,前端弹 VaultSetup)。
///
/// debug/release 行为分叉的原因:发给别人用的 release build 不该被他人 shell
/// 的 env 污染你的配置(见 task 设计决定 #3)。
fn resolve_startup_vault_path(app: &tauri::App) -> Option<std::path::PathBuf> {
    #[cfg(debug_assertions)]
    if let Ok(p) = std::env::var("KEYSIGHT_VAULT_PATH") {
        eprintln!("[keysight] debug build: using KEYSIGHT_VAULT_PATH env override: {p}");
        return Some(std::path::PathBuf::from(p));
    }

    let data_dir = app.path().app_data_dir().ok()?;
    match modules::config::load_startup_config(&data_dir) {
        Ok(cfg) => cfg.vault_path.map(std::path::PathBuf::from),
        Err(e) => {
            eprintln!("[keysight] 读取 config.json 失败(启动走 first-run flow): {e}");
            None
        }
    }
}

/// 供一次性维护脚本调用：清理历史遗留的 card title 转义。
///
/// 独立 process 调用,现场构造临时 `SelfWriteSuppression`(无 watcher 消费,仅占位);
/// 语义正确性不依赖 suppression 命中(script 跑完 Arc drop)。
pub fn cleanup_card_title_escapes(
    db_path: &std::path::Path,
    vault_path: &std::path::Path,
) -> Result<usize, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    modules::keysight::init(&conn).map_err(|e| e.to_string())?;
    let suppression = std::sync::Arc::new(
        modules::keysight::suppression::SelfWriteSuppression::new(),
    );
    modules::keysight::cleanup_card_title_escapes(&conn, vault_path, &suppression)
}

/// 供一次性维护脚本调用：把 legacy details/summary 迁移到 ?>> / ?<<。
pub fn migrate_toggle_syntax(
    db_path: &std::path::Path,
    vault_path: &std::path::Path,
) -> Result<modules::keysight::models::ToggleSyntaxMigrationReport, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    modules::keysight::init(&conn).map_err(|e| e.to_string())?;
    let suppression = std::sync::Arc::new(
        modules::keysight::suppression::SelfWriteSuppression::new(),
    );
    modules::keysight::migrate_toggle_syntax(&conn, vault_path, &suppression)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = make_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), BINDINGS_TS_PATH)
        .expect("Failed to export typescript bindings");

    let tauri_app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // KeysightRuntimeState 以 empty 态先 manage —— bootstrap_runtime 后续写入 inner。
        // 未配置 vault 时 keysight command 返 VaultNotConfigured,前端据此弹 VaultSetup。
        .manage(modules::keysight::runtime_state::KeysightRuntimeState::empty())
        // HTTP IPC server 容器 —— bootstrap_runtime 负责塞 Some;shutdown 路径 take。
        .manage(Mutex::new(
            None::<modules::keysight::server_state::ServerState>,
        ))
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let _t_setup = ScopedTimer::new("setup hook total");

            // 1. todo DB 放 app_data_dir(避免 Finder 启动 CWD=/ 导致写入 panic)
            let data_dir = app.path().app_data_dir().expect("无法获取 app_data_dir");
            let todo_conn = init_todo_database(&data_dir);
            app.manage(Mutex::new(todo_conn));

            // 2. 解析启动 vault 路径(debug env override > config.json > None)
            let vault_path = resolve_startup_vault_path(app);

            // 3. 若有路径则 bootstrap keysight;否则进入 first-run flow(前端弹 VaultSetup)
            if let Some(path) = vault_path {
                let state = app
                    .state::<modules::keysight::runtime_state::KeysightRuntimeState>();
                let app_handle = app.handle().clone();
                if let Err(e) =
                    modules::keysight::bootstrap_runtime(&app_handle, &state, &path)
                {
                    eprintln!(
                        "[keysight] bootstrap 失败({}): {e} —— 退化到 first-run flow",
                        path.display()
                    );
                }
            } else {
                eprintln!(
                    "[keysight] 未配置 vault_path,进入 first-run flow(前端将展示 VaultSetup)"
                );
            }

            builder.mount_events(app);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // design-v4 §3.6 关闭序列 —— ExitRequested 是真正的 app 退出信号
    // (对齐 rp-codex review:CloseRequested 可被 prevent,不适合 once-and-only-once 的 shutdown)
    tauri_app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            modules::keysight::shutdown(app_handle);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_bindings() {
        make_builder()
            .export(Typescript::default(), BINDINGS_TS_PATH)
            .expect("Failed to export typescript bindings");
    }
}
