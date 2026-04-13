#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;

/// 已知的实体类型。
const KNOWN_TYPES: &[&str] = &["atomic-card", "note", "project-task", "question"];

/// 文件标记，从 H1 标题中去掉。
const FILE_MARKERS: &[&str] = &["【ATC】", "【NOTE】", "【TASK】", "【QUE】"];

/// 原始 frontmatter 反序列化中间结构。
#[derive(Debug, Deserialize)]
struct RawEntityFrontmatter {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    understanding: Option<String>,
    #[serde(default, rename = "linkTo")]
    link_to: Vec<String>,
    #[serde(default)]
    related: Vec<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default, rename = "see-also")]
    see_also: Vec<String>,
    // task 字段
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    area: Option<String>,
    #[serde(default)]
    project: Option<String>,
}

/// parse_entity 的输出。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ParsedEntity {
    pub entity_type: String,
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub link_to: Vec<String>,
    pub related: Vec<String>,
    pub see_also: Vec<String>,
    pub understanding: String,
    pub source: String,
    pub task_status: Option<String>,
    pub task_area: Option<String>,
    pub task_project: Option<String>,
    pub question_status: Option<String>,
}

/// frontmatter 更新请求。
#[derive(Debug, Default)]
pub(super) struct FrontmatterUpdate {
    pub id: Option<String>,
    pub link_to: Option<Vec<String>>,
    pub related: Option<Vec<String>>,
    pub understanding: Option<String>,
    pub see_also: Option<Vec<String>>,
}

/// 提取 --- 之间的 YAML frontmatter 文本。
pub(super) fn extract_frontmatter(markdown: &str) -> Option<String> {
    let trimmed = markdown.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let close = after_first.find("\n---")?;
    Some(after_first[..close].to_string())
}

/// 返回 frontmatter 之后的文本。
pub(super) fn skip_frontmatter(markdown: &str) -> &str {
    let trimmed = markdown.trim_start();
    if !trimmed.starts_with("---") {
        return markdown;
    }
    let after_first = &trimmed[3..];
    match after_first.find("\n---") {
        Some(pos) => &after_first[pos + 4..], // 跳过 "\n---"
        None => markdown,
    }
}

/// 从 frontmatter 之后的文本中提取 H1 标题和正文。
fn extract_title_and_body(text: &str) -> (String, String) {
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(after_hash) = trimmed.strip_prefix("# ") {
            let mut title = after_hash.to_string();
            // 移除文件标记
            for marker in FILE_MARKERS {
                title = title.replace(marker, "");
            }
            let title = title.trim().to_string();
            // 正文 = H1 之后的所有行
            let body_lines: Vec<&str> = text.lines().skip(i + 1).collect();
            let body = body_lines.join("\n");
            let body = body.trim_start_matches('\n').to_string();
            let body = if body.is_empty() {
                body
            } else {
                format!("{body}\n")
            };
            return (title, body);
        }
    }
    // 没有 H1 — 用第一个非空行作标题
    let first_non_empty = text.lines().find(|l| !l.trim().is_empty());
    match first_non_empty {
        Some(line) => (line.trim().to_string(), String::new()),
        None => (String::new(), String::new()),
    }
}

/// 清理 title 中历史遗留的 markdown 转义。
///
/// 规则：
/// - 仅移除 `\` + 单个 ASCII 标点的转义
/// - `*` 和 `\` 不处理，避免破坏 `**bold**` 定界符、`\\`、`\\*` 等字面内容
pub(super) fn normalize_title_markdown_escapes(title: &str) -> String {
    let mut normalized = String::with_capacity(title.len());
    let mut chars = title.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            normalized.push(ch);
            continue;
        }

        match chars.peek().copied() {
            Some('\\') => {
                normalized.push('\\');
                normalized.push('\\');
                chars.next();
            }
            Some(next)
                if next.is_ascii_punctuation() && next != '*' && next != '\\' =>
            {
                normalized.push(next);
                chars.next();
            }
            _ => normalized.push('\\'),
        }
    }

    normalized
}

