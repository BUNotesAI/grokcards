#![allow(dead_code)]
use rusqlite::{params, Connection, OptionalExtension};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::id;
use crate::modules::keysight::models::{Position, TaskEntity, TaskStatus};
use crate::modules::keysight::vault_fs::VaultFs;

use super::sync;

// ============================================================================
// ProjectName 值对象 — 防火墙: 非空 + 禁路径分隔符 + 禁 Windows 禁用字符
// ============================================================================

/// Task 所属项目名 —— 决定文件目录的业务标识。
///
/// Invariant: 非空、trim 后非空、不含 `/ \\ : * ? " < > |` 这些路径分隔/Windows 禁用字符。
/// 构造只能通过 [`ProjectName::new`],构造后的 `as_str` 保证合法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::modules::keysight) struct ProjectName(String);

/// ProjectName 的最大长度(单个路径组件)。255 是大多数文件系统的上限,
/// 但 project name 还要和 task_id + title 拼在同一层目录,留足余量设 100。
const PROJECT_NAME_MAX_LEN: usize = 100;

/// Windows 保留的设备名(不区分大小写,不含扩展名)。
/// POSIX 不禁,但我们要跨平台,所以一并禁。
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

impl ProjectName {
    /// 构造 ProjectName,对非法输入返回 `InvalidProjectName`。
    ///
    /// ## 合法字符
    /// 允许除下列之外的任意 char(包含中文、空格、`-`、`_`):
    /// - 路径分隔符和 Windows 禁用字符:`/ \ : * ? " < > |`
    /// - NUL 字节 `\0`(POSIX 路径截断)
    /// - 控制字符(`\n` `\r` `\t` + ASCII < 0x20)
    ///
    /// ## 其他校验
    /// - **不 silently trim** —— 前后空白 reject 而非偷偷吞掉
    ///   (trim 后两个不同输入会变成同一个 project 名,是 bug 温床)
    /// - 禁 `.` 和 `..` 等 dots-only(path traversal)
    /// - 禁 leading `.`(Unix hidden file 约定)
    /// - 禁 Windows 保留设备名(CON / PRN / AUX / NUL / COM1-9 / LPT1-9)
    /// - 长度上限 [`PROJECT_NAME_MAX_LEN`] 字符
    ///
    /// P1-8 修复:之前只检查路径分隔符 + 自动 trim,对其余 filesystem 危险字符无防护。
    pub(in crate::modules::keysight) fn new(value: &str) -> Result<Self, KeysightError> {
        if value.is_empty() {
            return Err(KeysightError::InvalidProjectName("不能为空".to_string()));
        }
        // 不允许前后空白 —— reject 而非 trim(两个输入 trim 后同名是 bug)
        if value != value.trim() {
            return Err(KeysightError::InvalidProjectName(
                "前后不能有空白字符".to_string(),
            ));
        }
        // 长度上限
        if value.chars().count() > PROJECT_NAME_MAX_LEN {
            return Err(KeysightError::InvalidProjectName(format!(
                "长度超过 {PROJECT_NAME_MAX_LEN} 字符"
            )));
        }
        // 字符级检查
        for ch in value.chars() {
            if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                return Err(KeysightError::InvalidProjectName(format!(
                    "含路径非法字符 '{ch}'"
                )));
            }
            if ch == '\0' {
                return Err(KeysightError::InvalidProjectName(
                    "含 NUL 字节".to_string(),
                ));
            }
            if ch.is_control() {
                return Err(KeysightError::InvalidProjectName(format!(
                    "含控制字符 U+{:04X}",
                    ch as u32
                )));
            }
        }
        // dots-only(`.` / `..` / `...` 等)— path traversal
        if value.chars().all(|c| c == '.') {
            return Err(KeysightError::InvalidProjectName(
                "不能是 `.` / `..` 等纯点名".to_string(),
            ));
        }
        // leading `.`(Unix hidden file)
        if value.starts_with('.') {
            return Err(KeysightError::InvalidProjectName(
                "不能以 `.` 开头".to_string(),
            ));
        }
        // Windows 保留设备名(不区分大小写,不含扩展名)
        let upper = value.to_ascii_uppercase();
        if WINDOWS_RESERVED.contains(&upper.as_str()) {
            return Err(KeysightError::InvalidProjectName(format!(
                "`{value}` 是 Windows 保留设备名"
            )));
        }
        Ok(Self(value.to_string()))
    }

    /// 返回底层字符串切片,用于 SQL / 路径拼接。
    pub(in crate::modules::keysight) fn as_str(&self) -> &str {
        &self.0
    }

    /// 返回对应的 whiteboard_id(固定格式 `projects/{name}`,和 sync 的
    /// `derive_whiteboard_id` 反推规则对齐)。
    pub(in crate::modules::keysight) fn whiteboard_id(&self) -> String {
        format!("projects/{}", self.0)
    }
}

// ============================================================================
// TaskStatus 字符串互转 — parse/serialize 集中处
// ============================================================================

/// 把 DB / frontmatter 里的 status 字符串解析成 TaskStatus 枚举。
///
/// 未知字符串返回 `InvalidTaskStatus`,保证 Task 聚合路径的 status 是强类型。
fn parse_task_status(s: &str) -> Result<TaskStatus, KeysightError> {
    match s {
        "inbox" => Ok(TaskStatus::Inbox),
        "next" => Ok(TaskStatus::Next),
        "active" => Ok(TaskStatus::Active),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        other => Err(KeysightError::InvalidTaskStatus(other.to_string())),
    }
}

/// 把 TaskStatus 枚举序列化成 DB / frontmatter 里的小写字符串。
fn task_status_to_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Inbox => "inbox",
        TaskStatus::Next => "next",
        TaskStatus::Active => "active",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Done => "done",
    }
}

// ============================================================
// Canvas auto-position 支持
// ============================================================

