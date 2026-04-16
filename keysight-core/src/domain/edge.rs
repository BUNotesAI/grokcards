#![allow(dead_code)] // Phase A 子阶段 1 type sketch：impl 落在子阶段 2 才会消费这些类型

//! # Keysight Edge 判别联合 — Phase A 子阶段 1 类型骨架
//!
//! 本文件是 **type sketch**，只放类型声明 + 字符串边界 parse + 单测，
//! 零业务 impl。目的：让调用端在看到具体 shape 之前确认它是否「能表达
//! 全部合法组合，且非法组合根本写不出」。
//!
//! 子阶段 2 会把现有 `EntityGraph::connect(from: &str, to: &str, edge_type: EdgeType, ...)`
//! 的 stringly-typed 接口换成 `connect(edge: &Edge)`，并删除 `domain::entity::resolve_user_drawn_edge_type`
//! 这个绷带 helper（commit `d29d6df`）。
//!
//! ## 业务语义的关键背景
//!
//! LinkTo / Related / SeeAlso **都是画布层面的元素关系**（画布上未展开状态下看到的
//! 相关链接），**不是**从 card/note 正文 markdown 里 `[[...]]` 解析出来的 wiki link。
//! 它们是用户在白板上手动建立的独立数据，和正文内容解耦。子阶段 2 落 impl 时，
//! 这些 edge 的读写路径不应该触碰 markdown 正文 parse。
//!
//! ## 防火墙原则（CLAUDE.md L0「建模优先 + 强类型」）
//!
//! 1. **Section 和 Task 不主动发边**：[`Edge`] 里不存在 from 为 `SectionId` / `TaskId`
//!    的变体。编译期阻止它们作为 link source。
//!
//! 2. **Reader 必须穷尽 match [`EntityId`]，禁止 `_` 通配**：四个通用 link 变体
//!    （[`Edge::CardLink`] / [`Edge::NoteLink`] / [`Edge::AliasLink`] / [`Edge::QuestionLink`]）
//!    的 `to` 字段是 [`EntityId`]，涵盖 6 类 entity。任何 reader（如子阶段 2 的
//!    `note.rs::get` / `alias.rs::get` 的 reconstruct 路径）读到一条 edge 后，
//!    必须穷尽 match EntityId 所有 6 个 variant，**禁止用 `_` 通配吞掉未知 kind**。
//!    这是踩坑样例 1 的新防御核心 —— 旧 `note.rs::get` 的
//!    `if target.starts_with("card_") { ... } else if target.starts_with("sec_") { ... }`
//!    if-else 链就是 silent drop 的元凶，让 Question/Task 目标变成 orphan edge。
//!
//! 3. **Alias 的独立 edge 能力仅限 [`Edge::AliasLink`] 一种**（它自己画的箭头）。
//!    Alias 没有自己的 Related / SeeAlso —— UI 显示 alias 节点时，**顺着
//!    [`Edge::CardToAlias`] 反查到 owning card，继承读取 card 的 Related / SeeAlso
//!    显示信息**。这就是 `CardToAlias` 存在的根本语义：它不是冗余，是 alias 继承
//!    链路的载体。Alias 正文内容本身也直接用 owning card 的内容渲染。
//!
//! 4. **所有 entity id 必须经过 [`EntityId::parse`] 进入类型系统**，parse 一次后免检。
//!
//! 5. **SeeAlso 目标是 [`ObsidianLink`]，不是 entity**：Card 和 Note 的 SeeAlso
//!    指向外部 Obsidian 文档，点击时通过 `obsidian://open?vault=...&file=...` URI
//!    外呼 Obsidian 打开。Alias / Question 不支持 SeeAlso。
//!
//! ## 合法 happy path（fake usage，仅演示 type 表达力）
//!
//! ```ignore
//! fn demo(
//!     card: CardId,
//!     note: NoteId,
//!     sec: SectionId,
//!     alias: AliasId,
//!     question: QuestionId,
//!     task: TaskId,
//!     doc: ObsidianLink,
//! ) {
//!     // Card 的独立 edge 能力：CardLink (任何 entity) / CardRelated (仅 Card) / CardSeeAlso (Obsidian)
//!     let _ = Edge::CardLink { from: card.clone(), to: EntityId::Card(card.clone()) };
//!     let _ = Edge::CardLink { from: card.clone(), to: EntityId::Note(note.clone()) };
//!     let _ = Edge::CardLink { from: card.clone(), to: EntityId::Section(sec.clone()) };
//!     let _ = Edge::CardLink { from: card.clone(), to: EntityId::Alias(alias.clone()) };
//!     let _ = Edge::CardLink { from: card.clone(), to: EntityId::Question(question.clone()) };
//!     let _ = Edge::CardLink { from: card.clone(), to: EntityId::Task(task.clone()) };
//!     let _ = Edge::CardRelated { from: card.clone(), to: card.clone() };
//!     let _ = Edge::CardSeeAlso { from: card.clone(), to: doc.clone() };
//!
//!     // Note 的独立 edge 能力：NoteLink (任何 entity，含 Q/T) / NoteSeeAlso (Obsidian)
//!     let _ = Edge::NoteLink { from: note.clone(), to: EntityId::Card(card.clone()) };
//!     let _ = Edge::NoteLink { from: note.clone(), to: EntityId::Question(question.clone()) };
//!     let _ = Edge::NoteLink { from: note.clone(), to: EntityId::Task(task.clone()) };
//!     let _ = Edge::NoteSeeAlso { from: note.clone(), to: doc.clone() };
//!
//!     // Alias 唯一的独立 edge 能力：AliasLink (自己的箭头，可指向 6 类 entity)
//!     let _ = Edge::AliasLink { from: alias.clone(), to: EntityId::Question(question.clone()) };
//!     let _ = Edge::AliasLink { from: alias.clone(), to: EntityId::Task(task.clone()) };
//!
//!     // Question 的 link 能力：QuestionLink (任何 entity，但不能 SeeAlso Obsidian)
//!     let _ = Edge::QuestionLink { from: question, to: EntityId::Card(card.clone()) };
//!
//!     // Card 定义了某个 Alias —— alias 继承显示 Related/SeeAlso 的反查路径
//!     let _ = Edge::CardToAlias { card, alias };
//! }
//! ```
//!
//! ## 写不出的非法组合（编译失败演示）
//!
//! ```ignore
//! fn illegal(
//!     card: CardId,
//!     note: NoteId,
//!     sec: SectionId,
//!     alias: AliasId,
//!     question: QuestionId,
//!     task: TaskId,
//!     doc: ObsidianLink,
//! ) {
//!     // 编译失败: 不存在 SectionLink / SectionMember 变体，section 不主动发边
//!     // let _ = Edge::SectionLink { from: sec, to: EntityId::Card(card.clone()) };
//!
//!     // 编译失败: 不存在 TaskLink 变体，task 不主动发边
//!     // let _ = Edge::TaskLink { from: task, to: EntityId::Card(card.clone()) };
//!
//!     // 编译失败: 不存在 AliasSeeAlso / AliasRelated 变体，alias 继承 owning card
//!     // let _ = Edge::AliasSeeAlso { from: alias.clone(), to: doc.clone() };
//!     // let _ = Edge::AliasRelated { from: alias, to: card.clone() };
//!
//!     // 编译失败: 不存在 QuestionSeeAlso 变体，question 不支持 obsidian 外部引用
//!     // let _ = Edge::QuestionSeeAlso { from: question, to: doc.clone() };
//!
//!     // 编译失败: 不存在 NoteRelated 变体，Related 是 card 的独占能力
//!     // let _ = Edge::NoteRelated { from: note, to: card.clone() };
//!
//!     // 编译失败: CardRelated.to 是 CardId，传 EntityId 类型不匹配
//!     // let _ = Edge::CardRelated { from: card, to: EntityId::Note(note) };
//!
//!     // 编译失败: CardSeeAlso.to 是 ObsidianLink，传 CardId 类型不匹配
//!     // let _ = Edge::CardSeeAlso { from: card.clone(), to: card };
//!
//!     // 编译失败: NoteLink.from 是 NoteId，传 CardId 类型不匹配
//!     // let _ = Edge::NoteLink { from: card, to: EntityId::Note(note) };
//! }
//! ```

