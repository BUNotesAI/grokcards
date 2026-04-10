use tauri_specta::{collect_commands, Builder};
use specta_typescript::Typescript;

#[tauri::command]
#[specta::specta]
fn greet(name: String) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![greet])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = make_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
