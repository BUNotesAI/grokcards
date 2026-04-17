use std::collections::HashMap;

use rusqlite::Connection;
use tauri::State;

use super::domain::alias::{AliasStore, SqliteAliasStore};
use super::domain::card::{CardStore, SqliteCardStore};
use super::domain::edge::{user_draw_edge, Edge, EntityId};
use super::domain::entity::{EntityGraph, SqliteEntityGraph};
use super::domain::layout::{LayoutStore, SqliteLayoutStore};
use super::domain::legacy_import::{LegacyImporter, SqliteLegacyImporter, SqliteLegacyReader};
use super::domain::note::{NoteStore, SqliteNoteStore};
use super::domain::section::{SectionStore, SqliteSectionStore};
use super::domain::{overview, question, sync, task, whiteboard};
use super::models::{
    AtomicCard, CardAlias, CardLinksResponse, EdgeRow, EdgeType, GraphNote,
    GraphOverviewResponse, GraphSection, ImportSummary, NoteFileMigrationReport, Position,
    QuestionEntity, StatsResponse, Subtask, SyncFileResponse, SyncVaultReport, TaskEntity,
    TaskStatus, VaultInfoResponse, WhiteboardSummary,
};
use super::recording_vault_fs::RecordingVaultFs;
use super::runtime_state::KeysightRuntimeState;
use crate::app_error::AppError;
use crate::perf::{lock_db, ScopedTimer};

// ============================================================
// Card 读操作
// ============================================================

/// 按 ID 查询单张卡片。
#[tauri::command]
#[specta::specta]
pub fn card_get(state: State<'_, KeysightRuntimeState>, id: String) -> Result<AtomicCard, AppError> {
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
    ids: Vec<String>,
) -> Result<Vec<AtomicCard>, AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteCardStore::new(&conn);
    store.query_by_ids(&ids).map_err(Into::into)
}

