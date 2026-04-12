use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super_tauri_lib::cleanup_card_title_escapes;

fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn build_backup_path(db_path: &Path) -> PathBuf {
    let stem = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("keysight");
    let ext = db_path.extension().and_then(|s| s.to_str()).unwrap_or("db");
    db_path.with_file_name(format!("{stem}.backup-{}.{}", timestamp_secs(), ext))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "{}/Library/Application Support/co.bunotes.super-tauri/keysight.db",
                std::env::var("HOME").unwrap_or_default()
            ))
        });
    let vault_path = std::env::var("KEYSIGHT_VAULT_PATH")
        .map(PathBuf::from)
        .expect("环境变量 KEYSIGHT_VAULT_PATH 未设置");

    let backup_path = build_backup_path(&db_path);
    std::fs::copy(&db_path, &backup_path)?;

    let cleaned = cleanup_card_title_escapes(&db_path, &vault_path).map_err(std::io::Error::other)?;

    println!("backup_path={}", backup_path.display());
    println!("cleaned_cards={cleaned}");
    Ok(())
}
