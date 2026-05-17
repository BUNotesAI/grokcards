//! IPC types — keysight-cli 与 super-tauri server 共享的请求/响应 schema。
//!
//! `MutateParams` — 写命令请求参数 tagged enum(CLI 构造 + server dispatch)。
//! `MutateResponse` — server 写完后返回的通用响应形状。
//!
//! 本模块**纯 data types**(serde derive + Debug + Clone),不引 reqwest/tokio。
//! 被 `keysight-cli`(CLI 发)与 `super-tauri`(server 收)两侧共用,保证 JSON schema
//! 一致性(single source of truth)。

use serde::{Deserialize, Serialize};

use crate::domain::id::WhiteboardId;

/// 写命令请求参数 — tagged enum,wire 层 discriminator 字段为 `kind`(kebab-case)。
///
/// 与 keysight-cli 6.2a 的 9 live mutate CLI 命令 JSON shape 严格对齐。server
/// 侧 `handle_rpc` mutate 分支 deserialize 后 match variant 分派到对应 Store fn。
///
/// ## Variant 到 Store fn 映射
///
/// | Variant | Store fn | 返 entity_id |
/// |---|---|---|
/// | `SectionCreate` | `SectionStore::create(wb, title, color?)` | Yes |
/// | `NoteCreate` | `NoteStore::create(wb, title, content?, color?)` | Yes |
/// | `NoteUpdate` | `NoteStore::update(id, title?, content?, color?)` | No |
/// | `AliasCreate` | `AliasStore::create(wb, card_id)` | Yes |
/// | `SetPos` | `LayoutStore::set_position(wb, entity_id, x, y)` | No |
/// | `Connect` | `user_draw_edge(EntityId::parse(from)?, EntityId::parse(to)?)` | No |
/// | `Disconnect` | `EntityGraph::disconnect(from, to, edge_type.parse()?)` | No |
/// | `SectionAdd` | `SectionStore::add_member(section_id, entity_id)` | No |
/// | `SectionMove` | `SectionStore::move_to_whiteboard(section_id, target_wb)` | No |
///
/// ## Design note — Connect 由 EntityId 推导,Disconnect 仍 stringly-typed
///
/// `Connect`: wire 只传 `from` / `to`。Edge variant 由 `EntityId::parse` 得到的
/// kind 唯一决定(Card/Note/Alias/Question → 对应 `*Link` 变体;Section/Task →
/// `ConnectionNotAllowed`),无需调用方判别。
///
/// `Disconnect`: 仍透传 `edge_type: String`,server 侧 `EdgeType::from_db_str`
/// parse(domain `EntityGraph::disconnect` API 仍是 stringly-typed,子阶段统一
/// 升级待 D Theme 后再做)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MutateParams {
    SectionCreate {
        wb: WhiteboardId,
        title: String,
        #[serde(default)]
        color: Option<String>,
    },
    NoteCreate {
        wb: WhiteboardId,
        title: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        color: Option<String>,
    },
    NoteUpdate {
        id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        color: Option<String>,
    },
    AliasCreate {
        wb: WhiteboardId,
        card_id: String,
    },
    SetPos {
        wb: WhiteboardId,
        entity_id: String,
        x: f64,
        y: f64,
    },
    Connect {
        from: String,
        to: String,
    },
    Disconnect {
        from: String,
        to: String,
        edge_type: String,
    },
    SectionAdd {
        section_id: String,
        entity_id: String,
    },
    SectionMove {
        section_id: String,
        target_wb: WhiteboardId,
    },
}

/// 写命令响应 — success flag + 可选 entity_id + 可选 message。
///
/// Create 类 variant 成功时 `entity_id = Some(id_of_new_entity)`;
/// Update / 关系操作 variant 成功时 `entity_id = None`。
/// `message` 留给附加上下文(比如 "section-create OK: sec_abc12345");
/// 当前 6.2b 默认不填,CLI 侧 println! 只读 `success` + `entity_id`。
///
/// 失败路径不走本结构 —— server 返 HTTP 4xx/5xx,CLI 侧 `HttpClient::post_rpc`
/// 把非 2xx 转成 `CliError::Http(...)`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutateResponse {
    pub success: bool,
    pub entity_id: Option<String>,
    pub message: Option<String>,
}