/// 卡片总数。
#[tauri::command]
#[specta::specta]
pub fn card_count(state: State<'_, KeysightRuntimeState>) -> Result<i64, AppError> {
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
    id: String,
) -> Result<CardLinksResponse, AppError> {
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
/// - [`sync_file`] — 含 FTS 更新的全量同步
#[tauri::command]
#[specta::specta]
pub fn card_edit_title(
    state: State<'_, KeysightRuntimeState>,
    id: String,
    new_title: String,
) -> Result<(), AppError> {
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
/// - [`sync_file`] — 含 FTS 更新的全量同步
#[tauri::command]
#[specta::specta]
pub fn card_edit_body(
    state: State<'_, KeysightRuntimeState>,
    id: String,
    new_body: String,
) -> Result<(), AppError> {
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
    id: String,
    text: String,
) -> Result<(), AppError> {
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
    id: String,
    color: String,
) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:card_set_color");
    let conn = lock_db(&state.core.db, "card_set_color");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.set_color(&id, &color).map_err(Into::into)
}

// ============================================================
// Section
// ============================================================

/// 按 ID 查询单个 section。
#[tauri::command]
#[specta::specta]
pub fn section_get(
    state: State<'_, KeysightRuntimeState>,
    id: String,
) -> Result<GraphSection, AppError> {
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
    whiteboard_id: String,
) -> Result<Vec<GraphSection>, AppError> {
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
    whiteboard_id: String,
    title: String,
    color: Option<String>,
) -> Result<GraphSection, AppError> {
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
pub fn section_delete(state: State<'_, KeysightRuntimeState>, id: String) -> Result<(), AppError> {
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
    id: String,
    title: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
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
    section_id: String,
    entity_id: String,
) -> Result<(), AppError> {
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
    section_id: String,
    entity_id: String,
) -> Result<(), AppError> {
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
/// - [`layout_set_position`] — 在目标白板设置位置
#[tauri::command]
#[specta::specta]
pub fn section_move_to_whiteboard(
    state: State<'_, KeysightRuntimeState>,
    section_id: String,
    target_whiteboard_id: String,
) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteSectionStore::new(&conn);
    store
        .move_to_whiteboard(&section_id, &target_whiteboard_id)
        .map_err(Into::into)
}

// ============================================================
// Task
// ============================================================

/// 查询指定白板的所有 task。
#[tauri::command]
#[specta::specta]
pub fn task_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
) -> Result<Vec<TaskEntity>, AppError> {
    let _t = ScopedTimer::new("cmd:task_query_all");
    let conn = lock_db(&state.core.db, "task_query_all");
    task::query_all(&conn, &whiteboard_id).map_err(Into::into)
}

/// 查询 kanban view 数据 —— 跨项目或单项目 task list。
#[tauri::command]
#[specta::specta]
pub fn task_query_kanban(
    state: State<'_, KeysightRuntimeState>,
    project: Option<String>,
) -> Result<Vec<TaskEntity>, AppError> {
    let _t = ScopedTimer::new("cmd:task_query_kanban");
    let conn = lock_db(&state.core.db, "task_query_kanban");
    let project_name = match project {
        Some(p) => Some(task::ProjectName::new(&p).map_err(Into::<AppError>::into)?),
        None => None,
    };
    task::query_kanban(&conn, project_name.as_ref()).map_err(Into::into)
}

/// 创建新 task,写 markdown 文件 + 同步 DB。
///
/// ## 前置条件
/// - `project` 不能为空且不含路径分隔符(由 ProjectName 校验)
/// - `title` trim 后不能为空
///
/// ## 执行效果
/// 1. ProjectName::new 校验 project
/// 2. 调 domain::task::create —— 生成 task_id、渲染 markdown、写
///    `whiteboard/projects/{project}/{id} 【TASK】{title}.md`、sync 回 DB
/// 3. 返回新 TaskEntity
#[tauri::command]
#[specta::specta]
pub fn task_create(
    state: State<'_, KeysightRuntimeState>,
    project: String,
    title: String,
    content: Option<String>,
    status: TaskStatus,
    area: Option<String>,
    color: Option<String>,
) -> Result<TaskEntity, AppError> {
    let _t = ScopedTimer::new("cmd:task_create");
    let conn = lock_db(&state.core.db, "task_create");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    let project_name =
        task::ProjectName::new(&project).map_err(Into::<AppError>::into)?;
    task::create(
        &conn,
        &vault_fs,
        &project_name,
        task::TaskCreateInput {
            title: &title,
            content: content.as_deref(),
            status,
            area: area.as_deref(),
            color: color.as_deref(),
        },
    )
    .map_err(Into::into)
}

/// 更新已有 task 的任意字段(title / content / status / area / color),
/// None 保留 current。color "default" sentinel 清空。
#[tauri::command]
#[specta::specta]
pub fn task_update(
    state: State<'_, KeysightRuntimeState>,
    id: String,
    title: Option<String>,
    content: Option<String>,
    status: Option<TaskStatus>,
    area: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:task_update");
    let conn = lock_db(&state.core.db, "task_update");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    task::update(
        &conn,
        &vault_fs,
        &id,
        task::TaskUpdateInput {
            title: title.as_deref(),
            content: content.as_deref(),
            status,
            area: area.as_deref(),
            color: color.as_deref(),
        },
    )
    .map_err(Into::into)
}

/// 删除 task —— 文件 + DB 级联。
#[tauri::command]
#[specta::specta]
pub fn task_delete(state: State<'_, KeysightRuntimeState>, id: String) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:task_delete");
    let conn = lock_db(&state.core.db, "task_delete");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    task::delete(&conn, &vault_fs, &id).map_err(Into::into)
}

/// 设置或清空 task 背景色 —— 便捷命令,等价于 task_update 只传 color。
///
/// - `color == "default"` → 清空 color(和 note/card 的 "default" sentinel 一致)
/// - 其他 → 设置 color
#[tauri::command]
#[specta::specta]
pub fn task_set_color(
    state: State<'_, KeysightRuntimeState>,
    id: String,
    color: String,
) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:task_set_color");
    let conn = lock_db(&state.core.db, "task_set_color");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    task::update(
        &conn,
        &vault_fs,
        &id,
        task::TaskUpdateInput {
            title: None,
            content: None,
            status: None,
            area: None,
            color: Some(&color),
        },
    )
    .map_err(Into::into)
}