/// Task 节点默认高度。
///
/// 值 = 140 与前端 `src/components/keysight/types.ts` 的
/// `ENTITY_DIMENSIONS.task.height = 140` 对齐。Kanban 创建的实体以 task 为主,
/// 因此以 task 高度作为 canvas 垂直步长基准。
const DEFAULT_NODE_HEIGHT: f64 = 140.0;

/// 节点之间的垂直间距,用于 auto-position 计算。
const NODE_SPACING: f64 = 40.0;

/// 计算给定 whiteboard 上"最底元素下方"的坐标,供 Kanban 创建 task 时
/// 自动定位到 canvas 上不重叠位置。
///
/// 算法: `SELECT MAX(y) FROM positions WHERE whiteboard_id = ?`,
/// 然后 `new_y = max_y + DEFAULT_NODE_HEIGHT + NODE_SPACING`,`new_x = 0.0`。
///
/// 空白板返回 `Position { x: 0, y: 0 }`(不是 error;空态是合法的,
/// 意思是"这是白板第一个节点")。
pub(in crate::modules::keysight) fn compute_position_below_bottommost(
    conn: &Connection,
    whiteboard_id: &str,
) -> Result<Position, KeysightError> {
    let max_y: Option<f64> = conn.query_row(
        "SELECT MAX(y) FROM positions WHERE whiteboard_id = ?1",
        [whiteboard_id],
        |row| row.get::<_, Option<f64>>(0),
    )?;
    let new_y = match max_y {
        Some(y) => y + DEFAULT_NODE_HEIGHT + NODE_SPACING,
        None => 0.0,
    };
    Ok(Position { x: 0.0, y: new_y })
}

// ============================================================================
// 文件路径 + markdown 渲染 helpers
// ============================================================================

fn current_mtime_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn validate_title(title: &str) -> Result<&str, KeysightError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        Err(KeysightError::EmptyTitle)
    } else {
        Ok(trimmed)
    }
}

fn filename_title_component(text: &str) -> String {
    text.replace("**", "")
}

fn sanitize_file_component(text: &str) -> String {
    let cleaned = filename_title_component(text)
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        "Untitled".to_string()
    } else {
        cleaned
    }
}

/// 拼接 Task 文件的 vault 相对路径。
///
/// 格式:`whiteboard/projects/{project}/{task_id} 【TASK】{sanitized_title}.md`
fn task_relative_path(project: &ProjectName, task_id: &str, title: &str) -> String {
    format!(
        "whiteboard/projects/{}/{} 【TASK】{}.md",
        project.as_str(),
        task_id,
        sanitize_file_component(title),
    )
}

/// 计算 Task rename 时的目标路径。保留原路径如果它与 desired 一致。
fn desired_task_relative_path(
    project: &ProjectName,
    task_id: &str,
    title: &str,
    current_file_path: Option<&str>,
) -> String {
    let desired = task_relative_path(project, task_id, title);
    match current_file_path {
        Some(path) if !path.is_empty() && path == desired => path.to_string(),
        _ => desired,
    }
}

/// 渲染 Task markdown 文件内容。
///
/// Frontmatter 字段:`type: project-task` / `id` / `status` / `area?` / `project` / `color?`
/// 正文开头:`# 【TASK】{title}`
fn render_task_markdown(
    task_id: &str,
    title: &str,
    content: &str,
    status: TaskStatus,
    project: &ProjectName,
    area: Option<&str>,
    color: Option<&str>,
) -> String {
    let body = content.trim_end();
    let mut lines = vec![
        "---".to_string(),
        "type: project-task".to_string(),
        format!("id: {task_id}"),
        format!("status: {}", task_status_to_str(status)),
    ];
    if let Some(area) = area.filter(|value| !value.is_empty()) {
        lines.push(format!("area: {area}"));
    }
    lines.push(format!("project: {}", project.as_str()));
    if let Some(color) = color.filter(|value| !value.is_empty()) {
        // YAML 里裸的 `#` 会被当成行内注释,hex 色值必须加引号。
        lines.push(format!("color: \"{color}\""));
    }
    lines.extend([
        "---".to_string(),
        String::new(),
        format!("# 【TASK】{title}"),
        String::new(),
    ]);
    if !body.is_empty() {
        lines.push(body.to_string());
    }
    lines.push(String::new());
    lines.join("\n")
}

// ============================================================================
// 读取 pub fn
// ============================================================================