use crate::errors::KeysightError;

// ====================================================================
// Newtype id：每类实体一个强类型 wrapper
// ====================================================================

/// Card 实体 id（DB prefix: `card_`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CardId(String);

impl CardId {
    /// 仅限本文件内部使用的无校验构造入口，外部代码必须通过 [`EntityId::parse`] 进入
    fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    /// 取出底层字符串表示，供 DB 参数绑定 / 日志使用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Note 实体 id（DB prefix: `note_`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteId(String);

impl NoteId {
    fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Alias 实体 id（DB prefix: `alias_`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AliasId(String);

impl AliasId {
    fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Section 实体 id（DB prefix: `sec_`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SectionId(String);

impl SectionId {
    fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Question 实体 id（DB prefix: `q_`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuestionId(String);

impl QuestionId {
    fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Task 实体 id（DB prefix: `task_`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ====================================================================
// ObsidianLink：SeeAlso 边的目标 —— 不是 entity，是外部 Obsidian 文档引用
// ====================================================================

/// Obsidian 风格的文档引用，作为 [`Edge::CardSeeAlso`] / [`Edge::NoteSeeAlso`] 的目标。
///
/// 与 entity id 不同，SeeAlso 的目标不是库内的 card/note/...，而是一个
/// 可以通过 `obsidian://open?vault=...&file=...` URI schema 在 Obsidian
/// 中打开的外部文档引用（原始来源是 card 在画布上手动添加的 Obsidian
/// wiki link 字面量，格式可能是 `[[some-doc]]` / `[[some-doc#heading]]` /
/// `[[some-doc|alias]]`）。
///
/// 脱离 Obsidian 插件运行后，keysight 自身不再直接渲染目标文档，
/// 但保留 SeeAlso 作为独立变体，让 UI 点击时拼出 obsidian URI 外呼 Obsidian。
///
/// 子阶段 2 会补上构造入口：目前只在类型层存在，保证 [`Edge::CardSeeAlso`]
/// 的 shape 是明确的外部引用而非 entity 引用。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObsidianLink(String);

impl ObsidianLink {
    /// 仅限本文件内部使用的无校验构造入口。
    ///
    /// 子阶段 2 会决定是否补一个 `parse` 校验 wiki link 格式（`[[...]]` vs 裸路径）。
    /// Type sketch 阶段先不约束格式，避免在对 DB 真实存储值确认前就做过度校验。
    fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    /// 取出底层字符串表示，供拼 obsidian URI / DB 参数绑定使用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ====================================================================
// EntityId：所有实体 id 的判别联合，字符串边界的唯一入口
// ====================================================================

/// 所有 keysight 实体 id 的判别联合。
///
/// 跨 IPC / DB 边界过来的字符串必须通过 [`EntityId::parse`] 进入强类型，
/// 之后业务代码只接受 [`EntityId`] 或具体的 [`CardId`] / [`NoteId`] / ...，
/// 不再接受 `&str`。这是「parse don't validate」的落地。
///
/// [`EntityId`] 同时也作为通用 link 变体（[`Edge::CardLink`] / [`Edge::NoteLink`] /
/// [`Edge::AliasLink`] / [`Edge::QuestionLink`]）的 `to` 类型。任何 reader 读到
/// 这些变体时，**必须穷尽 match 所有 6 个 variant**，禁止用 `_` 通配 —— 这是
/// 踩坑样例 1 的新防御核心。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityId {
    Card(CardId),
    Note(NoteId),
    Alias(AliasId),
    Section(SectionId),
    Question(QuestionId),
    Task(TaskId),
}

impl EntityId {
    /// 从字符串 id 边界进入强类型。失败返回 [`IdError::UnknownPrefix`]。
    ///
    /// 当前只按 prefix 派发，不深度校验 prefix 后的 hex 后缀 —— prefix 匹配后
    /// 已经保证是这一类实体，后缀的完整性由 DB schema 和写入路径保证，读侧不
    /// 重复校验。Prefix 之间互相不是前缀关系（`card_` / `note_` / `alias_` /
    /// `sec_` / `q_` / `task_`），匹配顺序无关。空字符串也归入 `UnknownPrefix("")`。
    pub fn parse(s: &str) -> Result<Self, IdError> {
        if s.starts_with("card_") {
            Ok(Self::Card(CardId::new_unchecked(s.to_string())))
        } else if s.starts_with("note_") {
            Ok(Self::Note(NoteId::new_unchecked(s.to_string())))
        } else if s.starts_with("alias_") {
            Ok(Self::Alias(AliasId::new_unchecked(s.to_string())))
        } else if s.starts_with("sec_") {
            Ok(Self::Section(SectionId::new_unchecked(s.to_string())))
        } else if s.starts_with("q_") {
            Ok(Self::Question(QuestionId::new_unchecked(s.to_string())))
        } else if s.starts_with("task_") {
            Ok(Self::Task(TaskId::new_unchecked(s.to_string())))
        } else {
            Err(IdError::UnknownPrefix(s.to_string()))
        }
    }

