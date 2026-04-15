use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder};

mod app_error;
mod modules;
mod perf;

use perf::ScopedTimer;

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
    ])
}

/// 初始化 todo 的 SQLite 连接并建表。
fn init_todo_database() -> Connection {
    let conn = Connection::open("super_tauri.db").expect("无法打开数据库");
    modules::init_all(&conn).expect("建表失败");
    conn
}

/// 初始化 KeySight 的独立 SQLite 连接 + vault 路径。
/// 使用 app_data_dir 存放 keysight.db，确保跨平台路径正确。
fn init_keysight_state(app: &tauri::App) -> modules::keysight::state::KeysightState {
    let _t = ScopedTimer::new("init_keysight_state");
    let vault_path = std::env::var("KEYSIGHT_VAULT_PATH")
        .expect("环境变量 KEYSIGHT_VAULT_PATH 未设置，请设置为 Obsidian vault 根目录路径");

    let data_dir = app.path().app_data_dir().expect("无法获取 app_data_dir");
    std::fs::create_dir_all(&data_dir).expect("无法创建 app_data_dir");
    let db_path = data_dir.join("keysight.db");

    let conn = {
        let _t = ScopedTimer::new("init_keysight_state: open + init_db");
        let conn = Connection::open(&db_path).expect("无法打开 keysight 数据库");
        modules::keysight::init(&conn).expect("keysight 建表失败");
        conn
    };
    modules::keysight::state::KeysightState {
        db: Mutex::new(conn),
        vault_path: PathBuf::from(vault_path),
        db_path,
    }
}

/// 供一次性维护脚本调用：清理历史遗留的 card title 转义。
pub fn cleanup_card_title_escapes(
    db_path: &std::path::Path,
    vault_path: &std::path::Path,
) -> Result<usize, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    modules::keysight::init(&conn).map_err(|e| e.to_string())?;
    modules::keysight::cleanup_card_title_escapes(&conn, vault_path)
}

/// 供一次性维护脚本调用：把 legacy details/summary 迁移到 ?>> / ?<<。
pub fn migrate_toggle_syntax(
    db_path: &std::path::Path,
    vault_path: &std::path::Path,
) -> Result<modules::keysight::models::ToggleSyntaxMigrationReport, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    modules::keysight::init(&conn).map_err(|e| e.to_string())?;
    modules::keysight::migrate_toggle_syntax(&conn, vault_path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = make_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    let todo_conn = init_todo_database();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(todo_conn))
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let _t_setup = ScopedTimer::new("setup hook total");
            let keysight_state = init_keysight_state(app);
            app.manage(keysight_state);

            // 启动时全量同步 vault → DB
            let ks = app.state::<modules::keysight::state::KeysightState>();
            {
                let _t = ScopedTimer::new("startup_sync");
                match modules::keysight::startup_sync(&ks) {
                    Ok(report) => eprintln!(
                        "[keysight] startup sync: scanned={}, synced={}, removed={}, skipped={}, backfilled={}",
                        report.scanned, report.synced, report.removed, report.skipped, report.backfilled
                    ),
                    Err(e) => eprintln!("[keysight] startup sync failed: {e}"),
                }
            }

            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_bindings() {
        make_builder()
            .export(Typescript::default(), "../src/bindings.ts")
            .expect("Failed to export typescript bindings");
    }
}