/// 按 id 查询单个 Task(跨 entities + task_fields 联合查询)。
pub(in crate::modules::keysight) fn get(
    conn: &Connection,
    id: &str,
) -> Result<TaskEntity, KeysightError> {
    conn.query_row(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, \
         t.status, t.area, t.project, e.color \
         FROM entities e JOIN task_fields t ON e.id = t.entity_id \
         WHERE e.id = ?1 AND e.kind = 'task'",
        [id],
        |r| {
            Ok(TaskEntity {
                id: r.get(0)?,
                title: r.get(1)?,
                content: r.get::<_, String>(2)?.trim_end_matches('\n').to_string(),
                whiteboard_id: r.get(3)?,
                status: r.get(4)?,
                area: r.get(5)?,
                project: r.get(6)?,
                color: r.get(7)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
        other => KeysightError::Database(other),
    })
}

/// 查询指定白板的所有任务。
pub(in crate::modules::keysight) fn query_all(
    conn: &Connection,
    whiteboard_id: &str,
) -> Result<Vec<TaskEntity>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, \
         t.status, t.area, t.project, e.color \
         FROM entities e JOIN task_fields t ON e.id = t.entity_id \
         WHERE e.whiteboard_id = ?1 ORDER BY e.title",
    )?;
    let rows = stmt.query_map([whiteboard_id], |r| {
        Ok(TaskEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            content: r.get::<_, String>(2)?.trim_end_matches('\n').to_string(),
            whiteboard_id: r.get(3)?,
            status: r.get(4)?,
            area: r.get(5)?,
            project: r.get(6)?,
            color: r.get(7)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(KeysightError::from)
}

/// 查询 kanban view 数据 —— 跨项目或单项目 task list。
///
/// - `project = None` → 跨项目查所有 task entity
/// - `project = Some(name)` → 仅该 project 的 task(whiteboard_id = `projects/{name}`)
///
/// 不按 status 分组(留给前端按 `task.status` 渲染)。V1 按 `e.title` ASC 排序
/// (对齐 `query_all` 的现有行为),因为 entities 表当前没有 `created_at` 列。
/// V2 如需"最新优先"排序再单独加 migration + 字段。
pub(in crate::modules::keysight) fn query_kanban(
    conn: &Connection,
    project: Option<&ProjectName>,
) -> Result<Vec<TaskEntity>, KeysightError> {
    // 两条 SQL 只差一个 WHERE 子句。为了避免动态 param 借用生命周期纠结,
    // 分支展开成两个独立的 prepare/query_map 调用,mapper 复用同一个 closure。
    let mapper = |r: &rusqlite::Row<'_>| -> rusqlite::Result<TaskEntity> {
        Ok(TaskEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            content: r.get::<_, String>(2)?.trim_end_matches('\n').to_string(),
            whiteboard_id: r.get(3)?,
            status: r.get(4)?,
            area: r.get(5)?,
            project: r.get(6)?,
            color: r.get(7)?,
        })
    };

    match project {
        Some(p) => {
            let wb = p.whiteboard_id();
            let mut stmt = conn.prepare(
                "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, \
                 t.status, t.area, t.project, e.color \
                 FROM entities e JOIN task_fields t ON e.id = t.entity_id \
                 WHERE e.kind = 'task' AND e.whiteboard_id = ?1 \
                 ORDER BY e.title",
            )?;
            stmt.query_map([wb], mapper)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(KeysightError::from)
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.title, COALESCE(e.content, '') AS content, e.whiteboard_id, \
                 t.status, t.area, t.project, e.color \
                 FROM entities e JOIN task_fields t ON e.id = t.entity_id \
                 WHERE e.kind = 'task' \
                 ORDER BY e.title",
            )?;
            stmt.query_map([], mapper)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(KeysightError::from)
        }
    }
}

/// 按状态查询任务(返回 json,用于 command 层跨项目聚合视图)。
pub(in crate::modules::keysight) fn by_status(
    conn: &Connection,
    status: &str,
) -> Result<Vec<serde_json::Value>, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.title, e.whiteboard_id, e.content, t.status, t.area, t.project, e.color \
         FROM entities e JOIN task_fields t ON e.id = t.entity_id \
         WHERE t.status = ?1 ORDER BY e.title",
    )?;
    let rows = stmt.query_map([status], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "title": r.get::<_, String>(1)?,
            "whiteboardId": r.get::<_, String>(2)?,
            "content": r.get::<_, Option<String>>(3)?,
            "status": r.get::<_, String>(4)?,
            "area": r.get::<_, Option<String>>(5)?,
            "project": r.get::<_, Option<String>>(6)?,
            "color": r.get::<_, Option<String>>(7)?,
        }))
    })?;
    // P1-4 修复:不再 filter_map(|r| r.ok()) 吞行错误。坏数据行必须 loud fail,
    // 否则 kanban 视图会悄悄漏掉 task 不告诉任何人。
    let results = rows
        .collect::<rusqlite::Result<Vec<serde_json::Value>>>()
        .map_err(KeysightError::from)?;
    Ok(results)
}

// ============================================================================
// 写入 pub fn — create / update / delete(file-backed)
// ============================================================================

/// Task 创建输入 —— 把可变 + 可选字段打包成一个结构体,避免 create 参数爆炸。
pub(in crate::modules::keysight) struct TaskCreateInput<'a> {
    pub title: &'a str,
    pub content: Option<&'a str>,
    pub status: TaskStatus,
    pub area: Option<&'a str>,
    pub color: Option<&'a str>,
}

/// Task 更新输入 —— None 字段保留 current 值,Some 字段覆盖。
pub(in crate::modules::keysight) struct TaskUpdateInput<'a> {
    pub title: Option<&'a str>,
    pub content: Option<&'a str>,
    pub status: Option<TaskStatus>,
    pub area: Option<&'a str>,
    pub color: Option<&'a str>,
}

/// 创建新任务 —— 同时写 markdown 文件和 DB。
///
/// # 操作名称
/// Task 创建(文件回写)
///
/// # 前置条件
/// - `project` 必须合法(通过 `ProjectName::new` 构造,不能为空或含非法字符)
/// - `input.title` trim 后不能为空
///
/// # 执行效果
/// 1. 生成 task_id(`task_` 前缀)
/// 2. 计算文件路径 `whiteboard/projects/{project}/{id} 【TASK】{title}.md`
/// 3. 渲染 markdown(frontmatter + 正文)
/// 4. 通过 VaultFs 写文件(不存在的目录由 vault_fs 负责 mkdir -p)
/// 5. 调 `sync::sync_file` 把 markdown parse 回 DB,`whiteboard_id` 由路径推导为
///    `projects/{project}`,task_fields 表 UPSERT status/area/project
/// 6. 自动定位: 写 positions 行(x=0.0, y=最底元素下方一个 node 高度 + spacing),
///    空白板第一个节点落在 (0, 0)
/// 7. 返回 `TaskEntity`(通过 `get` 从 DB 重新读取)
///
/// # 不做的事
/// - 不校验 project 目录已存在(VaultFs 的 mkdir -p 负责)
/// - 不处理并发(同名 title 由 task_id 前缀保证文件名唯一)
/// - 不做智能布局(x 坐标始终是 0.0,y 是单列垂直堆叠;未来如需 grid 布局另起 helper)
///
/// # 幂等性
/// 非幂等 —— 每次调用生成新 task_id,产生新文件。
pub(in crate::modules::keysight) fn create(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    project: &ProjectName,
    input: TaskCreateInput<'_>,
) -> Result<TaskEntity, KeysightError> {
    let title = validate_title(input.title)?;
    let task_id = id::gen_task_id();
    let file_path = task_relative_path(project, &task_id, title);
    let body = input.content.unwrap_or_default();
    let markdown = render_task_markdown(
        &task_id,
        title,
        body,
        input.status,
        project,
        input.area,
        input.color,
    );
    vault_fs.write_file(&file_path, &markdown)?;
    sync::sync_file(conn, &file_path, &markdown, current_mtime_ms())?;
    // 自动定位: 在 canvas 最底元素下方一个 node 高度 + spacing 处
    // (为 Kanban view 创建的 task 提供 canvas 坐标,无需 TS 侧调 layout_set_position)
    let whiteboard_id = project.whiteboard_id();
    let position = compute_position_below_bottommost(conn, &whiteboard_id)?;
    conn.execute(
        "INSERT OR REPLACE INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, ?2, ?3, ?4)",
        params![&task_id, &whiteboard_id, position.x, position.y],
    )?;
    get(conn, &task_id)
}

