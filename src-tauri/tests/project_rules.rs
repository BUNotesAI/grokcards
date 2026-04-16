//! 项目级质量规则的机械验证 test
//!
//! 实施 `vault/docs/project.spec.md` 定义的两条 MVP 规则:
//!
//! - R1: 每个 `#[tauri::command]` 必带 `#[specta::specta]`(IPC 类型安全生命线)
//! - R2: `src-tauri/src/modules/**` 与 `keysight-core/src/**` 禁用 `.unwrap()` / `.expect()`,
//!   例外需 `// 例外:` 注释。business 代码在 task_dd9e57db Phase 1 里从 src-tauri
//!   抽到 keysight-core workspace crate,R2 覆盖面跟随扩展
//!
//! 核心 fn `check_r1_specta_annotation` 与 `check_r2_no_unwrap_in_business` 同时被
//! happy test(扫描真实 codebase)和 error test(内联 fixture 字符串)共享,满足
//! anti-test-theater 原则(测试调的是真实实现,不是复制出来的副本)。

use std::path::{Path, PathBuf};

use syn::{parse_file, Attribute, Item, ItemFn};
use walkdir::WalkDir;

// =============================================================================
// 类型建模(L0 防火墙)
// =============================================================================

/// 源代码里的行号,newtype 避免和 column / offset 混用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineNumber(pub u32);

/// R2 检测出的违规类型 — 消除 `is_unwrap: bool` 的 boolean blindness。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnwrapKind {
    Unwrap,
    Expect,
}

/// R1 违规:某 `#[tauri::command]` 函数缺失 `#[specta::specta]` 标注。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct R1Violation {
    pub file: PathBuf,
    pub fn_name: String,
    pub line: LineNumber,
}

/// R2 违规:某行出现裸 `.unwrap()` 或 `.expect()` 且上方没有 `// 例外:` 注释。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct R2Violation {
    pub file: PathBuf,
    pub line: LineNumber,
    pub kind: UnwrapKind,
}

// =============================================================================
// R1 核心 fn —— 检查 #[tauri::command] 必带 #[specta::specta]
// =============================================================================

/// 扫描一段 Rust 源码字符串,返回第一个缺失 `#[specta::specta]` 的 `#[tauri::command]`
/// 函数违规;若全部合规则返回 `Ok(())`。
///
/// # 参数
/// - `source`: Rust 源文件内容(内联字符串或真实文件读入结果均可)
/// - `file_for_error`: 用于填入 `R1Violation.file` 的 path(happy test 传真实 path,
///   error test 传假名即可)
pub fn check_r1_specta_annotation(
    source: &str,
    file_for_error: &Path,
) -> Result<(), R1Violation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        // 例外: 解析失败说明源文件本身不是合法 Rust,不属于 R1 的职责范围;
        // 返回 Ok 让调用方依赖 cargo check / clippy 去发现语法错误。
        Err(_) => return Ok(()),
    };

    for item in ast.items {
        if let Item::Fn(item_fn) = item
            && has_tauri_command_attr(&item_fn.attrs)
            && !has_specta_attr(&item_fn.attrs)
        {
            return Err(R1Violation {
                file: file_for_error.to_path_buf(),
                fn_name: item_fn.sig.ident.to_string(),
                line: LineNumber(line_of(&item_fn)),
            });
        }
    }
    Ok(())
}

fn has_tauri_command_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| attr_path_matches(a, &["tauri", "command"]))
}

fn has_specta_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| attr_path_matches(a, &["specta", "specta"]))
}

/// 判断某 attribute 的路径是否等于指定路径(支持 `tauri::command` / `command` 两种写法)。
fn attr_path_matches(attr: &Attribute, target: &[&str]) -> bool {
    let segs: Vec<String> = attr
        .path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    // 完整匹配 tauri::command
    if segs.len() == target.len()
        && segs.iter().zip(target.iter()).all(|(a, b)| a == *b)
    {
        return true;
    }
    // 允许单段写法 command(已 use tauri::command) / specta(已 use specta::specta)
    if segs.len() == 1 && target.last().is_some_and(|t| segs[0] == *t) {
        return true;
    }
    false
}

