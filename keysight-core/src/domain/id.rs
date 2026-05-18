//! Id newtype 模块 —— keysight 业务 id 的强类型边界。
//!
//! ## 不变量
//!
//! 1. 6 个 entity id(card_*/note_*/alias_*/sec_*/q_*/task_*)跨 IPC / DB 边界
//!    进入业务层时必须 parse 到对应 newtype。业务代码不接受 &str。
//! 2. WhiteboardId 是 owner 命名空间标识(非 entity);业务里实际值有 3 种形态
//!    (`wb_*` 兜底根白板 / `projects/{name}` Kanban 项目白板 / 扁平命名白板),
//!    因此 parse 规则**不强制单一 prefix**,只**拒绝以 6 种 entity prefix 起头**
//!    的字符串(防止 wb_id 与 entity_id 参数顺序混用),以及空字符串。
//! 3. 内部字段 String 私有,只能通过 as_str() 取 &str 给 SQL 参数 / 日志使用。
//! 4. EntityId 是 6 个 entity newtype 的 union;WhiteboardId 不属于 EntityId
//!    (wb 不是 entity,wb owns entities)。
//!
//! ## 反序列化校验
//!
//! 每个 newtype 用 `#[serde(try_from = "String", into = "String")]` 强制
//! deserialize 走 `parse()` 校验。**禁止** serde transparent 属性 —— 它会
//! 绕过 prefix 校验,让 illegal id 穿透 IPC 边界进入业务层。
//! (具体 attr 名称在 source audit test 的拼接逻辑里,不在 doc comment 字面给出,
//! 避免 include_str! 自污染。)
//!
//! ## prefixed_id! macro
//!
//! 6 个 entity newtype 完全同构(只 prefix 字面量不同),抽 macro 把
//! "newtype + parse + new_unchecked + as_str + TryFrom<String> + From for String"
//! 这套完整契约一次定义。新增 entity newtype 时只加一行 invocation,
//! 不变量(try_from / pub(crate) new_unchecked / specta type)由 macro 强制。
//! WhiteboardId 因 parse 规则不同(多形态接受 + entity prefix 黑名单)单独手写。

use serde::{Deserialize, Serialize};

// ====================================================================
// IdError —— parse 边界错误
// ====================================================================

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum IdError {
    /// 传入字符串不匹配任何已知 id prefix(含空字符串)
    #[error("未知的 id prefix: {0:?}")]
    UnknownPrefix(String),
}

// ====================================================================
// prefixed_id! macro —— 一次定义,产 newtype 完整契约
// ====================================================================
//
// 展开后等同于:
//   pub struct $name(String);
//   impl $name {
//       const PREFIX: &'static str = $prefix;
//       pub fn parse(s: impl Into<String>) -> Result<Self, IdError> { ... }
//       pub(crate) fn new_unchecked(raw: String) -> Self { Self(raw) }
//       pub fn as_str(&self) -> &str { &self.0 }
//   }
//   impl TryFrom<String> for $name { ... 调 parse ... }
//   impl From<$name> for String { ... }
//
// 改 macro 定义 = 改 7 个 newtype 行为,touchpoint 极小。
//
// 警告:本 macro 强制 try_from / into 路径,**禁止**在 macro 内或 invocation 旁
// 加 serde transparent 属性 —— 那会绕过 parse 校验。

macro_rules! prefixed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
        #[serde(try_from = "String", into = "String")]
        #[specta(type = String)]
        pub struct $name(String);

        impl $name {
            const PREFIX: &'static str = $prefix;

            /// 跨 IPC / DB 边界进入强类型的入口。校验 prefix,失败返 `IdError::UnknownPrefix(原 string)`。
            pub fn parse(s: impl Into<String>) -> Result<Self, IdError> {
                let s = s.into();
                if s.starts_with(Self::PREFIX) {
                    Ok(Self(s))
                } else {
                    Err(IdError::UnknownPrefix(s))
                }
            }

            /// 无校验构造 —— 仅 crate 内 DB read path 使用,信任 schema invariant。
            ///
            /// **W2 决定**:守住 `pub(crate)` 不开放 —— 跨 crate(src-tauri command 层)
            /// 即使在 `if from_id.starts_with("card_")` 之后,也强制走 `parse(...)`
            /// 重新校验(O(starts_with) 开销可忽略,换取 API 表面不外泄 unchecked 构造)。
            #[allow(dead_code)] // W0 阶段尚未渗透业务层,W1+ 渗透后会有真实调用
            pub(crate) fn new_unchecked(raw: String) -> Self {
                Self(raw)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;
            fn try_from(s: String) -> Result<Self, IdError> {
                Self::parse(s)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> String {
                id.0
            }
        }

        // SQL bind 支持。让 `params![card_id]` / `Vec<&dyn ToSql>` 直接接 newtype,
        // 不需要在每个调用点写 `.as_str()`(W2 batch_load 等 `iter().map(|id| ...)`
        // collect 到 `Vec<&dyn ToSql>` 的场景必须靠 ToSql impl)。
        impl rusqlite::types::ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                Ok(rusqlite::types::ToSqlOutput::Borrowed(
                    rusqlite::types::ValueRef::Text(self.0.as_bytes()),
                ))
            }
        }
    };
}

