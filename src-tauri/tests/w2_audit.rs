//! W2 CardId 渗透 audit test
//!
//! 实施 `vault/docs/tasks/task_ee2b6926/spec.md` 内 W2 段 3 个 scenario:
//!
//! - test_card_id_propagated_in_card_layer   —— card.rs trait+impl+helper 与
//!   dispatcher.rs op_alias_create 内 card id 角色参数为 CardId
//! - test_card_tauri_commands_use_card_id    —— commands.rs `card_*` 命令 +
//!   含 `card_id|from_card_id|to_card_id` 参数的命令,card id 角色参数为 CardId
//! - W2 e2e gate(test_w2_completion_passes_all_gates) —— close 阶段 manual
//!   验证 5 项 gate,无单独测试代码
//!
//! Audit 核心 fn(`find_card_str_residue_in_card_rs` /
//! `find_card_str_residue_in_dispatcher` / `find_command_card_residue`)
//! 同时被 happy test(扫真实代码)与 error test(内联 fixture 字符串)共享,
//! 符合 anti-test-theater 原则。
//!
//! 与 W1 的差异:
//!
//! 1. **参数名 list**:W1 命中 `whiteboard_id|wb_id|target_*`;W2 命中
//!    `id|ids|card_id|card_ids|from_card_id|to_card_id`(`id` 单字仅在
//!    card.rs 内 scope,角色明确)。
//! 2. **类型 list**:W2 额外覆盖 `&[String]` 与 `Vec<String>`(`query_by_ids` /
//!    `batch_load_tags` / `batch_load_edges` / `sort_cards_by_id_order` 等
//!    helper 入参)。
//! 3. **dispatcher 扫描精确到 op_alias_create**:dispatcher 里没有 op_card_*,
//!    op_alias_create 因含 `card_id: &str` 入参被纳入。

use std::path::{Path, PathBuf};

use syn::{FnArg, ImplItem, Item, Pat, PatIdent, PatType, TraitItem, Type, parse_file};

// =============================================================================
// 类型建模(L0 防火墙)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardResidueKind {
    /// `card.rs` 内 trait 方法 / impl 方法 / free fn 中 card id 参数残留
    CardDomain,
    /// `dispatcher.rs` 内 op_alias_create / 任何带 card_id 入参的 op_* 残留
    DispatcherOp,
    /// `commands.rs` 内 `#[tauri::command]` 函数中 card id 参数残留
    TauriCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardResidueViolation {
    pub file: PathBuf,
    pub line: u32,
    pub kind: CardResidueKind,
    pub fn_name: String,
    pub param_name: String,
}

// =============================================================================
// 参数名 / 类型识别
// =============================================================================

/// card.rs 内命中名:card id 角色的所有参数命名。
///
/// 含 `id` 单字 —— 因为 CardStore trait 方法的 `id: &str` 语义显然是 card_id
/// (self 是 CardStore,角色由 self 限定)。
fn is_card_param_name_in_card_rs(s: &str) -> bool {
    matches!(
        s,
        "id" | "ids" | "card_id" | "card_ids" | "from_card_id" | "to_card_id"
    )
}

/// dispatcher.rs / commands.rs 内命中名:仅含 `card_id` 前缀的命名。
///
/// **不**含 `id` 单字 —— dispatcher 里其他 op_ helper 的 `id` 可能是
/// alias_id / section_id / entity_id 等其他 newtype 的角色,W2 不能误伤。
fn is_card_id_prefixed_name(s: &str) -> bool {
    matches!(
        s,
        "card_id" | "card_ids" | "from_card_id" | "to_card_id"
    )
}

/// commands.rs 内,`card_*` 命令额外允许命中 `id` / `ids`(语义为 card id)。
fn is_card_param_name_in_card_command(s: &str) -> bool {
    is_card_id_prefixed_name(s) || matches!(s, "id" | "ids")
}

/// 判定一个 type 是否属于 str / String 家族,且语义为 card id(单值或 list)。
///
/// 命中:`&str` / `&String` / `String` / `&[String]` / `Vec<String>` /
/// `&[&str]` / `Option<String>` / `Option<&str>`。
fn is_str_family_id_type(ty: &Type) -> bool {
    match ty {
        Type::Reference(r) => is_str_family_id_type(&r.elem),
        Type::Slice(s) => is_str_family_id_type(&s.elem),
        Type::Path(p) => {
            let Some(seg) = p.path.segments.last() else {
                return false;
            };
            let name = seg.ident.to_string();
            // 简单 str/String 命中。
            if name == "str" || name == "String" {
                return true;
            }
            // Vec<String> / Option<String> 等单参容器:看泛型实参。
            if matches!(name.as_str(), "Vec" | "Option")
                && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            {
                return is_str_family_id_type(inner);
            }
            false
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
// Scenario 1a:card.rs trait/impl/free fn 中 card id 参数 全为 CardId
// =============================================================================

/// 扫描 `card.rs` 源码,返回所有 card id 角色参数仍是 str 家族的违规。
pub fn find_card_str_residue_in_card_rs(
    source: &str,
    file_for_error: &Path,
) -> Vec<CardResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        // 例外:源码语法错误不属于 W2 audit 职责,留给 cargo check / clippy 报。
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
                            CardResidueKind::CardDomain,
                            is_card_param_name_in_card_rs,
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
                            CardResidueKind::CardDomain,
                            is_card_param_name_in_card_rs,
                        );
                    }
                }
            }
            Item::Fn(f) => {
                scan_fn_sig(
                    &f.sig,
                    file_for_error,
                    &mut out,
                    CardResidueKind::CardDomain,
                    is_card_param_name_in_card_rs,
                );
            }
            _ => {}
        }
    }
    out
}

