#![allow(dead_code)] // domain 模块逐步实现后自然消除

use serde::{Deserialize, Serialize};

/// 白板坐标位置。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// 实体类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum EntityKind {
    Card,
    Note,
    Alias,
    Section,
    Task,
    Question,
}

/// 实体数据来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitySource {
    FileBacked,
    DbOnly,
}

impl EntityKind {
    /// 该实体类型的数据来源。
    pub fn source(&self) -> EntitySource {
        match self {
            EntityKind::Card | EntityKind::Note | EntityKind::Task | EntityKind::Question => {
                EntitySource::FileBacked
            }
            EntityKind::Section | EntityKind::Alias => EntitySource::DbOnly,
        }
    }

    /// ID 前缀。
    pub fn id_prefix(&self) -> &'static str {
        match self {
            EntityKind::Card => "card_",
            EntityKind::Note => "note_",
            EntityKind::Alias => "alias_",
            EntityKind::Section => "sec_",
            EntityKind::Task => "task_",
            EntityKind::Question => "q_",
        }
    }

    /// 从 id 前缀推断实体类型。
    pub fn from_id(id: &str) -> Option<Self> {
        if id.starts_with("card_") {
            Some(EntityKind::Card)
        } else if id.starts_with("note_") {
            Some(EntityKind::Note)
        } else if id.starts_with("alias_") {
            Some(EntityKind::Alias)
        } else if id.starts_with("sec_") {
            Some(EntityKind::Section)
        } else if id.starts_with("task_") {
            Some(EntityKind::Task)
        } else if id.starts_with("q_") {
            Some(EntityKind::Question)
        } else {
            None
        }
    }

    /// 数据库中 kind 列的字符串值。
    pub fn as_db_str(&self) -> &'static str {
        match self {
            EntityKind::Card => "card",
            EntityKind::Note => "note",
            EntityKind::Alias => "alias",
            EntityKind::Section => "section",
            EntityKind::Task => "task",
            EntityKind::Question => "question",
        }
    }

    /// 从数据库 kind 字符串解析。
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "card" => Some(EntityKind::Card),
            "note" => Some(EntityKind::Note),
            "alias" => Some(EntityKind::Alias),
            "section" => Some(EntityKind::Section),
            "task" => Some(EntityKind::Task),
            "question" => Some(EntityKind::Question),
            _ => None,
        }
    }
}

/// 边类型（实体间关系）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum EdgeType {
    LinkTo,
    Related,
    SeeAlso,
    SectionLink,
    NoteLink,
    AliasLink,
    CardToAlias,
}

impl EdgeType {
    /// 数据库中 edge_type 列的字符串值。
    pub fn as_db_str(&self) -> &'static str {
        match self {
            EdgeType::LinkTo => "link_to",
            EdgeType::Related => "related",
            EdgeType::SeeAlso => "see_also",
            EdgeType::SectionLink => "section_link",
            EdgeType::NoteLink => "note_link",
            EdgeType::AliasLink => "alias_link",
            EdgeType::CardToAlias => "card_to_alias",
        }
    }

    /// 从数据库字符串解析。
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "link_to" => Some(EdgeType::LinkTo),
            "related" => Some(EdgeType::Related),
            "see_also" => Some(EdgeType::SeeAlso),
            "section_link" => Some(EdgeType::SectionLink),
            "note_link" => Some(EdgeType::NoteLink),
            "alias_link" => Some(EdgeType::AliasLink),
            "card_to_alias" => Some(EdgeType::CardToAlias),
            _ => None,
        }
    }
}

/// 边的视觉样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum EdgeStyle {
    Solid,
    Dashed,
    Dotted,
}

impl EdgeStyle {
    /// 数据库中 style 列的字符串值。
    pub fn as_db_str(&self) -> &'static str {
        match self {
            EdgeStyle::Solid => "solid",
            EdgeStyle::Dashed => "dashed",
            EdgeStyle::Dotted => "dotted",
        }
    }

    /// 从数据库字符串解析。
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "solid" => Some(EdgeStyle::Solid),
            "dashed" => Some(EdgeStyle::Dashed),
            "dotted" => Some(EdgeStyle::Dotted),
            _ => None,
        }
    }
}

/// 任务状态。Inbox 是新增的收集阶段入口，用于 Kanban view 待分配需求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Inbox,
    Next,
    Active,
    Done,
    Blocked,
}

/// 问题状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum QuestionStatus {
    Pending,
    Doing,
    Done,
    Understood,
}