/// 更新已有任务 —— rewrite markdown 文件 + 同步 DB。
///
/// # 操作名称
/// Task 更新(文件回写 + 自动 rename)
///
/// # 前置条件
/// - Task 必须存在(否则返回 `NotFound`)
/// - 任何 `Some` 参数都必须合法(title 非空、status 是合法 enum variant)
///
/// # 执行效果
/// 1. 读取当前 TaskEntity(含 project / status / 等)
/// 2. 用新参数覆盖,None 的字段保留 current 值
/// 3. 计算 desired 文件路径:title 未变 → 同路径;title 变 → 新路径
/// 4. 渲染新 markdown + 写文件
/// 5. 调 sync_file 同步 DB
/// 6. 如果路径变了:删除旧文件 + 清理 file_mtimes 记录
///
/// # 不做的事
/// - 不允许更改 project(project 决定目录,跨目录迁移是另一个操作)
/// - 不删除旧数据(rename 通过 写新文件 + 删旧文件 完成)
///
/// # 幂等性
/// 幂等 —— 同样的参数多次调用结果一致(包括 rename 路径)。
pub(in crate::modules::keysight) fn update(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    id: &str,
    input: TaskUpdateInput<'_>,
) -> Result<(), KeysightError> {
    let current = get(conn, id)?;
    let file_path: Option<String> = conn
        .query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'task'",
            [id],
            |r| r.get(0),
        )
        .optional()?;

    // project 必须从 current 读出来 —— 不允许通过 update 改 project
    let current_project_str = current
        .project
        .clone()
        .ok_or_else(|| KeysightError::InvalidProjectName(format!("task {id} 缺 project")))?;
    let project = ProjectName::new(&current_project_str)?;

    // status 从 current.status 严格解析 —— 如果 DB 里的状态不在 enum 集合中,
    // 说明数据被外部污染,必须 loud fail 而不是 silently 重置为 Next。
    // P1-3 修复:之前 `unwrap_or(TaskStatus::Next)` 会让 task::update 调用
    //   "只改 title" 的路径,悄悄把 status 从 "wip" 改成 "next",永久丢失原值。
    let current_status = parse_task_status(&current.status)?;
    let next_status = input.status.unwrap_or(current_status);

    let next_title = match input.title {
        Some(value) => validate_title(value)?.to_string(),
        None => current.title.clone(),
    };
    let next_content = input.content.unwrap_or(&current.content).to_string();
    let next_area = input
        .area
        .map(|s| s.to_string())
        .or_else(|| current.area.clone());
    // color "default" sentinel 清空(和 note.rs / question.rs / card.rs 一致):
    //   Some("default") → None(清空)
    //   Some(other)     → Some(other.to_string())(覆盖)
    //   None            → current.color.clone()(保留)
    let next_color = match input.color {
        Some("default") => None,
        Some(other) => Some(other.to_string()),
        None => current.color.clone(),
    };

    let previous_path = file_path.filter(|path| !path.is_empty());
    let relative_path =
        desired_task_relative_path(&project, id, &next_title, previous_path.as_deref());
    let markdown = render_task_markdown(
        id,
        &next_title,
        &next_content,
        next_status,
        &project,
        next_area.as_deref(),
        next_color.as_deref(),
    );
    vault_fs.write_file(&relative_path, &markdown)?;
    sync::sync_file(conn, &relative_path, &markdown, current_mtime_ms())?;
    if let Some(previous_path) = previous_path
        && previous_path != relative_path
    {
        vault_fs.delete_file(&previous_path)?;
        conn.execute(
            "DELETE FROM file_mtimes WHERE filePath = ?1",
            [&previous_path],
        )?;
    }
    Ok(())
}

