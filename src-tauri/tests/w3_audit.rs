//! W3 NoteId 渗透 audit test
//!
//! 实施 `vault/docs/tasks/task_ee2b6926/spec.md` 内 W3 段 2 个 scenario:
//!
//! - test_note_id_propagated_in_note_layer  —— `keysight-core/src/domain/note.rs`
//!   trait+impl+private inherent+free fn,与 `dispatcher.rs` op_note_* helper,
//!   以及 `sync.rs` 中处理 note 单 entity 的 helper 函数签名,全部 note id 参数
//!   为 `&NoteId` 或 `NoteId`。
//! - test_note_tauri_commands_use_note_id —— `commands.rs` `note_*` 命令 +
//!   含 `note_id|note_ids` 参数的命令,note id 角色参数为 `NoteId`。
//!
//! W3 e2e gate(test_w3_completion_passes_all_gates) —— close 阶段 manual
//! 验证 5 项 gate,无单独测试代码。
//!
//! Audit 核心 fn(`find_note_str_residue_in_note_rs` /
//! `find_note_str_residue_in_dispatcher` / `find_note_str_residue_in_sync` /
//! `find_command_note_residue`)同时被 happy test(扫真实代码)与 error test
//! (内联 fixture 字符串)共享,符合 anti-test-theater 原则。
//!
//! 与 W2 的差异:
//!
//! 1. **参数名 list**:W2 命中 `id|ids|card_id|card_ids|from_card_id|to_card_id`;
//!    W3 收口为 `id|ids|note_id|note_ids`(note 域不存在 `from_*_id`/`to_*_id`
//!    类的多 endpoint 角色)。
//! 2. **扫描范围加 sync.rs**:spec 显式要求扫 `sync.rs` 中"处理 note 单 entity
//!    的 helper 函数",规则同 dispatcher.rs(只命中 note_id 前缀)。
//! 3. **dispatcher 扫描精确到 op_note_***:函数名以 `op_note_` 开头时,允许 `id`
//!    单字命中(角色由 fn 名限定);其他 op_ helper 仅命中 `note_id|note_ids`。

use std::path::{Path, PathBuf};

use syn::{FnArg, ImplItem, Item, Pat, PatIdent, PatType, TraitItem, Type, parse_file};

// =============================================================================
// 类型建模(L0 防火墙)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteResidueKind {
    /// `note.rs` 内 trait 方法 / impl 方法 / private inherent / free fn 中
    /// note id 参数残留
    NoteDomain,
    /// `dispatcher.rs` 内 op_note_* / 任何带 note_id 入参的 op_* 残留
    DispatcherOp,
    /// `sync.rs` 内 helper fn 含 note_id 入参残留
    SyncHelper,
    /// `commands.rs` 内 `#[tauri::command]` 函数中 note id 参数残留
    TauriCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteResidueViolation {
    pub file: PathBuf,
    pub line: u32,
    pub kind: NoteResidueKind,
    pub fn_name: String,
    pub param_name: String,
}

// =============================================================================
// 参数名 / 类型识别
// =============================================================================

/// note.rs 内命中名:note id 角色的所有参数命名。
///
/// 含 `id` 单字 —— 因为 NoteStore trait 方法的 `id: &str` 语义显然是 note_id
/// (self 是 NoteStore,角色由 self 限定);private inherent `current_snapshot`
/// / `sync_links_to_file` / `desired_note_relative_path` / `note_relative_path`
/// 同理。
fn is_note_param_name_in_note_rs(s: &str) -> bool {
    matches!(s, "id" | "ids" | "note_id" | "note_ids")
}

/// dispatcher.rs / commands.rs / sync.rs 内命中名:仅含 `note_id` 前缀的命名。
///
/// **不**含 `id` 单字 —— 这些文件里其他 op_ / helper 的 `id` 可能是
/// card_id / section_id / entity_id 等其他 newtype 的角色,W3 不能误伤。
fn is_note_id_prefixed_name(s: &str) -> bool {
    matches!(s, "note_id" | "note_ids")
}