fn line_of(item: &ItemFn) -> u32 {
    // syn 在 stable 下不直接暴露 line,只能 proc_macro2 的 span.start().line
    // 本工具链测试级,精度足够
    use proc_macro2::Span;
    let span: Span = item.sig.fn_token.span;
    span.start().line as u32
}

// =============================================================================
// walk_modules_dir —— I/O helper,仅 happy test 用
// =============================================================================

/// 返回 `src-tauri/src/modules/` 下所有 `.rs` 文件的 (path, content) 对。
/// 路径相对于 test 执行目录(cargo 设 CWD 为 `src-tauri/`)。
pub fn walk_modules_dir() -> Vec<(PathBuf, String)> {
    walk_rs_files(Path::new("src/modules"))
}

/// 返回 `keysight-core/src/` 下所有 `.rs` 文件的 (path, content) 对。
/// 路径相对于 src-tauri CWD — 向上走到 workspace root 下的 keysight-core。
/// task_dd9e57db Phase 1 之后,keysight 业务代码从 `src-tauri/src/modules/keysight/`
/// 抽到 `keysight-core/src/`,R2 扫描范围同步扩展,保持"业务代码无裸 unwrap"的全局不变量。
pub fn walk_keysight_core_dir() -> Vec<(PathBuf, String)> {
    walk_rs_files(Path::new("../keysight-core/src"))
}

/// R2 所覆盖的全部业务模块文件:src-tauri/src/modules/** + keysight-core/src/**。
/// R1(`#[tauri::command]` + `#[specta::specta]`)仅适用于 src-tauri/src/modules/,
/// 因为 tauri::command 宏只会出现在 Tauri app crate 里。
pub fn walk_business_modules() -> Vec<(PathBuf, String)> {
    let mut out = walk_modules_dir();
    out.extend(walk_keysight_core_dir());
    out
}

/// 给定 base 目录(相对 CWD),递归返回所有 `.rs` 文件的 (path, content) 对。
/// base 不存在时返回空,避免测试在新环境下硬崩。
fn walk_rs_files(base: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    if !base.exists() {
        return out;
    }
    for entry in WalkDir::new(base).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "rs") {
            let path = entry.path().to_path_buf();
            // 例外: test helper 读文件失败直接 panic,让测试立即显示 setup 问题。
            let content = std::fs::read_to_string(&path).unwrap();
            out.push((path, content));
        }
    }
    out
}

/// 仅返回 `src/modules/*/commands.rs` 文件(R1 只扫 command 定义处)。
fn walk_commands_files() -> Vec<(PathBuf, String)> {
    walk_modules_dir()
        .into_iter()
        .filter(|(p, _)| p.file_name().is_some_and(|n| n == "commands.rs"))
        .collect()
}

// =============================================================================
// R1 Tests —— H1 happy + E1 error
// =============================================================================

#[test]
fn test_all_tauri_commands_have_specta_annotation() {
    let files = walk_commands_files();
    assert!(!files.is_empty(), "未找到任何 commands.rs 文件 —— 检查 CWD 是否为 src-tauri/");

    let mut violations = Vec::new();
    for (path, source) in files {
        if let Err(v) = check_r1_specta_annotation(&source, &path) {
            violations.push(v);
        }
    }

    assert!(
        violations.is_empty(),
        "发现 {} 条 R1 违规(缺失 #[specta::specta]):{:#?}",
        violations.len(),
        violations
    );
}

#[test]
fn test_r1_detects_command_without_specta() {
    let source = r#"
        use tauri::State;

        #[tauri::command]
        pub fn bad_command(state: State<()>) -> Result<String, String> {
            Ok("oops".to_string())
        }

        #[tauri::command]
        #[specta::specta]
        pub fn good_command() -> Result<String, String> {
            Ok("ok".to_string())
        }
    "#;

    let result = check_r1_specta_annotation(source, Path::new("fixture.rs"));
    let violation = result.expect_err("应当检测到 bad_command 缺失 specta,但返回了 Ok");
    assert_eq!(violation.fn_name, "bad_command");
}

