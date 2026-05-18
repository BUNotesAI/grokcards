//! 写命令分派器 —— `handle_rpc` mutate 分支的 core logic。
//!
//! `MutateParams` → 对应 `{Store}::fn` → 成功后通过 `EventEmitter` 发
//! `"entity:changed"` event。
//!
//! ## 设计动机
//!
//! - **EventEmitter trait 注入**:production 用 `TauriEmitter` 包 `tauri::AppHandle`,
//!   测试用 `RecordingEmitter` 记录 call,不需要 spin Tauri runtime。
//! - **纯函数签名**(`&Connection + &dyn VaultFs + &dyn EventEmitter`):脱离 axum /
//!   tauri 也可直接测,对齐 "Test real code paths(anti test theater)" 原则。
//! - **变体对 Store fn 的 1:1 映射**:match 穷尽,新增 variant 必须补 case(编译器强制)。
//! - **文件 sync 严格对齐 Tauri command**:`entity_connect` / `entity_disconnect` 的
//!   Edge 变体穷尽 match 在这里复刻,保证 CLI 写和 UI 写的副作用一致。

use keysight_core::domain::alias::{AliasStore, SqliteAliasStore};
use keysight_core::domain::card::SqliteCardStore;
use keysight_core::domain::edge::{user_draw_edge, Edge, EntityId};
use keysight_core::domain::entity::{EntityGraph, SqliteEntityGraph};
use keysight_core::domain::id::{CardId, WhiteboardId};
use keysight_core::domain::layout::{LayoutStore, SqliteLayoutStore};
use keysight_core::domain::note::{NoteStore, SqliteNoteStore};
use keysight_core::domain::question;
use keysight_core::domain::section::{SectionStore, SqliteSectionStore};
use keysight_core::errors::KeysightError;
use keysight_core::ipc::{MutateParams, MutateResponse};
use keysight_core::models::EdgeType;
use keysight_core::vault_fs::VaultFs;
use rusqlite::Connection;
use serde_json::{json, Value};

/// 事件发射契约 —— production 包 `tauri::AppHandle`,tests 注入 recorder。
///
/// `Send + Sync` 因为要放进 `Arc<dyn EventEmitter>` 跨线程共享(axum handler 多
/// 并发 connection 共用同一 emitter)。
pub(super) trait EventEmitter: Send + Sync {
    fn emit(&self, event: &str, payload: &Value);
}

/// Production 实现:包装 `tauri::AppHandle`。emit 失败不影响 dispatch 成功
/// (emit 失败只是 listener 未注册,非业务错误)。
pub(super) struct TauriEmitter {
    pub(super) app: tauri::AppHandle,
}

impl EventEmitter for TauriEmitter {
    fn emit(&self, event: &str, payload: &Value) {
        use tauri::Emitter;
        if let Err(e) = self.app.emit(event, payload) {
            eprintln!("[keysight] emit {event} failed: {e}");
        }
    }
}

