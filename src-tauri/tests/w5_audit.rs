//! W5 EntityId 跨实体统一 audit test
//!
//! 实施 `vault/docs/tasks/task_ee2b6926/spec.md` 内 W5 段 4 个 scenario 中
//! 的 audit 部分(scenario 1 + 2);scenario 3 是 IPC reject integration test
//! 由 Green 阶段单独加;scenario 4 是 close 阶段整仓 grep + 5 项 gate,无单独
//! 测试代码。
//!
//! - test_entity_id_propagated_in_cross_entity_layer —— layout.rs / entity.rs
//!   / section.rs 中 trait/impl/free fn + dispatcher.rs 中 op_* 跨实体 helper
//!   的 entity id 角色参数全部为 `&EntityId` / `EntityId`
//! - test_entity_tauri_commands_use_entity_id —— commands.rs 中所有
//!   `#[tauri::command]` 的 entity id 角色参数全部为 `EntityId`,保留 specta 标注
//!
//! 设计要点:
//!
//! 1. **W5 不再按 entity 切**:W4 模板的 `EntityConfig`(section/task/alias/
//!    question 独立配置)在 W5 演化为"按 file scope + entity-role name 列表",
//!    因为 EntityId 是 union 类型,角色不绑定单一 entity 前缀。
//! 2. **entity-role name 集合**:`entity_id` / `entity_ids` / `from_id` / `to_id`。
//!    单字 `from` / `to` 仅在 dispatcher op_* 跨实体 scope 内启用(`op_connect` /
//!    `op_disconnect` 用单字命名),防止误伤 helper fn 中无关 from/to 参数。
//! 3. **section.rs 纳入扫描**:虽然 spec scenario 1 字面只列 layout/entity,
//!    但 section.rs::add_member/remove_member 的 entity_id: &str 完全符合"任
//!    意 entity id 角色"语义,且 scenario 4 整仓 grep 也会扫到。早扫早修。
//! 4. **不扫 ipc.rs**:`MutateParams::SetPos.entity_id` 是 struct field,audit
//!    scanner 仅扫 fn 签名;Green 阶段同步升级 IPC 层(让 dispatcher 解构出的
//!    entity_id 自然变成 `EntityId`)。
//! 5. **不扫 entity.rs::db_insert_values 返回值**:那是 DB 内部反序列化辅助
//!    (spec 显式豁免),返回类型 `(&str, &str, &'static str)` 是 SQL 参数绑
//!    定边界,不是业务参数。scanner 只看入参不看返回值,自然豁免。

use std::path::{Path, PathBuf};

use syn::{FnArg, ImplItem, Item, Pat, PatIdent, PatType, Signature, TraitItem, Type, parse_file};

// =============================================================================
// 类型建模(L0 防火墙)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum W5ResidueKind {
    /// layout.rs / entity.rs / section.rs 中 trait/impl/free fn 的 entity id 角色参数
    Domain,
    /// dispatcher.rs 中 op_* 跨实体 helper 的 entity id 角色参数
    DispatcherOp,
    /// commands.rs 中 `#[tauri::command]` 的 entity id 角色参数
    TauriCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W5ResidueViolation {
    pub file: PathBuf,
    pub line: u32,
    pub kind: W5ResidueKind,
    pub fn_name: String,
    pub param_name: String,
}

// =============================================================================
// 参数名 / 类型识别
// =============================================================================

/// 跨实体 entity id 角色命名:`entity_id` / `entity_ids` / `from_id` / `to_id`
fn is_entity_role_id_name(name: &str) -> bool {
    matches!(name, "entity_id" | "entity_ids" | "from_id" | "to_id")
}

/// dispatcher 跨实体 op_* scope:额外允许单字 `from` / `to`(`op_connect` /
/// `op_disconnect` 的 IPC wire 命名习惯)
fn is_entity_role_in_relation_scope(name: &str) -> bool {
    is_entity_role_id_name(name) || matches!(name, "from" | "to")
}

