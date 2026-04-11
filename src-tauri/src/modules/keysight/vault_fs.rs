#![allow(dead_code)]

use std::path::{Path, PathBuf};

use super::errors::KeysightError;

/// 文件系统操作契约 — 测试时用 mock 替代真实文件系统。
pub(in crate::modules::keysight) trait VaultFs {
    /// 读取 vault 内相对路径的文件内容。
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError>;
    /// 写入 vault 内相对路径的文件内容。
    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError>;
    /// 列出指定子目录下所有 .md 文件及其 mtime（epoch ms）。
    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError>;
}

/// 真实文件系统实现。
pub(in crate::modules::keysight) struct RealVaultFs {
    vault_path: String,
}

impl RealVaultFs {
    pub fn new(vault_path: String) -> Self {
        Self { vault_path }
    }

    fn abs_path(&self, relative: &str) -> String {
        if self.vault_path.ends_with('/') {
            format!("{}{}", self.vault_path, relative)
        } else {
            format!("{}/{}", self.vault_path, relative)
        }
    }

    /// 递归遍历目录，收集所有 .md 文件的相对路径和 mtime（epoch ms）。
    fn walk_md_files(
        dir: &Path,
        vault_root: &Path,
        results: &mut Vec<(String, f64)>,
    ) -> Result<(), KeysightError> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| KeysightError::FileError(format!("读取目录 {}: {e}", dir.display())))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| KeysightError::FileError(format!("遍历目录项: {e}")))?;
            let path = entry.path();
            if path.is_dir() {
                Self::walk_md_files(&path, vault_root, results)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let relative = path
                    .strip_prefix(vault_root)
                    .map_err(|e| KeysightError::FileError(format!("计算相对路径: {e}")))?
                    .to_string_lossy()
                    .to_string();
                let metadata = std::fs::metadata(&path)
                    .map_err(|e| KeysightError::FileError(format!("读取元数据 {}: {e}", path.display())))?;
                let mtime = metadata
                    .modified()
                    .map_err(|e| KeysightError::FileError(format!("读取 mtime {}: {e}", path.display())))?
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
                    * 1000.0;
                results.push((relative, mtime));
            }
        }
        Ok(())
    }
}

impl VaultFs for RealVaultFs {
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError> {
        let abs = self.abs_path(relative_path);
        std::fs::read_to_string(&abs)
            .map_err(|e| KeysightError::FileError(format!("读取 {abs}: {e}")))
    }

    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError> {
        let abs = self.abs_path(relative_path);
        if let Some(parent) = Path::new(&abs).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| KeysightError::FileError(format!("创建目录 {}: {e}", parent.display())))?;
        }
        std::fs::write(&abs, content)
            .map_err(|e| KeysightError::FileError(format!("写入 {abs}: {e}")))
    }

    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError> {
        let vault_root = PathBuf::from(&self.vault_path);
        let target_dir = vault_root.join(subdir);
        if !target_dir.exists() {
            return Ok(vec![]);
        }
        let mut results = Vec::new();
        Self::walk_md_files(&target_dir, &vault_root, &mut results)?;
        Ok(results)
    }
}

/// 测试用 mock 文件系统。
#[cfg(test)]
pub(super) struct MockVaultFs {
    files: std::cell::RefCell<std::collections::HashMap<String, String>>,
    mtimes: std::cell::RefCell<std::collections::HashMap<String, f64>>,
}

#[cfg(test)]
impl MockVaultFs {
    pub fn new() -> Self {
        Self {
            files: std::cell::RefCell::new(std::collections::HashMap::new()),
            mtimes: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.files
            .borrow_mut()
            .insert(path.to_string(), content.to_string());
        self
    }

    /// 设置文件的 mtime（epoch ms）。
    pub fn with_mtime(self, path: &str, mtime: f64) -> Self {
        self.mtimes
            .borrow_mut()
            .insert(path.to_string(), mtime);
        self
    }

    /// 同时设置文件内容和 mtime。
    pub fn with_file_and_mtime(self, path: &str, content: &str, mtime: f64) -> Self {
        self.files
            .borrow_mut()
            .insert(path.to_string(), content.to_string());
        self.mtimes
            .borrow_mut()
            .insert(path.to_string(), mtime);
        self
    }

    /// 获取 mock 文件系统中的文件内容（用于测试断言）。
    pub fn get_file(&self, path: &str) -> Option<String> {
        self.files.borrow().get(path).cloned()
    }
}

#[cfg(test)]
impl VaultFs for MockVaultFs {
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError> {
        self.files
            .borrow()
            .get(relative_path)
            .cloned()
            .ok_or_else(|| KeysightError::FileError(format!("文件不存在: {relative_path}")))
    }

    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError> {
        self.files
            .borrow_mut()
            .insert(relative_path.to_string(), content.to_string());
        Ok(())
    }

    fn list_md_files(&self, subdir: &str) -> Result<Vec<(String, f64)>, KeysightError> {
        let prefix = format!("{subdir}/");
        let files = self.files.borrow();
        let mtimes = self.mtimes.borrow();
        let results = files
            .keys()
            .filter(|k| k.starts_with(&prefix) && k.ends_with(".md"))
            .map(|k| {
                let mtime = mtimes.get(k.as_str()).copied().unwrap_or(1000.0);
                (k.clone(), mtime)
            })
            .collect();
        Ok(results)
    }
}