    /// 取出底层字符串表示，供 DB 参数绑定 / 日志使用
    pub fn as_str(&self) -> &str {
        match self {
            Self::Card(id) => id.as_str(),
            Self::Note(id) => id.as_str(),
            Self::Alias(id) => id.as_str(),
            Self::Section(id) => id.as_str(),
            Self::Question(id) => id.as_str(),
            Self::Task(id) => id.as_str(),
        }
    }
}

// ====================================================================
// Edge：合法连接的判别联合
// ====================================================================

/// 合法的 Edge 组合。每个变体锁死 (from_kind, to_kind) 组合 —— 非法组合
/// （如 section 作为 from、alias 有独立 SeeAlso）根本没有对应变体，编译期就死。
///
/// ## 变体全景
///
/// | Edge variant     | from      | to             | 子阶段 2 DB `edge_type` 映射 |
/// |------------------|-----------|----------------|-------------------------------|
/// | `CardLink`       | Card      | EntityId (任意)| `link_to`                     |
/// | `CardRelated`    | Card      | Card           | `related`                     |
/// | `CardSeeAlso`    | Card      | Obsidian 外部  | `see_also`                    |
/// | `NoteLink`       | Note      | EntityId (任意)| `note_link`                   |
/// | `NoteSeeAlso`    | Note      | Obsidian 外部  | TBD（旧 schema 无，新增）     |
/// | `AliasLink`      | Alias     | EntityId (任意)| `alias_link`                  |
/// | `QuestionLink`   | Question  | EntityId (任意)| TBD（旧 schema 无，新增）     |
/// | `CardToAlias`    | Card      | Alias          | `card_to_alias`               |
///
/// ## 故意不存在的变体（编译期死）
///
/// - **`SectionLink` / `SectionMember`** — section 不主动发边（用户决策）。
///   section 拥有成员的关系在独立的 `section_members` 表，**不属于** Edge 枚举，
///   建模留给未来另一个 task。
/// - **`TaskLink`** — task 不主动发边。
/// - **`AliasSeeAlso` / `AliasRelated`** — alias 没有独立的 SeeAlso / Related。
///   UI 显示 alias 节点时，顺着 [`Edge::CardToAlias`] 反查 owning card，继承读取
///   card 的这两类信息。Alias 唯一的独立 edge 能力就是 [`AliasLink`](Edge::AliasLink)
///   （它自己画的箭头）。
/// - **`QuestionSeeAlso`** — question 不支持 obsidian 外部引用。
/// - **`NoteRelated`** — Related 是 card 的独占能力（Related picker）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edge {
    /// Card 到任意 entity 的 wiki 链接（DB: `link_to`）。
    ///
    /// 业务上 Card 的箭头可以指向 6 类 entity 中的任意一种。
    CardLink { from: CardId, to: EntityId },

