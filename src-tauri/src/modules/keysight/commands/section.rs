use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::edge::EntityId;
use crate::modules::keysight::domain::id::{SectionId, WhiteboardId};
use crate::modules::keysight::domain::section::{SectionStore, SqliteSectionStore};
use crate::modules::keysight::models::GraphSection;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

/// 按 ID 查询单个 section。
#[tauri::command]
#[specta::specta]
pub fn section_get(
    state: State<'_, KeysightRuntimeState>,
    id: SectionId,
) -> Result<GraphSection, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 section。
#[tauri::command]
#[specta::specta]
pub fn section_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
) -> Result<Vec<GraphSection>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:section_query_all");
    let conn = lock_db(&state.core.db, "section_query_all");
    let store = SqliteSectionStore::new(&conn);
    store.query_all(&whiteboard_id).map_err(Into::into)
}

/// # section_create
///
/// ## 前置条件
/// - title 非空
///
/// ## 执行效果
/// 1. 生成 sec_ 前缀的唯一 ID
/// 2. 插入 entities 表（kind=section）
///
/// ## 不做的事
/// - 不自动添加成员（需调用 section_add_member）
/// - 不设置位置（需调用 layout_set_position）
///
/// ## 幂等性
/// 非幂等 — 每次调用创建新 section
///
/// ## 关联操作
/// - [`section_add_member`] — 添加成员
/// - [`section_delete`] — 删除
#[tauri::command]
#[specta::specta]
pub fn section_create(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
    title: String,
    color: Option<String>,
) -> Result<GraphSection, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .create(&whiteboard_id, &title, color.as_deref())
        .map_err(Into::into)
}

/// # section_delete
///
/// ## 前置条件
/// - id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 删除 entities 表中的 section 行
/// 2. 删除 section_members 中该 section 的所有成员关系
/// 3. 删除 edges 中该 section 的所有边
/// 4. 删除 positions 中该 section 的位置
///
/// ## 不做的事
/// - 不删除成员实体本身（只删除成员关系）
///
/// ## 幂等性
/// 幂等 — 已删除的 section 再次删除不报错（NotFound）
///
/// ## 关联操作
/// - [`section_create`] — 创建（逆操作）
#[tauri::command]
#[specta::specta]
pub fn section_delete(state: State<'_, KeysightRuntimeState>, id: SectionId) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store.delete(&id).map_err(Into::into)
}

/// # section_update
///
/// ## 前置条件
/// - id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 更新 entities 表中 title 和/或 color（仅更新提供的字段）
///
/// ## 不做的事
/// - 不修改成员列表
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`section_create`] — 创建
#[tauri::command]
#[specta::specta]
pub fn section_update(
    state: State<'_, KeysightRuntimeState>,
    id: SectionId,
    title: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .update(&id, title.as_deref(), color.as_deref())
        .map_err(Into::into)
}

/// # section_add_member
///
/// ## 前置条件
/// - section_id 和 entity_id 都必须存在
///
/// ## 执行效果
/// 1. 插入 section_members 表
///
/// ## 不做的事
/// - 不设置成员位置
///
/// ## 幂等性
/// 幂等 — 重复添加同一成员无额外效果（INSERT OR IGNORE）
///
/// ## 关联操作
/// - [`section_remove_member`] — 移除成员（逆操作）
#[tauri::command]
#[specta::specta]
pub fn section_add_member(
    state: State<'_, KeysightRuntimeState>,
    section_id: SectionId,
    entity_id: EntityId,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store.add_member(&section_id, &entity_id).map_err(Into::into)
}

/// # section_remove_member
///
/// ## 前置条件
/// - section_id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 删除 section_members 表中该行
///
/// ## 幂等性
/// 幂等 — 不存在的成员关系删除无效果
///
/// ## 关联操作
/// - [`section_add_member`] — 添加成员（逆操作）
#[tauri::command]
#[specta::specta]
pub fn section_remove_member(
    state: State<'_, KeysightRuntimeState>,
    section_id: SectionId,
    entity_id: EntityId,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .remove_member(&section_id, &entity_id)
        .map_err(Into::into)
}

/// # section_move_to_whiteboard
///
/// ## 前置条件
/// - section_id 对应的 section 必须存在
///
/// ## 执行效果
/// 1. 更新 entities 表中 section 的 whiteboard_id
/// 2. 清理 positions 表中旧白板的位置
/// 3. 清理跨白板的 section_link edges
///
/// ## 不做的事
/// - 不移动 section 的成员实体
/// - 不在目标白板设置位置
///
/// ## 幂等性
/// 幂等 — 移动到同一白板无额外效果
///
/// ## 关联操作
/// - [`section_create`] — 创建新 section
/// - [`crate::modules::keysight::commands::layout_set_position`] — 在目标白板设置位置
#[tauri::command]
#[specta::specta]
pub fn section_move_to_whiteboard(
    state: State<'_, KeysightRuntimeState>,
    section_id: SectionId,
    target_whiteboard_id: WhiteboardId,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .move_to_whiteboard(&section_id, &target_whiteboard_id)
        .map_err(Into::into)
}
