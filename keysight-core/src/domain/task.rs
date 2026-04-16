#![allow(dead_code)]
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params, Connection, OptionalExtension};

use crate::errors::KeysightError;
use crate::id;
use crate::models::{Position, Subtask, TaskEntity, TaskStatus};
use crate::vault_fs::VaultFs;

use super::sync;

// ============================================================================
// ProjectName 值对象 — 防火墙: 非空 + 禁路径分隔符 + 禁 Windows 禁用字符
// ============================================================================

/// Task 所属项目名 —— 决定文件目录的业务标识。
///
/// Invariant: 非空、trim 后非空、不含 `/ \\ : * ? " < > |` 这些路径分隔/Windows 禁用字符。
/// 构造只能通过 [`ProjectName::new`],构造后的 `as_str` 保证合法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectName(String);

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
    pub fn new(value: &str) -> Result<Self, KeysightError> {
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
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 返回对应的 whiteboard_id(固定格式 `projects/{name}`,和 sync 的
    /// `derive_whiteboard_id` 反推规则对齐)。
    pub fn whiteboard_id(&self) -> String {
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

// ============================================================================
// rusqlite 互操作: TaskStatus ↔ TEXT column
//
// 让 `r.get::<_, TaskStatus>(col)` 直接生效,并让 `params![TaskStatus]` 免转字符串。
// 反序列化非法字符串时把 `KeysightError::InvalidTaskStatus` 装进 FromSqlError::Other,
// 外层 `extract_keysight_err` 通过 downcast 还原,保证 DB 脏数据 loud fail 而非 silent coerce。
// ============================================================================

impl FromSql for TaskStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        parse_task_status(s).map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

impl ToSql for TaskStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(
            task_status_to_str(*self).as_bytes(),
        )))
    }
}

/// `rusqlite::Error` → `KeysightError` 转换,特别处理 `FromSqlConversionFailure`:
/// 如果底层 boxed error 是 `KeysightError`(来自 TaskStatus::FromSql 的 `InvalidTaskStatus`),
/// 直接 unbox 还原,让调用方拿到语义清晰的错误变体。
///
/// P1-3 回归保证:legacy "wip" 之类的非法 status 被 SQL 读取时抛出
/// `InvalidTaskStatus` 而非 `Database(...)`,test_update_task_rejects_invalid_legacy_status
/// 依赖此行为。
fn extract_keysight_err(e: rusqlite::Error) -> KeysightError {
    if let rusqlite::Error::FromSqlConversionFailure(_, _, boxed) = e {
        match boxed.downcast::<KeysightError>() {
            Ok(ks) => *ks,
            Err(other) => KeysightError::Database(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                other,
            )),
        }
    } else {
        KeysightError::Database(e)
    }
}

// ============================================================================
// V1.1 Subtask — GFM checklist parser
//
// Task body 里用 `- [ ]` / `- [x]` / `- [X]` 形式列出子任务。Parser 把 body 逐行
// 扫描,识别顶层顶格的 checklist 行并提取 {text, done}。
//
// V1 严格规则: `^- \[( |x|X)\] .+$`,禁前置空白 / 禁 `*+` marker / 禁 checkbox
// 内非法字符 / 禁 checkbox 后无空格 / 禁空 text / 禁嵌套。其他情形视作普通文本,
// 不报错也不 log(parser 宽松,不返回 Result)。
//
// V2 决策: 公开 API `parse_task_checklist` 只返回 `Vec<Subtask>`(无 line_index),
// 位置信息由私有 `ParsedItem` 在 Rust 内部管理,不跨 IPC 泄漏。Write path 用
// `parse_task_checklist_with_positions` 拿位置信息做 checklist 行替换。
// ============================================================================

