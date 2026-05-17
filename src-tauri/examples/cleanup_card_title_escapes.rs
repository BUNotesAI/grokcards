mod common;

use super_tauri_lib::cleanup_card_title_escapes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = common::default_db_path();
    let vault_path = common::read_vault_path();
    let backup_path = common::backup_db(&db_path)?;

    let cleaned = cleanup_card_title_escapes(&db_path, &vault_path).map_err(std::io::Error::other)?;

    println!("backup_path={}", backup_path.display());
    println!("cleaned_cards={cleaned}");
    Ok(())
}