/// 原子卡片。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AtomicCard {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub link_to: Vec<String>,
    pub related: Vec<String>,
    pub understanding: String,
    pub source: String,
    pub see_also: Vec<String>,
    #[serde(default)]
    pub mtime: Option<f64>,
    /// 卡片背景色 — B2 新增,`None` 表示默认色。
    #[serde(default)]
    pub color: Option<String>,
}

/// 图谱分组。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphSection {
    pub id: String,
    pub title: String,
    pub card_ids: Vec<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub linked_section_ids: Option<Vec<String>>,
}

/// 图谱笔记。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNote {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub linked_section_ids: Option<Vec<String>>,
    /// 历史字段：card + alias 混装(alias 视觉上按 card 渲染)。保持原语义不变。
    #[serde(default)]
    pub linked_card_ids: Option<Vec<String>>,
    #[serde(default)]
    pub linked_note_ids: Option<Vec<String>>,
    /// Phase A 子阶段 2b 新增:允许 Note 的 `note_link` edge 指向 Question entity。
    #[serde(default)]
    pub linked_question_ids: Option<Vec<String>>,
    /// Phase A 子阶段 2b 新增:允许 Note 的 `note_link` edge 指向 Task entity。
    #[serde(default)]
    pub linked_task_ids: Option<Vec<String>>,
}

/// 卡片别名。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardAlias {
    pub alias_id: String,
    pub card_id: String,
    /// 历史字段：card + alias 混装(alias 视觉上按 card 渲染)。保持原语义不变。
    #[serde(default)]
    pub linked_card_ids: Option<Vec<String>>,
    #[serde(default)]
    pub linked_section_ids: Option<Vec<String>>,
    #[serde(default)]
    pub linked_note_ids: Option<Vec<String>>,
    /// Phase A 子阶段 2b 新增:允许 Alias 的 `alias_link` edge 指向 Question entity。
    #[serde(default)]
    pub linked_question_ids: Option<Vec<String>>,
    /// Phase A 子阶段 2b 新增:允许 Alias 的 `alias_link` edge 指向 Task entity。
    #[serde(default)]
    pub linked_task_ids: Option<Vec<String>>,
    #[serde(default)]
    pub incoming_card_ids: Option<Vec<String>>,
}

/// 任务实体（entities + task_fields 的联合查询结果）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskEntity {
    pub id: String,
    pub title: String,
    pub content: String,
    pub whiteboard_id: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub area: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    /// 任务卡片背景色 — B2 新增,`None` 表示默认色。
    #[serde(default)]
    pub color: Option<String>,
}

/// 问题实体（entities + question_fields 的联合查询结果）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct QuestionEntity {
    pub id: String,
    pub title: String,
    pub content: String,
    pub whiteboard_id: String,
    pub status: String,
    /// 问题卡片背景色 — B2 新增,`None` 表示默认色。
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub linked_section_ids: Option<Vec<String>>,
    #[serde(default)]
    pub linked_card_ids: Option<Vec<String>>,
    #[serde(default)]
    pub linked_note_ids: Option<Vec<String>>,
    #[serde(default)]
    pub linked_question_ids: Option<Vec<String>>,
    #[serde(default)]
    pub linked_task_ids: Option<Vec<String>>,
}

/// `edges` 表一行的投影（DB 原始列 + 透明字符串字段），供 graph reader 返回原始边数据。
///
/// 注意：本类型是 **DB 行 DTO**，不做类型安全校验；语义层面的「合法 Edge 组合」
/// 由 [`crate::modules::keysight::domain::edge::Edge`] 判别联合表达。Phase A
/// 子阶段 2 会把 `EntityGraph::edges_from` / `edges_to` 的返回值从 `EdgeRow`
/// 升级为 `Edge`（判别联合），届时本类型可能只保留为 bindings 兼容层或彻底删除。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EdgeRow {
    pub from_id: String,
    pub to_id: String,
    pub edge_type: String,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

/// 文件同步结果。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncFileResponse {
    pub updated: u32,
    pub inserted: u32,
    pub deleted: u32,
    pub needs_id_backfill: bool,
    pub assigned_id: String,
}

/// Vault 全量同步结果报告。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncVaultReport {
    /// 文件系统扫描到的 .md 文件总数
    pub scanned: u32,
    /// 实际同步的文件数（new + changed）
    pub synced: u32,
    /// 孤儿清理数（DB 有但文件不存在）
    pub removed: u32,
    /// mtime 未变跳过数
    pub skipped: u32,
    /// ID 回写数
    pub backfilled: u32,
}