/// Rust 内部的 parsed item,含位置信息。**不暴露给 IPC**。
///
/// Write path (`render_subtasks_into_body`) 用 `line_index` 定位原 body 中的
/// checklist 行,做单 block 替换 / 多 block 检测。Read path(`parse_task_checklist`)
/// 扔掉 line_index 只返回 `Subtask`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedItem {
    pub subtask: Subtask,
    /// 在 `body.split('\n')` 后的 0-based 行索引。
    pub line_index: usize,
}

/// 解析 task body,返回 GFM checklist 项的 Subtask 列表(无位置信息)。
///
/// 严格规则 `^- \[( |x|X)\] .+$`,详见模块头注释。不识别的行全部跳过。
pub fn parse_task_checklist(body: &str) -> Vec<Subtask> {
    parse_task_checklist_with_positions(body)
        .into_iter()
        .map(|p| p.subtask)
        .collect()
}

/// 带位置信息的版本,供 write path 用。
///
/// 返回的 `Vec<ParsedItem>` 按 body 行顺序升序,`line_index` 即在
/// `body.split('\n')` 后的 0-based 索引。
pub fn parse_task_checklist_with_positions(
    body: &str,
) -> Vec<ParsedItem> {
    body.split('\n')
        .enumerate()
        .filter_map(|(idx, line)| parse_checklist_line(line).map(|s| ParsedItem {
            subtask: s,
            line_index: idx,
        }))
        .collect()
}

/// 尝试解析单行为 Subtask。非 checklist 行返回 `None`。
///
/// 严格规则(V1): 行必须精确匹配 `^- \[( |x|X)\] .+$`。
///
/// - 禁前置空白: 行必须以 `-` 起始
/// - 禁 `*` / `+` marker: 只认 `-`
/// - checkbox 标记只认 ` ` / `x` / `X`,其他字符跳过
/// - checkbox 后必须恰好一个空格
/// - text 必须非空(trim 后)
fn parse_checklist_line(line: &str) -> Option<Subtask> {
    // 必须以 "- [" 起头(避免前置空白被允许)
    let rest = line.strip_prefix("- [")?;
    // 接下来是 ` ` / `x` / `X` 之一,然后是 `]`
    let (marker, after_bracket) = {
        let mut chars = rest.chars();
        let first = chars.next()?;
        let second = chars.next()?;
        if second != ']' {
            return None;
        }
        (first, &rest[2..])
    };
    let done = match marker {
        ' ' => false,
        'x' | 'X' => true,
        _ => return None,
    };
    // `]` 后必须恰好一个空格,然后是非空 text
    let text = after_bracket.strip_prefix(' ')?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(Subtask {
        text: trimmed.to_string(),
        done,
    })
}

/// 把单个 Subtask 渲染成一行 `- [ ] text` 或 `- [x] text`。
fn render_checklist_line(item: &Subtask) -> String {
    let marker = if item.done { 'x' } else { ' ' };
    format!("- [{marker}] {}", item.text)
}

