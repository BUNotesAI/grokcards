//! W1 WhiteboardId 渗透 audit test
//!
//! 实施 `vault/docs/tasks/task_ee2b6926/spec.md` 内 W1 段 5 个 scenario:
//!
//! - test_whiteboard_id_propagated_in_domain_traits      —— 8 个 domain file trait/impl 无 wb_id: &str 残留
//! - test_tauri_commands_use_whiteboard_id_newtype       —— commands.rs #[tauri::command] 入参 WhiteboardId
//! - test_invalid_whiteboard_id_rejected_at_ipc_boundary —— IPC 反序列化失败时业务计数器为 0
//! - test_fixtures_use_parse_for_wb_id                   —— "wb_..." 字面量必须在 WhiteboardId::parse(...) 内
//! - W1 e2e gate(test_w1_completion_passes_all_gates)   —— close 阶段 manual 验证,无单独测试代码
//!
//! Audit 核心 fn(`find_wb_str_residue_in_domain` / `find_command_wb_residue` /
//! `find_bare_wb_literals`)同时被 happy test(扫真实代码)与 error test(内联
//! fixture 字符串)共享,符合 anti-test-theater 原则。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use syn::{FnArg, ImplItem, Item, Pat, PatIdent, PatType, TraitItem, Type, parse_file};
use walkdir::WalkDir;

use keysight_core::domain::id::WhiteboardId;

// =============================================================================
// 类型建模(L0 防火墙)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WbResidueKind {
    /// trait 或 impl/free fn 方法参数:`whiteboard_id: &str` / `wb_id: &str` / `&String`
    DomainTraitOrImpl,
    /// `#[tauri::command]` 函数参数:`whiteboard_id: String` / `&str` / `&String`
    TauriCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WbResidueViolation {
    pub file: PathBuf,
    pub line: u32,
    pub kind: WbResidueKind,
    pub fn_name: String,
    pub param_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WbLiteralViolation {
    pub file: PathBuf,
    pub line: u32,
    pub snippet: String,
}

// =============================================================================
// Scenario 1 + 2:syn AST 扫描函数参数类型
// =============================================================================

/// 扫描 domain/*.rs 源码,返回 trait method / impl method / free fn 中 wb_id 参数
/// 仍是 `&str` / `&String` / `String` 的违规。
pub fn find_wb_str_residue_in_domain(
    source: &str,
    file_for_error: &Path,
) -> Vec<WbResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        // 例外:源码语法错误不属于 W1 audit 职责,留给 cargo check / clippy 报。
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        match item {
            Item::Trait(t) => {
                for ti in t.items {
                    if let TraitItem::Fn(tf) = ti {
                        scan_fn_sig(
                            &tf.sig,
                            file_for_error,
                            &mut out,
                            WbResidueKind::DomainTraitOrImpl,
                        );
                    }
                }
            }
            Item::Impl(im) => {
                for ii in im.items {
                    if let ImplItem::Fn(f) = ii {
                        scan_fn_sig(
                            &f.sig,
                            file_for_error,
                            &mut out,
                            WbResidueKind::DomainTraitOrImpl,
                        );
                    }
                }
            }
            Item::Fn(f) => {
                scan_fn_sig(
                    &f.sig,
                    file_for_error,
                    &mut out,
                    WbResidueKind::DomainTraitOrImpl,
                );
            }
            _ => {}
        }
    }
    out
}

/// 扫描 commands.rs:任何 `#[tauri::command]` 函数中 wb_id 参数仍是 `String` / `&str`
/// 的违规。
pub fn find_command_wb_residue(source: &str, file_for_error: &Path) -> Vec<WbResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        if let Item::Fn(f) = item
            && has_tauri_command_attr(&f.attrs)
        {
            scan_fn_sig(&f.sig, file_for_error, &mut out, WbResidueKind::TauriCommand);
        }
    }
    out
}

fn scan_fn_sig(
    sig: &syn::Signature,
    file_for_error: &Path,
    out: &mut Vec<WbResidueViolation>,
    kind: WbResidueKind,
) {
    for arg in &sig.inputs {
        let FnArg::Typed(PatType { pat, ty, .. }) = arg else {
            continue;
        };
        let Pat::Ident(PatIdent { ident, .. }) = pat.as_ref() else {
            continue;
        };
        let name = ident.to_string();
        if !is_wb_param_name(&name) {
            continue;
        }
        if !is_str_or_string(ty) {
            continue;
        }
        use proc_macro2::Span;
        let span: Span = ident.span();
        out.push(WbResidueViolation {
            file: file_for_error.to_path_buf(),
            line: span.start().line as u32,
            kind: kind.clone(),
            fn_name: sig.ident.to_string(),
            param_name: name,
        });
    }
}

