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

/// 任务状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime: Option<f64>,
}

/// 图谱分组。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphSection {
    pub id: String,
    pub title: String,
    pub card_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_section_ids: Option<Vec<String>>,
}

/// 图谱笔记。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNote {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_section_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_card_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_note_ids: Option<Vec<String>>,
}

/// 卡片别名。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CardAlias {
    pub alias_id: String,
    pub card_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_card_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_section_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_note_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incoming_card_ids: Option<Vec<String>>,
}

/// 边（edges 表的行）。
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from_id: String,
    pub to_id: String,
    pub edge_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