// -----------------------------------------------------------------------------
// Tests —— JSON shape 与 CLI 6.2a `commands.rs::graph_*` 构造对齐的 round-trip 锁
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `SectionCreate` 的 JSON shape 与 CLI 侧 `graph_section_create` 构造一致
    #[test]
    fn test_mutate_params_section_create_matches_cli_shape() {
        let params = MutateParams::SectionCreate {
            wb: WhiteboardId::parse("wb_root").unwrap(),
            title: "MySection".to_string(),
            color: Some("#ff0000".to_string()),
        };
        let j = serde_json::to_value(&params).unwrap();
        assert_eq!(
            j,
            json!({
                "kind": "section-create",
                "wb": "wb_root",
                "title": "MySection",
                "color": "#ff0000",
            })
        );
    }

    /// `NoteCreate` 带 None content/color → wire null(CLI 侧 `serde_json::json!` 同样产生 null)
    #[test]
    fn test_mutate_params_note_create_none_fields_emit_null() {
        let params = MutateParams::NoteCreate {
            wb: WhiteboardId::parse("wb_root").unwrap(),
            title: "untitled".into(),
            content: None,
            color: None,
        };
        let j = serde_json::to_value(&params).unwrap();
        assert_eq!(j["kind"], "note-create");
        assert_eq!(j["content"], serde_json::Value::Null);
        assert_eq!(j["color"], serde_json::Value::Null);
    }

    /// `SetPos` wire name 是 `set-pos`(不是 `setpos` / `set_pos`)
    #[test]
    fn test_mutate_params_set_pos_kebab_case() {
        let params = MutateParams::SetPos {
            wb: WhiteboardId::parse("wb_root").unwrap(),
            entity_id: "sec_abc12345".into(),
            x: 100.5,
            y: -50.25,
        };
        let j = serde_json::to_value(&params).unwrap();
        assert_eq!(j["kind"], "set-pos");
        assert_eq!(j["x"], 100.5);
        assert_eq!(j["y"], -50.25);
    }

    /// `Connect` wire shape 不含 edge_type:Edge variant 由 EntityId 推导,
    /// 字段已退役。CLI / TS 构造时只需 from + to。
    #[test]
    fn test_mutate_params_connect_wire_has_no_edge_type() {
        let params = MutateParams::Connect {
            from: "note_aaa".into(),
            to: "card_bbb".into(),
        };
        let j = serde_json::to_value(&params).unwrap();
        assert_eq!(
            j,
            json!({
                "kind": "connect",
                "from": "note_aaa",
                "to": "card_bbb",
            })
        );
        assert!(j.get("edge_type").is_none(), "edge_type 应已从 wire 移除");
    }

    /// 所有 9 kind 字符串 round-trip 稳定(避免 variant rename 悄悄破协议)
    #[test]
    fn test_mutate_params_all_kinds_roundtrip() {
        let cases: &[(MutateParams, &str)] = &[
            (
                MutateParams::SectionCreate {
                    wb: WhiteboardId::parse("w").unwrap(),
                    title: "t".into(),
                    color: None,
                },
                "section-create",
            ),
            (
                MutateParams::NoteCreate {
                    wb: WhiteboardId::parse("w").unwrap(),
                    title: "t".into(),
                    content: None,
                    color: None,
                },
                "note-create",
            ),
            (
                MutateParams::NoteUpdate {
                    id: "n".into(),
                    title: None,
                    content: None,
                    color: None,
                },
                "note-update",
            ),
            (
                MutateParams::AliasCreate {
                    wb: WhiteboardId::parse("w").unwrap(),
                    card_id: "c".into(),
                },
                "alias-create",
            ),
            (
                MutateParams::SetPos {
                    wb: WhiteboardId::parse("w").unwrap(),
                    entity_id: "e".into(),
                    x: 0.0,
                    y: 0.0,
                },
                "set-pos",
            ),
            (
                MutateParams::Connect {
                    from: "a".into(),
                    to: "b".into(),
                },
                "connect",
            ),
            (
                MutateParams::Disconnect {
                    from: "a".into(),
                    to: "b".into(),
                    edge_type: "link_to".into(),
                },
                "disconnect",
            ),
            (
                MutateParams::SectionAdd {
                    section_id: "s".into(),
                    entity_id: "e".into(),
                },
                "section-add",
            ),
            (
                MutateParams::SectionMove {
                    section_id: "s".into(),
                    target_wb: WhiteboardId::parse("wb_target").unwrap(),
                },
                "section-move",
            ),
        ];
        for (params, expected_kind) in cases {
            let j = serde_json::to_value(params).unwrap();
            assert_eq!(j["kind"], *expected_kind, "variant kind mismatch: {:?}", params);
            let back: MutateParams = serde_json::from_value(j).unwrap();
            // 能 round-trip 即可;不断言完全相等(float / String clone)
            let back_kind = serde_json::to_value(&back).unwrap();
            assert_eq!(back_kind["kind"], *expected_kind);
        }
    }

    /// `MutateResponse` wire 形状锁
    #[test]
    fn test_mutate_response_create_has_entity_id() {
        let resp = MutateResponse {
            success: true,
            entity_id: Some("sec_abc12345".to_string()),
            message: None,
        };
        let j = serde_json::to_value(&resp).unwrap();
        assert_eq!(j["success"], true);
        assert_eq!(j["entity_id"], "sec_abc12345");
        assert_eq!(j["message"], serde_json::Value::Null);
    }

    /// `MutateResponse` update 类 entity_id 为 null
    #[test]
    fn test_mutate_response_update_entity_id_is_null() {
        let resp = MutateResponse {
            success: true,
            entity_id: None,
            message: None,
        };
        let j = serde_json::to_value(&resp).unwrap();
        assert_eq!(j["entity_id"], serde_json::Value::Null);
    }
}