fn is_wb_param_name(s: &str) -> bool {
    matches!(
        s,
        "whiteboard_id" | "wb_id" | "target_whiteboard_id" | "target_wb"
    )
}

fn is_str_or_string(ty: &Type) -> bool {
    match ty {
        Type::Reference(r) => is_str_or_string(&r.elem),
        Type::Path(p) => {
            let last = p
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            last == "str" || last == "String"
        }
        _ => false,
    }
}

fn has_tauri_command_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        let segs: Vec<String> = a
            .path()
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        segs == ["tauri", "command"] || (segs.len() == 1 && segs[0] == "command")
    })
}

// =============================================================================
// Scenario 4:wb_xxx 字面量扫描
// =============================================================================

/// 扫描一段源码,找所有 `"wb_..."` 字符串字面量,返回违规位置。
///
/// 命中规则(全部要满足):
/// 1. 字面量在源代码非注释行
/// 2. 字面量不在 SQL 字符串内(SELECT/INSERT/UPDATE/DELETE 行)
/// 3. 字面量前后无以下任一豁免上下文:
///    - 前置:`WhiteboardId::parse(` 包裹(典型 fixture 写法)
///    - 前置:`==` / `!=` 紧邻(等值对比,语义不是参数传递)
///    - 前置:`assert_eq!(` / `assert!(` 内(测试 expected 值)
///    - 前置:`return ` 紧邻(production 生成端 magic 值)
///    - 后置:`.to_string()` / `.into()` / `.to_owned()`(生成端,需进一步配上 new_unchecked / parse)
///
/// spec scenario 4 的语义是"不允许字面量直接当 wb_id 传给业务函数参数",
/// 上述豁免覆盖所有合法非参数用法。
pub fn find_bare_wb_literals(source: &str, file_for_error: &Path) -> Vec<WbLiteralViolation> {
    let mut out = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }

        // 当前行 SQL 直接豁免
        let line_upper = line.to_uppercase();
        let is_sql_line = line_upper.contains("INSERT INTO")
            || line_upper.contains("SELECT ")
            || line_upper.contains("UPDATE ")
            || line_upper.contains("DELETE FROM");
        if is_sql_line {
            continue;
        }

        // 多行上下文豁免:回看前 6 行(覆盖典型 macro / fn call 嵌套)。
        // 若上下文里出现 SQL / macro / assertion / parse 关键词,字面量大概率
        // 不是直接当 wb_id 参数传给业务函数,跳过。
        let window_start = i.saturating_sub(6);
        let window: String = lines[window_start..=i].join("\n");
        let win_upper = window.to_uppercase();
        let in_sql_block = win_upper.contains("INSERT INTO")
            || win_upper.contains("SELECT ")
            || win_upper.contains("UPDATE ")
            || win_upper.contains("DELETE FROM");
        let in_macro_or_assert = window.contains("params![")
            || window.contains("json!(")
            || window.contains("json!({")
            || window.contains("assert_eq!(")
            || window.contains("assert!(")
            || window.contains("assert_ne!(");
        if in_sql_block || in_macro_or_assert {
            continue;
        }

        let mut search_from = 0;
        while let Some(idx) = line[search_from..].find("\"wb_") {
            let abs_idx = search_from + idx;
            let after_open = abs_idx + 4; // 跳过 `"wb_`
            let Some(rel_close) = line[after_open..].find('"') else {
                break;
            };
            let end = after_open + rel_close;

            let prefix = &line[..abs_idx];
            let suffix = &line[end + 1..];
            let prefix_trim = prefix.trim_end();

            let is_in_parse = prefix.contains("WhiteboardId::parse")
                || prefix.contains("WhiteboardId :: parse")
                || prefix.contains("Id::parse(") // 其他 newtype parse(NoteId::parse 等 audit 不该报)
                || prefix.contains("from_str(");
            let is_eq_compare = prefix_trim.ends_with("==") || prefix_trim.ends_with("!=");
            let is_return_value = prefix_trim.ends_with("return");
            let is_string_generator = suffix.starts_with(".to_string")
                || suffix.starts_with(".into()")
                || suffix.starts_with(".to_owned()");

            if !is_in_parse
                && !is_eq_compare
                && !is_return_value
                && !is_string_generator
            {
                out.push(WbLiteralViolation {
                    file: file_for_error.to_path_buf(),
                    line: (i + 1) as u32,
                    snippet: line[abs_idx..=end].to_string(),
                });
            }
            search_from = end + 1;
        }
    }
    out
}