// =============================================================================
// Scenario 1b:dispatcher.rs op_alias_create 等含 card_id 参数的 op_* 全为 CardId
// =============================================================================

/// 扫描 dispatcher.rs:任何 free fn(op_*)入参命中 `card_id|card_ids|
/// from_card_id|to_card_id` 且类型为 str 家族的违规。
pub fn find_card_str_residue_in_dispatcher(
    source: &str,
    file_for_error: &Path,
) -> Vec<CardResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        if let Item::Fn(f) = item {
            scan_fn_sig(
                &f.sig,
                file_for_error,
                &mut out,
                CardResidueKind::DispatcherOp,
                is_card_id_prefixed_name,
            );
        }
    }
    out
}

// =============================================================================
// Scenario 2:commands.rs `#[tauri::command]` 函数中 card id 参数 全为 CardId
// =============================================================================

/// 扫描 commands.rs:
///
/// - 函数名以 `card_` 开头的 `#[tauri::command]`:命中 `id|ids|card_id|...`
/// - 其他 `#[tauri::command]`:命中 `card_id|card_ids|from_card_id|to_card_id`
pub fn find_command_card_residue(
    source: &str,
    file_for_error: &Path,
) -> Vec<CardResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        if let Item::Fn(f) = item
            && has_tauri_command_attr(&f.attrs)
        {
            let fn_name = f.sig.ident.to_string();
            let matcher: fn(&str) -> bool = if fn_name.starts_with("card_") {
                is_card_param_name_in_card_command
            } else {
                is_card_id_prefixed_name
            };
            scan_fn_sig(
                &f.sig,
                file_for_error,
                &mut out,
                CardResidueKind::TauriCommand,
                matcher,
            );
        }
    }
    out
}

// =============================================================================
// 公共扫描器
// =============================================================================

fn scan_fn_sig(
    sig: &syn::Signature,
    file_for_error: &Path,
    out: &mut Vec<CardResidueViolation>,
    kind: CardResidueKind,
    name_matcher: fn(&str) -> bool,
) {
    for arg in &sig.inputs {
        let FnArg::Typed(PatType { pat, ty, .. }) = arg else {
            continue;
        };
        let Pat::Ident(PatIdent { ident, .. }) = pat.as_ref() else {
            continue;
        };
        let name = ident.to_string();
        if !name_matcher(&name) {
            continue;
        }
        if !is_str_family_id_type(ty) {
            continue;
        }
        use proc_macro2::Span;
        let span: Span = ident.span();
        out.push(CardResidueViolation {
            file: file_for_error.to_path_buf(),
            line: span.start().line as u32,
            kind: kind.clone(),
            fn_name: sig.ident.to_string(),
            param_name: name,
        });
    }
}

// =============================================================================
// I/O helpers —— 仅 happy test 用
// =============================================================================

fn read_file(p: &Path) -> Option<(PathBuf, String)> {
    let s = std::fs::read_to_string(p).ok()?;
    Some((p.to_path_buf(), s))
}

fn read_card_rs() -> Option<(PathBuf, String)> {
    read_file(Path::new("../keysight-core/src/domain/card.rs"))
}

fn read_dispatcher_rs() -> Option<(PathBuf, String)> {
    read_file(Path::new("src/modules/keysight/dispatcher.rs"))
}

/// task_a60ceca2 后 commands 已拆为子目录;扫描 `src/modules/keysight/commands/`
/// 全部 `.rs` 文件,合并为单一 source 供 audit 扫描。
fn read_commands_rs() -> Option<(PathBuf, String)> {
    let dir = PathBuf::from("src/modules/keysight/commands");
    let mut combined = String::new();
    for entry in std::fs::read_dir(&dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            combined.push_str(&std::fs::read_to_string(&path).ok()?);
            combined.push('\n');
        }
    }
    if combined.is_empty() { None } else { Some((dir, combined)) }
}

// =============================================================================
// Scenario 1:card domain layer 全为 CardId
// =============================================================================

