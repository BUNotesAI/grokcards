#![allow(dead_code)]

use rand::Rng;

/// 生成 8 字符小写十六进制字符串。
fn gen_hex8() -> String {
    let n: u32 = rand::rng().random();
    format!("{:08x}", n)
}

pub fn gen_card_id() -> String { format!("card_{}", gen_hex8()) }
pub fn gen_sec_id() -> String { format!("sec_{}", gen_hex8()) }
pub fn gen_note_id() -> String { format!("note_{}", gen_hex8()) }
pub fn gen_alias_id() -> String { format!("alias_{}", gen_hex8()) }
pub fn gen_task_id() -> String { format!("task_{}", gen_hex8()) }
pub fn gen_question_id() -> String { format!("q_{}", gen_hex8()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_card_id_format() {
        let id = gen_card_id();
        assert!(id.starts_with("card_"), "应以 card_ 开头，实际: {id}");
        assert_eq!(id.len(), 13, "card_ (5) + 8 hex = 13，实际: {}", id.len());
    }

    #[test]
    fn test_gen_sec_id_format() {
        let id = gen_sec_id();
        assert!(id.starts_with("sec_"));
        assert_eq!(id.len(), 12); // sec_ (4) + 8 hex
    }

    #[test]
    fn test_gen_note_id_format() {
        let id = gen_note_id();
        assert!(id.starts_with("note_"));
        assert_eq!(id.len(), 13);
    }

    #[test]
    fn test_gen_alias_id_format() {
        let id = gen_alias_id();
        assert!(id.starts_with("alias_"));
        assert_eq!(id.len(), 14); // alias_ (6) + 8 hex
    }

    #[test]
    fn test_gen_task_id_format() {
        let id = gen_task_id();
        assert!(id.starts_with("task_"));
        assert_eq!(id.len(), 13);
    }

    #[test]
    fn test_gen_question_id_format() {
        let id = gen_question_id();
        assert!(id.starts_with("q_"));
        assert_eq!(id.len(), 10); // q_ (2) + 8 hex
    }

    #[test]
    fn test_ids_are_unique() {
        let ids: Vec<String> = (0..100).map(|_| gen_card_id()).collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "100 个 ID 应全部唯一");
    }

    #[test]
    fn test_hex_chars_only() {
        let id = gen_card_id();
        let hex_part = &id[5..]; // 跳过 "card_"
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "hex 部分应全为十六进制字符，实际: {hex_part}"
        );
    }
}