/// V1.1 Kanban Subtask 专用写入命令 —— 同时更新 task 元数据和 checklist 子任务。
///
/// ## 前置条件
/// - `id` 对应的 task 必须存在(否则返回 `AppError::Keysight` NotFound)
/// - `subtasks` 中每项的 text 非空(TS 侧应已过滤;Rust 侧 render 时非空保证)
/// - 被编辑的 task body 的 checklist 必须是**单连续 block**
///
/// ## 执行效果
/// 1. `task::get` 读当前 task(拿完整 body 作为 merge base)
/// 2. `task::render_subtasks_into_body(&id, &current.content, &subtasks)` 把新
///    subtasks 合并进原 body,保留所有非 checklist 行的原位置(只替换 checklist
///    行)。多 block 时 fail-closed 返回 `KeysightError::MultiBlockChecklist`,
///    经 `From<KeysightError> for AppError` 转成 `AppError::MultiBlockChecklist`
///    透传到 TS
/// 3. 调 `task::update(...)` 把新 body 作为 content 写入(复用现有文件重写 +
///    sync_file + 可能的 rename + file_mtimes 更新)
/// 4. `task::get` 返回 fresh TaskEntity(含重新 parse 的 subtasks)
///
/// ## 不做的事
/// - 不允许改 project(`task::update` 的契约)
/// - 不直接修改 body 的非 checklist 部分(用户要改自由文本,V1.1 得直接编辑
///   markdown 文件;V1.2 计划加 body 编辑 UI)
///
/// ## 幂等性
/// 幂等 —— 同样的 (title, subtasks, status, area, color) 多次调用结果一致。
#[tauri::command]
#[specta::specta]
pub fn task_update_with_subtasks(
    state: State<'_, KeysightRuntimeState>,
    id: String,
    title: Option<String>,
    subtasks: Vec<Subtask>,
    status: Option<TaskStatus>,
    area: Option<String>,
    color: Option<String>,
) -> Result<TaskEntity, AppError> {
    let _t = ScopedTimer::new("cmd:task_update_with_subtasks");
    let conn = lock_db(&state.core.db, "task_update_with_subtasks");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());

    // 1. 读 current 作为 body merge base
    let current = task::get(&conn, &id).map_err(Into::<AppError>::into)?;

    // 2. 把新 subtasks 合并进 current.content —— 多 block 时 fail-closed
    let new_body = task::render_subtasks_into_body(&id, &current.content, &subtasks)
        .map_err(Into::<AppError>::into)?;

    // 3. 调 domain::task::update 复用文件 rename + sync_file + file_mtimes 路径
    task::update(
        &conn,
        &vault_fs,
        &id,
        task::TaskUpdateInput {
            title: title.as_deref(),
            content: Some(&new_body),
            status,
            area: area.as_deref(),
            color: color.as_deref(),
        },
    )
    .map_err(Into::<AppError>::into)?;

    // 4. 返回 fresh TaskEntity,含重新 parse 的 subtasks
    task::get(&conn, &id).map_err(Into::into)
}

// ============================================================
// Question
// ============================================================

/// 查询指定白板的所有 question。
#[tauri::command]
#[specta::specta]
pub fn question_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
) -> Result<Vec<QuestionEntity>, AppError> {
    let _t = ScopedTimer::new("cmd:question_query_all");
    let conn = lock_db(&state.core.db, "question_query_all");
    question::query_all(&conn, &whiteboard_id).map_err(Into::into)
}