#[test]
fn test_card_id_propagated_in_card_layer() {
    let (card_path, card_src) = read_card_rs().expect(
        "card.rs 未找到 —— 检查 CWD 是否为 src-tauri/",
    );
    let (disp_path, disp_src) = read_dispatcher_rs().expect(
        "dispatcher.rs 未找到 —— 检查 CWD 是否为 src-tauri/",
    );

    let mut violations = Vec::new();
    violations.extend(find_card_str_residue_in_card_rs(&card_src, &card_path));
    violations.extend(find_card_str_residue_in_dispatcher(&disp_src, &disp_path));

    assert!(
        violations.is_empty(),
        "发现 {} 处 W2 残留(card id 角色参数仍为 str|String|&[String]|Vec<String>):\n{:#?}",
        violations.len(),
        violations
    );
}

#[test]
fn test_w2_audit_detects_card_str_residue_in_trait_fixture() {
    let source = r#"
trait CardStore {
    fn get(&self, id: &str) -> Result<(), ()>;
    fn edit_title(&self, id: &str, new_title: &str) -> Result<(), ()>;
    fn query_by_ids(&self, ids: &[String]) -> Result<(), ()>;
    fn safe_method(&self, file_path: &str) -> Result<(), ()>;
}
    "#;
    let v = find_card_str_residue_in_card_rs(source, Path::new("fixture.rs"));
    // 应命中 get / edit_title / query_by_ids(共 3 处),safe_method 不应命中
    assert_eq!(v.len(), 3, "命中 3 处,safe_method 应豁免:{v:#?}");
    let names: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(names.contains(&"get"));
    assert!(names.contains(&"edit_title"));
    assert!(names.contains(&"query_by_ids"));
}

#[test]
fn test_w2_audit_ignores_card_id_newtype_in_trait_fixture() {
    let source = r#"
trait CardStore {
    fn get(&self, id: &CardId) -> Result<(), ()>;
    fn query_by_ids(&self, ids: &[CardId]) -> Result<(), ()>;
    fn edit_title(&self, id: &CardId, new_title: &str) -> Result<(), ()>;
}
    "#;
    let v = find_card_str_residue_in_card_rs(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "CardId newtype 参数不应被报为违规:{v:#?}");
}

#[test]
fn test_w2_audit_detects_card_id_in_dispatcher_op_fixture() {
    let source = r#"
fn op_alias_create(conn: &Connection, wb: &WhiteboardId, card_id: &str) -> OpResult {
    Ok(())
}

fn op_set_pos(conn: &Connection, wb: &WhiteboardId, entity_id: &str, x: f64, y: f64) -> OpResult {
    Ok(())
}

fn op_section_add(conn: &Connection, section_id: &str, entity_id: &str) -> OpResult {
    Ok(())
}
    "#;
    let v = find_card_str_residue_in_dispatcher(source, Path::new("fixture.rs"));
    // 只应命中 op_alias_create 的 card_id;entity_id / section_id 不属 CardId 角色
    assert_eq!(v.len(), 1, "仅 op_alias_create.card_id 应命中:{v:#?}");
    assert_eq!(v[0].fn_name, "op_alias_create");
    assert_eq!(v[0].param_name, "card_id");
}

// =============================================================================
// Scenario 2:`card_*` Tauri command 全部接 CardId
// =============================================================================

#[test]
fn test_card_tauri_commands_use_card_id() {
    let (p, s) = read_commands_rs().expect("commands.rs 未找到,检查 CWD 是否为 src-tauri/");
    let v = find_command_card_residue(&s, &p);
    assert!(
        v.is_empty(),
        "发现 {} 处 #[tauri::command] 函数 card id 参数仍是 String|&str:\n{:#?}",
        v.len(),
        v
    );
}

#[test]
fn test_w2_audit_detects_card_id_in_command_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn card_get(id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn card_query_by_ids(ids: Vec<String>) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn alias_create(whiteboard_id: WhiteboardId, card_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn entity_relate(from_card_id: String, to_card_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn card_safe(file_path: String) -> Result<(), ()> { Ok(()) }

pub fn not_a_command(card_id: String) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_card_residue(source, Path::new("fixture.rs"));
    // 命中:card_get.id / card_query_by_ids.ids / alias_create.card_id /
    //       entity_relate.from_card_id / entity_relate.to_card_id —— 共 5 处
    // 豁免:card_safe.file_path(非 card_id 角色) / not_a_command(非 command)
    assert_eq!(v.len(), 5, "应命中 5 处:{v:#?}");
    let fn_names: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fn_names.contains(&"card_get"));
    assert!(fn_names.contains(&"card_query_by_ids"));
    assert!(fn_names.contains(&"alias_create"));
    assert!(fn_names.iter().filter(|n| **n == "entity_relate").count() == 2);
    assert!(!fn_names.contains(&"card_safe"));
    assert!(!fn_names.contains(&"not_a_command"));
}

#[test]
fn test_w2_audit_ignores_card_id_newtype_in_command_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn card_get(id: CardId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn alias_create(whiteboard_id: WhiteboardId, card_id: CardId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn entity_relate(from_card_id: CardId, to_card_id: CardId) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_card_residue(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "CardId newtype 参数不应被报为违规:{v:#?}");
}
