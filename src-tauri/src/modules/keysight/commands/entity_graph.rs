use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::card::SqliteCardStore;
use crate::modules::keysight::domain::edge::{user_draw_edge, Edge, EntityId};
use crate::modules::keysight::domain::entity::{EntityGraph, SqliteEntityGraph};
use crate::modules::keysight::domain::id::CardId;
use crate::modules::keysight::domain::note::SqliteNoteStore;
use crate::modules::keysight::domain::question;
use crate::modules::keysight::models::{EdgeRow, EdgeType};
use crate::modules::keysight::recording_vault_fs::RecordingVaultFs;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;

/// 查询指定实体的出边。
#[tauri::command]
#[specta::specta]
pub fn entity_edges_from(
    state: State<'_, KeysightRuntimeState>,
    entity_id: EntityId,
) -> Result<Vec<EdgeRow>, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph.edges_from(&entity_id).map_err(Into::into)
}

/// 查询指定实体的入边。
#[tauri::command]
#[specta::specta]
pub fn entity_edges_to(
    state: State<'_, KeysightRuntimeState>,
    entity_id: EntityId,
) -> Result<Vec<EdgeRow>, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph.edges_to(&entity_id).map_err(Into::into)
}

/// # entity_connect
///
/// 「用户从画布节点 A 画箭头到节点 B」的强类型入口。替代旧的 5 参数 stringly
/// typed 版本(edge_type / style / label 参数已整体退役)。
///
/// ## 前置条件
/// - from_id / to_id 均为 [`EntityId`](W5 后 IPC 边界 try_from 已校验 prefix,
///   非法 id 不会到达本函数)
/// - from 不能是 section / task (业务规则:这两类不主动发边),否则返
///   `KeysightError::ConnectionNotAllowed`
///
/// ## 执行效果
/// 1. [`user_draw_edge`] 派发为具体 [`Edge`] 变体(CardLink/NoteLink/AliasLink/
///    QuestionLink)
/// 2. [`SqliteEntityGraph::connect`] 落 DB
/// 3. 按 edge 变体穷尽 match 决定是否需要同步 source 的 md 文件
///
/// ## 不做的事
/// - 不处理 Related picker (用 [`entity_relate`])
/// - 不处理 SeeAlso 创建 (子阶段 2b 或后续 feature 单独添加 command)
///
/// ## 幂等性
/// 幂等 — 相同边重复插入无额外效果(主键约束)
///
/// ## 关联操作
/// - [`entity_relate`] — 建立 card→card Related 关系
/// - [`entity_disconnect`] — 删除边(逆操作)
#[tauri::command]
#[specta::specta]
pub fn entity_connect(
    state: State<'_, KeysightRuntimeState>,
    from_id: EntityId,
    to_id: EntityId,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();

    let edge = user_draw_edge(from_id, to_id).map_err(AppError::from)?;

    let graph = SqliteEntityGraph::new(&conn);
    graph.connect(&edge).map_err(AppError::from)?;

    // 文件同步路由:按 Edge 变体穷尽 match 派发(防火墙原则 — 禁止 _ 通配)
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    match &edge {
        Edge::CardLink { from, .. }
        | Edge::CardRelated { from, .. }
        | Edge::CardSeeAlso { from, .. } => {
            let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
            store.sync_edges_to_file(from).map_err(AppError::from)?;
        }
        Edge::NoteLink { from, .. } | Edge::NoteSeeAlso { from, .. } => {
            let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
            store.sync_links_to_file(from).map_err(AppError::from)?;
        }
        Edge::AliasLink { .. } => {
            // alias 无独立文件内容(继承 owning card),不 sync
        }
        Edge::QuestionLink { from, .. } => {
            question::sync_links_to_file(&conn, &vault_fs, from).map_err(AppError::from)?;
        }
        Edge::CardToAlias { .. } => {
            // alias 定义关系的反查路径,不单独写回 card file
        }
    }

    Ok(())
}

/// # entity_relate
///
/// 「用户在 Related picker 里选了目标 card」的强类型入口。与 [`entity_connect`]
/// 不同,本命令只接受 card → card 关系,建立 [`Edge::CardRelated`] 边(DB
/// edge_type = `related`)。
///
/// ## 前置条件
/// - from_card_id / to_card_id 必须都是 `card_*` 前缀的合法 id
///
/// ## 执行效果
/// 1. 构造 [`Edge::CardRelated`] 并 [`SqliteEntityGraph::connect`] 落 DB
/// 2. 同步 source card 的 md 文件
///
/// ## 幂等性
/// 幂等 — 主键约束
///
/// ## 关联操作
/// - [`entity_connect`] — 建立 LinkTo 类 edge(不同意图,用于 ⋯ 菜单的
///   Draw connection)
#[tauri::command]
#[specta::specta]
pub fn entity_relate(
    state: State<'_, KeysightRuntimeState>,
    from_card_id: CardId,
    to_card_id: CardId,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();

    // 类型已强制保证 card → card,不需要 EntityId::parse 反推 + match 排错。
    let edge = Edge::CardRelated {
        from: from_card_id.clone(),
        to: to_card_id,
    };
    let graph = SqliteEntityGraph::new(&conn);
    graph.connect(&edge).map_err(AppError::from)?;

    // Related 是 card→card,source card 需要同步文件
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.sync_edges_to_file(&from_card_id).map_err(AppError::from)?;

    Ok(())
}

/// # entity_disconnect
///
/// ## 前置条件
/// - 无
///
/// ## 执行效果
/// 1. 删除 edges 表中匹配的行
///
/// ## 幂等性
/// 幂等 — 不存在则无效果
///
/// ## 关联操作
/// - [`entity_connect`] — 创建边（逆操作）
#[tauri::command]
#[specta::specta]
pub fn entity_disconnect(
    state: State<'_, KeysightRuntimeState>,
    from_id: EntityId,
    to_id: EntityId,
    edge_type: EdgeType,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph
        .disconnect(&from_id, &to_id, edge_type)
        .map_err(AppError::from)?;

    // 文件 sync —— W5 后直接 match EntityId variant 拿强类型 inner id,删去原
    // `starts_with` + 跨 crate `CardId::parse` 重校验(那两步等价于 IPC 边界已
    // 做过的校验,且 new_unchecked 仍是 pub(crate),不外泄)。
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    match (&from_id, edge_type) {
        (EntityId::Card(card_id), EdgeType::LinkTo)
        | (EntityId::Card(card_id), EdgeType::Related)
        | (EntityId::Card(card_id), EdgeType::SeeAlso) => {
            let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
            store.sync_edges_to_file(card_id).map_err(AppError::from)?;
        }
        (EntityId::Note(note_id), EdgeType::NoteLink) => {
            let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
            store.sync_links_to_file(note_id).map_err(AppError::from)?;
        }
        // 例外:其余 (EntityId, EdgeType) 组合无独立文件同步副作用:
        //   - Alias 继承 owning card,无独立 md 文件
        //   - Section / Task 不主动发边(业务规则),不会进入 disconnect
        //   - Question 的文件同步走 question::sync_links_to_file 独立路径,
        //     由 entity_connect 路径处理,非 disconnect 副作用
        //   - Card / Note 的其他 EdgeType 在 connect 路径由 user_draw_edge
        //     编译期穷尽,生产路径不会出现非 (Card, LinkTo|Related|SeeAlso) /
        //     (Note, NoteLink) 的组合
        // 未来若 EntityId 新增 variant 或 EdgeType 加新业务行为,需评估是否补 arm。
        _ => {}
    }

    Ok(())
}