// =============================================================================
// R2 核心 fn —— 检查 .unwrap() / .expect() 必须有 // 例外: 注释
// =============================================================================

/// 逐行扫描源码字符串,返回所有缺失 `// 例外:` 注释的 `.unwrap()` / `.expect()` 违规。
///
/// # 规则
/// - 匹配 `.unwrap()` 或 `.expect(` 的行
/// - 上一行（trimmed）以 `// 例外:` 开头则豁免
/// - `#[cfg(test)]` 块内的代码跳过不检查
pub fn check_r2_no_unwrap_in_business(
    source: &str,
    file_for_error: &Path,
) -> Result<(), Vec<R2Violation>> {
    let lines: Vec<&str> = source.lines().collect();
    let mut violations = Vec::new();
    let mut in_test_block = false;
    let mut brace_depth: i32 = 0;
    let mut test_block_start_depth: i32 = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // 追踪 #[cfg(test)] 块
        if trimmed == "#[cfg(test)]" {
            in_test_block = true;
            test_block_start_depth = brace_depth;
        }

        // 追踪大括号深度
        for ch in trimmed.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth -= 1;
                    if in_test_block && brace_depth <= test_block_start_depth {
                        in_test_block = false;
                    }
                }
                _ => {}
            }
        }

        if in_test_block {
            continue;
        }

        // 检查 .unwrap() 和 .expect(
        let has_unwrap = trimmed.contains(".unwrap()");
        let has_expect = trimmed.contains(".expect(");

        if !has_unwrap && !has_expect {
            continue;
        }

        // 检查上一行是否有 // 例外: 注释
        let has_exception = i > 0 && lines[i - 1].trim().starts_with("// 例外:");

        if !has_exception {
            let kind = if has_unwrap {
                UnwrapKind::Unwrap
            } else {
                UnwrapKind::Expect
            };
            violations.push(R2Violation {
                file: file_for_error.to_path_buf(),
                line: LineNumber((i + 1) as u32),
                kind,
            });
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

// =============================================================================
// R2 Tests —— H2 happy + E2/E3 error
// =============================================================================

#[test]
fn test_no_unwrap_in_business_modules() {
    let files = walk_business_modules();
    assert!(
        !files.is_empty(),
        "未找到任何业务 .rs 文件 —— 检查 CWD 是否为 src-tauri/ 且 keysight-core 存在"
    );

    let mut all_violations = Vec::new();
    for (path, source) in files {
        if let Err(vs) = check_r2_no_unwrap_in_business(&source, &path) {
            all_violations.extend(vs);
        }
    }

    assert!(
        all_violations.is_empty(),
        "发现 {} 条 R2 违规(裸 .unwrap()/.expect() 缺 // 例外: 注释):\n{:#?}",
        all_violations.len(),
        all_violations
    );
}

#[test]
fn test_r2_detects_bare_unwrap() {
    let source = r#"
fn some_function() {
    let conn = state.db.lock().unwrap();
    let value = result.expect("should work");
}
"#;

    let result = check_r2_no_unwrap_in_business(source, Path::new("fixture.rs"));
    let violations = result.expect_err("应当检测到裸 unwrap/expect,但返回了 Ok");
    assert!(violations.len() >= 2, "应至少检测到 2 条违规,实际 {}", violations.len());
    assert_eq!(violations[0].kind, UnwrapKind::Unwrap);
    assert_eq!(violations[1].kind, UnwrapKind::Expect);
}

#[test]
fn test_r2_allows_unwrap_with_exception_comment() {
    let source = r#"
fn some_function() {
    // 例外: Mutex poisoning unrecoverable
    let conn = state.db.lock().unwrap();
    // 例外: 配置加载失败不可恢复
    let cfg = config.expect("config must exist");
}
"#;

    let result = check_r2_no_unwrap_in_business(source, Path::new("fixture.rs"));
    assert!(result.is_ok(), "带 // 例外: 注释的 unwrap 不应被报告为违规");
}