/// 创建新的 question markdown 文件并同步入库。
#[tauri::command]
#[specta::specta]
pub fn question_create(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
    title: String,
    content: Option<String>,
    status: Option<String>,
    color: Option<String>,
) -> Result<QuestionEntity, AppError> {
    let _t = ScopedTimer::new("cmd:question_create");
    let conn = lock_db(&state.core.db, "question_create");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    question::create(
        &conn,
        &vault_fs,
        &whiteboard_id,
        &title,
        content.as_deref(),
        status.as_deref(),
        color.as_deref(),
    )
    .map_err(Into::into)
}

/// 更新 question markdown 文件并重新同步。
///
/// color 语义(和 note_update 一致):
/// - `None` → 保留 current.color
/// - `Some("default")` → 清空 color
/// - `Some(other)` → 覆盖为 `other`
#[tauri::command]
#[specta::specta]
pub fn question_update(
    state: State<'_, KeysightRuntimeState>,
    id: String,
    title: Option<String>,
    content: Option<String>,
    status: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:question_update");
    let conn = lock_db(&state.core.db, "question_update");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    question::update(
        &conn,
        &vault_fs,
        &id,
        title.as_deref(),
        content.as_deref(),
        status.as_deref(),
        color.as_deref(),
    )
    .map_err(Into::into)
}

/// 删除 question markdown 文件并清理数据库。
#[tauri::command]
#[specta::specta]
pub fn question_delete(
    state: State<'_, KeysightRuntimeState>,
    id: String,
) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:question_delete");
    let conn = lock_db(&state.core.db, "question_delete");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    question::delete(&conn, &vault_fs, &id).map_err(Into::into)
}

// ============================================================
// Note
// ============================================================

/// 按 ID 查询单个 note。
#[tauri::command]
#[specta::specta]
pub fn note_get(state: State<'_, KeysightRuntimeState>, id: String) -> Result<GraphNote, AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteNoteStore::new(&conn);
    store.get(&id).map_err(Into::into)
}

/// 查询指定白板的所有 note。
#[tauri::command]
#[specta::specta]
pub fn note_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
) -> Result<Vec<GraphNote>, AppError> {
    let _t = ScopedTimer::new("cmd:note_query_all");
    let conn = lock_db(&state.core.db, "note_query_all");
    let store = SqliteNoteStore::new(&conn);
    store.query_all(&whiteboard_id).map_err(Into::into)
}

/// # note_create
///
/// ## 前置条件
/// - title 非空
///
/// ## 执行效果
/// 1. 生成 note_ 前缀的唯一 ID
/// 2. 插入 entities 表（kind=note）
///
/// ## 幂等性
/// 非幂等 — 每次调用创建新 note
///
/// ## 关联操作
/// - [`note_delete`] — 删除（逆操作）
/// - [`note_update`] — 更新
#[tauri::command]
#[specta::specta]
pub fn note_create(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
    title: String,
    content: Option<String>,
    color: Option<String>,
) -> Result<GraphNote, AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
    store
        .create(&whiteboard_id, &title, content.as_deref(), color.as_deref())
        .map_err(Into::into)
}

/// # note_delete
///
/// ## 前置条件
/// - id 对应的 note 必须存在
///
/// ## 执行效果
/// 1. 删除 entities 表中的 note 行
/// 2. 删除 edges 中该 note 的所有边
/// 3. 删除 positions 中该 note 的位置
///
/// ## 幂等性
/// 幂等 — 已删除的 note 再次删除报 NotFound
///
/// ## 关联操作
/// - [`note_create`] — 创建（逆操作）
#[tauri::command]
#[specta::specta]
pub fn note_delete(state: State<'_, KeysightRuntimeState>, id: String) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
    store.delete(&id).map_err(Into::into)
}