// ====================================================================
// 6 个 entity newtype(prefix 强校验,由 macro 一次定义)
// ====================================================================

prefixed_id!(CardId,       "card_");
prefixed_id!(NoteId,       "note_");
prefixed_id!(AliasId,      "alias_");
prefixed_id!(SectionId,    "sec_");
prefixed_id!(QuestionId,   "q_");
prefixed_id!(TaskId,       "task_");

// ====================================================================
// WhiteboardId —— owner 命名空间标识,parse 规则与 entity 不同
// ====================================================================
//
// 业务里 wb_id 实际值由 `sync::derive_whiteboard_id` 或 `ProjectName::whiteboard_id`
// 派生,有 3 种形态:
//   - `wb_root`               兜底根白板(`wb_` 前缀,历史 / 默认)
//   - `projects/{name}`       Kanban project 白板(reserved 二级路径)
//   - `{flat_name}`           普通扁平白板(`whiteboard/{sub}/...` → `sub`)
//
// 因此 WhiteboardId::parse **不强制单一 prefix**,只拒绝两类输入:
//   (a) 空字符串
//   (b) 以 6 种 entity prefix(card_/note_/alias_/sec_/q_/task_)起头的字符串
//       —— 防止 wb_id 与 entity_id 参数顺序混用
//
// IPC 边界仍通过 `#[serde(try_from = "String", into = "String")]` 强制 parse,
// 业务代码不接受 &str。

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(try_from = "String", into = "String")]
#[specta(type = String)]
pub struct WhiteboardId(String);

impl WhiteboardId {
    /// 跨 IPC / DB 边界进入强类型的入口。
    ///
    /// 接受任意非空字符串,但拒绝以 6 种 entity prefix(card_/note_/alias_/sec_/q_/task_)
    /// 起头的字符串。失败时返回 `IdError::UnknownPrefix(原 string)`。
    pub fn parse(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        if s.is_empty() {
            return Err(IdError::UnknownPrefix(s));
        }
        if [
            CardId::PREFIX,
            NoteId::PREFIX,
            AliasId::PREFIX,
            SectionId::PREFIX,
            QuestionId::PREFIX,
            TaskId::PREFIX,
        ]
        .iter()
        .any(|p| s.starts_with(p))
        {
            return Err(IdError::UnknownPrefix(s));
        }
        Ok(Self(s))
    }