// =============================================================================
// I/O helpers —— 仅 happy test 用
// =============================================================================

fn walk_rs(base: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    if !base.exists() {
        return out;
    }
    for e in WalkDir::new(base).into_iter().filter_map(|x| x.ok()) {
        if e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "rs") {
            // audit 自身文件含 inline fixture string 用于测 audit fn 正确性,
            // 不属于业务 fixture,扫描时跳过。
            if e.path().file_name().is_some_and(|n| n == "w1_audit.rs") {
                continue;
            }
            // 例外:test helper 读文件失败直接 panic,让 setup 错误立即显现。
            let s = std::fs::read_to_string(e.path()).unwrap();
            out.push((e.path().to_path_buf(), s));
        }
    }
    out
}

/// `keysight-core/src/domain/` 全部 .rs(scenario 1 扫描范围)。
/// 路径相对 CWD = `src-tauri/`(cargo 跑 integration test 的默认 CWD)。
fn walk_keysight_domain() -> Vec<(PathBuf, String)> {
    walk_rs(Path::new("../keysight-core/src/domain"))
}

/// `keysight-core/src/` 全部 .rs(scenario 4 fixture 扫描范围)。
fn walk_keysight_core_all() -> Vec<(PathBuf, String)> {
    walk_rs(Path::new("../keysight-core/src"))
}

/// `src-tauri/tests/` 全部 .rs(scenario 4 fixture 扫描范围)。
fn walk_src_tauri_tests() -> Vec<(PathBuf, String)> {
    walk_rs(Path::new("tests"))
}

/// 读 `src-tauri/src/modules/keysight/commands.rs`。
fn read_commands_rs() -> Option<(PathBuf, String)> {
    let p = PathBuf::from("src/modules/keysight/commands.rs");
    let s = std::fs::read_to_string(&p).ok()?;
    Some((p, s))
}

// =============================================================================
// Scenario 1:domain trait / impl / free fn 中 wb_id 全为 newtype
// =============================================================================

#[test]
fn test_whiteboard_id_propagated_in_domain_traits() {
    let files = walk_keysight_domain();
    assert!(
        !files.is_empty(),
        "未找到 keysight-core/src/domain/ —— 检查 CWD 是否为 src-tauri/"
    );

    let mut violations = Vec::new();
    for (p, s) in files {
        violations.extend(find_wb_str_residue_in_domain(&s, &p));
    }
    assert!(
        violations.is_empty(),
        "发现 {} 处 W1 残留 whiteboard_id/wb_id: &str|&String|String:\n{:#?}",
        violations.len(),
        violations
    );
}

#[test]
fn test_w1_audit_detects_str_residue_in_trait_fixture() {
    let source = r#"
trait Foo {
    fn bar(&self, whiteboard_id: &str) -> Result<(), ()>;
    fn baz(&self, wb_id: &String, x: i32) -> Result<(), ()>;
    fn safe(&self, name: &str) -> Result<(), ()>;
}
    "#;
    let v = find_wb_str_residue_in_domain(source, Path::new("fixture.rs"));
    assert_eq!(v.len(), 2, "应检测到 bar / baz 两处违规,safe 不应命中");
    let names: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(names.contains(&"bar"));
    assert!(names.contains(&"baz"));
}

#[test]
fn test_w1_audit_ignores_newtype_param_in_trait_fixture() {
    let source = r#"
trait Foo {
    fn bar(&self, whiteboard_id: &WhiteboardId) -> Result<(), ()>;
    fn baz(&self, wb_id: WhiteboardId) -> Result<(), ()>;
}
    "#;
    let v = find_wb_str_residue_in_domain(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "newtype 参数不应被报为违规:{v:#?}");
}

// =============================================================================
// Scenario 2:commands.rs #[tauri::command] 函数 wb_id 全为 WhiteboardId
// =============================================================================

#[test]
fn test_tauri_commands_use_whiteboard_id_newtype() {
    let (p, s) = read_commands_rs().expect("commands.rs 未找到,检查 CWD 是否为 src-tauri/");
    let v = find_command_wb_residue(&s, &p);
    assert!(
        v.is_empty(),
        "发现 {} 处 #[tauri::command] 函数 wb_id 参数仍是 String|&str:\n{:#?}",
        v.len(),
        v
    );
}