/// # note_update
///
/// ## 前置条件
/// - id 对应的 note 必须存在
///
/// ## 执行效果
/// 1. 更新 entities 表中 title / content / color（仅更新提供的字段）
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`note_create`] — 创建
#[tauri::command]
#[specta::specta]
pub fn note_update(
    state: State<'_, KeysightRuntimeState>,
    id: String,
    title: Option<String>,
    content: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:note_update");
    let conn = lock_db(&state.core.db, "note_update");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
    store
        .update(&id, title.as_deref(), content.as_deref(), color.as_deref())
        .map_err(Into::into)
}

/// 备份 DB 和 `whiteboard/` 后，把 DB-only notes 导出成 markdown 文件。
#[tauri::command]
#[specta::specta]
pub fn note_migrate_to_files(
    state: State<'_, KeysightRuntimeState>,
) -> Result<NoteFileMigrationReport, AppError> {
    let _t = ScopedTimer::new("cmd:note_migrate_to_files");
    let conn = lock_db(&state.core.db, "note_migrate_to_files");
    super::domain::note::migrate_db_notes_to_files(&conn, &state.core.db_path, &state.core.vault_path)
        .map_err(Into::into)
}

// ============================================================
// Alias
// ============================================================

/// 按 ID 查询单个 alias。
#[tauri::command]
#[specta::specta]
pub fn alias_get(state: State<'_, KeysightRuntimeState>, id: String) -> Result<CardAlias, AppError> {
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
    whiteboard_id: String,
) -> Result<Vec<CardAlias>, AppError> {
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
    whiteboard_id: String,
    card_id: String,
) -> Result<CardAlias, AppError> {
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
pub fn alias_delete(state: State<'_, KeysightRuntimeState>, id: String) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteAliasStore::new(&conn);
    store.delete(&id).map_err(Into::into)
}

// ============================================================
// Layout
// ============================================================

/// 查询指定白板所有实体的位置。
#[tauri::command]
#[specta::specta]
pub fn layout_query_positions(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
) -> Result<HashMap<String, Position>, AppError> {
    let _t = ScopedTimer::new("cmd:layout_query_positions");
    let conn = lock_db(&state.core.db, "layout_query_positions");
    let store = SqliteLayoutStore::new(&conn);
    store.query_positions(&whiteboard_id).map_err(Into::into)
}

/// # layout_set_position
///
/// ## 前置条件
/// - 无（upsert 语义）
///
/// ## 执行效果
/// 1. INSERT OR REPLACE into positions 表
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`layout_remove_position`] — 移除位置（逆操作）
/// - [`layout_query_positions`] — 查询
#[tauri::command]
#[specta::specta]
pub fn layout_set_position(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
    entity_id: String,
    x: f64,
    y: f64,
) -> Result<(), AppError> {
    let _t = ScopedTimer::new("cmd:layout_set_position");
    let conn = lock_db(&state.core.db, "layout_set_position");
    let store = SqliteLayoutStore::new(&conn);
    store
        .set_position(&whiteboard_id, &entity_id, x, y)
        .map_err(Into::into)
}

/// # layout_remove_position
///
/// ## 前置条件
/// - 无
///
/// ## 执行效果
/// 1. 删除 positions 表中该行
///
/// ## 幂等性
/// 幂等 — 不存在则无效果
///
/// ## 关联操作
/// - [`layout_set_position`] — 设置位置（逆操作）
#[tauri::command]
#[specta::specta]
pub fn layout_remove_position(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: String,
    entity_id: String,
) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let store = SqliteLayoutStore::new(&conn);
    store
        .remove_position(&whiteboard_id, &entity_id)
        .map_err(Into::into)
}

// ============================================================
// Entity Graph
// ============================================================

