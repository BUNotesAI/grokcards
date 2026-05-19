use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::alias::{AliasStore, SqliteAliasStore};
use crate::modules::keysight::domain::id::{AliasId, CardId, WhiteboardId};
use crate::modules::keysight::models::CardAlias;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

/// 按 ID 查询单个 alias。
#[tauri::command]
#[specta::specta]
pub fn alias_get(state: State<'_, KeysightRuntimeState>, id: AliasId) -> Result<CardAlias, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 alias。
#[tauri::command]
#[specta::specta]
pub fn alias_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
) -> Result<Vec<CardAlias>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:alias_query_all");
    let conn = lock_db(&state.core.db, "alias_query_all");
    let store = SqliteAliasStore::new(&conn);
    store.query_all(&whiteboard_id).map_err(Into::into)
}

/// # alias_create
///
/// ## 前置条件
/// - card_id 对应的 card 必须存在
///
/// ## 执行效果
/// 1. 生成 alias_ 前缀的唯一 ID
/// 2. 插入 entities 表（kind=alias）+ alias_fields 表
///
/// ## 幂等性
/// 非幂等 — 每次调用创建新 alias
///
/// ## 关联操作
/// - [`alias_delete`] — 删除（逆操作）
#[tauri::command]
#[specta::specta]
pub fn alias_create(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
    card_id: CardId,
) -> Result<CardAlias, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.create(&whiteboard_id, &card_id).map_err(Into::into)
}

/// # alias_delete
///
/// ## 前置条件
/// - id 对应的 alias 必须存在
///
/// ## 执行效果
/// 1. 删除 entities 表中的 alias 行
/// 2. 删除 alias_fields 中该 alias 的行
/// 3. 删除 edges 中该 alias 的所有边
///
/// ## 幂等性
/// 幂等 — 已删除的 alias 再次删除报 NotFound
///
/// ## 关联操作
/// - [`alias_create`] — 创建（逆操作）
#[tauri::command]
#[specta::specta]
pub fn alias_delete(state: State<'_, KeysightRuntimeState>, id: AliasId) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.delete(&id).map_err(Into::into)
}