/// 把历史 `<details><summary>...</summary>...</details>` 规范化成 `?>> / ?<<`。
pub(super) fn normalize_legacy_toggle_syntax(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut result = Vec::with_capacity(lines.len());
    let had_trailing_newline = markdown.ends_with('\n');
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if !trimmed.starts_with("<details") {
            result.push(line.to_string());
            i += 1;
            continue;
        }

        let Some(summary_line) = lines.get(i + 1) else {
            result.push(line.to_string());
            i += 1;
            continue;
        };
        let summary_trimmed = summary_line.trim();
        let Some(summary_inner) = summary_trimmed
            .strip_prefix("<summary>")
            .and_then(|rest| rest.strip_suffix("</summary>"))
        else {
            result.push(line.to_string());
            i += 1;
            continue;
        };

        let mut cursor = i + 2;
        while cursor < lines.len() && lines[cursor].trim().is_empty() {
            cursor += 1;
        }

        let mut content_lines = Vec::new();
        while cursor < lines.len() && lines[cursor].trim() != "</details>" {
            content_lines.push(lines[cursor]);
            cursor += 1;
        }

        if cursor >= lines.len() {
            result.push(line.to_string());
            i += 1;
            continue;
        }

        while matches!(content_lines.last(), Some(line) if line.trim().is_empty()) {
            content_lines.pop();
        }

        result.push(format!("?>> {}", summary_inner.trim()));
        result.extend(content_lines.into_iter().map(str::to_string));
        result.push("?<<".to_string());
        i = cursor + 1;
    }

    let mut normalized = result.join("\n");
    if had_trailing_newline && !normalized.is_empty() {
        normalized.push('\n');
    }
    normalized
}

/// 解析 markdown 文件为多类型实体。
pub(super) fn parse_entity(markdown: &str) -> Option<ParsedEntity> {
    let fm_str = extract_frontmatter(markdown)?;
    let raw: RawEntityFrontmatter = serde_yaml::from_str(&fm_str).ok()?;

    let entity_type = raw.r#type.as_deref()?;
    if !KNOWN_TYPES.contains(&entity_type) {
        return None;
    }

    let after_fm = skip_frontmatter(markdown);
    let (raw_title, content) = extract_title_and_body(after_fm);
    let content = normalize_legacy_toggle_syntax(&content);
    let title = if entity_type == "atomic-card" {
        normalize_title_markdown_escapes(&raw_title)
    } else {
        raw_title
    };

    // id: 优先用 id 字段，fallback 到 uuid（向后兼容）
    let id = raw.id.or(raw.uuid);

    // task/question 状态字段
    let (task_status, task_area, task_project, question_status) = match entity_type {
        "project-task" => (raw.status.clone(), raw.area.clone(), raw.project.clone(), None),
        "question" => (None, None, None, raw.status.clone()),
        _ => (None, None, None, None),
    };

    Some(ParsedEntity {
        entity_type: entity_type.to_string(),
        id,
        title,
        content,
        tags: raw.tags,
        link_to: raw.link_to,
        related: raw.related,
        see_also: raw.see_also,
        understanding: raw.understanding.unwrap_or_default(),
        source: raw.source.unwrap_or_default(),
        task_status,
        task_area,
        task_project,
        question_status,
    })
}

