//! W4 SectionId / TaskId / AliasId / QuestionId 合并波 audit test
//!
//! 实施 `vault/docs/tasks/task_ee2b6926/spec.md` 内 W4 段 3 个 scenario:
//!
//! - test_w4_entities_use_newtype       —— 4 个 domain trait/impl/free fn 中
//!   对应 id 角色参数为各自 newtype(SectionId / TaskId / AliasId / QuestionId)
//! - test_w4_dispatcher_ops_use_newtype —— `dispatcher.rs` 中处理 section / task /
//!   alias / question 的 op_ helper + commands.rs 内对应 Tauri command id 参数
//!   为各自 newtype
//! - W4 e2e gate(test_w4_completion_passes_all_gates)—— close 阶段 manual
//!   验证 5 项 gate,无单独测试代码
//!
//! 设计要点:
//!
//! 1. **通用 Scanner**:`scan_entity_residue` 接收 `EntityConfig` 配置 + 命中
//!    名 matcher,W2/W3 一个 entity 一组 finder 的模式在 W4 抽成参数化。
//!    四个 entity 共享一份扫描代码,无重复,符合 anti-test-theater 原则。
//! 2. **fixture 反向覆盖**:每个 finder 同时有 happy(扫真实代码)与 fixture
//!    (命中 + 豁免双向)两类 test,scanner 自身的正确性独立可验证。
//! 3. **dispatcher / commands matcher**:函数名以 `op_<entity>_` / `<entity>_`
//!    开头时允许 `id` 单字命中(角色由 fn 名限定);其他 fn 仅命中 `<entity>_id`
//!    / `<entity>_ids` 显式前缀,防误伤跨实体调用方。
//! 4. **不扫 ipc.rs**:`MutateParams::Section*.section_id` 是 struct field,
//!    audit scanner 仅扫 fn 签名;Green 阶段同步升级 IPC 层(让 dispatcher 解
//!    构出的 section_id 自然变成 `SectionId`)。

use std::path::{Path, PathBuf};

use syn::{FnArg, ImplItem, Item, Pat, PatIdent, PatType, TraitItem, Type, parse_file};

