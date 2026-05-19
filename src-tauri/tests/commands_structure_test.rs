//! task_a60ceca2 W3 close — commands/ 子目录拆解的结构断言测试。
//!
//! 验证原 commands.rs(1538 行 / 58 cmd / 13 段)的拆解契约不被回归:
//! 1. 11 entity 子文件存在
//! 2. mod.rs 用 `pub use {entity}::*;` 暴露平铺接口(每文件 1 行)
//! 3. 全 commands/ 累计 `#[tauri::command]` 出现 58 次
//!
//! 拆解后 lib.rs `collect_commands![]` / src/bindings.ts 零变更是 spec 横切硬 Gate,
//! 由 `cargo test export_bindings` + `git diff src/bindings.ts` 在 CI / pre-commit
//! 阶段独立保证,本测试只负责文件结构与暴露面。

use std::collections::HashSet;
use std::path::PathBuf;

/// 期望的 11 个 entity 子文件名(W3 终态)。
const EXPECTED_ENTITY_FILES: &[&str] = &[
    "alias.rs",
    "card.rs",
    "entity_graph.rs",
    "layout.rs",
    "legacy_import.rs",
    "note.rs",
    "overview.rs",
    "question.rs",
    "section.rs",
    "sync.rs",
    "task.rs",
];

/// 原 commands.rs 的 `#[tauri::command]` 计数(13 段累计)。本数字在
/// task_a60ceca2 拆解前为 58,拆解后应继续等于 58(只允许全 task 范围内 cmd
/// 增减,触发本断言相应调整)。
const EXPECTED_TAURI_COMMAND_COUNT: usize = 58;

fn commands_dir() -> PathBuf {
    PathBuf::from("src/modules/keysight/commands")
}

#[test]
fn test_commands_dir_contains_expected_entity_files() {
    let dir = commands_dir();
    let mut actual: HashSet<String> = HashSet::new();
    for entry in std::fs::read_dir(&dir).expect("commands/ 目录未找到 —— CWD 应是 src-tauri/") {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .expect("file name utf-8")
                .to_string();
            // mod.rs 不算 entity 子文件,跳过
            if name != "mod.rs" {
                actual.insert(name);
            }
        }
    }
    let expected: HashSet<String> = EXPECTED_ENTITY_FILES
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        actual, expected,
        "commands/ 目录的 entity 子文件不匹配预期:\n  缺少 = {:?}\n  多余 = {:?}",
        expected.difference(&actual).collect::<Vec<_>>(),
        actual.difference(&expected).collect::<Vec<_>>(),
    );
}

#[test]
fn test_mod_rs_pub_use_covers_all_entity_files() {
    let mod_rs_path = commands_dir().join("mod.rs");
    let mod_rs = std::fs::read_to_string(&mod_rs_path).expect("commands/mod.rs 读取失败");
    for entity_file in EXPECTED_ENTITY_FILES {
        let entity_stem = entity_file.trim_end_matches(".rs");
        let expected_decl = format!("mod {};", entity_stem);
        let expected_use = format!("pub use {}::*;", entity_stem);
        assert!(
            mod_rs.contains(&expected_decl),
            "mod.rs 缺少 `{}` 声明",
            expected_decl
        );
        assert!(
            mod_rs.contains(&expected_use),
            "mod.rs 缺少 `{}`(让外部 modules::keysight::commands::xxx 路径零变更)",
            expected_use
        );
    }
}

#[test]
fn test_total_tauri_command_count_unchanged() {
    let dir = commands_dir();
    let mut count = 0usize;
    for entry in std::fs::read_dir(&dir).expect("commands/ 目录未找到") {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let src = std::fs::read_to_string(&path).expect("read .rs");
            for line in src.lines() {
                if line.trim() == "#[tauri::command]" {
                    count += 1;
                }
            }
        }
    }
    assert_eq!(
        count, EXPECTED_TAURI_COMMAND_COUNT,
        "commands/ 子目录累计 #[tauri::command] 出现次数 = {},预期 = {}。\
         拆解 task 应保持总 cmd 数不变;如有意增减,请同步更新 EXPECTED_TAURI_COMMAND_COUNT。",
        count, EXPECTED_TAURI_COMMAND_COUNT
    );
}

#[test]
fn test_no_allow_dead_code_or_unused_imports_in_commands_dir() {
    let dir = commands_dir();
    for entry in std::fs::read_dir(&dir).expect("commands/ 目录未找到") {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let src = std::fs::read_to_string(&path).expect("read .rs");
            for (i, line) in src.lines().enumerate() {
                let t = line.trim();
                assert!(
                    !t.starts_with("#[allow(dead_code)"),
                    "{} L{} 含 `#[allow(dead_code)]`,拆解 task 禁止抑制 warning",
                    path.display(),
                    i + 1
                );
                assert!(
                    !t.starts_with("#[allow(unused_imports)"),
                    "{} L{} 含 `#[allow(unused_imports)]`,拆解 task 禁止抑制 warning",
                    path.display(),
                    i + 1
                );
                assert!(
                    !t.starts_with("#![allow("),
                    "{} L{} 含 `#![allow(...)]` 模块级抑制,拆解 task 禁止",
                    path.display(),
                    i + 1
                );
            }
        }
    }
}