    /// Card 到 Card 的 Related 关系（DB: `related`，Related picker 建立）
    CardRelated { from: CardId, to: CardId },

    /// Card 到 **Obsidian 外部文档** 的「另见」引用（DB: `see_also`）。
    ///
    /// 目标不是库内 entity，而是 [`ObsidianLink`] —— 点击时通过
    /// `obsidian://open?vault=...&file=...` URI 在 Obsidian 中打开。
    CardSeeAlso { from: CardId, to: ObsidianLink },

    /// Note 到任意 entity 的链接（DB: `note_link`）。
    ///
    /// 业务上 Note 的箭头可以指向 6 类 entity 中的任意一种，**包括 Question / Task**。
    ///
    /// 踩坑样例 1 的防御不再靠「NoteLinkTarget 不含 Q/T」的不可表达机制，
    /// 而是改为**强制 reader 穷尽 match [`EntityId`]**。参见模块 `//!` doc
    /// 第 2 条防火墙原则。
    NoteLink { from: NoteId, to: EntityId },

    /// Note 到 **Obsidian 外部文档** 的「另见」引用（DB: TBD 新增 edge_type）
    NoteSeeAlso { from: NoteId, to: ObsidianLink },

    /// Alias 到任意 entity 的链接（DB: `alias_link`）
    ///
    /// **这是 alias 唯一的独立 edge 能力**。Alias 的 Related / SeeAlso 等其他
    /// 显示信息由 UI 层顺着 [`Edge::CardToAlias`] 反查 owning card 继承显示，
    /// 不是 alias 自己存的。
    AliasLink { from: AliasId, to: EntityId },