// =============================================================================
// 类型建模(L0 防火墙)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum W4ResidueKind {
    /// 4 个 domain 文件(section.rs / task.rs / alias.rs / question.rs)中
    /// trait / impl / free fn 中对应 entity id 角色参数残留
    Domain,
    /// dispatcher.rs 内 op_<entity>_* / 任何带 <entity>_id 入参的 op_* 残留
    DispatcherOp,
    /// commands.rs 内 `#[tauri::command]` 函数中对应 entity id 参数残留
    TauriCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W4ResidueViolation {
    pub file: PathBuf,
    pub line: u32,
    pub kind: W4ResidueKind,
    pub entity: &'static str,
    pub fn_name: String,
    pub param_name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct EntityConfig {
    /// 实体英文名(用于错误消息),例如 "section" / "task" / "alias" / "question"
    pub name: &'static str,
    /// 实体的 id 前缀(audit 不直接用,留作文档):"sec_" / "task_" / "alias_" / "q_"
    pub _prefix: &'static str,
    /// 实体的 domain rs 文件路径(相对 src-tauri/)
    pub domain_path: &'static str,
    /// dispatcher / commands 内对应函数名前缀(`op_section_` 用 "section",
    /// `section_get` 用 "section";命中 entity_<id>_ids 时用 "<id>")
    pub fn_prefix: &'static str,
    /// 参数名 prefix(`section_id` / `task_id` / `alias_id` / `question_id`)
    pub id_param_prefix: &'static str,
}

const SECTION: EntityConfig = EntityConfig {
    name: "section",
    _prefix: "sec_",
    domain_path: "../keysight-core/src/domain/section.rs",
    fn_prefix: "section",
    id_param_prefix: "section_id",
};

const TASK: EntityConfig = EntityConfig {
    name: "task",
    _prefix: "task_",
    domain_path: "../keysight-core/src/domain/task.rs",
    fn_prefix: "task",
    id_param_prefix: "task_id",
};

const ALIAS: EntityConfig = EntityConfig {
    name: "alias",
    _prefix: "alias_",
    domain_path: "../keysight-core/src/domain/alias.rs",
    fn_prefix: "alias",
    id_param_prefix: "alias_id",
};

const QUESTION: EntityConfig = EntityConfig {
    name: "question",
    _prefix: "q_",
    domain_path: "../keysight-core/src/domain/question.rs",
    fn_prefix: "question",
    id_param_prefix: "question_id",
};

const ALL_ENTITIES: &[EntityConfig] = &[SECTION, TASK, ALIAS, QUESTION];

// =============================================================================
// 参数名 / 类型识别
// =============================================================================

/// domain 文件内命中名:`id | ids | <entity>_id | <entity>_ids`(`id` 单字
/// 角色由 trait / file 限定)。
fn is_param_name_in_domain(cfg: &EntityConfig, s: &str) -> bool {
    let ids = format!("{}s", cfg.id_param_prefix);
    s == "id" || s == "ids" || s == cfg.id_param_prefix || s == ids
}

/// dispatcher / commands 内命中名:仅含 `<entity>_id` / `<entity>_ids` 前缀。
/// **不**含 `id` 单字 —— 防误伤其他实体的 id 参数。
fn is_id_prefixed_name(cfg: &EntityConfig, s: &str) -> bool {
    let ids = format!("{}s", cfg.id_param_prefix);
    s == cfg.id_param_prefix || s == ids
}

/// dispatcher op_<entity>_* / commands <entity>_* 内:额外允许 `id` / `ids`
/// 命中(语义由 fn 名前缀限定)。
fn is_param_name_in_scoped_fn(cfg: &EntityConfig, s: &str) -> bool {
    is_id_prefixed_name(cfg, s) || s == "id" || s == "ids"
}

/// 判定一个 type 是否属于 str / String 家族,且语义为对应 entity id(单值或 list)。
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
// 通用 Scanner —— Scenario 1:domain 文件
// =============================================================================

/// 扫描某 domain rs 文件,返回所有对应 entity id 角色参数仍是 str 家族的违规。
pub fn find_str_residue_in_domain(
    cfg: &EntityConfig,
    source: &str,
    file_for_error: &Path,
) -> Vec<W4ResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        match item {
            Item::Trait(t) => {
                for ti in t.items {
                    if let TraitItem::Fn(tf) = ti {
                        scan_fn_sig(
                            cfg,
                            &tf.sig,
                            file_for_error,
                            &mut out,
                            W4ResidueKind::Domain,
                            |s| is_param_name_in_domain(cfg, s),
                        );
                    }
                }
            }
            Item::Impl(im) => {
                for ii in im.items {
                    if let ImplItem::Fn(f) = ii {
                        scan_fn_sig(
                            cfg,
                            &f.sig,
                            file_for_error,
                            &mut out,
                            W4ResidueKind::Domain,
                            |s| is_param_name_in_domain(cfg, s),
                        );
                    }
                }
            }
            Item::Fn(f) => {
                scan_fn_sig(
                    cfg,
                    &f.sig,
                    file_for_error,
                    &mut out,
                    W4ResidueKind::Domain,
                    |s| is_param_name_in_domain(cfg, s),
                );
            }
            _ => {}
        }
    }
    out
}

// =============================================================================
// 通用 Scanner —— Scenario 2a:dispatcher op_<entity>_* helper
// =============================================================================

/// 扫描 dispatcher.rs:
///
/// - 函数名以 `op_<entity>_` 开头的 free fn:命中 `id|ids|<entity>_id|<entity>_ids`
/// - 其他 free fn:仅命中 `<entity>_id|<entity>_ids`
pub fn find_str_residue_in_dispatcher(
    cfg: &EntityConfig,
    source: &str,
    file_for_error: &Path,
) -> Vec<W4ResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    let op_prefix = format!("op_{}_", cfg.fn_prefix);
    for item in ast.items {
        if let Item::Fn(f) = item {
            let fn_name = f.sig.ident.to_string();
            let in_scope = fn_name.starts_with(&op_prefix);
            scan_fn_sig(
                cfg,
                &f.sig,
                file_for_error,
                &mut out,
                W4ResidueKind::DispatcherOp,
                |s| {
                    if in_scope {
                        is_param_name_in_scoped_fn(cfg, s)
                    } else {
                        is_id_prefixed_name(cfg, s)
                    }
                },
            );
        }
    }
    out
}

// =============================================================================
// 通用 Scanner —— Scenario 2b:commands.rs `<entity>_*` Tauri command
// =============================================================================