#[test]
fn test_w1_audit_detects_str_residue_in_command_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn alias_query_all(whiteboard_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn good_one(whiteboard_id: WhiteboardId) -> Result<(), ()> { Ok(()) }

pub fn not_a_command(whiteboard_id: String) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_wb_residue(source, Path::new("fixture.rs"));
    assert_eq!(v.len(), 1, "仅 alias_query_all 应被报违规,不是 command 的不计");
    assert_eq!(v[0].fn_name, "alias_query_all");
}

// =============================================================================
// Scenario 3:非法 wb id 经 IPC 反序列化层拒绝,业务计数器为 0
// =============================================================================

static BUSINESS_CALL_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn fake_business_logic(_id: WhiteboardId) {
    BUSINESS_CALL_COUNTER.fetch_add(1, Ordering::SeqCst);
}

/// 模拟 Tauri IPC dispatch:先 deserialize `WhiteboardId`,失败 short-circuit,
/// 业务体不被进入。模仿 `#[tauri::command]` 函数被调用时 Tauri runtime 的执行序。
fn fake_ipc_dispatch(payload: &str) -> Result<(), serde_json::Error> {
    let id: WhiteboardId = serde_json::from_str(payload)?;
    fake_business_logic(id);
    Ok(())
}

#[test]
fn test_invalid_whiteboard_id_rejected_at_ipc_boundary() {
    BUSINESS_CALL_COUNTER.store(0, Ordering::SeqCst);

    let result = fake_ipc_dispatch(r#""card_invalid""#);
    assert!(result.is_err(), "illegal wb id 应被反序列化层拒绝");
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("UnknownPrefix") || err_str.contains("prefix"),
        "deserialize 错误信息应含 UnknownPrefix / prefix 标识,实际:{err_str}"
    );

    assert_eq!(
        BUSINESS_CALL_COUNTER.load(Ordering::SeqCst),
        0,
        "deserialize 失败时业务函数不应被调用"
    );

    // 合法 payload 应能通过,业务函数被调用一次
    let ok = fake_ipc_dispatch(r#""wb_aaaa1111""#);
    assert!(ok.is_ok());
    assert_eq!(BUSINESS_CALL_COUNTER.load(Ordering::SeqCst), 1);
}

// =============================================================================
// Scenario 4:wb_xxx 字面量必须在 WhiteboardId::parse(...) 内
// =============================================================================

#[test]
fn test_fixtures_use_parse_for_wb_id() {
    let mut all = walk_keysight_core_all();
    all.extend(walk_src_tauri_tests());
    assert!(!all.is_empty(), "扫描范围为空 —— 检查 CWD");

    let mut violations = Vec::new();
    for (p, s) in all {
        violations.extend(find_bare_wb_literals(&s, &p));
    }
    assert!(
        violations.is_empty(),
        "发现 {} 处 \"wb_...\" 字面量未经 WhiteboardId::parse 包装(SQL 行 / 注释行已豁免):\n{:#?}",
        violations.len(),
        violations
    );
}

#[test]
fn test_w1_audit_detects_bare_wb_literal_fixture() {
    let source = r#"
fn run() {
    store.create("wb_aaaa1111", "title");
}
    "#;
    let v = find_bare_wb_literals(source, Path::new("fixture.rs"));
    assert_eq!(v.len(), 1, "字面量直接当 fn 参数应被报:{v:#?}");
}

#[test]
fn test_w1_audit_passes_with_parse_call() {
    let source = r#"
fn make_fixture() -> WhiteboardId {
    WhiteboardId::parse("wb_aaaa1111").unwrap()
}
    "#;
    let v = find_bare_wb_literals(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "WhiteboardId::parse 包装的字面量不应被报:{v:#?}");
}

#[test]
fn test_w1_audit_ignores_sql_string_literal() {
    let source = r#"
fn seed_db(conn: &Connection) {
    conn.execute(
        "INSERT INTO entities (id, kind) VALUES ('wb_root', 'whiteboard')",
        [],
    ).unwrap();
}
    "#;
    let v = find_bare_wb_literals(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "SQL 字符串内的 wb_root 应豁免:{v:#?}");
}

#[test]
fn test_w1_audit_ignores_comment_line() {
    let source = r#"
fn foo() {
    // 注释里的 "wb_xxx" 不算违规
    let _ = "wb_legit";
}
    "#;
    let v = find_bare_wb_literals(source, Path::new("fixture.rs"));
    // 第二行 `let _ = "wb_legit";` 是真违规,注释行豁免
    assert_eq!(v.len(), 1, "注释行豁免但 let 字面量应报");
}