    /// 无校验构造 —— 仅 crate 内 DB read path / trusted 派生函数(如
    /// `ProjectName::whiteboard_id` / `sync::derive_whiteboard_id`)使用,
    /// 信任源 invariant。
    pub(crate) fn new_unchecked(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WhiteboardId {
    type Error = IdError;
    fn try_from(s: String) -> Result<Self, IdError> {
        Self::parse(s)
    }
}

impl From<WhiteboardId> for String {
    fn from(id: WhiteboardId) -> String {
        id.0
    }
}

// ====================================================================
// EntityId —— 6 个 entity newtype 的判别联合
// WhiteboardId 不属于 EntityId(wb owns entities,不是 entity)
// ====================================================================

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
    /// 跨 IPC / DB 边界从 string 进入强类型。按 prefix 派发到对应 entity newtype。
    pub fn parse(s: &str) -> Result<Self, IdError> {
        if s.starts_with(CardId::PREFIX) {
            CardId::parse(s.to_string()).map(Self::Card)
        } else if s.starts_with(NoteId::PREFIX) {
            NoteId::parse(s.to_string()).map(Self::Note)
        } else if s.starts_with(AliasId::PREFIX) {
            AliasId::parse(s.to_string()).map(Self::Alias)
        } else if s.starts_with(SectionId::PREFIX) {
            SectionId::parse(s.to_string()).map(Self::Section)
        } else if s.starts_with(QuestionId::PREFIX) {
            QuestionId::parse(s.to_string()).map(Self::Question)
        } else if s.starts_with(TaskId::PREFIX) {
            TaskId::parse(s.to_string()).map(Self::Task)
        } else {
            Err(IdError::UnknownPrefix(s.to_string()))
        }
    }

    /// 取出底层字符串表示,供 DB 参数绑定 / 日志使用。
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
// Tests —— W0 scenarios 1-6
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Scenario 1: 模块导出 7 个 newtype + EntityId enum + IdError ----------

    #[test]
    fn id_module_exposes_all_types() {
        // 7 个 newtype 必须存在为 struct(以 PhantomData 占位,不需要实例化)
        use std::marker::PhantomData;
        let _: PhantomData<WhiteboardId> = PhantomData;
        let _: PhantomData<CardId> = PhantomData;
        let _: PhantomData<NoteId> = PhantomData;
        let _: PhantomData<AliasId> = PhantomData;
        let _: PhantomData<SectionId> = PhantomData;
        let _: PhantomData<QuestionId> = PhantomData;
        let _: PhantomData<TaskId> = PhantomData;

        // EntityId 必须是 enum 含 6 个 variant(穷尽 match)
        fn _ensure_entity_id_variants(e: EntityId) {
            match e {
                EntityId::Card(_) => (),
                EntityId::Note(_) => (),
                EntityId::Alias(_) => (),
                EntityId::Section(_) => (),
                EntityId::Question(_) => (),
                EntityId::Task(_) => (),
            }
        }

        // IdError 必须存在
        let _: PhantomData<IdError> = PhantomData;
    }

    // ---------- Scenario 2: parse 接受合法 prefix ----------

    #[test]
    fn whiteboard_id_parse_accepts_wb_prefix() {
        let id = WhiteboardId::parse("wb_aaaa1111").unwrap();
        assert_eq!(id.as_str(), "wb_aaaa1111");
    }

    #[test]
    fn whiteboard_id_parse_accepts_projects_path() {
        let id = WhiteboardId::parse("projects/super-tauri").unwrap();
        assert_eq!(id.as_str(), "projects/super-tauri");
    }

    #[test]
    fn whiteboard_id_parse_accepts_flat_name() {
        let id = WhiteboardId::parse("myboard").unwrap();
        assert_eq!(id.as_str(), "myboard");
    }

    #[test]
    fn whiteboard_id_parse_accepts_wb_root_default() {
        let id = WhiteboardId::parse("wb_root").unwrap();
        assert_eq!(id.as_str(), "wb_root");
    }

    #[test]
    fn card_id_parse_accepts_valid_prefix() {
        let id = CardId::parse("card_xxxx2222").unwrap();
        assert_eq!(id.as_str(), "card_xxxx2222");
    }

    #[test]
    fn note_id_parse_accepts_valid_prefix() {
        let id = NoteId::parse("note_yyyy3333").unwrap();
        assert_eq!(id.as_str(), "note_yyyy3333");
    }

    #[test]
    fn alias_id_parse_accepts_valid_prefix() {
        let id = AliasId::parse("alias_zzzz4444").unwrap();
        assert_eq!(id.as_str(), "alias_zzzz4444");
    }

    #[test]
    fn section_id_parse_accepts_valid_prefix() {
        let id = SectionId::parse("sec_1234abcd").unwrap();
        assert_eq!(id.as_str(), "sec_1234abcd");
    }

    #[test]
    fn question_id_parse_accepts_valid_prefix() {
        let id = QuestionId::parse("q_56785678").unwrap();
        assert_eq!(id.as_str(), "q_56785678");
    }

    #[test]
    fn task_id_parse_accepts_valid_prefix() {
        let id = TaskId::parse("task_abcd9999").unwrap();
        assert_eq!(id.as_str(), "task_abcd9999");
    }

    // ---------- Scenario 3: parse 拒绝非匹配 prefix ----------

    #[test]
    fn whiteboard_id_parse_rejects_card_prefix() {
        let err = WhiteboardId::parse("card_xxx").unwrap_err();
        assert_eq!(err, IdError::UnknownPrefix("card_xxx".to_string()));
    }

    #[test]
    fn whiteboard_id_parse_rejects_all_entity_prefixes() {
        for entity_id in [
            "card_xxx",
            "note_xxx",
            "alias_xxx",
            "sec_xxx",
            "q_xxx",
            "task_xxx",
        ] {
            let err = WhiteboardId::parse(entity_id).unwrap_err();
            assert_eq!(
                err,
                IdError::UnknownPrefix(entity_id.to_string()),
                "{entity_id} 应被 WhiteboardId 拒(entity prefix 黑名单)"
            );
        }
    }

    #[test]
    fn card_id_parse_rejects_note_prefix() {
        let err = CardId::parse("note_xxx").unwrap_err();
        assert_eq!(err, IdError::UnknownPrefix("note_xxx".to_string()));
    }

    #[test]
    fn note_id_parse_rejects_wb_prefix() {
        let err = NoteId::parse("wb_xxx").unwrap_err();
        assert_eq!(err, IdError::UnknownPrefix("wb_xxx".to_string()));
    }

    #[test]
    fn whiteboard_id_parse_rejects_empty_string() {
        let err = WhiteboardId::parse("").unwrap_err();
        assert_eq!(err, IdError::UnknownPrefix(String::new()));
    }

    #[test]
    fn card_id_parse_rejects_unknown_prefix() {
        let err = CardId::parse("xyz_garbage").unwrap_err();
        assert_eq!(err, IdError::UnknownPrefix("xyz_garbage".to_string()));
    }

    // ---------- Scenario 4: serde try_from 强校验 ----------

    #[test]
    fn ipc_deserialize_whiteboard_id_rejects_invalid_prefix() {
        let result: Result<WhiteboardId, _> = serde_json::from_str("\"card_invalid\"");
        assert!(result.is_err(), "card_invalid 不应被反序列化为 WhiteboardId");
    }

    #[test]
    fn ipc_deserialize_whiteboard_id_accepts_valid_prefix() {
        let id: WhiteboardId = serde_json::from_str("\"wb_aaaa1111\"").unwrap();
        assert_eq!(id.as_str(), "wb_aaaa1111");
    }

    #[test]
    fn ipc_deserialize_card_id_rejects_note_prefix() {
        let result: Result<CardId, _> = serde_json::from_str("\"note_xxx\"");
        assert!(result.is_err());
    }

    #[test]
    fn ipc_serialize_whiteboard_id_emits_plain_string() {
        let id = WhiteboardId::parse("wb_aaaa1111").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"wb_aaaa1111\"");
    }

