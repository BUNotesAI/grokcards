use tauri::State;

use crate::app_error::AppError;
use crate::modules::keysight::domain::id::{TaskId, WhiteboardId};
use crate::modules::keysight::domain::task;
use crate::modules::keysight::models::{Subtask, TaskCreateRequest, TaskEntity, TaskStatus};
use crate::modules::keysight::recording_vault_fs::RecordingVaultFs;
use crate::modules::keysight::runtime_state::KeysightRuntimeState;
use crate::perf::{lock_db, ScopedTimer};

/// 查询指定白板的所有 task。
#[tauri::command]
#[specta::specta]
pub fn task_query_all(
    state: State<'_, KeysightRuntimeState>,
    whiteboard_id: WhiteboardId,
) -> Result<Vec<TaskEntity>, AppError> {
    let state = state.resolved()?;
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
    let state = state.resolved()?;
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
/// 3. 如果 `position` 存在,用它作为初始 canvas 坐标;否则由 domain 自动放到
///    当前 project 白板最底部
/// 4. 返回新 TaskEntity
#[tauri::command]
#[specta::specta]
pub fn task_create(
    state: State<'_, KeysightRuntimeState>,
    input: TaskCreateRequest,
) -> Result<TaskEntity, AppError> {
    let state = state.resolved()?;
    let _t = ScopedTimer::new("cmd:task_create");
    let conn = lock_db(&state.core.db, "task_create");
    let vault_fs = RecordingVaultFs::wrap_real(state.core.vault_path.to_string_lossy().into_owned(), state.suppression.clone());
    let project_name =
        task::ProjectName::new(&input.project).map_err(Into::<AppError>::into)?;
    task::create(
        &conn,
        &vault_fs,
        &project_name,
        task::TaskCreateInput {
            title: &input.title,
            content: input.content.as_deref(),
            status: input.status,
            area: input.area.as_deref(),
            color: input.color.as_deref(),
            position: input.position,
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
    id: TaskId,
    title: Option<String>,
    content: Option<String>,
    status: Option<TaskStatus>,
    area: Option<String>,
    color: Option<String>,
) -> Result<(), AppError> {
    let state = state.resolved()?;
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
pub fn task_delete(state: State<'_, KeysightRuntimeState>, id: TaskId) -> Result<(), AppError> {
    let state = state.resolved()?;
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
    id: TaskId,
    color: String,
) -> Result<(), AppError> {
    let state = state.resolved()?;
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
    id: TaskId,
    title: Option<String>,
    subtasks: Vec<Subtask>,
    status: Option<TaskStatus>,
    area: Option<String>,
    color: Option<String>,
) -> Result<TaskEntity, AppError> {
    let state = state.resolved()?;
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