    /// Question 到任意 entity 的链接（DB: TBD 新增 edge_type）
    ///
    /// Question 可以主动 link 到 6 类 entity 中的任意一种，但**不能 SeeAlso
    /// obsidian 外部文档**（没有 `QuestionSeeAlso` 变体）。
    QuestionLink { from: QuestionId, to: EntityId },

    /// Card 定义了某个 Alias（DB: `card_to_alias`）
    ///
    /// **这是 alias 继承机制的载体**：UI 显示 alias 节点时，顺着本变体反查到
    /// owning card，继承读取 card 的 Related / SeeAlso / 正文内容等显示信息。
    /// 因此本变体**不是冗余**（和 `alias_fields.card_id` 字段看似重复），而是
    /// 独立且必要的指针。子阶段 2 落 impl 时须确认两者是否需要保持同步。
    CardToAlias { card: CardId, alias: AliasId },
}

impl Edge {
    /// 把 [`Edge`] 变体拆成 DB `edges` 表一行的三个核心字段:
    /// `(from_id_str, to_id_str, edge_type_str)`。
    ///
    /// 子阶段 2 [`crate::domain::entity::SqliteEntityGraph::connect`]
    /// 调用本方法落 SQL。style / label 当前统一写 NULL(旧 API 的 style/label
    /// 参数已整体退役 —— TS 从未使用,生产代码从未设值)。
    pub fn db_insert_values(&self) -> (&str, &str, &'static str) {
        match self {
            Self::CardLink { from, to } => (from.as_str(), to.as_str(), "link_to"),
            Self::CardRelated { from, to } => (from.as_str(), to.as_str(), "related"),
            Self::CardSeeAlso { from, to } => (from.as_str(), to.as_str(), "see_also"),
            Self::NoteLink { from, to } => (from.as_str(), to.as_str(), "note_link"),
            Self::NoteSeeAlso { from, to } => (from.as_str(), to.as_str(), "note_see_also"),
            Self::AliasLink { from, to } => (from.as_str(), to.as_str(), "alias_link"),
            Self::QuestionLink { from, to } => (from.as_str(), to.as_str(), "question_link"),
            Self::CardToAlias { card, alias } => (card.as_str(), alias.as_str(), "card_to_alias"),
        }
    }
}

// ====================================================================
// 意图函数：将用户意图（画箭头 / Related / SeeAlso）映射为合法 Edge
// ====================================================================

/// 把用户「从画布节点 A 画箭头到节点 B」的意图映射为合法 [`Edge`]。
///
/// 这是替代旧 `commands.rs::entity_connect(from: &str, to: &str, edge_type: EdgeType)`
/// 三参数逃生舱口的强类型入口 —— 参数已经是 [`EntityId`]（调用方必须先跑
/// [`EntityId::parse`]），从 kind 决定返回哪种 `*Link` 变体，编译器强制穷尽 match。
///
/// 业务规则：
/// - `Card` / `Note` / `Alias` / `Question` 作为 from → 对应 `*Link` 变体 +
///   `to: EntityId` 保留全部 6 类 entity 目标
/// - `Section` / `Task` 作为 from → [`KeysightError::ConnectionNotAllowed`]
///   （section / task 不主动发边，防火墙原则 1）
///
/// 本函数**不处理** Related picker 和 SeeAlso 的用户意图：
/// - `Edge::CardRelated` 由 commands 层直接构造（意图来自 Related picker UI）
/// - `Edge::CardSeeAlso` / `Edge::NoteSeeAlso` 由 commands 层直接构造
///   （意图来自 SeeAlso 面板 UI，to 是 [`ObsidianLink`] 不是 [`EntityId`]）
pub fn user_draw_edge(
    from: EntityId,
    to: EntityId,
) -> Result<Edge, KeysightError> {
    match from {
        EntityId::Card(c) => Ok(Edge::CardLink { from: c, to }),
        EntityId::Note(n) => Ok(Edge::NoteLink { from: n, to }),
        EntityId::Alias(a) => Ok(Edge::AliasLink { from: a, to }),
        EntityId::Question(q) => Ok(Edge::QuestionLink { from: q, to }),
        EntityId::Section(_) => Err(KeysightError::ConnectionNotAllowed {
            from_kind: "section",
        }),
        EntityId::Task(_) => Err(KeysightError::ConnectionNotAllowed {
            from_kind: "task",
        }),
    }
}

// ====================================================================
// IdError：parse 边界的错误类型
// ====================================================================

/// id 边界 parse 时可能发生的错误
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdError {
    /// 传入字符串不匹配任何已知 id prefix（含空字符串 —— 空字符串也归入此类）
    #[error("未知的 id prefix: {0:?}")]
    UnknownPrefix(String),
}

// ====================================================================
// Tests：EntityId::parse 覆盖 6 个合法 prefix + 1 个非法 prefix
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_card_prefix() {
        let id = EntityId::parse("card_aaaa1111").unwrap();
        assert!(matches!(id, EntityId::Card(ref c) if c.as_str() == "card_aaaa1111"));
    }

