#![allow(dead_code)]

use std::path::Path;

use crate::errors::KeysightError;
use crate::models::WhiteboardSummary;

fn validate_name(name: &str) -> Result<String, KeysightError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(KeysightError::ParseError("whiteboard name must not be empty".to_string()));
    }
    if trimmed.contains('/') || trimmed.contains(':') {
        return Err(KeysightError::ParseError(format!(
            "whiteboard name must not contain '/' or ':', got '{trimmed}'"
        )));
    }
    Ok(trimmed.to_string())
}

/// 在 vault 下创建一个空的 `whiteboard/{name}/` 子目录。
pub fn create_folder(
    vault_path: &Path,
    name: &str,
) -> Result<WhiteboardSummary, KeysightError> {
    let name = validate_name(name)?;
    let target = vault_path.join("whiteboard").join(&name);

    if target.exists() {
        return Err(KeysightError::FileError(format!(
            "folder already exists: whiteboard/{name}"
        )));
    }

    std::fs::create_dir_all(&target)
        .map_err(|e| KeysightError::FileError(format!("创建目录 {}: {e}", target.display())))?;

    Ok(WhiteboardSummary {
        whiteboard_id: name,
        cards: 0,
        notes: 0,
        sections: 0,
        aliases: 0,
        tasks: 0,
        questions: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault_path() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "keysight-whiteboard-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(path.join("whiteboard")).unwrap();
        path
    }

    #[test]
    fn test_create_folder_creates_empty_subdirectory() {
        let vault = temp_vault_path();

        let summary = create_folder(&vault, "agent").unwrap();

        assert_eq!(summary.whiteboard_id, "agent");
        assert!(vault.join("whiteboard").join("agent").is_dir());

        std::fs::remove_dir_all(vault).unwrap();
    }

    #[test]
    fn test_create_folder_rejects_invalid_name() {
        let vault = temp_vault_path();

        let empty = create_folder(&vault, "   ").unwrap_err();
        assert!(matches!(empty, KeysightError::ParseError(_)));

        let slash = create_folder(&vault, "a/b").unwrap_err();
        assert!(matches!(slash, KeysightError::ParseError(_)));

        std::fs::remove_dir_all(vault).unwrap();
    }

    #[test]
    fn test_create_folder_rejects_existing_folder() {
        let vault = temp_vault_path();
        std::fs::create_dir_all(vault.join("whiteboard").join("agent")).unwrap();

        let err = create_folder(&vault, "agent").unwrap_err();

        assert!(matches!(err, KeysightError::FileError(_)));
        std::fs::remove_dir_all(vault).unwrap();
    }
}