/// 分派 `MutateParams` 到对应 Store,成功后发 `"entity:changed"`。
///
/// ## 前置条件
/// - `conn` 是 keysight SQLite 的写连接(`handle_rpc` 里已 lock 过 `Arc<Mutex<Connection>>`)
/// - `vault_fs` 是 vault 根目录对应的 `RealVaultFs`(或测试 mock)
/// - `emitter` 在 dispatch 成功时被调 **恰好一次** `emit("entity:changed", payload)`
///
/// ## 执行效果
/// 1. match variant → 调对应 `{Store}::fn` / `user_draw_edge` + `EntityGraph::connect`
/// 2. Connect / Disconnect 变体按 `Edge` / `EdgeType` 穷尽 match,sync 源 entity 的
///    md 文件(对齐 `entity_connect` / `entity_disconnect` Tauri command)
/// 3. 成功后 emit `"entity:changed"`,payload 含 `kind` + (新增 id / 修改 id 信息)
///
/// ## 不做的事
/// - 不处理 HTTP / auth / 反序列化错误(由 `handle_rpc` 上层负责)
/// - 不管理 Connection lock(由 caller 负责)
/// - 失败时**不** emit 事件(避免 UI 因为 DB 没落地但事件到了做错误刷新)
///
/// ## 幂等性
/// 遵循各 Store fn 的幂等性定义(`SectionCreate` 非幂等,`SectionAdd` 幂等等)。
pub(super) fn dispatch_mutate(
    params: MutateParams,
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    emitter: &dyn EventEmitter,
) -> Result<MutateResponse, KeysightError> {
    let (entity_id, emit_payload) = match params {
        MutateParams::SectionCreate { wb, title, color } => {
            op_section_create(conn, &wb, &title, color.as_deref())?
        }
        MutateParams::NoteCreate { wb, title, content, color } => {
            op_note_create(conn, vault_fs, &wb, &title, content.as_deref(), color.as_deref())?
        }
        MutateParams::NoteUpdate { id, title, content, color } => {
            op_note_update(conn, vault_fs, id, title.as_deref(), content.as_deref(), color.as_deref())?
        }
        MutateParams::AliasCreate { wb, card_id } => {
            op_alias_create(conn, &wb, &card_id)?
        }
        MutateParams::SetPos { wb, entity_id, x, y } => {
            op_set_pos(conn, &wb, &entity_id, x, y)?
        }
        MutateParams::Connect { from, to } => {
            op_connect(conn, vault_fs, from, to)?
        }
        MutateParams::Disconnect { from, to, edge_type } => {
            op_disconnect(conn, vault_fs, from, to, edge_type)?
        }
        MutateParams::SectionAdd { section_id, entity_id } => {
            op_section_add(conn, &section_id, &entity_id)?
        }
        MutateParams::SectionMove { section_id, target_wb } => {
            op_section_move(conn, &section_id, &target_wb)?
        }
    };

    // 所有 variant 成功后统一 emit。失败路径(? propagate)不 emit,前端不会错误刷新。
    emitter.emit("entity:changed", &emit_payload);

    Ok(MutateResponse {
        success: true,
        entity_id,
        message: None,
    })
}

/// 每个 op_* helper 的返回:`(新增 entity_id, emit_payload)`。
/// `entity_id = Some` 仅 Create 类 variant;Update / 关系操作返 `None`。
type OpResult = Result<(Option<String>, Value), KeysightError>;

fn op_section_create(conn: &Connection, wb: &WhiteboardId, title: &str, color: Option<&str>) -> OpResult {
    let sec = SqliteSectionStore::new(conn).create(wb, title, color)?;
    let id = sec.id.clone();
    Ok((Some(id.clone()), json!({ "kind": "section", "id": id, "wb": wb.as_str() })))
}

fn op_note_create(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    wb: &WhiteboardId,
    title: &str,
    content: Option<&str>,
    color: Option<&str>,
) -> OpResult {
    let note = SqliteNoteStore::with_vault_fs(conn, vault_fs).create(wb, title, content, color)?;
    let id = note.id.clone();
    Ok((Some(id.clone()), json!({ "kind": "note", "id": id, "wb": wb.as_str() })))
}

fn op_note_update(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    id: String,
    title: Option<&str>,
    content: Option<&str>,
    color: Option<&str>,
) -> OpResult {
    SqliteNoteStore::with_vault_fs(conn, vault_fs).update(&id, title, content, color)?;
    Ok((None, json!({ "kind": "note", "id": id })))
}

fn op_alias_create(conn: &Connection, wb: &WhiteboardId, card_id: &CardId) -> OpResult {
    let alias = SqliteAliasStore::new(conn).create(wb, card_id)?;
    let id = alias.alias_id.clone();
    Ok((
        Some(id.clone()),
        json!({ "kind": "alias", "id": id, "wb": wb.as_str(), "card_id": card_id.as_str() }),
    ))
}

fn op_set_pos(conn: &Connection, wb: &WhiteboardId, entity_id: &str, x: f64, y: f64) -> OpResult {
    SqliteLayoutStore::new(conn).set_position(wb, entity_id, x, y)?;
    Ok((
        None,
        json!({ "kind": "position", "entity_id": entity_id, "wb": wb.as_str(), "x": x, "y": y }),
    ))
}