/// 判定一个 type 是否属于 str / String 家族(单值或 list/Option 包裹)。
fn is_str_family_type(ty: &Type) -> bool {
    match ty {
        Type::Reference(r) => is_str_family_type(&r.elem),
        Type::Slice(s) => is_str_family_type(&s.elem),
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
                return is_str_family_type(inner);
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
// 公共扫描器
// =============================================================================

fn scan_fn_sig(
    sig: &Signature,
    file_for_error: &Path,
    out: &mut Vec<W5ResidueViolation>,
    kind: W5ResidueKind,
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
        if !is_str_family_type(ty) {
            continue;
        }
        use proc_macro2::Span;
        let span: Span = ident.span();
        out.push(W5ResidueViolation {
            file: file_for_error.to_path_buf(),
            line: span.start().line as u32,
            kind: kind.clone(),
            fn_name: sig.ident.to_string(),
            param_name: name,
        });
    }
}

// =============================================================================
// Scanner —— Scenario 1 part a: domain 文件 (layout/entity/section)
// =============================================================================

/// 扫描 domain rs 文件(layout.rs / entity.rs / section.rs),返回所有 entity id
/// 角色参数仍为 str 家族的违规。覆盖 trait / impl / free fn。
pub fn find_domain_residue(source: &str, file_for_error: &Path) -> Vec<W5ResidueViolation> {
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
                            &tf.sig,
                            file_for_error,
                            &mut out,
                            W5ResidueKind::Domain,
                            is_entity_role_id_name,
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
                            W5ResidueKind::Domain,
                            is_entity_role_id_name,
                        );
                    }
                }
            }
            Item::Fn(f) => {
                scan_fn_sig(
                    &f.sig,
                    file_for_error,
                    &mut out,
                    W5ResidueKind::Domain,
                    is_entity_role_id_name,
                );
            }
            _ => {}
        }
    }
    out
}

// =============================================================================
// Scanner —— Scenario 1 part b: dispatcher.rs op_* 跨实体 helper
// =============================================================================

/// 跨实体 dispatcher op_* 函数白名单 —— 这些 op 的 wire 入参带 entity_id /
/// from / to 角色,W5 渗透后必须 `&EntityId` / `EntityId`。
const CROSS_ENTITY_OPS: &[&str] = &[
    "op_set_pos",
    "op_connect",
    "op_disconnect",
    "op_section_add",
];

/// 扫描 dispatcher.rs:
///
/// - 函数名命中 `CROSS_ENTITY_OPS`:命中 `entity_id` / `from_id` / `to_id` /
///   `from` / `to` / `entity_ids`
/// - 其他 free fn:仅命中 `entity_id` / `from_id` / `to_id` / `entity_ids`
///   (不命中单字 `from`/`to`,防误伤无关 helper)
pub fn find_dispatcher_residue(source: &str, file_for_error: &Path) -> Vec<W5ResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        if let Item::Fn(f) = item {
            let fn_name = f.sig.ident.to_string();
            let in_scope = CROSS_ENTITY_OPS.contains(&fn_name.as_str());
            scan_fn_sig(
                &f.sig,
                file_for_error,
                &mut out,
                W5ResidueKind::DispatcherOp,
                |s| {
                    if in_scope {
                        is_entity_role_in_relation_scope(s)
                    } else {
                        is_entity_role_id_name(s)
                    }
                },
            );
        }
    }
    out
}

// =============================================================================
// Scanner —— Scenario 2: commands.rs `#[tauri::command]`
// =============================================================================

/// 扫描 commands.rs 内所有 `#[tauri::command]` 函数,命中 entity id 角色参数:
/// `entity_id` / `entity_ids` / `from_id` / `to_id`
pub fn find_command_residue(source: &str, file_for_error: &Path) -> Vec<W5ResidueViolation> {
    let ast = match parse_file(source) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in ast.items {
        if let Item::Fn(f) = item
            && has_tauri_command_attr(&f.attrs)
        {
            scan_fn_sig(
                &f.sig,
                file_for_error,
                &mut out,
                W5ResidueKind::TauriCommand,
                is_entity_role_id_name,
            );
        }
    }
    out
}

// =============================================================================
// I/O helpers —— happy test 用
// =============================================================================