/// Legacy details/summary → ?>> / ?<< 迁移报告。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ToggleSyntaxMigrationReport {
    pub files_updated: u32,
    pub db_notes_updated: u32,
}

/// DB-only notes -> markdown files 迁移报告。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteFileMigrationReport {
    pub migrated_notes: u32,
    pub skipped_notes: u32,
    pub db_backup_path: String,
    pub whiteboard_backup_path: String,
}

/// 统计信息。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub cards: u64,
    pub notes: u64,
    pub sections: u64,
    pub aliases: u64,
    pub tasks: u64,
    pub questions: u64,
    pub edges: u64,
}

/// 单卡片完整链接图谱。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardLinksResponse {
    /// 出边 link_to 的目标 id
    pub link_to: Vec<String>,
    /// 出边 related 的目标 id
    pub related: Vec<String>,
    /// 出边 see_also 的目标 id
    pub see_also: Vec<String>,
    /// 入边 link_to（谁 link 到我）
    pub linked_from: Vec<String>,
    /// 入边 related（谁和我 related）
    pub related_from: Vec<String>,
    /// 入边 see_also（谁 see_also 我）
    pub see_also_from: Vec<String>,
}

/// 卡片摘要（用于 overview）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardSummary {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub incoming_link_count: u64,
}

/// 单个白板的概览。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WhiteboardOverview {
    pub whiteboard_id: String,
    pub cards: u64,
    pub sections: u64,
    pub notes: u64,
    pub aliases: u64,
    pub card_summaries: Vec<CardSummary>,
}

/// 图谱总览。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GraphOverviewResponse {
    pub whiteboards: Vec<WhiteboardOverview>,
}

/// 子白板摘要 — 轻量级，不含卡片列表。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WhiteboardSummary {
    pub whiteboard_id: String,
    pub cards: i64,
    pub notes: i64,
    pub sections: i64,
    pub aliases: i64,
    pub tasks: i64,
    pub questions: i64,
}

/// 当前 vault 配置信息。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VaultInfoResponse {
    pub vault_path: String,
}

// ============================================================
// Legacy Import 中间类型
// ============================================================

/// 旧 DB insights 表的一行（v1 schema）。
#[derive(Debug, Clone)]
pub struct LegacyInsight {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub link_to: Vec<String>,
    pub related: Vec<String>,
    pub see_also: Vec<String>,
    pub understanding: String,
    pub source: String,
    pub mtime: f64,
}

/// 旧 DB meta 表中 graph_sections JSON 元素。
#[derive(Debug, Clone)]
pub struct LegacySection {
    pub id: String,
    pub title: String,
    pub color: Option<String>,
    pub card_ids: Vec<String>,
    pub linked_section_ids: Vec<String>,
}

/// 旧 DB meta 表中 graph_notes JSON 元素。
#[derive(Debug, Clone)]
pub struct LegacyNote {
    pub id: String,
    pub title: String,
    pub content: String,
    pub linked_card_ids: Vec<String>,
    pub linked_note_ids: Vec<String>,
    pub linked_section_ids: Vec<String>,
}

/// 旧 DB meta 表中 graph_aliases JSON 元素。
#[derive(Debug, Clone)]
pub struct LegacyAlias {
    pub alias_id: String,
    pub card_id: String,
    pub linked_card_ids: Vec<String>,
    pub linked_section_ids: Vec<String>,
    pub incoming_card_ids: Vec<String>,
}

/// 旧 DB meta 表中 graph_positions JSON 对象的一个 entry。
#[derive(Debug, Clone)]
pub struct LegacyPosition {
    pub entity_id: String,
    pub x: f64,
    pub y: f64,
}

/// meta key 后缀 → 新 whiteboard_id 的映射。
#[derive(Debug, Clone)]
pub struct WhiteboardMapping {
    /// None = root（无后缀的 meta key），Some("chentian") = 子白板
    pub meta_suffix: Option<String>,
    /// 新 DB 中的 whiteboard_id："wb_root" 或子白板名
    pub whiteboard_id: String,
}

/// 导入汇总报告。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub cards: usize,
    pub sections: usize,
    pub notes: usize,
    pub aliases: usize,
    pub edges: usize,
    pub positions: usize,
    pub section_members: usize,
    pub skipped: Vec<SkippedItem>,
}

/// 导入时跳过的条目。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkippedItem {
    pub entity_id: String,
    pub reason: String,
}