fn op_connect(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    from: String,
    to: String,
) -> OpResult {
    // Edge variant 由 `EntityId::parse(from)` 的 kind 决定(Card/Note/Alias/Question →
    // 对应 `*Link`;Section/Task → `ConnectionNotAllowed`)。wire 不传 edge_type:
    // 类型已强制保证唯一合法形状。
    let from_id = EntityId::parse(&from)
        .map_err(|e| KeysightError::ParseError(format!("from_id 解析失败: {e}")))?;
    let to_id = EntityId::parse(&to)
        .map_err(|e| KeysightError::ParseError(format!("to_id 解析失败: {e}")))?;
    let edge = user_draw_edge(from_id, to_id)?;
    SqliteEntityGraph::new(conn).connect(&edge)?;

    sync_source_file_for_edge(conn, vault_fs, &edge)?;

    Ok((None, json!({ "kind": "edge", "op": "connect", "from": from, "to": to })))
}

/// 文件 sync —— 对齐 entity_connect 的 Edge 变体穷尽 match(硬约束:禁 `_` 通配,踩坑样例 1)
fn sync_source_file_for_edge(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    edge: &Edge,
) -> Result<(), KeysightError> {
    match edge {
        Edge::CardLink { from, .. }
        | Edge::CardRelated { from, .. }
        | Edge::CardSeeAlso { from, .. } => {
            SqliteCardStore::with_vault_fs(conn, vault_fs).sync_edges_to_file(from)?;
        }
        Edge::NoteLink { from, .. } | Edge::NoteSeeAlso { from, .. } => {
            SqliteNoteStore::with_vault_fs(conn, vault_fs).sync_links_to_file(from.as_str())?;
        }
        Edge::AliasLink { .. } => {
            // alias 无独立文件内容(继承 owning card),不 sync
        }
        Edge::QuestionLink { from, .. } => {
            question::sync_links_to_file(conn, vault_fs, from.as_str())?;
        }
        Edge::CardToAlias { .. } => {
            // alias 定义关系反查路径,不单独写回 card file
        }
    }
    Ok(())
}

fn op_disconnect(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    from: String,
    to: String,
    edge_type: String,
) -> OpResult {
    let et = EdgeType::from_db_str(&edge_type)
        .ok_or_else(|| KeysightError::ParseError(format!("未知 edge_type: {edge_type}")))?;
    SqliteEntityGraph::new(conn).disconnect(&from, &to, et)?;

    // 文件 sync —— 对齐 entity_disconnect(stringly-typed 旧 API 保留形状)
    if from.starts_with("card_")
        && matches!(et, EdgeType::LinkTo | EdgeType::Related | EdgeType::SeeAlso)
    {
        // 同 commands::entity_disconnect:prefix-checked,跨 crate parse 重校验,
        // O(starts_with) 开销可忽略,不开放 new_unchecked API 表面;失败走 propagate。
        let card_id = CardId::parse(from.clone()).map_err(|e| {
            KeysightError::ParseError(format!("内部不变量:from 已 prefix-check 为 card_ 但 parse 失败: {e}"))
        })?;
        SqliteCardStore::with_vault_fs(conn, vault_fs).sync_edges_to_file(&card_id)?;
    } else if from.starts_with("note_") && et == EdgeType::NoteLink {
        SqliteNoteStore::with_vault_fs(conn, vault_fs).sync_links_to_file(&from)?;
    }

    Ok((
        None,
        json!({ "kind": "edge", "op": "disconnect", "from": from, "to": to, "edge_type": edge_type }),
    ))
}

fn op_section_add(conn: &Connection, section_id: &str, entity_id: &str) -> OpResult {
    SqliteSectionStore::new(conn).add_member(section_id, entity_id)?;
    Ok((
        None,
        json!({ "kind": "section_member", "op": "add", "section_id": section_id, "entity_id": entity_id }),
    ))
}

fn op_section_move(conn: &Connection, section_id: &str, target_wb: &WhiteboardId) -> OpResult {
    SqliteSectionStore::new(conn).move_to_whiteboard(section_id, target_wb)?;
    Ok((
        None,
        json!({ "kind": "section", "id": section_id, "wb": target_wb.as_str() }),
    ))
}

// -----------------------------------------------------------------------------
// 测试工具 —— RecordingEmitter 给单元 + 集成测试共用
// -----------------------------------------------------------------------------