fn read_file(p: &Path) -> Option<(PathBuf, String)> {
    let s = std::fs::read_to_string(p).ok()?;
    Some((p.to_path_buf(), s))
}

const DOMAIN_FILES: &[&str] = &[
    "../keysight-core/src/domain/layout.rs",
    "../keysight-core/src/domain/entity.rs",
    "../keysight-core/src/domain/section.rs",
];

// =============================================================================
// Scenario 1:domain (layout/entity/section) + dispatcher op_* 跨实体层全 EntityId
// =============================================================================

#[test]
fn test_entity_id_propagated_in_cross_entity_layer() {
    let mut violations = Vec::new();
    for path_str in DOMAIN_FILES {
        let (path, src) = read_file(Path::new(path_str)).unwrap_or_else(|| {
            panic!("domain 文件未找到 {path_str} —— 检查 CWD 是否为 src-tauri/");
        });
        violations.extend(find_domain_residue(&src, &path));
    }
    let (disp_path, disp_src) = read_file(Path::new("src/modules/keysight/dispatcher.rs"))
        .expect("dispatcher.rs 未找到 —— 检查 CWD 是否为 src-tauri/");
    violations.extend(find_dispatcher_residue(&disp_src, &disp_path));
    assert!(
        violations.is_empty(),
        "发现 {} 处 W5 残留(domain/dispatcher 跨实体 entity id 角色参数仍为 str|String):\n{:#?}",
        violations.len(),
        violations
    );
}

// =============================================================================
// Scenario 2:commands.rs 所有 `#[tauri::command]` 入参 EntityId 化
// =============================================================================

#[test]
fn test_entity_tauri_commands_use_entity_id() {
    let (cmd_path, cmd_src) = read_file(Path::new("src/modules/keysight/commands.rs"))
        .expect("commands.rs 未找到 —— 检查 CWD 是否为 src-tauri/");
    let violations = find_command_residue(&cmd_src, &cmd_path);
    assert!(
        violations.is_empty(),
        "发现 {} 处 W5 残留(commands.rs `#[tauri::command]` 中 entity id 角色参数仍为 str|String):\n{:#?}",
        violations.len(),
        violations
    );
}

// =============================================================================
// Fixture tests —— 验证 scanner 自身正确性
// =============================================================================

#[test]
fn test_w5_audit_detects_layout_str_residue_fixture() {
    let source = r#"
trait LayoutStore {
    fn set_position(&self, wb: &WhiteboardId, entity_id: &str, x: f64, y: f64) -> Result<(), ()>;
    fn remove_position(&self, wb: &WhiteboardId, entity_id: &str) -> Result<(), ()>;
    fn query_positions(&self, wb: &WhiteboardId) -> Result<(), ()>;
}
struct SqliteLayoutStore;
impl LayoutStore for SqliteLayoutStore {
    fn set_position(&self, wb: &WhiteboardId, entity_id: &str, x: f64, y: f64) -> Result<(), ()> { Ok(()) }
    fn remove_position(&self, wb: &WhiteboardId, entity_id: &str) -> Result<(), ()> { Ok(()) }
    fn query_positions(&self, wb: &WhiteboardId) -> Result<(), ()> { Ok(()) }
}
    "#;
    let v = find_domain_residue(source, Path::new("fixture.rs"));
    // 命中:trait set/remove_position.entity_id × 2 + impl set/remove_position.entity_id × 2 = 4
    // 豁免:query_positions(无 entity_id 入参)
    assert_eq!(v.len(), 4, "应命中 4 处 layout 残留:{v:#?}");
    let pn: Vec<&str> = v.iter().map(|x| x.param_name.as_str()).collect();
    assert!(pn.iter().all(|p| *p == "entity_id"));
}