/// 修改 frontmatter 并返回完整 markdown。
pub(super) fn write_frontmatter(markdown: &str, updates: FrontmatterUpdate) -> String {
    let fm_str = extract_frontmatter(markdown);
    let mut map: serde_yaml::Mapping = match &fm_str {
        Some(s) => serde_yaml::from_str(s).unwrap_or_default(),
        None => serde_yaml::Mapping::new(),
    };

    if let Some(id) = updates.id {
        map.remove(serde_yaml::Value::String("uuid".to_string()));
        map.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(id),
        );
    }
    if let Some(link_to) = updates.link_to {
        let val: Vec<serde_yaml::Value> =
            link_to.into_iter().map(serde_yaml::Value::String).collect();
        map.insert(
            serde_yaml::Value::String("linkTo".to_string()),
            serde_yaml::Value::Sequence(val),
        );
    }
    if let Some(related) = updates.related {
        let val: Vec<serde_yaml::Value> =
            related.into_iter().map(serde_yaml::Value::String).collect();
        map.insert(
            serde_yaml::Value::String("related".to_string()),
            serde_yaml::Value::Sequence(val),
        );
    }
    if let Some(understanding) = updates.understanding {
        map.insert(
            serde_yaml::Value::String("understanding".to_string()),
            serde_yaml::Value::String(understanding),
        );
    }
    if let Some(see_also) = updates.see_also {
        let val: Vec<serde_yaml::Value> =
            see_also.into_iter().map(serde_yaml::Value::String).collect();
        map.insert(
            serde_yaml::Value::String("see-also".to_string()),
            serde_yaml::Value::Sequence(val),
        );
    }

    let after_fm = skip_frontmatter(markdown);
    let yaml = serde_yaml::to_string(&map).unwrap_or_default();
    format!("---\n{}---\n{}", yaml, after_fm)
}

/// understanding 字段可以是 string 或 list（旧版兼容）。
fn string_or_list<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrList;
    impl<'de> de::Visitor<'de> for StringOrList {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string or list of strings")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut parts = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                parts.push(s);
            }
            Ok(Some(parts.join("\n")))
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(StringOrList)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD_MD: &str = "\
---
type: atomic-card
id: card_abc12345
tags:
  - rust
  - concurrency
linkTo:
  - card_def67890
related:
  - card_xyz11111
understanding: 深入理解所有权模型
source: https://doc.rust-lang.org
see-also:
  - card_see11111
---

# 【ATC】Rust Ownership

Body content here.
";

    const NOTE_MD: &str = "\
---
type: note
id: note_abc12345
tags:
  - study
---

# 【NOTE】Study Notes

Note body content.
";

    const TASK_MD: &str = "\
---
type: project-task
id: task_abc12345
status: active
area: backend
project: keysight
---

# 【TASK】Implement parser

Task description.
";

    const QUESTION_MD: &str = "\
---
type: question
id: q_abc12345
status: pending
---

# 【QUE】How does async work?

