#![allow(dead_code)]

use std::path::Path;

use super::errors::KeysightError;

/// 文件系统操作契约 — 测试时用 mock 替代真实文件系统。
pub(super) trait VaultFs {
    /// 读取 vault 内相对路径的文件内容。
    fn read_file(&self, relative_path: &str) -> Result<String, KeysightError>;
    /// 写入 vault 内相对路径的文件内容。
    fn write_file(&self, relative_path: &str, content: &str) -> Result<(), KeysightError>;
}

/// 真实文件系统实现。
pub(super) struct RealVaultFs {
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
}

/// 测试用 mock 文件系统。
#[cfg(test)]
pub(super) struct MockVaultFs {
    files: std::cell::RefCell<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
impl MockVaultFs {
    pub fn new() -> Self {
        Self {
            files: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.files
            .borrow_mut()
            .insert(path.to_string(), content.to_string());
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
}