#[test]
fn test_w5_audit_detects_entity_relation_residue_fixture() {
    let source = r#"
trait EntityGraph {
    fn disconnect(&self, from_id: &str, to_id: &str, edge_type: EdgeType) -> Result<(), ()>;
    fn edges_from(&self, entity_id: &str) -> Result<(), ()>;
    fn edges_to(&self, entity_id: &str) -> Result<(), ()>;
    fn connect(&self, edge: &Edge) -> Result<(), ()>;
}
    "#;
    let v = find_domain_residue(source, Path::new("fixture.rs"));
    // 命中:disconnect.from_id+to_id = 2 / edges_from.entity_id = 1 / edges_to.entity_id = 1 = 4
    // 豁免:disconnect.edge_type(EdgeType 非 str)/ connect.edge(已是 &Edge)
    assert_eq!(v.len(), 4, "应命中 4 处 entity relation 残留:{v:#?}");
}

#[test]
fn test_w5_audit_ignores_newtype_in_domain_fixture() {
    let source = r#"
trait LayoutStore {
    fn set_position(&self, wb: &WhiteboardId, entity_id: &EntityId, x: f64, y: f64) -> Result<(), ()>;
    fn remove_position(&self, wb: &WhiteboardId, entity_id: &EntityId) -> Result<(), ()>;
}
trait EntityGraph {
    fn disconnect(&self, from_id: &EntityId, to_id: &EntityId, edge_type: EdgeType) -> Result<(), ()>;
    fn edges_from(&self, entity_id: &EntityId) -> Result<(), ()>;
}
trait SectionStore {
    fn add_member(&self, section_id: &SectionId, entity_id: &EntityId) -> Result<(), ()>;
    fn remove_member(&self, section_id: &SectionId, entity_id: &EntityId) -> Result<(), ()>;
}
    "#;
    let v = find_domain_residue(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "EntityId newtype 参数不应被报为违规:{v:#?}");
}

#[test]
fn test_w5_audit_detects_dispatcher_residue_fixture() {
    let source = r#"
fn op_set_pos(conn: &Connection, wb: &WhiteboardId, entity_id: &str, x: f64, y: f64) -> OpResult { Ok(()) }
fn op_connect(conn: &Connection, vault_fs: &dyn VaultFs, from: String, to: String) -> OpResult { Ok(()) }
fn op_disconnect(conn: &Connection, vault_fs: &dyn VaultFs, from: String, to: String, edge_type: String) -> OpResult { Ok(()) }
fn op_section_add(conn: &Connection, section_id: &SectionId, entity_id: &str) -> OpResult { Ok(()) }
fn op_alias_create(conn: &Connection, wb: &WhiteboardId, card_id: &CardId) -> OpResult { Ok(()) }
fn sync_source_file_for_edge(conn: &Connection, vault_fs: &dyn VaultFs, edge: &Edge) -> Result<(), ()> { Ok(()) }
fn unrelated_helper(name: &str, from: &str) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_dispatcher_residue(source, Path::new("fixture.rs"));
    // 命中:
    //   op_set_pos.entity_id = 1
    //   op_connect.from + to = 2(in CROSS_ENTITY_OPS,单字 from/to 命中)
    //   op_disconnect.from + to = 2(edge_type 不命中 entity_role)
    //   op_section_add.entity_id = 1
    //   共 6
    // 豁免:op_alias_create.card_id(CardId 非 str)/ sync_source_file_for_edge.edge /
    //       unrelated_helper.from(非 CROSS_ENTITY_OPS scope,单字 from 不命中)
    assert_eq!(v.len(), 6, "应命中 6 处 dispatcher 残留:{v:#?}");
    let fnn: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fnn.contains(&"op_set_pos"));
    assert!(fnn.contains(&"op_connect"));
    assert!(fnn.contains(&"op_disconnect"));
    assert!(fnn.contains(&"op_section_add"));
    assert!(!fnn.contains(&"unrelated_helper"));
    assert!(!fnn.contains(&"op_alias_create"));
}