Question body.
";

    // --- parse_entity ---

    #[test]
    fn test_parse_card() {
        let parsed = parse_entity(CARD_MD).expect("应成功解析 card");
        assert_eq!(parsed.entity_type, "atomic-card");
        assert_eq!(parsed.id, Some("card_abc12345".to_string()));
        assert_eq!(parsed.title, "Rust Ownership");
        assert_eq!(parsed.content, "Body content here.\n");
        assert_eq!(parsed.tags, vec!["rust", "concurrency"]);
        assert_eq!(parsed.link_to, vec!["card_def67890"]);
        assert_eq!(parsed.related, vec!["card_xyz11111"]);
        assert_eq!(parsed.see_also, vec!["card_see11111"]);
        assert_eq!(parsed.understanding, "深入理解所有权模型");
        assert_eq!(parsed.source, "https://doc.rust-lang.org");
    }

    #[test]
    fn test_parse_note() {
        let parsed = parse_entity(NOTE_MD).expect("应成功解析 note");
        assert_eq!(parsed.entity_type, "note");
        assert_eq!(parsed.id, Some("note_abc12345".to_string()));
        assert_eq!(parsed.title, "Study Notes");
        assert_eq!(parsed.tags, vec!["study"]);
    }

    #[test]
    fn test_parse_task() {
        let parsed = parse_entity(TASK_MD).expect("应成功解析 task");
        assert_eq!(parsed.entity_type, "project-task");
        assert_eq!(parsed.task_status, Some("active".to_string()));
        assert_eq!(parsed.task_area, Some("backend".to_string()));
        assert_eq!(parsed.task_project, Some("keysight".to_string()));
    }

    #[test]
    fn test_parse_question() {
        let parsed = parse_entity(QUESTION_MD).expect("应成功解析 question");
        assert_eq!(parsed.entity_type, "question");
        assert_eq!(parsed.question_status, Some("pending".to_string()));
    }

    #[test]
    fn test_parse_unknown_type_returns_none() {
        let md = "---\ntype: unknown\n---\n# Title\nBody\n";
        assert!(parse_entity(md).is_none());
    }

    #[test]
    fn test_parse_no_frontmatter_returns_none() {
        let md = "# Just a title\nNo frontmatter here.\n";
        assert!(parse_entity(md).is_none());
    }

    #[test]
    fn test_parse_no_type_returns_none() {
        let md = "---\ntags:\n  - test\n---\n# Title\nBody\n";
        assert!(parse_entity(md).is_none());
    }

    #[test]
    fn test_card_without_id() {
        let md = "---\ntype: atomic-card\ntags:\n  - test\n---\n\n# 【ATC】No ID Card\n\nBody.\n";
        let parsed = parse_entity(md).expect("应成功解析");
        assert_eq!(parsed.id, None);
        assert_eq!(parsed.title, "No ID Card");
    }

    #[test]
    fn test_parse_card_title_unescapes_punctuation() {
        let md = "\
---
type: atomic-card
id: card_escape001
---

# 【ATC】**Arc\\<T\\>** 原子引用计数 \\[sync\\] \\(send\\) \\| \\#
";

        let parsed = parse_entity(md).expect("应成功解析");
        assert_eq!(
            parsed.title,
            "**Arc<T>** 原子引用计数 [sync] (send) | #"
        );
    }

    #[test]
    fn test_parse_card_title_keeps_literal_backslash_star() {
        let md = "\
---
type: atomic-card
id: card_escape002
---

# 【ATC】保留字面反斜杠星号 \\\\* 和双反斜杠 \\\\\\\\
";

        let parsed = parse_entity(md).expect("应成功解析");
        assert_eq!(parsed.title, "保留字面反斜杠星号 \\\\* 和双反斜杠 \\\\\\\\");
    }

    #[test]
    fn test_parse_card_normalizes_legacy_details_summary_to_toggle_syntax() {
        let md = "\
---
type: atomic-card
id: card_toggle001
---

# 【ATC】Toggle Card

before
<details>
<summary>折叠标题</summary>

这里是详细内容
</details>
after
";

        let parsed = parse_entity(md).expect("应成功解析");
        assert_eq!(
            parsed.content,
            "before\n?>> 折叠标题\n这里是详细内容\n?<<\nafter\n"
        );
    }

    // --- extract_frontmatter ---

    #[test]
    fn test_extract_frontmatter() {
        let fm = extract_frontmatter(CARD_MD).expect("应提取 frontmatter");
        assert!(fm.contains("type: atomic-card"));
        assert!(fm.contains("id: card_abc12345"));
    }

    #[test]
    fn test_extract_frontmatter_no_fm() {
        assert!(extract_frontmatter("# No frontmatter").is_none());
    }

    // --- skip_frontmatter ---

    #[test]
    fn test_skip_frontmatter() {
        let after = skip_frontmatter(CARD_MD);
        assert!(after.contains("# 【ATC】Rust Ownership"));
        assert!(!after.contains("type: atomic-card"));
    }

    // --- write_frontmatter ---

    #[test]
    fn test_write_frontmatter_updates_id() {
        let updated = write_frontmatter(
            CARD_MD,
            FrontmatterUpdate {
                id: Some("card_new00001".to_string()),
                ..Default::default()
            },
        );
        assert!(updated.contains("card_new00001"));
        assert!(updated.contains("Body content here."));
    }
}