    #[test]
    fn parse_note_prefix() {
        let id = EntityId::parse("note_bbbb2222").unwrap();
        assert!(matches!(id, EntityId::Note(ref n) if n.as_str() == "note_bbbb2222"));
    }

    #[test]
    fn parse_alias_prefix() {
        let id = EntityId::parse("alias_cccc3333").unwrap();
        assert!(matches!(id, EntityId::Alias(ref a) if a.as_str() == "alias_cccc3333"));
    }

    #[test]
    fn parse_section_prefix() {
        let id = EntityId::parse("sec_dddd4444").unwrap();
        assert!(matches!(id, EntityId::Section(ref s) if s.as_str() == "sec_dddd4444"));
    }

    #[test]
    fn parse_question_prefix() {
        let id = EntityId::parse("q_eeee5555").unwrap();
        assert!(matches!(id, EntityId::Question(ref q) if q.as_str() == "q_eeee5555"));
    }

    #[test]
    fn parse_task_prefix() {
        let id = EntityId::parse("task_ffff6666").unwrap();
        assert!(matches!(id, EntityId::Task(ref t) if t.as_str() == "task_ffff6666"));
    }

    #[test]
    fn parse_unknown_prefix_returns_error() {
        let err = EntityId::parse("xyz_garbage").unwrap_err();
        assert_eq!(err, IdError::UnknownPrefix("xyz_garbage".to_string()));
    }