/// 把新 subtasks 合并回 task body,保留所有非 checklist 文本的相对位置。
///
/// # 算法(V2 定稿 - 单 block only)
///
/// 步骤 1:解析 old_body 找出所有 checklist 行位置(`parse_task_checklist_with_positions`)。
///
/// 步骤 2(原 body 无 checklist 行):空 subtasks → 返回 old_body 原样;非空
/// subtasks → 追加到 body 末尾(保证末尾换行)。
///
/// 步骤 3(原 body 有 checklist 行):检查所有 checklist 行的 line_index 是否
/// 形成连续整数序列(之间无非 checklist 行夹杂)。非连续(多 block)返回
/// `Err(KeysightError::MultiBlockChecklist)`。连续则记录 block 起止(min, max),
/// 删除 `[min..=max]` 范围内所有行,在 min 位置插入 new_subtasks 渲染行,join
/// 回字符串。
///
/// # Why fail-closed on multi-block
///
/// V1 "全删后在首位插回"算法会把原本夹在 checklist 之间的自由文本挪位置,
/// 造成文档结构语义丢失。V2 决策(2026-04-15 codex review):V1.1 只支持
/// 单连续 block,多 block 通过 typed error fail-closed 让 TS 显示友好提示
/// 要求用户直接编辑 markdown 文件。V1.2+ 再考虑 per-block 精确 patch。
///
/// # 参数
/// - `task_id`: 用于构造 `MultiBlockChecklist` error 的上下文字段(让 TS
///   知道是哪个 task 出问题)
/// - `old_body`: 当前 task body(通常来自 `current.content` merge base)
/// - `new_subtasks`: TS 侧传来的新 subtasks 列表
pub fn render_subtasks_into_body(
    task_id: &str,
    old_body: &str,
    new_subtasks: &[Subtask],
) -> Result<String, KeysightError> {
    let parsed = parse_task_checklist_with_positions(old_body);

    // Case 1: 原 body 无 checklist 行
    if parsed.is_empty() {
        if new_subtasks.is_empty() {
            return Ok(old_body.to_string());
        }
        // 空 body 特判:`"".split('\n')` 会产生单 ["" ] 而不是空 Vec,直接拼
        // subtasks 避免生成 "\n- [ ] ..." 前导换行
        if old_body.is_empty() {
            return Ok(new_subtasks
                .iter()
                .map(render_checklist_line)
                .collect::<Vec<_>>()
                .join("\n"));
        }
        // 非空 body + 追加 subtasks 到末尾
        let mut lines: Vec<String> = old_body.split('\n').map(|s| s.to_string()).collect();
        // 若末行非空,加一个空行分隔(保证 body 和 checklist 之间有视觉空白)
        if lines.last().map(|s| !s.is_empty()).unwrap_or(false) {
            lines.push(String::new());
        }
        for item in new_subtasks {
            lines.push(render_checklist_line(item));
        }
        return Ok(lines.join("\n"));
    }

    // Case 2: 原 body 有 checklist 行 —— 检查是否单连续 block
    let positions: Vec<usize> = parsed.iter().map(|p| p.line_index).collect();
    let min = positions[0];
    // 例外: positions 由上方 parsed.iter() 构建,parsed 非空已在 Case 1 排除
    let max = *positions.last().unwrap();
    // 单 block 的充要条件: items 数量 == (max - min + 1)
    let is_contig = positions.len() == (max - min + 1);
    if !is_contig {
        // 粗略计算 block 数(相邻 index 差 > 1 时开启新 block)
        let mut block_count = 1usize;
        for w in positions.windows(2) {
            if w[1] - w[0] > 1 {
                block_count += 1;
            }
        }
        return Err(KeysightError::MultiBlockChecklist {
            task_id: task_id.to_string(),
            block_count,
        });
    }

    // Case 2c: 连续 block —— 把 [min..=max] 替换成 new_subtasks
    let mut lines: Vec<String> = old_body.split('\n').map(|s| s.to_string()).collect();
    // 删除原 checklist 范围
    lines.drain(min..=max);
    // 在 min 位置插入新 subtasks
    for (i, item) in new_subtasks.iter().enumerate() {
        lines.insert(min + i, render_checklist_line(item));
    }
    Ok(lines.join("\n"))
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
///
/// TODO(B3 P2): WhiteboardId newtype —— 参数 `whiteboard_id: &str` 仍是 stringly typed,
/// plan 明确推迟到 Phase B3 P2(WhiteboardId newtype + `for_project` 构造器)。
/// 仅有一个调用点 `task::create`,通过 `project.whiteboard_id()` 派生,入口类型安全。
pub fn compute_position_below_bottommost(
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
pub fn get(
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
            let content = r.get::<_, String>(2)?.trim_end_matches('\n').to_string();
            let subtasks = parse_task_checklist(&content);
            Ok(TaskEntity {
                id: r.get(0)?,
                title: r.get(1)?,
                content,
                whiteboard_id: r.get(3)?,
                status: r.get(4)?,
                area: r.get(5)?,
                project: r.get(6)?,
                color: r.get(7)?,
                subtasks,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
        other => extract_keysight_err(other),
    })
}

/// 查询指定白板的所有任务。
pub fn query_all(
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
        let content = r.get::<_, String>(2)?.trim_end_matches('\n').to_string();
        let subtasks = parse_task_checklist(&content);
        Ok(TaskEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            content,
            whiteboard_id: r.get(3)?,
            status: r.get(4)?,
            area: r.get(5)?,
            project: r.get(6)?,
            color: r.get(7)?,
            subtasks,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(extract_keysight_err)
}

/// 查询 kanban view 数据 —— 跨项目或单项目 task list。
///
/// - `project = None` → 跨项目查所有 task entity
/// - `project = Some(name)` → 仅该 project 的 task(whiteboard_id = `projects/{name}`)
///
/// 不按 status 分组(留给前端按 `task.status` 渲染)。V1 按 `e.title` ASC 排序
/// (对齐 `query_all` 的现有行为),因为 entities 表当前没有 `created_at` 列。
/// V2 如需"最新优先"排序再单独加 migration + 字段。
pub fn query_kanban(
    conn: &Connection,
    project: Option<&ProjectName>,
) -> Result<Vec<TaskEntity>, KeysightError> {
    // 两条 SQL 只差一个 WHERE 子句。为了避免动态 param 借用生命周期纠结,
    // 分支展开成两个独立的 prepare/query_map 调用,mapper 复用同一个 closure。
    let mapper = |r: &rusqlite::Row<'_>| -> rusqlite::Result<TaskEntity> {
        let content = r.get::<_, String>(2)?.trim_end_matches('\n').to_string();
        let subtasks = parse_task_checklist(&content);
        Ok(TaskEntity {
            id: r.get(0)?,
            title: r.get(1)?,
            content,
            whiteboard_id: r.get(3)?,
            status: r.get(4)?,
            area: r.get(5)?,
            project: r.get(6)?,
            color: r.get(7)?,
            subtasks,
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
pub fn by_status(
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
pub struct TaskCreateInput<'a> {
    pub title: &'a str,
    pub content: Option<&'a str>,
    pub status: TaskStatus,
    pub area: Option<&'a str>,
    pub color: Option<&'a str>,
}

/// Task 更新输入 —— None 字段保留 current 值,Some 字段覆盖。
pub struct TaskUpdateInput<'a> {
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
pub fn create(
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
pub fn update(
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

    // current.status 已是强类型 TaskStatus(FromSql 在 get() 里已严格解析,
    // legacy 脏数据会 loud fail 为 InvalidTaskStatus,不会走到这里)。
    // P1-3 回归保证: "只改 title" 的 update 不会悄悄把 "wip" 改成 "next"。
    let next_status = input.status.unwrap_or(current.status);

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
pub fn delete(
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
    use crate::db::init_db;
    use crate::vault_fs::MockVaultFs;

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

    // --- parse_task_checklist (V1.1 Phase 6.1) ---

    fn subtask(text: &str, done: bool) -> Subtask {
        Subtask {
            text: text.to_string(),
            done,
        }
    }

    #[test]
    fn test_parse_checklist_empty_string() {
        assert_eq!(parse_task_checklist(""), vec![]);
    }

    #[test]
    fn test_parse_checklist_pure_text_no_checklist() {
        let body = "This is a task description.\n\nSome more text.";
        assert_eq!(parse_task_checklist(body), vec![]);
    }

    #[test]
    fn test_parse_checklist_single_undone() {
        assert_eq!(
            parse_task_checklist("- [ ] Buy milk"),
            vec![subtask("Buy milk", false)]
        );
    }

    #[test]
    fn test_parse_checklist_single_done() {
        assert_eq!(
            parse_task_checklist("- [x] Write tests"),
            vec![subtask("Write tests", true)]
        );
    }

    #[test]
    fn test_parse_checklist_capital_x_is_done() {
        assert_eq!(
            parse_task_checklist("- [X] Capital X"),
            vec![subtask("Capital X", true)]
        );
    }

    #[test]
    fn test_parse_checklist_mixed_with_free_text() {
        let body = "intro paragraph\n\n- [ ] first\n- [x] second\n\n一些说明\n- [X] third\n\nfooter";
        assert_eq!(
            parse_task_checklist(body),
            vec![
                subtask("first", false),
                subtask("second", true),
                subtask("third", true),
            ]
        );
    }

    #[test]
    fn test_parse_checklist_rejects_leading_whitespace() {
        // V1 不支持嵌套,前置空白的行一律不识别为 checklist
        assert_eq!(parse_task_checklist("  - [ ] nested"), vec![]);
        assert_eq!(parse_task_checklist("\t- [x] tab indent"), vec![]);
    }

    #[test]
    fn test_parse_checklist_rejects_no_space_after_bracket() {
        assert_eq!(parse_task_checklist("- [x]No space"), vec![]);
        assert_eq!(parse_task_checklist("- [ ]NoSpace"), vec![]);
    }

    #[test]
    fn test_parse_checklist_rejects_star_marker() {
        // V1 只认 `-`,不认 `*` / `+`
        assert_eq!(parse_task_checklist("* [ ] star marker"), vec![]);
        assert_eq!(parse_task_checklist("+ [ ] plus marker"), vec![]);
    }

    #[test]
    fn test_parse_checklist_rejects_invalid_checkbox_char() {
        // checkbox 内只认 ` ` / `x` / `X`
        assert_eq!(parse_task_checklist("- [y] invalid char"), vec![]);
        assert_eq!(parse_task_checklist("- [-] dash"), vec![]);
        assert_eq!(parse_task_checklist("- [/] slash"), vec![]);
    }

    #[test]
    fn test_parse_checklist_rejects_empty_text() {
        // `- [ ]` 后必须非空 text
        assert_eq!(parse_task_checklist("- [ ] "), vec![]);
        assert_eq!(parse_task_checklist("- [ ]    "), vec![]);
        assert_eq!(parse_task_checklist("- [x] "), vec![]);
    }

    #[test]
    fn test_parse_checklist_with_positions_preserves_line_indices() {
        let body = "intro\n- [ ] a\nmid text\n- [x] b\nouttro";
        let parsed = parse_task_checklist_with_positions(body);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].subtask, subtask("a", false));
        assert_eq!(parsed[0].line_index, 1);
        assert_eq!(parsed[1].subtask, subtask("b", true));
        assert_eq!(parsed[1].line_index, 3);
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
        assert_eq!(task.status, TaskStatus::Active);
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

    // --- Subtasks 填充 (V1.1 Phase 6.2) ---

    /// 创建一个含 GFM checklist body 的 task,通过 get 读回后 subtasks 应该被
    /// parse_task_checklist 正确填充(不为空)。
    #[test]
    fn test_get_task_populates_subtasks_from_checklist_body() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let body = "描述段\n\n- [ ] step one\n- [x] step two\n- [X] step three\n\n备注";
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some(body),
                ..task_create_defaults("With checklist")
            },
        )
        .unwrap();

        let loaded = get(&conn, &task.id).unwrap();
        assert_eq!(
            loaded.subtasks,
            vec![
                subtask("step one", false),
                subtask("step two", true),
                subtask("step three", true),
            ]
        );
    }

    /// 没有 checklist 行的 task body → subtasks 为空 Vec(不是缺失字段)。
    #[test]
    fn test_get_task_empty_subtasks_when_body_has_no_checklist() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("只是一段自由文本\n没有任何 checklist"),
                ..task_create_defaults("No checklist")
            },
        )
        .unwrap();

        let loaded = get(&conn, &task.id).unwrap();
        assert!(loaded.subtasks.is_empty());
    }

    /// query_all 返回的所有 task 都应该有 subtasks 字段正确填充(不是全部空)。
    #[test]
    fn test_query_all_populates_subtasks_per_task() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("- [ ] a\n- [x] b"),
                ..task_create_defaults("Task one")
            },
        )
        .unwrap();
        create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("free text only"),
                ..task_create_defaults("Task two")
            },
        )
        .unwrap();

        let tasks = query_all(&conn, "projects/super-tauri").unwrap();
        assert_eq!(tasks.len(), 2);
        // tasks 按 title ASC 排序 → Task one 先,Task two 后
        let one = tasks.iter().find(|t| t.title == "Task one").unwrap();
        let two = tasks.iter().find(|t| t.title == "Task two").unwrap();
        assert_eq!(
            one.subtasks,
            vec![subtask("a", false), subtask("b", true)]
        );
        assert!(two.subtasks.is_empty());
    }

    /// query_kanban 跨项目 + 单项目两种模式都要填充 subtasks。
    #[test]
    fn test_query_kanban_populates_subtasks() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("- [ ] one\n- [ ] two\n- [x] done"),
                ..task_create_defaults("K task")
            },
        )
        .unwrap();

        // 单项目模式
        let single = query_kanban(&conn, Some(&project)).unwrap();
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].subtasks.len(), 3);
        assert_eq!(single[0].subtasks[2], subtask("done", true));

        // 跨项目模式
        let all = query_kanban(&conn, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].subtasks.len(), 3);
    }

    // --- render_subtasks_into_body (V1.1 Phase 6.3) ---

    /// 空 body + 空 subtasks → 返回空字符串原样。
    #[test]
    fn test_render_subtasks_empty_body_empty_subtasks() {
        let out = render_subtasks_into_body("task_id", "", &[]).unwrap();
        assert_eq!(out, "");
    }

    /// 空 body + 非空 subtasks → 追加到末尾。
    #[test]
    fn test_render_subtasks_empty_body_with_subtasks() {
        let out = render_subtasks_into_body(
            "task_id",
            "",
            &[subtask("first", false), subtask("second", true)],
        )
        .unwrap();
        assert_eq!(out, "- [ ] first\n- [x] second");
    }

    /// 有 body 无 checklist + 非空 subtasks → 追加到 body 末尾(含空行分隔)。
    #[test]
    fn test_render_subtasks_non_checklist_body_appends() {
        let out = render_subtasks_into_body(
            "task_id",
            "free text\nline two",
            &[subtask("a", false)],
        )
        .unwrap();
        assert_eq!(out, "free text\nline two\n\n- [ ] a");
    }

    /// 单连续 block 替换 —— 原 checklist 行被删除,新 subtasks 插入原位置,
    /// 前后非 checklist 文本保留原位。
    #[test]
    fn test_render_subtasks_single_block_replace() {
        let body = "intro\n- [ ] old one\n- [x] old two\nfooter";
        let new = [subtask("new a", false), subtask("new b", true)];
        let out = render_subtasks_into_body("task_id", body, &new).unwrap();
        assert_eq!(out, "intro\n- [ ] new a\n- [x] new b\nfooter");
    }

    /// 单 block + 清空 subtasks → 原 checklist 行全删,前后文本保留。
    #[test]
    fn test_render_subtasks_single_block_clear() {
        let body = "intro\n- [ ] a\n- [x] b\nfooter";
        let out = render_subtasks_into_body("task_id", body, &[]).unwrap();
        assert_eq!(out, "intro\nfooter");
    }

    /// 多 block 检测 → Err(MultiBlockChecklist),block_count 正确。
    #[test]
    fn test_render_subtasks_multi_block_returns_error() {
        // line 0: "intro", 1: "- [ ] a", 2: "mid text", 3: "- [x] b", 4: "footer"
        // positions = [1, 3],len=2 != max-min+1=3 → 多 block
        let body = "intro\n- [ ] a\nmid text\n- [x] b\nfooter";
        let result = render_subtasks_into_body("task_abc", body, &[subtask("new", false)]);
        match result {
            Err(KeysightError::MultiBlockChecklist {
                task_id,
                block_count,
            }) => {
                assert_eq!(task_id, "task_abc");
                assert_eq!(block_count, 2);
            }
            other => panic!("expected MultiBlockChecklist, got {other:?}"),
        }
    }

    /// 多 block(3 个 block)→ block_count = 3。
    #[test]
    fn test_render_subtasks_three_blocks() {
        // lines 0-8: intro / item1 / text1 / item2 / text2 / item3 / text3 / item4 / end
        let body = "intro\n- [ ] a\ntext1\n- [ ] b\ntext2\n- [ ] c\ntext3\n- [ ] d\nend";
        let result = render_subtasks_into_body("t", body, &[]);
        match result {
            Err(KeysightError::MultiBlockChecklist { block_count, .. }) => {
                assert_eq!(block_count, 4);
            }
            other => panic!("expected MultiBlockChecklist with 4 blocks, got {other:?}"),
        }
    }

    /// 幂等性:render 出的结果再 parse 再 render 应该和一次 render 完全一致。
    #[test]
    fn test_render_subtasks_idempotent() {
        let body = "intro\n- [ ] a\n- [x] b\nfooter";
        let subs = parse_task_checklist(body);
        let once = render_subtasks_into_body("t", body, &subs).unwrap();
        let subs2 = parse_task_checklist(&once);
        let twice = render_subtasks_into_body("t", &once, &subs2).unwrap();
        assert_eq!(once, twice);
    }

    /// 归一化:`- [X]` 大写输入 → render 后变小写 `- [x]`。
    #[test]
    fn test_render_subtasks_normalizes_capital_x() {
        let body = "- [X] capital";
        let subs = parse_task_checklist(body);
        assert_eq!(subs, vec![subtask("capital", true)]);
        let rendered = render_subtasks_into_body("t", body, &subs).unwrap();
        assert_eq!(rendered, "- [x] capital");
    }

    // --- task_update_with_subtasks 的端到端模拟(V1.1 Phase 6.3)---
    //
    // command 层的测试通过 integration test 验证(需要 Tauri State),
    // 这里通过 domain 函数直接组合模拟 command 内部流程(get → render → update → get)。

    /// Happy path: 创建含 checklist 的 task,"command"调用改 subtasks,get 返回
    /// 新 subtasks 且非 checklist 文本保留。
    #[test]
    fn test_update_with_subtasks_replaces_checklist_and_preserves_free_text() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("intro para\n\n- [ ] a\n- [x] b\n\nfooter"),
                ..task_create_defaults("With checklist")
            },
        )
        .unwrap();

        // 模拟 command: get → render → update
        let current = get(&conn, &task.id).unwrap();
        let new_subs = vec![
            subtask("replaced 1", true),
            subtask("replaced 2", false),
            subtask("new 3", false),
        ];
        let new_body =
            render_subtasks_into_body(&task.id, &current.content, &new_subs).unwrap();
        update(
            &conn,
            &vfs,
            &task.id,
            TaskUpdateInput {
                content: Some(&new_body),
                ..task_update_defaults()
            },
        )
        .unwrap();

        let fresh = get(&conn, &task.id).unwrap();
        assert_eq!(fresh.subtasks, new_subs);
        // 非 checklist 文本保留 —— "intro para" 和 "footer" 都还在
        assert!(fresh.content.contains("intro para"));
        assert!(fresh.content.contains("footer"));
    }

    /// 清空 subtasks:传空 Vec,body 的 checklist 行全部消失,非 checklist 文本保留。
    #[test]
    fn test_update_with_subtasks_clear_all_keeps_free_text() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("说明段\n\n- [ ] a\n- [x] b\n\n结尾段"),
                ..task_create_defaults("Has items")
            },
        )
        .unwrap();

        let current = get(&conn, &task.id).unwrap();
        let new_body = render_subtasks_into_body(&task.id, &current.content, &[]).unwrap();
        update(
            &conn,
            &vfs,
            &task.id,
            TaskUpdateInput {
                content: Some(&new_body),
                ..task_update_defaults()
            },
        )
        .unwrap();

        let fresh = get(&conn, &task.id).unwrap();
        assert!(fresh.subtasks.is_empty());
        assert!(fresh.content.contains("说明段"));
        assert!(fresh.content.contains("结尾段"));
    }

    /// Multi-block 场景:task body 有多 block,render 返 Err,task 数据未被修改。
    #[test]
    fn test_update_with_subtasks_multi_block_fails_closed() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let body = "前言\n- [ ] a\n中间说明\n- [x] b\n结尾";
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some(body),
                ..task_create_defaults("Multi block")
            },
        )
        .unwrap();

        let current = get(&conn, &task.id).unwrap();
        let result = render_subtasks_into_body(
            &task.id,
            &current.content,
            &[subtask("new", false)],
        );
        assert!(matches!(
            result,
            Err(KeysightError::MultiBlockChecklist { .. })
        ));

        // task 原数据未变
        let unchanged = get(&conn, &task.id).unwrap();
        assert_eq!(unchanged.subtasks.len(), 2);
        assert_eq!(unchanged.subtasks[0].text, "a");
        assert_eq!(unchanged.subtasks[1].text, "b");
    }

    /// 同时改 title + subtasks:title rename + body subtasks 替换都生效。
    #[test]
    fn test_update_with_subtasks_and_title_simultaneously() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let project = ProjectName::new("super-tauri").unwrap();
        let task = create(
            &conn,
            &vfs,
            &project,
            TaskCreateInput {
                content: Some("- [ ] old item"),
                ..task_create_defaults("Old Title")
            },
        )
        .unwrap();

        let current = get(&conn, &task.id).unwrap();
        let new_subs = vec![subtask("new item", true)];
        let new_body =
            render_subtasks_into_body(&task.id, &current.content, &new_subs).unwrap();
        update(
            &conn,
            &vfs,
            &task.id,
            TaskUpdateInput {
                title: Some("New Title"),
                content: Some(&new_body),
                ..task_update_defaults()
            },
        )
        .unwrap();

        let fresh = get(&conn, &task.id).unwrap();
        assert_eq!(fresh.title, "New Title");
        assert_eq!(fresh.subtasks, new_subs);
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
        assert_eq!(loaded.status, TaskStatus::Active);
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

    #[test]
    fn test_query_kanban_covers_all_5_statuses_and_orders_by_title() {
        let conn = test_conn();
        let fs = MockVaultFs::new();
        let project = ProjectName::new("test").unwrap();

        // 故意乱序创建,让 query 排序能力能被观测到
        create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                status: TaskStatus::Next,
                ..task_create_defaults("n task")
            },
        )
        .unwrap();
        create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                status: TaskStatus::Inbox,
                ..task_create_defaults("i task")
            },
        )
        .unwrap();
        create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                status: TaskStatus::Done,
                ..task_create_defaults("d task")
            },
        )
        .unwrap();
        create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                status: TaskStatus::Active,
                ..task_create_defaults("a task")
            },
        )
        .unwrap();
        create(
            &conn,
            &fs,
            &project,
            TaskCreateInput {
                status: TaskStatus::Blocked,
                ..task_create_defaults("b task")
            },
        )
        .unwrap();

        let tasks = query_kanban(&conn, Some(&project)).unwrap();
        assert_eq!(tasks.len(), 5);

        // ORDER BY e.title ASC —— 不是按插入顺序或 status 顺序
        let titles: Vec<&str> = tasks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["a task", "b task", "d task", "i task", "n task"]
        );

        // 5 个 status 每个都出现一次
        let mut statuses: Vec<&str> = tasks
            .iter()
            .map(|t| task_status_to_str(t.status))
            .collect();
        statuses.sort();
        assert_eq!(
            statuses,
            vec!["active", "blocked", "done", "inbox", "next"]
        );
    }
}