/// 扫描 commands.rs:
///
/// - 函数名以 `<entity>_` 开头的 `#[tauri::command]`:命中 `id|ids|<entity>_id|<entity>_ids`
/// - 其他 `#[tauri::command]`:仅命中 `<entity>_id|<entity>_ids`
pub fn find_command_residue(
    cfg: &EntityConfig,
    source: &str,
    file_for_error: &Path,
) -> Vec<W4ResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    let cmd_prefix = format!("{}_", cfg.fn_prefix);
    for item in ast.items {
        if let Item::Fn(f) = item
            && has_tauri_command_attr(&f.attrs)
        {
            let fn_name = f.sig.ident.to_string();
            let in_scope = fn_name.starts_with(&cmd_prefix);
            scan_fn_sig(
                cfg,
                &f.sig,
                file_for_error,
                &mut out,
                W4ResidueKind::TauriCommand,
                |s| {
                    if in_scope {
                        is_param_name_in_scoped_fn(cfg, s)
                    } else {
                        is_id_prefixed_name(cfg, s)
                    }
                },
            );
        }
    }
    out
}

// =============================================================================
// 公共扫描器
// =============================================================================

fn scan_fn_sig(
    cfg: &EntityConfig,
    sig: &syn::Signature,
    file_for_error: &Path,
    out: &mut Vec<W4ResidueViolation>,
    kind: W4ResidueKind,
    name_matcher: impl Fn(&str) -> bool,
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
        out.push(W4ResidueViolation {
            file: file_for_error.to_path_buf(),
            line: span.start().line as u32,
            kind: kind.clone(),
            entity: cfg.name,
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
// Scenario 1:4 个 domain 文件 trait/impl/free fn id 角色参数 全为对应 newtype
// =============================================================================

#[test]
fn test_w4_entities_use_newtype() {
    let mut violations = Vec::new();
    for cfg in ALL_ENTITIES {
        let (path, src) = read_file(Path::new(cfg.domain_path)).unwrap_or_else(|| {
            panic!("domain 文件未找到 {} —— 检查 CWD 是否为 src-tauri/", cfg.domain_path);
        });
        violations.extend(find_str_residue_in_domain(cfg, &src, &path));
    }
    assert!(
        violations.is_empty(),
        "发现 {} 处 W4 残留(domain 层 entity id 角色参数仍为 str|String|&[String]|Vec<String>):\n{:#?}",
        violations.len(),
        violations
    );
}

#[test]
fn test_w4_audit_detects_section_str_residue_in_trait_fixture() {
    let source = r#"
trait SectionStore {
    fn get(&self, id: &str) -> Result<(), ()>;
    fn delete(&self, id: &str) -> Result<(), ()>;
    fn add_member(&self, section_id: &str, entity_id: &str) -> Result<(), ()>;
    fn move_to_whiteboard(&self, section_id: &str, target_whiteboard_id: &str) -> Result<(), ()>;
    fn query_by_file(&self, file_path: &str) -> Result<(), ()>;
}
    "#;
    let v = find_str_residue_in_domain(&SECTION, source, Path::new("fixture.rs"));
    // 命中:get.id / delete.id / add_member.section_id / move_to_whiteboard.section_id = 4
    // 豁免:add_member.entity_id(非 section 角色)/ move_to_whiteboard.target_whiteboard_id(同)
    //       / query_by_file.file_path
    assert_eq!(v.len(), 4, "应命中 4 处 section 残留:{v:#?}");
    let pn: Vec<&str> = v.iter().map(|x| x.param_name.as_str()).collect();
    assert!(pn.contains(&"id"));
    assert!(pn.contains(&"section_id"));
    assert!(!pn.contains(&"entity_id"));
    assert!(!pn.contains(&"target_whiteboard_id"));
}

#[test]
fn test_w4_audit_ignores_section_newtype_in_trait_fixture() {
    let source = r#"
trait SectionStore {
    fn get(&self, id: &SectionId) -> Result<(), ()>;
    fn add_member(&self, section_id: &SectionId, entity_id: &EntityId) -> Result<(), ()>;
    fn move_to_whiteboard(&self, section_id: &SectionId, target_whiteboard_id: &WhiteboardId) -> Result<(), ()>;
}
    "#;
    let v = find_str_residue_in_domain(&SECTION, source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "SectionId newtype 参数不应被报为违规:{v:#?}");
}

#[test]
fn test_w4_audit_question_free_fn_residue_fixture() {
    // question.rs 是 free fn 风格(非 trait),scanner 也要扫到
    let source = r#"
pub fn get(conn: &Connection, id: &str) -> Result<Question, ()> { Ok(Question {}) }
pub fn update(conn: &Connection, id: &str, title: Option<&str>) -> Result<(), ()> { Ok(()) }
fn question_relative_path(wb: &WhiteboardId, question_id: &str, title: &str) -> String { String::new() }
fn validate_title(title: &str) -> Result<&str, ()> { Ok(title) }
    "#;
    let v = find_str_residue_in_domain(&QUESTION, source, Path::new("fixture.rs"));
    // 命中:get.id / update.id / question_relative_path.question_id = 3
    // 豁免:validate_title.title(非 id 角色)
    assert_eq!(v.len(), 3, "应命中 3 处 question free fn 残留:{v:#?}");
    let fnn: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fnn.contains(&"get"));
    assert!(fnn.contains(&"update"));
    assert!(fnn.contains(&"question_relative_path"));
    assert!(!fnn.contains(&"validate_title"));
}

// =============================================================================
// Scenario 2:dispatcher op_<entity>_* + commands `<entity>_*` 全为对应 newtype
// =============================================================================

#[test]
fn test_w4_dispatcher_ops_use_newtype() {
    let (disp_path, disp_src) = read_dispatcher_rs().expect(
        "dispatcher.rs 未找到 —— 检查 CWD 是否为 src-tauri/",
    );
    let (cmd_path, cmd_src) = read_commands_rs().expect(
        "commands.rs 未找到 —— 检查 CWD 是否为 src-tauri/",
    );

    let mut violations = Vec::new();
    for cfg in ALL_ENTITIES {
        violations.extend(find_str_residue_in_dispatcher(cfg, &disp_src, &disp_path));
        violations.extend(find_command_residue(cfg, &cmd_src, &cmd_path));
    }
    assert!(
        violations.is_empty(),
        "发现 {} 处 W4 残留(dispatcher op_/commands 层 entity id 角色参数仍为 str|String):\n{:#?}",
        violations.len(),
        violations
    );
}

#[test]
fn test_w4_audit_detects_section_dispatcher_residue_fixture() {
    let source = r#"
fn op_section_create(conn: &Connection, wb: &WhiteboardId, title: &str) -> OpResult { Ok(()) }
fn op_section_add(conn: &Connection, section_id: &str, entity_id: &str) -> OpResult { Ok(()) }
fn op_section_move(conn: &Connection, section_id: &str, target_wb: &WhiteboardId) -> OpResult { Ok(()) }
fn op_alias_create(conn: &Connection, section_id: &str) -> OpResult { Ok(()) }
fn op_set_pos(conn: &Connection, wb: &WhiteboardId, entity_id: &str) -> OpResult { Ok(()) }
    "#;
    let v = find_str_residue_in_dispatcher(&SECTION, source, Path::new("fixture.rs"));
    // 命中:op_section_add.section_id / op_section_move.section_id /
    //       op_alias_create.section_id(其他 op_ 含 section_id 也应命中)= 3
    // 豁免:op_section_create 无 section_id 入参 / op_set_pos.entity_id 非 section 角色
    assert_eq!(v.len(), 3, "应命中 3 处 section dispatcher 残留:{v:#?}");
    let fnn: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fnn.contains(&"op_section_add"));
    assert!(fnn.contains(&"op_section_move"));
    assert!(fnn.contains(&"op_alias_create"));
    assert!(!fnn.contains(&"op_section_create"));
    assert!(!fnn.contains(&"op_set_pos"));
}

#[test]
fn test_w4_audit_detects_section_command_residue_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn section_get(id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn section_add_member(section_id: String, entity_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn section_query_all(whiteboard_id: WhiteboardId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn entity_relate(section_id: String) -> Result<(), ()> { Ok(()) }

pub fn not_a_command(section_id: String) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_residue(&SECTION, source, Path::new("fixture.rs"));
    // 命中:section_get.id / section_add_member.section_id / entity_relate.section_id = 3
    // 豁免:section_query_all.whiteboard_id / not_a_command(非 command) /
    //       section_add_member.entity_id(非 section 角色)
    assert_eq!(v.len(), 3, "应命中 3 处 section command 残留:{v:#?}");
    let fnn: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fnn.contains(&"section_get"));
    assert!(fnn.contains(&"section_add_member"));
    assert!(fnn.contains(&"entity_relate"));
    assert!(!fnn.contains(&"section_query_all"));
    assert!(!fnn.contains(&"not_a_command"));
}

#[test]
fn test_w4_audit_ignores_newtype_in_command_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn section_get(id: SectionId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn section_add_member(section_id: SectionId, entity_id: EntityId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn task_update(id: TaskId, title: Option<String>) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn alias_get(id: AliasId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn question_delete(id: QuestionId) -> Result<(), ()> { Ok(()) }
    "#;
    for cfg in ALL_ENTITIES {
        let v = find_command_residue(cfg, source, Path::new("fixture.rs"));
        assert!(
            v.is_empty(),
            "{} newtype 参数不应被报为违规:{:#?}",
            cfg.name,
            v
        );
    }
}