    // --------------------------------------------------------------
    // user_draw_edge：6 种 from kind 全覆盖
    // --------------------------------------------------------------

    #[test]
    fn user_draw_edge_card_source_returns_card_link() {
        let from = EntityId::parse("card_aaaa1111").unwrap();
        let to = EntityId::parse("note_bbbb2222").unwrap();
        let edge = user_draw_edge(from, to).unwrap();
        match edge {
            Edge::CardLink { from, to } => {
                assert_eq!(from.as_str(), "card_aaaa1111");
                assert!(matches!(to, EntityId::Note(ref n) if n.as_str() == "note_bbbb2222"));
            }
            other => panic!("期望 Edge::CardLink，实际: {other:?}"),
        }
    }

    #[test]
    fn user_draw_edge_note_source_returns_note_link() {
        let from = EntityId::parse("note_aaaa1111").unwrap();
        let to = EntityId::parse("q_bbbb2222").unwrap();
        let edge = user_draw_edge(from, to).unwrap();
        match edge {
            Edge::NoteLink { from, to } => {
                assert_eq!(from.as_str(), "note_aaaa1111");
                assert!(matches!(to, EntityId::Question(ref q) if q.as_str() == "q_bbbb2222"));
            }
            other => panic!("期望 Edge::NoteLink，实际: {other:?}"),
        }
    }

    #[test]
    fn user_draw_edge_alias_source_returns_alias_link() {
        let from = EntityId::parse("alias_aaaa1111").unwrap();
        let to = EntityId::parse("task_bbbb2222").unwrap();
        let edge = user_draw_edge(from, to).unwrap();
        match edge {
            Edge::AliasLink { from, to } => {
                assert_eq!(from.as_str(), "alias_aaaa1111");
                assert!(matches!(to, EntityId::Task(ref t) if t.as_str() == "task_bbbb2222"));
            }
            other => panic!("期望 Edge::AliasLink，实际: {other:?}"),
        }
    }

    #[test]
    fn user_draw_edge_question_source_returns_question_link() {
        let from = EntityId::parse("q_aaaa1111").unwrap();
        let to = EntityId::parse("card_bbbb2222").unwrap();
        let edge = user_draw_edge(from, to).unwrap();
        match edge {
            Edge::QuestionLink { from, to } => {
                assert_eq!(from.as_str(), "q_aaaa1111");
                assert!(matches!(to, EntityId::Card(ref c) if c.as_str() == "card_bbbb2222"));
            }
            other => panic!("期望 Edge::QuestionLink，实际: {other:?}"),
        }
    }

    #[test]
    fn user_draw_edge_section_source_returns_not_allowed() {
        let from = EntityId::parse("sec_aaaa1111").unwrap();
        let to = EntityId::parse("card_bbbb2222").unwrap();
        let err = user_draw_edge(from, to).unwrap_err();
        assert!(matches!(
            err,
            KeysightError::ConnectionNotAllowed { from_kind: "section" }
        ));
    }

    #[test]
    fn user_draw_edge_task_source_returns_not_allowed() {
        let from = EntityId::parse("task_aaaa1111").unwrap();
        let to = EntityId::parse("card_bbbb2222").unwrap();
        let err = user_draw_edge(from, to).unwrap_err();
        assert!(matches!(
            err,
            KeysightError::ConnectionNotAllowed { from_kind: "task" }
        ));
    }
}