/// 删除任务 —— 删除 markdown 文件 + 级联清理 DB 行。
///
/// # 操作名称
/// Task 删除(文件 + DB)
///
/// # 前置条件
/// Task 必须存在(通过 file_path 存在性间接校验)。
///
/// # 执行效果
/// 1. 查 entities.file_path
/// 2. 有文件 → vault_fs.delete_file + sync::remove_file(级联清 DB 行)
/// 3. 无文件 → 直接 DELETE task_fields / positions / entities(legacy DB-only 兼容)
///
/// # 幂等性
/// 非幂等 —— 重复调用第二次找不到 task 返回 `NotFound`。
pub(in crate::modules::keysight) fn delete(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
    id: &str,
) -> Result<(), KeysightError> {
    let file_path: Option<String> = conn
        .query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'task'",
            [id],
            |r| r.get(0),
        )
        .optional()?;

    if let Some(file_path) = file_path.filter(|path| !path.is_empty()) {
        vault_fs.delete_file(&file_path)?;
        sync::remove_file(conn, &file_path)?;
        return Ok(());
    }

    // Legacy DB-only 路径:没 file_path 的旧数据,手动级联清理
    //
    // P1-5 修复:之前只删 task_fields / positions / entities,漏了
    // entity_tags / edges / entities_fts / section_members,导致 task 有外部
    // 关联时留下悬挂的 edge / fts 行 / section 成员记录。
    // 对齐 `sync::remove_file` 的完整清理列表。
    let rows = conn.execute("DELETE FROM task_fields WHERE entity_id = ?1", [id])?;
    if rows == 0 {
        return Err(KeysightError::NotFound(id.to_string()));
    }
    conn.execute("DELETE FROM entity_tags WHERE entity_id = ?1", [id])?;
    conn.execute("DELETE FROM edges WHERE from_id = ?1 OR to_id = ?1", [id])?;
    conn.execute("DELETE FROM entities_fts WHERE id = ?1", [id])?;
    conn.execute("DELETE FROM positions WHERE entity_id = ?1", [id])?;
    conn.execute("DELETE FROM section_members WHERE entity_id = ?1", [id])?;
    conn.execute("DELETE FROM entities WHERE id = ?1", [id])?;
    Ok(())
}

// ============================================================================
// Legacy 单字段更新 —— 未来可能移除,现在保留供 overview.rs / 其他读路径使用
// ============================================================================