#[test]
fn test_w5_audit_detects_command_residue_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn entity_connect(state: State<()>, from_id: String, to_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn entity_disconnect(state: State<()>, from_id: String, to_id: String, edge_type: EdgeType) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn entity_edges_from(state: State<()>, entity_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn layout_set_position(state: State<()>, whiteboard_id: WhiteboardId, entity_id: String, x: f64, y: f64) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn section_add_member(state: State<()>, section_id: SectionId, entity_id: String) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn card_get(state: State<()>, id: CardId) -> Result<(), ()> { Ok(()) }

pub fn not_a_command(from_id: String) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_residue(source, Path::new("fixture.rs"));
    // 命中:
    //   entity_connect.from_id+to_id = 2
    //   entity_disconnect.from_id+to_id = 2(edge_type 不是 str family)
    //   entity_edges_from.entity_id = 1
    //   layout_set_position.entity_id = 1
    //   section_add_member.entity_id = 1
    //   共 7
    // 豁免:card_get.id(单字 id 不在 entity_role 集合)/ not_a_command(非 command)
    assert_eq!(v.len(), 7, "应命中 7 处 command 残留:{v:#?}");
    let fnn: Vec<&str> = v.iter().map(|x| x.fn_name.as_str()).collect();
    assert!(fnn.contains(&"entity_connect"));
    assert!(fnn.contains(&"entity_disconnect"));
    assert!(fnn.contains(&"entity_edges_from"));
    assert!(fnn.contains(&"layout_set_position"));
    assert!(fnn.contains(&"section_add_member"));
    assert!(!fnn.contains(&"card_get"));
    assert!(!fnn.contains(&"not_a_command"));
}

#[test]
fn test_w5_audit_ignores_entity_id_newtype_in_command_fixture() {
    let source = r#"
#[tauri::command]
#[specta::specta]
pub fn entity_connect(state: State<()>, from_id: EntityId, to_id: EntityId) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn entity_disconnect(state: State<()>, from_id: EntityId, to_id: EntityId, edge_type: EdgeType) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn layout_set_position(state: State<()>, whiteboard_id: WhiteboardId, entity_id: EntityId, x: f64, y: f64) -> Result<(), ()> { Ok(()) }

#[tauri::command]
#[specta::specta]
pub fn section_add_member(state: State<()>, section_id: SectionId, entity_id: EntityId) -> Result<(), ()> { Ok(()) }
    "#;
    let v = find_command_residue(source, Path::new("fixture.rs"));
    assert!(v.is_empty(), "EntityId newtype 参数不应被报为违规:{v:#?}");
}

// =============================================================================
// Scenario 3:非法 entity id 通过 IPC 入参时被 EntityId deserialize 拒绝
// =============================================================================
//
// 业务计数器反证:反序列化失败时 dispatch_mutate 不会被调用,业务路径不可达,
// 任何业务侧 atomic counter 增量都 trivially 为 0。直接验证 serde_json::from_value
// 在 EntityId try_from 边界 reject "unknown_xxx",并覆盖 4 个 W5 范围 variant
// (SetPos / Connect / Disconnect / SectionAdd)。

#[test]
fn test_invalid_entity_id_rejected_at_ipc_boundary() {
    use keysight_core::ipc::MutateParams;
    use serde_json::json;

    let cases: &[(&str, serde_json::Value)] = &[
        (
            "set-pos.entity_id=unknown_xxx",
            json!({
                "kind": "set-pos",
                "wb": "wb_root",
                "entity_id": "unknown_xxx",
                "x": 0.0,
                "y": 0.0,
            }),
        ),
        (
            "connect.from=garbage_no_prefix",
            json!({
                "kind": "connect",
                "from": "garbage_no_prefix",
                "to": "card_aaa",
            }),
        ),
        (
            "disconnect.to=bogus_id",
            json!({
                "kind": "disconnect",
                "from": "card_aaa",
                "to": "bogus_id",
                "edge_type": "link_to",
            }),
        ),
        (
            "section-add.entity_id=bad_entity",
            json!({
                "kind": "section-add",
                "section_id": "sec_abc12345",
                "entity_id": "bad_entity",
            }),
        ),
    ];

    for (label, payload) in cases {
        let result: Result<MutateParams, _> = serde_json::from_value(payload.clone());
        assert!(
            result.is_err(),
            "{label}: 非法 prefix 应被 EntityId try_from 拒绝,实际成功:{result:?}"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("UnknownPrefix") || err_msg.contains("prefix"),
            "{label}: 错误消息应含 UnknownPrefix 标识,实际:{err_msg}"
        );
    }
}