    // ---------- Scenario 5 & 6: 源码静态扫描 ----------
    //
    // 这些 test 用 include_str! 读自身源码,**必须先截断到 `#[cfg(test)]` 之前**,
    // 否则 assertion message / 字面量参数里的目标字符串会被误判为生产代码命中。
    // 用 `production_src()` helper 封装截断逻辑,所有 source audit test 共用。

    fn production_src() -> &'static str {
        let src = include_str!("id.rs");
        src.split("#[cfg(test)]")
            .next()
            .expect("id.rs 源码必有非测试段")
    }

    #[test]
    fn no_serde_transparent_on_id_newtypes() {
        let target = "transparent";
        let attr_pattern = format!("#[serde({target})]");
        assert!(
            !production_src().contains(&attr_pattern),
            "生产段不允许 serde transparent 属性 —— 它会绕过 parse 校验",
        );
    }

    /// 验证 prefixed_id! macro 自身仍强制 try_from / into 路径,且 6 个 entity newtype
    /// 都通过 macro invocation 生成(不旁路出手写 entity newtype 偷偷用 transparent)。
    ///
    /// WhiteboardId 单独手写(parse 规则不同),不在此 audit 范围。
    #[test]
    fn macro_enforces_try_from_and_six_invocations() {
        // 1. macro 定义本身必须含 try_from / into 标记(确认未被改成 transparent)
        let try_from_attr = format!(
            "#[serde({lhs}, {rhs})]",
            lhs = "try_from = \"String\"",
            rhs = "into = \"String\"",
        );
        let macro_defines_try_from = production_src().contains(&try_from_attr);
        assert!(
            macro_defines_try_from,
            "prefixed_id! macro 定义中应包含 try_from/into 序列化路径",
        );

        // 2. invocation 数量必须 ≥ 6(每个 entity newtype 一次,prefix 各异)
        let invocation_count = production_src().matches("prefixed_id!(").count();
        assert!(
            invocation_count >= 6,
            "至少 6 个 prefixed_id! invocation,实际命中 {invocation_count} 次",
        );

        // 3. production 段不应有任何具名 entity newtype 的手写 expanded form,绕过 macro
        //    macro 定义里用 `$name` placeholder,具体 entity newtype name 只在 invocation 出现;
        //    任何 `pub struct CardId(String);` 字面量都是手写绕过,fail
        //    WhiteboardId 单独手写(parse 规则与 entity 不同),不在 audit 范围。
        let manual_patterns = [
            "pub struct CardId(String);",
            "pub struct NoteId(String);",
            "pub struct AliasId(String);",
            "pub struct SectionId(String);",
            "pub struct QuestionId(String);",
            "pub struct TaskId(String);",
        ];
        for p in manual_patterns {
            assert!(
                !production_src().contains(p),
                "production 段不应有手写 `{p}` 绕过 prefixed_id! macro",
            );
        }
    }

    #[test]
    fn new_unchecked_is_pub_crate_only() {
        let bare_pub = "pub fn new_unchecked";
        let crate_pub = "pub(crate) fn new_unchecked";
        let src = production_src();
        assert!(
            !src.contains(bare_pub),
            "生产段 new_unchecked 必须为 pub(crate) 或更窄",
        );
        assert!(
            src.contains(crate_pub),
            "生产段应有 pub(crate) fn new_unchecked 供 DB read path 使用",
        );
    }
}