/// 查询指定实体的出边。
#[tauri::command]
#[specta::specta]
pub fn entity_edges_from(
    state: State<'_, KeysightRuntimeState>,
    entity_id: String,
) -> Result<Vec<EdgeRow>, AppError> {
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
    entity_id: String,
) -> Result<Vec<EdgeRow>, AppError> {
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
/// - from_id 和 to_id 都必须是合法 entity id(prefix 匹配 card_/note_/alias_/
///   sec_/q_/task_),否则 parse 失败返 `AppError::Keysight`
/// - from 不能是 section / task (业务规则:这两类不主动发边),否则返
///   `KeysightError::ConnectionNotAllowed`
///
/// ## 执行效果
/// 1. [`EntityId::parse`] 两端字符串 → 强类型
/// 2. [`user_draw_edge`] 派发为具体 [`Edge`] 变体(CardLink/NoteLink/AliasLink/
///    QuestionLink)
/// 3. [`SqliteEntityGraph::connect`] 落 DB
/// 4. 按 edge 变体穷尽 match 决定是否需要同步 source 的 md 文件
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
    from_id: String,
    to_id: String,
) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();

    let from = EntityId::parse(&from_id).map_err(|e| AppError::Keysight {
        message: format!("from_id 解析失败: {e}"),
    })?;
    let to = EntityId::parse(&to_id).map_err(|e| AppError::Keysight {
        message: format!("to_id 解析失败: {e}"),
    })?;

    let edge = user_draw_edge(from, to).map_err(AppError::from)?;

    let graph = SqliteEntityGraph::new(&conn);
    graph.connect(&edge).map_err(AppError::from)?;

    // 文件同步路由:按 Edge 变体穷尽 match 派发(防火墙原则 — 禁止 _ 通配)
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    match &edge {
        Edge::CardLink { from, .. }
        | Edge::CardRelated { from, .. }
        | Edge::CardSeeAlso { from, .. } => {
            let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
            store.sync_edges_to_file(from.as_str()).map_err(AppError::from)?;
        }
        Edge::NoteLink { from, .. } | Edge::NoteSeeAlso { from, .. } => {
            let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
            store.sync_links_to_file(from.as_str()).map_err(AppError::from)?;
        }
        Edge::AliasLink { .. } => {
            // alias 无独立文件内容(继承 owning card),不 sync
        }
        Edge::QuestionLink { from, .. } => {
            question::sync_links_to_file(&conn, &vault_fs, from.as_str()).map_err(AppError::from)?;
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
    from_card_id: String,
    to_card_id: String,
) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();

    let from_entity = EntityId::parse(&from_card_id).map_err(|e| AppError::Keysight {
        message: format!("from_card_id 解析失败: {e}"),
    })?;
    let to_entity = EntityId::parse(&to_card_id).map_err(|e| AppError::Keysight {
        message: format!("to_card_id 解析失败: {e}"),
    })?;

    let (from, to) = match (from_entity, to_entity) {
        (EntityId::Card(a), EntityId::Card(b)) => (a, b),
        _ => {
            return Err(AppError::Keysight {
                message: "entity_relate 只接受 card → card 关系".to_string(),
            })
        }
    };

    let edge = Edge::CardRelated {
        from: from.clone(),
        to,
    };
    let graph = SqliteEntityGraph::new(&conn);
    graph.connect(&edge).map_err(AppError::from)?;

    // Related 是 card→card,source card 需要同步文件
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
    store.sync_edges_to_file(from.as_str()).map_err(AppError::from)?;

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
    from_id: String,
    to_id: String,
    edge_type: EdgeType,
) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    let graph = SqliteEntityGraph::new(&conn);
    graph
        .disconnect(&from_id, &to_id, edge_type)
        .map_err(AppError::from)?;

    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    if from_id.starts_with("card_") && matches!(edge_type, EdgeType::LinkTo | EdgeType::Related | EdgeType::SeeAlso) {
        let store = SqliteCardStore::with_vault_fs(&conn, &vault_fs);
        store.sync_edges_to_file(&from_id).map_err(AppError::from)?;
    } else if from_id.starts_with("note_") && edge_type == EdgeType::NoteLink {
        let store = SqliteNoteStore::with_vault_fs(&conn, &vault_fs);
        store.sync_links_to_file(&from_id).map_err(AppError::from)?;
    }

    Ok(())
}

