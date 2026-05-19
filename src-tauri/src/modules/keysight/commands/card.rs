use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::card::{CardStore, SqliteCardStore};
use crate::modules::keysight::domain::id::CardId;
use crate::modules::keysight::models::{AtomicCard, CardLinksResponse};
use crate::modules::keysight::recording_vault_fs::RecordingVaultFs;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

// ============================================================
// Card 读操作
// ============================================================

/// 按 ID 查询单张卡片。
#[tauri::command]
#[specta::specta]
pub fn card_get(state: State<'_, KeysightRuntimeState>, id: CardId) -> Result<AtomicCard, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询所有卡片，按 mtime 降序，支持分页。
#[tauri::command]
#[specta::specta]
pub fn card_query_all(
    state: State<'_, KeysightRuntimeState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<AtomicCard>, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:card_query_all");
    let conn = lock_db(&state.core.db, "card_query_all");
    let store = SqliteCardStore::new(&conn);
    store.query_all(limit, offset).map_err(Into::into)
}

/// 按文件路径查询卡片。
#[tauri::command]
#[specta::specta]
pub fn card_query_by_file(
    state: State<'_, KeysightRuntimeState>,
    file_path: String,
) -> Result<Vec<AtomicCard>, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_by_file(&file_path).map_err(Into::into)
}

/// 按 ID 列表批量查询卡片。
#[tauri::command]
#[specta::specta]
pub fn card_query_by_ids(
    state: State<'_, KeysightRuntimeState>,
    ids: Vec<CardId>,
) -> Result<Vec<AtomicCard>, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_by_ids(&ids).map_err(Into::into)
}

/// 卡片总数。
#[tauri::command]
#[specta::specta]
pub fn card_count(state: State<'_, KeysightRuntimeState>) -> Result<i64, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.count().map_err(Into::into)
}

/// 全文搜索卡片（FTS5 + LIKE fallback）。
#[tauri::command]
#[specta::specta]
pub fn card_search(
    state: State<'_, KeysightRuntimeState>,
    text: String,
) -> Result<Vec<AtomicCard>, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.search(&text).map_err(Into::into)
}

/// 查询单卡片的完整链接图谱（出边 + 入边）。
#[tauri::command]
#[specta::specta]
pub fn card_query_links(
    state: State<'_, KeysightRuntimeState>,
    id: CardId,
) -> Result<CardLinksResponse, AppError> {
    let state = state.resolved()?;
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_links(&id).map_err(Into::into)
}

// ============================================================
// Card 写操作
// ============================================================

/// # card_edit_title
///
/// ## 前置条件
/// - id 对应的 card 必须存在（否则 NotFound）
/// - new_title trim 后非空（否则 EmptyTitle）
///
/// ## 执行效果
/// 1. 通过 VaultFs 读取 markdown 文件，替换 H1 标题行
/// 2. 通过 VaultFs 写回更新后的文件
/// 3. 更新 entities 表的 title 字段
///
/// ## 不做的事
/// - 不更新 FTS 索引（需 sync_file 触发）
/// - 不触发前端事件通知
///
/// ## 幂等性
/// 幂等 — 相同 title 重复调用无额外效果
///
/// ## 关联操作
/// - [`card_edit_body`] — 编辑正文
/// - [`crate::modules::keysight::commands::sync_file`] — 含 FTS 更新的全量同步
#[tauri::command]
#[specta::specta]
pub fn card_edit_title(
    state: State<'_, KeysightRuntimeState>,
    id: CardId,
    new_title: String,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:card_edit_title");
    let conn = lock_db(&state.core.db, "card_edit_title");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.edit_title(&id, &new_title).map_err(Into::into)
}

/// # card_edit_body
///
/// ## 前置条件
/// - id 对应的 card 必须存在（否则 NotFound）
///
/// ## 执行效果
/// 1. 通过 VaultFs 读取 markdown 文件
/// 2. 保留 frontmatter + H1，替换 H1 之后的正文
/// 3. 通过 VaultFs 写回更新后的文件
/// 4. 更新 entities 表的 content 字段
///
/// ## 不做的事
/// - 不更新 FTS 索引（需 sync_file 触发）
/// - 不修改 frontmatter 和标题
///
/// ## 幂等性
/// 幂等 — 相同 body 重复调用无额外效果
///
/// ## 关联操作
/// - [`card_edit_title`] — 编辑标题
/// - [`crate::modules::keysight::commands::sync_file`] — 含 FTS 更新的全量同步
#[tauri::command]
#[specta::specta]
pub fn card_edit_body(
    state: State<'_, KeysightRuntimeState>,
    id: CardId,
    new_body: String,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:card_edit_body");
    let conn = lock_db(&state.core.db, "card_edit_body");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.edit_body(&id, &new_body).map_err(Into::into)
}

/// # card_update_understanding
///
/// ## 前置条件
/// - id 对应的 card 必须存在（否则 NotFound）
///
/// ## 执行效果
/// 1. 通过 VaultFs 读取 markdown 文件
/// 2. 更新 frontmatter 中的 understanding 字段
/// 3. 通过 VaultFs 写回更新后的文件
/// 4. 更新 card_fields 表的 understanding 字段
///
/// ## 不做的事
/// - 不更新 FTS 索引
/// - 不修改正文和标题
///
/// ## 幂等性
/// 幂等 — 相同 text 重复调用无额外效果
///
/// ## 关联操作
/// - [`card_edit_title`] / [`card_edit_body`] — 其他卡片编辑操作
#[tauri::command]
#[specta::specta]
pub fn card_update_understanding(
    state: State<'_, KeysightRuntimeState>,
    id: CardId,
    text: String,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:card_update_understanding");
    let conn = lock_db(&state.core.db, "card_update_understanding");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.update_understanding(&id, &text).map_err(Into::into)
}

/// # card_set_color
///
/// ## 前置条件
/// - id 对应的 card 必须存在
///
/// ## 执行效果
/// 1. 读取 card 的 markdown 文件(通过 VaultFs)
/// 2. 在 frontmatter 中设置/清空 `color` 字段(serde_yaml 序列化,自动处理 hex)
/// 3. 写回文件
/// 4. 更新 entities.color 列
///
/// ## 参数
/// - `color == "default"` → 清空 color(移除 frontmatter 字段 + entities.color = NULL)
/// - 其他 → 设置 color 为该值
#[tauri::command]
#[specta::specta]
pub fn card_set_color(
    state: State<'_, KeysightRuntimeState>,
    id: CardId,
    color: String,
) -> Result<(), AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:card_set_color");
    let conn = lock_db(&state.core.db, "card_set_color");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.set_color(&id, &color).map_err(Into::into)
}