#[cfg(test)]
pub(super) struct RecordingEmitter {
    events: std::sync::Mutex<Vec<(String, Value)>>,
}

#[cfg(test)]
impl RecordingEmitter {
    pub(super) fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub(super) fn events(&self) -> Vec<(String, Value)> {
        self.events.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl EventEmitter for RecordingEmitter {
    fn emit(&self, event: &str, payload: &Value) {
        self.events
            .lock()
            .unwrap()
            .push((event.to_string(), payload.clone()));
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use keysight_core::db::init_db;
    use keysight_core::vault_fs::RealVaultFs;

    /// 构造一个带 schema + `wb_root` whiteboard seed 的 in-memory 连接
    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id) VALUES ('wb_root', 'whiteboard', 'Root', 'wb_root')",
            [],
        )
        .unwrap();
        conn
    }

    /// `dispatch_mutate(MutateParams::SectionCreate)` → SQLite 新增 `sec_*` 行 +
    /// `MutateResponse { success: true, entity_id: Some(sec_*) }`。
    #[test]
    fn test_mutate_dispatcher_routes_params_to_store() {
        let conn = test_conn();
        let tmp = tempfile::tempdir().unwrap();
        let fs = RealVaultFs::new(tmp.path().to_string_lossy().to_string());
        let emitter = RecordingEmitter::new();

        let params = MutateParams::SectionCreate {
            wb: WhiteboardId::parse("wb_root").unwrap(),
            title: "TestSec".to_string(),
            color: None,
        };

        let resp = dispatch_mutate(params, &conn, &fs, &emitter)
            .expect("dispatch_mutate 应返回 Ok");

        assert!(resp.success, "success flag 应为 true");
        let id = resp.entity_id.expect("create 类 variant 应返 entity_id");
        assert!(id.starts_with("sec_"), "section id 应以 sec_ 开头,实际 {id}");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE id = ?1 AND kind = 'section' AND title = 'TestSec'",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "DB 应有且仅有 1 行对应的 section entity");
    }

    /// dispatch_mutate 成功后必须 emit 恰好 1 次 `"entity:changed"`,
    /// payload 含 entity 的 kind + id。
    #[test]
    fn test_mutate_dispatcher_emits_entity_changed() {
        let conn = test_conn();
        let tmp = tempfile::tempdir().unwrap();
        let fs = RealVaultFs::new(tmp.path().to_string_lossy().to_string());
        let emitter = RecordingEmitter::new();

        let params = MutateParams::SectionCreate {
            wb: WhiteboardId::parse("wb_root").unwrap(),
            title: "EventSec".to_string(),
            color: None,
        };

        let resp = dispatch_mutate(params, &conn, &fs, &emitter)
            .expect("dispatch_mutate 应返回 Ok");

        let events = emitter.events();
        assert_eq!(
            events.len(),
            1,
            "应 emit 恰好 1 次 event,实际 {}",
            events.len()
        );
        let (name, payload) = &events[0];
        assert_eq!(name, "entity:changed", "event 名应为 entity:changed");
        assert_eq!(payload["kind"], "section", "payload.kind 应为 section");
        let id = resp.entity_id.unwrap();
        assert_eq!(payload["id"], id, "payload.id 应与 response.entity_id 一致");
    }

    /// dispatch_mutate 失败路径:NotFound 类错误应 propagate 且**不** emit 事件。
    #[test]
    fn test_mutate_dispatcher_does_not_emit_on_failure() {
        let conn = test_conn();
        let tmp = tempfile::tempdir().unwrap();
        let fs = RealVaultFs::new(tmp.path().to_string_lossy().to_string());
        let emitter = RecordingEmitter::new();

        // NoteUpdate 用不存在的 id,内部 current_snapshot 会 NotFound
        let params = MutateParams::NoteUpdate {
            id: "note_deadbeef".to_string(),
            title: Some("X".to_string()),
            content: None,
            color: None,
        };

        let result = dispatch_mutate(params, &conn, &fs, &emitter);
        assert!(result.is_err(), "不存在的 note 应返 Err,实际 {:?}", result);
        assert!(
            emitter.events().is_empty(),
            "失败路径不应 emit 事件,实际 {:?}",
            emitter.events()
        );
    }
}