/// dispatcher.rs / commands.rs 内,fn 名以 `op_note_` / `note_` 开头时额外允许
/// `id` / `ids` 命中(语义为 note id,由 fn 名限定)。
fn is_note_param_name_in_note_scoped_fn(s: &str) -> bool {
    is_note_id_prefixed_name(s) || matches!(s, "id" | "ids")
}

/// 判定一个 type 是否属于 str / String 家族,且语义为 note id(单值或 list)。
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
            if name == "str" || name == "String" {
                return true;
            }
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
// Scenario 1a:note.rs trait/impl/private inherent/free fn note id 参数 全为 NoteId
// =============================================================================

/// 扫描 `note.rs` 源码,返回所有 note id 角色参数仍是 str 家族的违规。
pub fn find_note_str_residue_in_note_rs(
    source: &str,
    file_for_error: &Path,
) -> Vec<NoteResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        // 例外:源码语法错误不属于 W3 audit 职责,留给 cargo check / clippy 报。
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
                            NoteResidueKind::NoteDomain,
                            is_note_param_name_in_note_rs,
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
                            NoteResidueKind::NoteDomain,
                            is_note_param_name_in_note_rs,
                        );
                    }
                }
            }
            Item::Fn(f) => {
                scan_fn_sig(
                    &f.sig,
                    file_for_error,
                    &mut out,
                    NoteResidueKind::NoteDomain,
                    is_note_param_name_in_note_rs,
                );
            }
            _ => {}
        }
    }
    out
}

// =============================================================================
// Scenario 1b:dispatcher.rs op_note_* helper 全为 NoteId
// =============================================================================

/// 扫描 dispatcher.rs:
///
/// - 函数名以 `op_note_` 开头的 free fn:命中 `id|ids|note_id|note_ids`
/// - 其他 free fn:仅命中 `note_id|note_ids`(防御性:其他 op_* 若引用 note 也要报)
pub fn find_note_str_residue_in_dispatcher(
    source: &str,
    file_for_error: &Path,
) -> Vec<NoteResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        if let Item::Fn(f) = item {
            let fn_name = f.sig.ident.to_string();
            let matcher: fn(&str) -> bool = if fn_name.starts_with("op_note_") {
                is_note_param_name_in_note_scoped_fn
            } else {
                is_note_id_prefixed_name
            };
            scan_fn_sig(
                &f.sig,
                file_for_error,
                &mut out,
                NoteResidueKind::DispatcherOp,
                matcher,
            );
        }
    }
    out
}

// =============================================================================
// Scenario 1c:sync.rs 中处理 note 单 entity 的 helper fn 全为 NoteId
// =============================================================================

/// 扫描 sync.rs:任何 free fn / impl fn 入参命中 `note_id|note_ids` 且类型为
/// str 家族的违规。sync.rs 是跨实体的同步器,`id` 单字角色不明确,所以只命中
/// 显式带 `note_` 前缀的参数。
pub fn find_note_str_residue_in_sync(
    source: &str,
    file_for_error: &Path,
) -> Vec<NoteResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        match item {
            Item::Fn(f) => {
                scan_fn_sig(
                    &f.sig,
                    file_for_error,
                    &mut out,
                    NoteResidueKind::SyncHelper,
                    is_note_id_prefixed_name,
                );
            }
            Item::Impl(im) => {
                for ii in im.items {
                    if let ImplItem::Fn(f) = ii {
                        scan_fn_sig(
                            &f.sig,
                            file_for_error,
                            &mut out,
                            NoteResidueKind::SyncHelper,
                            is_note_id_prefixed_name,
                        );
                    }
                }
            }
            _ => {}
        }
    }
    out
}

// =============================================================================
// Scenario 2:commands.rs `note_*` Tauri command 全为 NoteId
// =============================================================================