/// 快速更新任务状态(仅 DB,不回写文件 —— 有漂移风险,新代码用 `update`)。
pub(super) fn transition_status(
    conn: &Connection,
    id: &str,
    status: &str,
) -> Result<(), KeysightError> {
    let rows = conn.execute(
        "UPDATE task_fields SET status = ?1 WHERE entity_id = ?2",
        params![status, id],
    )?;
    if rows == 0 {
        return Err(KeysightError::NotFound(id.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keysight::db::init_db;
    use crate::modules::keysight::vault_fs::MockVaultFs;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    // --- ProjectName ---

    #[test]
    fn test_project_name_valid() {
        let p = ProjectName::new("super-tauri").unwrap();
        assert_eq!(p.as_str(), "super-tauri");
        assert_eq!(p.whiteboard_id(), "projects/super-tauri");
    }

    #[test]
    fn test_project_name_allows_spaces_and_chinese() {
        let p = ProjectName::new("我的 项目").unwrap();
        assert_eq!(p.as_str(), "我的 项目");
    }

    /// P1-8 变更:不再 silently trim,前后空白直接 reject(两个输入 trim 后
    /// 同名是 bug 温床)
    #[test]
    fn test_project_name_rejects_leading_trailing_whitespace() {
        assert!(matches!(
            ProjectName::new("  spaced  "),
            Err(KeysightError::InvalidProjectName(_))
        ));
        assert!(matches!(
            ProjectName::new(" leading"),
            Err(KeysightError::InvalidProjectName(_))
        ));
        assert!(matches!(
            ProjectName::new("trailing "),
            Err(KeysightError::InvalidProjectName(_))
        ));
    }

    #[test]
    fn test_project_name_empty_rejected() {
        assert!(matches!(
            ProjectName::new(""),
            Err(KeysightError::InvalidProjectName(_))
        ));
        assert!(matches!(
            ProjectName::new("   "),
            Err(KeysightError::InvalidProjectName(_))
        ));
    }

    #[test]
    fn test_project_name_rejects_path_separators() {
        for bad in &["a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b"] {
            assert!(
                matches!(
                    ProjectName::new(bad),
                    Err(KeysightError::InvalidProjectName(_))
                ),
                "expected {bad} to be rejected"
            );
        }
    }

    /// P1-8 新加:禁 NUL 字节和控制字符(POSIX 路径截断 / 显示破坏)
    #[test]
    fn test_project_name_rejects_nul_and_control_chars() {
        assert!(matches!(
            ProjectName::new("a\0b"),
            Err(KeysightError::InvalidProjectName(_))
        ));
        assert!(matches!(
            ProjectName::new("a\nb"),
            Err(KeysightError::InvalidProjectName(_))
        ));
        assert!(matches!(
            ProjectName::new("a\tb"),
            Err(KeysightError::InvalidProjectName(_))
        ));
        assert!(matches!(
            ProjectName::new("a\rb"),
            Err(KeysightError::InvalidProjectName(_))
        ));
    }

    /// P1-8 新加:禁 dots-only(path traversal)
    #[test]
    fn test_project_name_rejects_dots_only() {
        for bad in &[".", "..", "...", "...."] {
            assert!(
                matches!(
                    ProjectName::new(bad),
                    Err(KeysightError::InvalidProjectName(_))
                ),
                "expected {bad:?} to be rejected"
            );
        }
    }

    /// P1-8 新加:禁 leading dot(Unix hidden file)
    #[test]
    fn test_project_name_rejects_leading_dot() {
        assert!(matches!(
            ProjectName::new(".hidden"),
            Err(KeysightError::InvalidProjectName(_))
        ));
        assert!(matches!(
            ProjectName::new(".config"),
            Err(KeysightError::InvalidProjectName(_))
        ));
    }

    /// P1-8 新加:禁 Windows 保留设备名(跨平台安全)
    #[test]
    fn test_project_name_rejects_windows_reserved() {
        for bad in &["CON", "con", "PRN", "aux", "NUL", "COM1", "lpt9", "COM5"] {
            assert!(
                matches!(
                    ProjectName::new(bad),
                    Err(KeysightError::InvalidProjectName(_))
                ),
                "expected {bad:?} to be rejected as Windows reserved name"
            );
        }
    }

    /// P1-8 新加:禁超长名(> 100 字符)
    #[test]
    fn test_project_name_rejects_too_long() {
        let too_long: String = "a".repeat(101);
        assert!(matches!(
            ProjectName::new(&too_long),
            Err(KeysightError::InvalidProjectName(_))
        ));
        // 100 字符刚好合法
        let exactly: String = "a".repeat(100);
        assert!(ProjectName::new(&exactly).is_ok());
    }

    // --- TaskStatus 互转 ---

    #[test]
    fn test_parse_task_status_roundtrip() {
        for status in [
            TaskStatus::Inbox,
            TaskStatus::Next,
            TaskStatus::Active,
            TaskStatus::Blocked,
            TaskStatus::Done,
        ] {
            let s = task_status_to_str(status);
            assert_eq!(parse_task_status(s).unwrap(), status);
        }
    }

    #[test]
    fn test_parse_task_status_unknown() {
        assert!(matches!(
            parse_task_status("wip"),
            Err(KeysightError::InvalidTaskStatus(_))
        ));
    }

    #[test]
    fn test_parse_task_status_inbox() {
        assert_eq!(parse_task_status("inbox").unwrap(), TaskStatus::Inbox);
    }

    #[test]
    fn test_task_status_inbox_serializes_lowercase() {
        let json = serde_json::to_string(&TaskStatus::Inbox).unwrap();
        assert_eq!(json, "\"inbox\"");
    }

    // --- task_relative_path + render ---

    #[test]
    fn test_task_relative_path_format() {
        let project = ProjectName::new("super-tauri").unwrap();
        let path = task_relative_path(&project, "task_abc12345", "Add login");
        assert_eq!(
            path,
            "whiteboard/projects/super-tauri/task_abc12345 【TASK】Add login.md"
        );
    }

    #[test]
    fn test_task_relative_path_sanitizes_title() {
        let project = ProjectName::new("super-tauri").unwrap();
        let path = task_relative_path(&project, "task_x", "**Add** / Login?");
        assert!(path.ends_with("task_x 【TASK】Add _ Login_.md"));
    }

    #[test]
    fn test_render_task_markdown_happy_path() {
        let project = ProjectName::new("super-tauri").unwrap();
        let md = render_task_markdown(
            "task_abc12345",
            "Add login",
            "Body with\n- [ ] subtask",
            TaskStatus::Active,
            &project,
            Some("backend"),
            Some("#ffadad"),
        );
        assert!(md.contains("type: project-task"));
        assert!(md.contains("id: task_abc12345"));
        assert!(md.contains("status: active"));
        assert!(md.contains("area: backend"));
        assert!(md.contains("project: super-tauri"));
        assert!(md.contains("color: \"#ffadad\""));
        assert!(md.contains("# 【TASK】Add login"));
        assert!(md.contains("- [ ] subtask"));
    }

    #[test]
    fn test_render_task_markdown_no_optional_fields() {
        let project = ProjectName::new("super-tauri").unwrap();
        let md = render_task_markdown(
            "task_xyz",
            "Simple",
            "",
            TaskStatus::Next,
            &project,
            None,
            None,
        );
        assert!(md.contains("status: next"));
        assert!(md.contains("project: super-tauri"));
        assert!(!md.contains("area:"));
        assert!(!md.contains("color:"));
    }

    // --- create / get / update / delete ---

    /// 测试 helper:构造全默认的 create 输入(title 自填)。
    fn task_create_defaults(title: &str) -> TaskCreateInput<'_> {
        TaskCreateInput {
            title,
            content: None,
            status: TaskStatus::Next,
            area: None,
            color: None,
        }
    }

    /// 测试 helper:构造全 None 的 update 输入(什么也不改)。
    fn task_update_defaults() -> TaskUpdateInput<'static> {
        TaskUpdateInput {
            title: None,
            content: None,
            status: None,
            area: None,
            color: None,
        }
    }

    #[test]
    fn test_create_task_writes_file_and_syncs_db() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();

        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                title: "Add login",
                content: Some("Task body"),
                status: TaskStatus::Active,
                area: Some("backend"),
                color: None,
            },
        )
        .unwrap();

        assert_eq!(task.title, "Add login");
        assert_eq!(task.status, "active");
        assert_eq!(task.area, Some("backend".to_string()));
        assert_eq!(task.project, Some("super-tauri".to_string()));
        assert_eq!(task.whiteboard_id, "projects/super-tauri");

        // 文件写到 vault 了
        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1",
                [&task.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(file_path.starts_with("whiteboard/projects/super-tauri/"));
        assert!(file_path.ends_with("【TASK】Add login.md"));
        let content = vfs.get_file(&file_path).unwrap();
        assert!(content.contains("type: project-task"));
        assert!(content.contains("status: active"));
        assert!(content.contains("# 【TASK】Add login"));
    }

    #[test]
    fn test_create_task_with_color() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();

        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                title: "Colored task",
                color: Some("#ffadad"),
                ..task_create_defaults("Colored task")
            },
        )
        .unwrap();

        assert_eq!(task.color, Some("#ffadad".to_string()));
        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1",
                [&task.id],
                |r| r.get(0),
            )
            .unwrap();
        let content = vfs.get_file(&file_path).unwrap();
        assert!(content.contains("color: \"#ffadad\""));
    }

    #[test]
    fn test_create_task_rejects_empty_title() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();

        let result = create(&conn, &vfs, &project, task_create_defaults("  "));
        assert!(matches!(result, Err(KeysightError::EmptyTitle)));
    }

    #[test]
    fn test_update_task_rewrites_file() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("Old body"),
                ..task_create_defaults("Original")
            },
        )
        .unwrap();

        update(
            &conn,
            &vfs,
            &task.id,
            TaskUpdateInput {
                content: Some("New body"),
                status: Some(TaskStatus::Active),
                ..task_update_defaults()
            },
        )
        .unwrap();

        let loaded = get(&conn, &task.id).unwrap();
        assert_eq!(loaded.content, "New body");
        assert_eq!(loaded.status, "active");
        assert_eq!(loaded.title, "Original");
    }

    #[test]
    fn test_update_task_renames_file_when_title_changes() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(&conn, &vfs, &project, task_create_defaults("Old")).unwrap();
        let old_file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1",
                [&task.id],
                |r| r.get(0),
            )
            .unwrap();

        update(
            &conn,
            &vfs,
            &task.id,
            TaskUpdateInput {
                title: Some("New Title"),
                ..task_update_defaults()
            },
        )
        .unwrap();

        let new_file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1",
                [&task.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(new_file_path, old_file_path);
        assert!(new_file_path.ends_with("【TASK】New Title.md"));
        assert!(vfs.get_file(&old_file_path).is_none());
        assert!(vfs.get_file(&new_file_path).is_some());
    }

    #[test]
    fn test_update_task_sets_color() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(&conn, &vfs, &project, task_create_defaults("Task")).unwrap();

        update(
            &conn,
            &vfs,
            &task.id,
            TaskUpdateInput {
                color: Some("#a0c4ff"),
                ..task_update_defaults()
            },
        )
        .unwrap();

        let loaded = get(&conn, &task.id).unwrap();
        assert_eq!(loaded.color, Some("#a0c4ff".to_string()));
    }

    /// P1-3 回归测试:`task::update` 遇到 DB 中的非法 status 字符串必须 loud fail,
    /// 不能 silently 重置为 Next。之前 `parse_task_status(...).unwrap_or(Next)`
    /// 会让简单的 title rename 调用意外覆盖 legacy 状态值(例如 "wip" → "next"),
    /// 永久丢失用户数据,违反 L0 "想出错都难" 原则。
    #[test]
    fn test_update_task_rejects_invalid_legacy_status() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();

        // 用 sync_file 直接注入带非法 status 的 task(模拟 legacy / 污染数据)
        let md = "---\ntype: project-task\nid: task_legacy0001\nstatus: wip\nproject: super-tauri\n---\n\n# 【TASK】Legacy Task\n\n";
        sync::sync_file(
            &conn,
            "whiteboard/projects/super-tauri/task_legacy0001 【TASK】Legacy Task.md",
            md,
            1000.0,
        )
        .unwrap();

        // 尝试 update(只改 title)—— 必须返回 InvalidTaskStatus,不得静默改状态
        let result = update(
            &conn,
            &vfs,
            "task_legacy0001",
            TaskUpdateInput {
                title: Some("Renamed"),
                ..task_update_defaults()
            },
        );
        assert!(
            matches!(result, Err(KeysightError::InvalidTaskStatus(_))),
            "expected InvalidTaskStatus, got {result:?}"
        );

        // 确认 DB 里的原值没有被 silently 覆盖
        let status: String = conn
            .query_row(
                "SELECT status FROM task_fields WHERE entity_id = 'task_legacy0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "wip", "原始 legacy 值必须保留");
    }

    /// P0-1 回归测试:`task::update` 必须识别 `"default"` sentinel 清空 color
    /// (和 note/question/card/section 一致)。之前实现只有 map + or_else,
    /// 导致 `Some("default")` 被写成字面字符串到 DB/frontmatter。
    #[test]
    fn test_update_task_clears_color_on_default_sentinel() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                color: Some("#ffadad"),
                ..task_create_defaults("Task with color")
            },
        )
        .unwrap();
        assert_eq!(task.color, Some("#ffadad".to_string()));

        // 用 "default" sentinel 清空
        update(
            &conn,
            &vfs,
            &task.id,
            TaskUpdateInput {
                color: Some("default"),
                ..task_update_defaults()
            },
        )
        .unwrap();

        let loaded = get(&conn, &task.id).unwrap();
        assert_eq!(loaded.color, None);

        // 文件 frontmatter 里也不应该有字面 "default" 字符串
        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1",
                [&task.id],
                |r| r.get(0),
            )
            .unwrap();
        let file = vfs.get_file(&file_path).unwrap();
        assert!(
            !file.contains("color: \"default\""),
            "sentinel 不应该被写到文件: {file}"
        );
        assert!(!file.contains("color: default"), "同上");
    }

    #[test]
    fn test_delete_task_removes_file_and_db_rows() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(&conn, &vfs, &project, task_create_defaults("To delete")).unwrap();
        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM entities WHERE id = ?1",
                [&task.id],
                |r| r.get(0),
            )
            .unwrap();

        delete(&conn, &vfs, &task.id).unwrap();

        assert!(vfs.get_file(&file_path).is_none());
        assert!(matches!(
            get(&conn, &task.id),
            Err(KeysightError::NotFound(_))
        ));
    }

    #[test]
    fn test_get_not_found() {
        let conn = test_conn();
        let result = get(&conn, "task_nonexistent");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_query_all_returns_typed_tasks() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        create(&conn, &vfs, &project, task_create_defaults("Task one")).unwrap();
        create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                status: TaskStatus::Active,
                area: Some("backend"),
                ..task_create_defaults("Task two")
            },
        )
        .unwrap();

        let tasks = query_all(&conn, "projects/super-tauri").unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|t| t.whiteboard_id == "projects/super-tauri"));
    }

    #[test]
    fn test_query_all_empty_other_whiteboard() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        create(&conn, &vfs, &project, task_create_defaults("Task")).unwrap();

        let tasks = query_all(&conn, "wb_root").unwrap();
        assert!(tasks.is_empty());
    }

    // --- transition_status(legacy 保留) ---

    fn seed_task_legacy(conn: &Connection) {
        let md = "---\ntype: project-task\nid: task_test0001\nstatus: next\narea: backend\nproject: keysight\n---\n\n# 【TASK】Test Task\n\nTask body.\n";
        sync::sync_file(conn, "whiteboard/projects/keysight/legacy.md", md, 1000.0).unwrap();
    }

    #[test]
    fn test_transition_status_legacy() {
        let conn = test_conn();
        seed_task_legacy(&conn);
        transition_status(&conn, "task_test0001", "active").unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM task_fields WHERE entity_id = 'task_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "active");
    }

    #[test]
    fn test_by_status_legacy_seed() {
        let conn = test_conn();
        seed_task_legacy(&conn);
        let tasks = by_status(&conn, "next").unwrap();
        assert_eq!(tasks.len(), 1);
    }

    // ============================================================
    // compute_position_below_bottommost — auto-position helper
    // ============================================================

    #[test]
    fn test_compute_position_empty_whiteboard_returns_origin() {
        let conn = test_conn();
        let pos = compute_position_below_bottommost(&conn, "projects/test").unwrap();
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 0.0);
    }

    #[test]
    fn test_compute_position_below_single_row() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO positions (entity_id, whiteboard_id, x, y) VALUES ('e1', 'projects/test', 100.0, 200.0)",
            [],
        )
        .unwrap();
        let pos = compute_position_below_bottommost(&conn, "projects/test").unwrap();
        // x 始终是 0.0(简化版),y = 200 + 140 + 40 = 380
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 380.0);
    }

    #[test]
    fn test_compute_position_isolates_whiteboards() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO positions (entity_id, whiteboard_id, x, y) VALUES ('e1', 'projects/A', 0.0, 500.0)",
            [],
        )
        .unwrap();
        // B 白板空,不受 A 白板影响
        let pos = compute_position_below_bottommost(&conn, "projects/B").unwrap();
        assert_eq!(pos.y, 0.0);
    }

    #[test]
    fn test_compute_position_n_rows_picks_max() {
        let conn = test_conn();
        for (i, y) in [50.0_f64, 200.0, 100.0].iter().enumerate() {
            conn.execute(
                "INSERT INTO positions (entity_id, whiteboard_id, x, y) VALUES (?1, 'projects/test', 0.0, ?2)",
                params![format!("e{}", i), *y],
            )
            .unwrap();
        }
        let pos = compute_position_below_bottommost(&conn, "projects/test").unwrap();
        // max(50, 200, 100) = 200, + 140 + 40 = 380
        assert_eq!(pos.y, 380.0);
    }

    // ============================================================
    // task::create — auto-position 集成测试
    // ============================================================

    #[test]
    fn test_create_task_writes_position_row_first_time() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let project = ProjectName::new("test").unwrap();
        let task = create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                title: "first task",
                content: None,
                status: TaskStatus::Inbox,
                area: None,
                color: None,
            },
        )
        .unwrap();

        let pos: (f64, f64) = conn
            .query_row(
                "SELECT x, y FROM positions WHERE entity_id = ?1 AND whiteboard_id = ?2",
                params![&task.id, "projects/test"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        // 首个 task,白板空 → 落在原点
        assert_eq!(pos, (0.0, 0.0));
    }

    #[test]
    fn test_create_task_writes_position_below_bottommost() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let project = ProjectName::new("test").unwrap();
        let _first = create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                title: "first",
                content: None,
                status: TaskStatus::Inbox,
                area: None,
                color: None,
            },
        )
        .unwrap();
        let second = create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                title: "second",
                content: None,
                status: TaskStatus::Inbox,
                area: None,
                color: None,
            },
        )
        .unwrap();

        let second_y: f64 = conn
            .query_row(
                "SELECT y FROM positions WHERE entity_id = ?1",
                params![&second.id],
                |row| row.get(0),
            )
            .unwrap();
        // 第一个 task 落在 y=0 → 第二个 = 0 + DEFAULT_NODE_HEIGHT (140) + NODE_SPACING (40) = 180
        assert_eq!(second_y, 180.0);
    }

    // ============================================================
    // query_kanban — Kanban view 数据源
    // ============================================================

    #[test]
    fn test_query_kanban_with_project_filter() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let project_a = ProjectName::new("alpha").unwrap();
        let project_b = ProjectName::new("beta").unwrap();

        create(
            &conn,
            &fs,
            &project_a,
            TaskCreateInput {
                title: "task in alpha",
                status: TaskStatus::Inbox,
                ..task_create_defaults("task in alpha")
            },
        )
        .unwrap();
        create(
            &conn,
            &fs,
            &project_a,
            TaskCreateInput {
                title: "another in alpha",
                status: TaskStatus::Next,
                ..task_create_defaults("another in alpha")
            },
        )
        .unwrap();
        create(
            &conn,
            &fs,
            &project_b,
            TaskCreateInput {
                title: "task in beta",
                status: TaskStatus::Active,
                ..task_create_defaults("task in beta")
            },
        )
        .unwrap();

        let alpha_tasks = query_kanban(&conn, Some(&project_a)).unwrap();
        assert_eq!(alpha_tasks.len(), 2);
        assert!(alpha_tasks
            .iter()
            .all(|t| t.project.as_deref() == Some("alpha")));
    }

    #[test]
    fn test_query_kanban_all_projects_returns_all_tasks() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let project_a = ProjectName::new("alpha").unwrap();
        let project_b = ProjectName::new("beta").unwrap();

        create(
            &conn,
            &fs,
            &project_a,
            task_create_defaults("a1"),
        )
        .unwrap();
        create(
            &conn,
            &fs,
            &project_b,
            TaskCreateInput {
                status: TaskStatus::Active,
                ..task_create_defaults("b1")
            },
        )
        .unwrap();

        let all = query_kanban(&conn, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_query_kanban_empty_returns_empty_vec() {
        let conn = test_conn();
        let result = query_kanban(&conn, None).unwrap();
        assert_eq!(result.len(), 0);
    }
}