// ============================================================
// Sync
// ============================================================

/// # sync_file
///
/// ## 前置条件
/// - file_path 非空
///
/// ## 执行效果
/// 1. 解析 markdown（frontmatter + 正文）
/// 2. 插入或更新 entities / card_fields / entity_tags / edges
/// 3. 更新 file_mtimes 表
/// 4. 同步 FTS5 索引（entities_fts）
///
/// ## 不做的事
/// - 不读取真实文件（content 由调用方提供）
///
/// ## 幂等性
/// 幂等 — 相同内容重复调用 → updated=1, inserted=0
///
/// ## 关联操作
/// - [`sync_remove_file`] — 按文件删除（逆操作）
/// - [`sync_all_file_mtimes`] — 查询所有文件时间戳
#[tauri::command]
#[specta::specta]
pub fn sync_file(
    state: State<'_, KeysightRuntimeState>,
    file_path: String,
    content: String,
    mtime: f64,
) -> Result<SyncFileResponse, AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    sync::sync_file(&conn, &file_path, &content, mtime).map_err(Into::into)
}

/// # sync_remove_file
///
/// ## 前置条件
/// - 无（不存在的文件删除无效果）
///
/// ## 执行效果
/// 1. 删除 entities 中 file_path 匹配的所有实体
/// 2. 删除 file_mtimes 中的记录
/// 3. 清理 FTS5 索引
///
/// ## 幂等性
/// 幂等
///
/// ## 关联操作
/// - [`sync_file`] — 同步文件（逆操作）
#[tauri::command]
#[specta::specta]
pub fn sync_remove_file(
    state: State<'_, KeysightRuntimeState>,
    file_path: String,
) -> Result<(), AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    sync::remove_file(&conn, &file_path).map_err(Into::into)
}

/// 查询所有已同步文件的 mtime。
#[tauri::command]
#[specta::specta]
pub fn sync_all_file_mtimes(
    state: State<'_, KeysightRuntimeState>,
) -> Result<Vec<(String, f64)>, AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    sync::all_file_mtimes(&conn).map_err(Into::into)
}

/// # sync_vault
///
/// ## 前置条件
/// - KEYSIGHT_VAULT_PATH 已设置且目录存在
///
/// ## 执行效果
/// 1. 扫描 whiteboard/ 下所有 .md 文件
/// 2. mtime diff → 同步变更文件到 DB
/// 3. 清理孤儿（DB 有但文件不存在的实体）
/// 4. 回写缺少 id 的文件 frontmatter
///
/// ## 幂等性
/// 幂等 — mtime 未变的文件不重复同步
///
/// ## 关联操作
/// - [`sync_file`] — 单文件同步（内部调用）
/// - [`sync_remove_file`] — 删除文件（内部调用）
#[tauri::command]
#[specta::specta]
pub fn sync_vault(state: State<'_, KeysightRuntimeState>) -> Result<SyncVaultReport, AppError> {
    let _t = ScopedTimer::new("cmd:sync_vault");
    let conn = lock_db(&state.core.db, "sync_vault");
    let fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    sync::sync_vault(&conn, &fs).map_err(Into::into)
}

// ============================================================
// Overview
// ============================================================

/// 查询全局统计信息。
#[tauri::command]
#[specta::specta]
pub fn overview_stats(state: State<'_, KeysightRuntimeState>) -> Result<StatsResponse, AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    overview::stats(&conn).map_err(Into::into)
}

/// 查询按白板聚合的图谱总览。
#[tauri::command]
#[specta::specta]
pub fn overview_graph(
    state: State<'_, KeysightRuntimeState>,
) -> Result<GraphOverviewResponse, AppError> {
    // 例外: Mutex poisoning 不可恢复
    let conn = state.core.db.lock().unwrap();
    overview::graph_overview(&conn).map_err(Into::into)
}