/// 扫描 commands.rs:
///
/// - 函数名以 `note_` 开头的 `#[tauri::command]`:命中 `id|ids|note_id|note_ids`
/// - 其他 `#[tauri::command]`:仅命中 `note_id|note_ids`
pub fn find_command_note_residue(
    source: &str,
    file_for_error: &Path,
) -> Vec<NoteResidueViolation> {
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
            let matcher: fn(&str) -> bool = if fn_name.starts_with("note_") {
                is_note_param_name_in_note_scoped_fn
            } else {
                is_note_id_prefixed_name
            };
            scan_fn_sig(
                &f.sig,
                file_for_error,
                &mut out,
                NoteResidueKind::TauriCommand,
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
    out: &mut Vec<NoteResidueViolation>,
    kind: NoteResidueKind,
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
        out.push(NoteResidueViolation {
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

fn read_note_rs() -> Option<(PathBuf, String)> {
    read_file(Path::new("../keysight-core/src/domain/note.rs"))
}

fn read_dispatcher_rs() -> Option<(PathBuf, String)> {
    read_file(Path::new("src/modules/keysight/dispatcher.rs"))
}

fn read_sync_rs() -> Option<(PathBuf, String)> {
    read_file(Path::new("../keysight-core/src/domain/sync.rs"))
}

fn read_commands_rs() -> Option<(PathBuf, String)> {
    read_file(Path::new("src/modules/keysight/commands.rs"))
}

// =============================================================================
// Scenario 1:note domain layer + dispatcher op_note_* + sync helper 全为 NoteId
// =============================================================================

#[test]
fn test_note_id_propagated_in_note_layer() {
    let (note_path, note_src) = read_note_rs().expect(
        "note.rs 未找到 —— 检查 CWD 是否为 src-tauri/",
    );
    let (disp_path, disp_src) = read_dispatcher_rs().expect(
        "dispatcher.rs 未找到 —— 检查 CWD 是否为 src-tauri/",
    );
    let (sync_path, sync_src) = read_sync_rs().expect(
        "sync.rs 未找到 —— 检查 CWD 是否为 src-tauri/",
    );

    let mut violations = Vec::new();
    violations.extend(find_note_str_residue_in_note_rs(&note_src, &note_path));
    violations.extend(find_note_str_residue_in_dispatcher(&disp_src, &disp_path));
    violations.extend(find_note_str_residue_in_sync(&sync_src, &sync_path));

    assert!(
        violations.is_empty(),
        "发现 {} 处 W3 残留(note id 角色参数仍为 str|String|&[String]|Vec<String>):\n{:#?}",
        violations.len(),
        violations
    );
}

#[test]
fn test_w3_audit_detects_note_str_residue_in_trait_fixture() {
    let source = r#"
trait NoteStore {
    fn get(&self, id: &str) -> Result<(), ()>;
    fn delete(&self, id: &str) -> Result<(), ()>;
    fn update(&self, id: &str, title: Option<&str>) -> Result<(), ()>;
    fn query_by_file(&self, file_path: &str) -> Result<(), ()>;
}
    "#;
    // 应命中 get / delete / update(共 3 处),query_by_file 不应命中(file_path 角色)
    let v = find_note_str_residue_in_note_rs(source, Path::new("fixture.rs"));
    assert_eq!(v.len(), 3, "命中 3 处,query_by_file 应豁免:{v:#?}");
    let names: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(names.contains(&"get"));
    assert!(names.contains(&"delete"));
    assert!(names.contains(&"update"));
}

#[test]
fn test_w3_audit_ignores_note_id_newtype_in_trait_fixture() {
    let source = r#"
trait NoteStore {
    fn get(&self, id: &NoteId) -> Result<(), ()>;
    fn delete(&self, id: &NoteId) -> Result<(), ()>;
    fn update(&self, id: &NoteId, title: Option<&str>) -> Result<(), ()>;
}
    "#;
    let v = find_note_str_residue_in_note_rs(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "NoteId newtype 参数不应被报为违规:{v:#?}");
}

#[test]
fn test_w3_audit_detects_note_id_in_dispatcher_op_fixture() {
    let source = r#"
fn op_note_update(conn: &Connection, vault_fs: &dyn VaultFs, id: String, title: Option<&str>) -> OpResult {
    Ok(())
}

fn op_note_create(conn: &Connection, vault_fs: &dyn VaultFs, wb: &WhiteboardId, title: &str) -> OpResult {
    Ok(())
}

fn op_set_pos(conn: &Connection, wb: &WhiteboardId, entity_id: &str, x: f64, y: f64) -> OpResult {
    Ok(())
}

fn op_other(conn: &Connection, note_id: &str) -> OpResult {
    Ok(())
}
    "#;
    let v = find_note_str_residue_in_dispatcher(source, Path::new("fixture.rs"));
    // 应命中:op_note_update.id(op_note_ 前缀允许 id 单字)+ op_other.note_id
    // 豁免:op_note_create 无 note id 参数 / op_set_pos.entity_id(不属 note 角色)
    assert_eq!(v.len(), 2, "应命中 2 处:{v:#?}");
    let fn_names: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fn_names.contains(&"op_note_update"));
    assert!(fn_names.contains(&"op_other"));
}

#[test]
fn test_w3_audit_ignores_note_id_newtype_in_dispatcher_fixture() {
    let source = r#"
fn op_note_update(conn: &Connection, id: NoteId, title: Option<&str>) -> OpResult {
    Ok(())
}

fn op_other(conn: &Connection, note_id: &NoteId) -> OpResult {
    Ok(())
}
    "#;
    let v = find_note_str_residue_in_dispatcher(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "NoteId newtype 参数不应被报为违规:{v:#?}");
}

#[test]
fn test_w3_audit_detects_note_id_in_sync_helper_fixture() {
    let source = r#"
fn sync_note_links(conn: &Connection, note_id: &str) -> Result<(), ()> {
    Ok(())
}

fn sync_card_link(conn: &Connection, card_id: &str) -> Result<(), ()> {
    Ok(())
}

fn sync_file(conn: &Connection, file_path: &str, content: &str) -> Result<(), ()> {
    Ok(())
}
    "#;
    let v = find_note_str_residue_in_sync(source, Path::new("fixture.rs"));
    // 只应命中 sync_note_links.note_id;card_id / file_path 不属 note 角色
    assert_eq!(v.len(), 1, "仅 sync_note_links.note_id 应命中:{v:#?}");
    assert_eq!(v[0].fn_name, "sync_note_links");
    assert_eq!(v[0].param_name, "note_id");
}

// =============================================================================
// Scenario 2:`note_*` Tauri command 全部接 NoteId
// =============================================================================

#[test]
fn test_note_tauri_commands_use_note_id() {
    let (p, s) = read_commands_rs().expect("commands.rs 未找到,检查 CWD 是否为 src-tauri/");
    let v = find_command_note_residue(&s, &p);
    assert!(
        v.is_empty(),
        "发现 {} 处 #[tauri::command] 函数 note id 参数仍是 String|&str:\n{:#?}",
        v.len(),
        v
    );
}

#[test]
fn test_w3_audit_detects_note_id_in_command_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn note_get(id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn note_delete(id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn note_update(id: String, title: Option<String>) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn note_query_all(whiteboard_id: WhiteboardId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn link_note_card(note_id: String, card_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn note_query_by_file(file_path: String) -> Result<(), ()> { Ok(()) }

pub fn not_a_command(note_id: String) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_note_residue(source, Path::new("fixture.rs"));
    // 命中:note_get.id / note_delete.id / note_update.id / link_note_card.note_id —— 共 4 处
    // 豁免:note_query_all.whiteboard_id(非 note id 角色)/
    //       link_note_card.card_id(card 域不属本 W3 audit)/
    //       note_query_by_file.file_path(非 note id 角色)/
    //       not_a_command(非 command)
    assert_eq!(v.len(), 4, "应命中 4 处:{v:#?}");
    let fn_names: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fn_names.contains(&"note_get"));
    assert!(fn_names.contains(&"note_delete"));
    assert!(fn_names.contains(&"note_update"));
    assert!(fn_names.contains(&"link_note_card"));
    assert!(!fn_names.contains(&"note_query_all"));
    assert!(!fn_names.contains(&"note_query_by_file"));
    assert!(!fn_names.contains(&"not_a_command"));
}

#[test]
fn test_w3_audit_ignores_note_id_newtype_in_command_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn note_get(id: NoteId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn note_update(id: NoteId, title: Option<String>) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn link_note_card(note_id: NoteId, card_id: CardId) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_note_residue(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "NoteId newtype 参数不应被报为违规:{v:#?}");
}
