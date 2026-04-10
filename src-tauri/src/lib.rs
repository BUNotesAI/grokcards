use std::sync::Mutex;

use rusqlite::Connection;
use tauri_specta::{collect_commands, Builder};
use specta_typescript::Typescript;

mod app_error;
mod modules;

fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        modules::todo::commands::list_todos,
        modules::todo::commands::create_todo,
        modules::todo::commands::update_todo,
        modules::todo::commands::toggle_todo,
        modules::todo::commands::delete_todo,
        modules::todo::commands::toggle_all,
        modules::todo::commands::clear_completed,
    ])
}

/// 初始化 SQLite 连接并建表。
fn init_database() -> Connection {
    let conn = Connection::open("super_tauri.db").expect("无法打开数据库");
    modules::init_all(&conn).expect("建表失败");
    conn
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = make_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    let conn = init_database();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(conn))
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
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