/// 查询所有子白板的轻量统计。
#[tauri::command]
#[specta::specta]
pub fn whiteboard_list(
    state: State<'_, KeysightRuntimeState>,
) -> Result<Vec<WhiteboardSummary>, AppError> {
    let _t = ScopedTimer::new("cmd:whiteboard_list");
    let conn = lock_db(&state.core.db, "whiteboard_list");
    let fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().to_string(), state.suppression.clone());
    overview::list_whiteboards(&conn, &fs).map_err(Into::into)
}

/// # whiteboard_create
///
/// ## 前置条件
/// - `name` trim 后非空
/// - `name` 不能包含 `/` 或 `:`
/// - `whiteboard/{name}/` 目录尚不存在
///
/// ## 执行效果
/// 1. 在 vault 下创建空目录 `whiteboard/{name}/`
/// 2. 返回该 whiteboard 的零统计摘要
///
/// ## 不做的事
/// - 不创建任何 card / section / note
/// - 不直接写入 positions；root 画布会在看到新 whiteboard 后自动补位置
///
/// ## 幂等性
/// 非幂等 — 已存在同名文件夹时报错
#[tauri::command]
#[specta::specta]
pub fn whiteboard_create(
    state: State<'_, KeysightRuntimeState>,
    name: String,
) -> Result<WhiteboardSummary, AppError> {
    let _t = ScopedTimer::new("cmd:whiteboard_create");
    whiteboard::create_folder(&state.core.vault_path, &name).map_err(Into::into)
}

// ============================================================
// Config
// ============================================================

/// 返回当前 vault 配置信息。
#[tauri::command]
#[specta::specta]
pub fn get_vault_info(state: State<'_, KeysightRuntimeState>) -> Result<VaultInfoResponse, AppError> {
    Ok(VaultInfoResponse {
        vault_path: state.core.vault_path.to_string_lossy().into_owned(),
    })
}

// ============================================================
// Legacy Import
// ============================================================

/// # import_legacy_db
///
/// ## 前置条件
/// - old_db_path 指向旧 Obsidian 插件的 keysight.db（v1 schema）
/// - 文件必须存在且可读
///
/// ## 执行效果
/// 1. 备份旧 DB（cp → {old_db_path}.bak-import-{timestamp}）
/// 2. 只读打开旧 DB
/// 3. 单事务写入新 DB（INSERT OR REPLACE）
/// 4. 同步 FTS 索引
///
/// ## 幂等性
/// 可重复执行，结果一致
///
/// ## 关联操作
/// - [`overview_stats`] — 导入后验证数据量
#[tauri::command]
#[specta::specta]
pub fn import_legacy_db(
    state: State<'_, KeysightRuntimeState>,
    old_db_path: String,
) -> Result<ImportSummary, AppError> {
    let path = std::path::Path::new(&old_db_path);
    if !path.exists() {
        return Err(AppError::Keysight {
            message: format!("旧 DB 文件不存在: {old_db_path}"),
        });
    }

    // 1. 备份旧 DB
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let backup_path = format!("{old_db_path}.bak-import-{timestamp}");
    std::fs::copy(path, &backup_path).map_err(|e| AppError::Keysight {
        message: format!("备份旧 DB 失败: {e}"),
    })?;

    // 2. 只读打开旧 DB
    let old_conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| AppError::Keysight {
        message: format!("打开旧 DB 失败: {e}"),
    })?;

    // 3. 导入
    // 例外: Mutex poisoning 不可恢复
    let new_conn = state.core.db.lock().unwrap();
    let reader = SqliteLegacyReader::new(&old_conn);
    let importer = SqliteLegacyImporter::new(&new_conn);
    importer.import(&reader).map_err(Into::into)
}
