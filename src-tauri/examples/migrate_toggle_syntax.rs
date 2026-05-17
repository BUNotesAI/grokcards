mod common;

use super_tauri_lib::migrate_toggle_syntax;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = common::default_db_path();
    let vault_path = common::read_vault_path();
    let backup_path = common::backup_db(&db_path)?;

    let report = migrate_toggle_syntax(&db_path, &vault_path).map_err(std::io::Error::other)?;

    println!("backup_path={}", backup_path.display());
    println!("files_updated={}", report.files_updated);
    println!("db_notes_updated={}", report.db_notes_updated);
    Ok(())
}
