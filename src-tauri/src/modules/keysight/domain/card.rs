#![allow(dead_code)]

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::modules::keysight::errors::KeysightError;
use crate::modules::keysight::models::AtomicCard;
use crate::modules::keysight::models::CardLinksResponse;
use crate::modules::keysight::parser;
use crate::modules::keysight::vault_fs::VaultFs;

/// 卡片存储契约。
pub(in crate::modules::keysight) trait CardStore {
    /// 按 ID 查询单张卡片（含 tags、edges、card_fields）。
    fn get(&self, id: &str) -> Result<AtomicCard, KeysightError>;
    /// 查询所有卡片，按 mtime 降序，支持分页。
    fn query_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 按文件路径查询。
    fn query_by_file(&self, file_path: &str) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 按 ID 列表批量查询。
    fn query_by_ids(&self, ids: &[String]) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 卡片总数。
    fn count(&self) -> Result<i64, KeysightError>;
    /// 编辑卡片标题（同时写回文件）。
    fn edit_title(&self, id: &str, new_title: &str) -> Result<(), KeysightError>;
    /// 编辑卡片正文（同时写回文件）。
    fn edit_body(&self, id: &str, new_body: &str) -> Result<(), KeysightError>;
    /// 更新理解笔记（同时写回文件 frontmatter）。
    fn update_understanding(&self, id: &str, text: &str) -> Result<(), KeysightError>;
    /// 设置或清空卡片背景色(同时写回 frontmatter)。
    ///
    /// `color == "default"` 时清空,其他值直接写入。用 serde_yaml 序列化避免
    /// hex `#ffadad` 被 YAML 当行内注释。
    fn set_color(&self, id: &str, color: &str) -> Result<(), KeysightError>;
    /// 全文搜索卡片。
    fn search(&self, text: &str) -> Result<Vec<AtomicCard>, KeysightError>;
    /// 查询单卡片完整链接图谱。
    fn query_links(&self, id: &str) -> Result<CardLinksResponse, KeysightError>;
}

pub(in crate::modules::keysight) struct SqliteCardStore<'a> {
    conn: &'a Connection,
    vault_fs: Option<&'a dyn VaultFs>,
}

impl<'a> SqliteCardStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn, vault_fs: None }
    }

    pub fn with_vault_fs(conn: &'a Connection, vault_fs: &'a dyn VaultFs) -> Self {
        Self { conn, vault_fs: Some(vault_fs) }
    }

    pub fn sync_edges_to_file(&self, id: &str) -> Result<(), KeysightError> {
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("sync_edges_to_file 需要 VaultFs".to_string())
        })?;
        let card = self.get(id)?;
        let content = vault_fs.read_file(&card.file_path)?;
        let updated = parser::write_frontmatter(&content, parser::FrontmatterUpdate {
            link_to: Some(card.link_to.clone()),
            related: Some(card.related.clone()),
            see_also: Some(card.see_also.clone()),
            ..Default::default()
        });
        vault_fs.write_file(&card.file_path, &updated)?;
        crate::modules::keysight::domain::sync::sync_file(
            self.conn,
            &card.file_path,
            &updated,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
                * 1000.0,
        )?;
        Ok(())
    }
}

/// 从一组 card id 批量加载 tags，返回 id → Vec<tag> 映射。
fn batch_load_tags(conn: &Connection, ids: &[String]) -> Result<HashMap<String, Vec<String>>, KeysightError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT entity_id, tag FROM entity_tags WHERE entity_id IN ({}) ORDER BY entity_id, tag",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        let entity_id: String = row.get(0)?;
        let tag: String = row.get(1)?;
        Ok((entity_id, tag))
    })?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for r in rows {
        let (entity_id, tag) = r?;
        map.entry(entity_id).or_default().push(tag);
    }
    Ok(map)
}

/// 从一组 card id 批量加载出边（link_to/related/see_also），返回 id → (link_to, related, see_also)。
#[allow(clippy::type_complexity)]
fn batch_load_edges(conn: &Connection, ids: &[String]) -> Result<HashMap<String, (Vec<String>, Vec<String>, Vec<String>)>, KeysightError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT from_id, to_id, edge_type FROM edges WHERE from_id IN ({}) AND edge_type IN ('link_to', 'related', 'see_also') ORDER BY from_id, rowid",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        let from_id: String = row.get(0)?;
        let to_id: String = row.get(1)?;
        let edge_type: String = row.get(2)?;
        Ok((from_id, to_id, edge_type))
    })?;

    let mut map: HashMap<String, (Vec<String>, Vec<String>, Vec<String>)> = HashMap::new();
    for r in rows {
        let (from_id, to_id, edge_type) = r?;
        let entry = map.entry(from_id).or_default();
        match edge_type.as_str() {
            "link_to" => entry.0.push(to_id),
            "related" => entry.1.push(to_id),
            "see_also" => entry.2.push(to_id),
            _ => {}
        }
    }
    Ok(map)
}

/// 查询卡片基础行（entities + card_fields + file_mtimes），不含 tags/edges。
struct CardRow {
    id: String,
    title: String,
    content: String,
    file_path: String,
    understanding: String,
    source: String,
    mtime: Option<f64>,
    color: Option<String>,
}

/// 用新的 title 重写卡片 markdown 的 H1 行。
fn rewrite_card_title_markdown(content: &str, new_title: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut found = false;
    for line in &mut lines {
        if line.trim().starts_with("# ") {
            *line = format!("# 【ATC】{new_title}");
            found = true;
            break;
        }
    }
    if !found {
        lines.push(format!("# 【ATC】{new_title}"));
    }
    lines.join("\n") + "\n"
}

/// 基础查询：entities + card_fields + file_mtimes。
fn query_card_rows(conn: &Connection, where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Vec<CardRow>, KeysightError> {
    let sql = format!(
        "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
         COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
         f.mtime, e.color \
         FROM entities e \
         LEFT JOIN card_fields c ON e.id = c.entity_id \
         LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
         WHERE e.kind = 'card' {where_clause} \
         ORDER BY f.mtime DESC",
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params, |row| {
        Ok(CardRow {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            file_path: row.get(3)?,
            understanding: row.get(4)?,
            source: row.get(5)?,
            mtime: row.get(6)?,
            color: row.get(7)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(KeysightError::from)
}

/// 将 CardRow + tags + edges 组装为 AtomicCard。
fn assemble_cards(conn: &Connection, card_rows: Vec<CardRow>) -> Result<Vec<AtomicCard>, KeysightError> {
    let ids: Vec<String> = card_rows.iter().map(|r| r.id.clone()).collect();
    let tags_map = batch_load_tags(conn, &ids)?;
    let edges_map = batch_load_edges(conn, &ids)?;

    let cards = card_rows
        .into_iter()
        .map(|row| {
            let tags = tags_map.get(&row.id).cloned().unwrap_or_default();
            let (link_to, related, see_also) = edges_map
                .get(&row.id)
                .cloned()
                .unwrap_or_default();
            AtomicCard {
                id: row.id,
                file_path: row.file_path,
                title: row.title,
                content: row.content,
                tags,
                link_to,
                related,
                understanding: row.understanding,
                source: row.source,
                see_also,
                mtime: row.mtime,
                color: row.color,
            }
        })
        .collect();
    Ok(cards)
}

fn sort_cards_by_id_order(cards: Vec<AtomicCard>, ids: &[String]) -> Vec<AtomicCard> {
    let rank_by_id: HashMap<&str, usize> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();

    let mut cards = cards;
    cards.sort_by_key(|card| rank_by_id.get(card.id.as_str()).copied().unwrap_or(usize::MAX));
    cards
}

fn search_rank(card: &AtomicCard, query: &str, words: &[&str]) -> usize {
    let title = card.title.to_lowercase();
    let content = card.content.to_lowercase();

    if title == query {
        return 0;
    }
    if title.contains(query) {
        return 1;
    }
    if !words.is_empty() && words.iter().all(|word| title.contains(word)) {
        return 2;
    }
    if content.contains(query) {
        return 3;
    }
    4
}

fn rank_search_results(mut cards: Vec<AtomicCard>, text: &str) -> Vec<AtomicCard> {
    let query = text.trim().to_lowercase();
    let words: Vec<&str> = query.split_whitespace().filter(|word| !word.is_empty()).collect();

    cards.sort_by(|a, b| {
        let rank_a = search_rank(a, &query, &words);
        let rank_b = search_rank(b, &query, &words);

        rank_a
            .cmp(&rank_b)
            .then_with(|| b.mtime.partial_cmp(&a.mtime).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.title.cmp(&b.title))
    });

    cards
}

/// # 清理卡片标题历史转义
///
/// ## 前置条件
/// - `entities.kind='card'` 的记录必须带有可读取的 `file_path`
/// - `vault_fs` 指向真实 vault 或测试替身
///
/// ## 执行效果
/// 1. 逐张读取 card markdown 文件
/// 2. 只针对 H1 title 清理历史遗留的 `\<`、`\>` 等安全标点转义
/// 3. 如有变化，同步写回 markdown 文件、`entities.title` 和 `entities_fts.title`
///
/// ## 不做的事
/// - 不修改 body/content
/// - 不修改 note / section / task / question 等其他实体
/// - 不处理 `\*`、`\\` 等可能有字面含义的序列
///
/// ## 幂等性
/// 干净数据重复调用不会产生额外写入。
///
/// ## 关联操作
/// - [`CardStore::edit_title`] — 用户主动编辑单张卡片标题
pub(in crate::modules::keysight) fn cleanup_dirty_card_title_escapes(
    conn: &Connection,
    vault_fs: &dyn VaultFs,
) -> Result<usize, KeysightError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, file_path FROM entities WHERE kind = 'card' AND COALESCE(file_path, '') <> '' ORDER BY id",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);

    let mut cleaned = 0usize;

    for (id, db_title, file_path) in rows {
        let content = vault_fs.read_file(&file_path)?;
        let parsed = match parser::parse_entity(&content) {
            Some(parsed) if parsed.entity_type == "atomic-card" => parsed,
            _ => continue,
        };
        let clean_title = parsed.title;
        let updated_content = rewrite_card_title_markdown(&content, &clean_title);
        let file_changed = updated_content != content;
        let db_changed = db_title != clean_title;

        if !file_changed && !db_changed {
            continue;
        }

        if file_changed {
            vault_fs.write_file(&file_path, &updated_content)?;
        }

        conn.execute(
            "UPDATE entities SET title = ?1 WHERE id = ?2",
            params![clean_title, id],
        )?;
        conn.execute(
            "UPDATE entities_fts SET title = ?1 WHERE id = ?2",
            params![clean_title, id],
        )?;
        cleaned += 1;
    }

    Ok(cleaned)
}

impl CardStore for SqliteCardStore<'_> {
    fn get(&self, id: &str) -> Result<AtomicCard, KeysightError> {
        let rows = query_card_rows(self.conn, "AND e.id = ?1", &[&id])?;
        match rows.into_iter().next() {
            Some(row) => {
                let mut cards = assemble_cards(self.conn, vec![row])?;
                Ok(cards.remove(0))
            }
            None => Err(KeysightError::NotFound(id.to_string())),
        }
    }

    fn query_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<AtomicCard>, KeysightError> {
        let mut where_clause = String::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(l) = limit {
            where_clause.push_str(" LIMIT ?1");
            param_values.push(Box::new(l));
            if let Some(o) = offset {
                where_clause.push_str(" OFFSET ?2");
                param_values.push(Box::new(o));
            }
        }

        // query_card_rows 内部已有 ORDER BY，LIMIT/OFFSET 追加到末尾
        // 但 query_card_rows 的 where_clause 插在 WHERE 和 ORDER BY 之间
        // 所以这里不能用 where_clause 传 LIMIT — 需要直接构造 SQL
        let sql = if let Some(l) = limit {
            if let Some(o) = offset {
                format!(
                    "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
                     COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
                     f.mtime, e.color \
                     FROM entities e \
                     LEFT JOIN card_fields c ON e.id = c.entity_id \
                     LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
                     WHERE e.kind = 'card' \
                     ORDER BY f.mtime DESC \
                     LIMIT {} OFFSET {}", l, o
                )
            } else {
                format!(
                    "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
                     COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
                     f.mtime, e.color \
                     FROM entities e \
                     LEFT JOIN card_fields c ON e.id = c.entity_id \
                     LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
                     WHERE e.kind = 'card' \
                     ORDER BY f.mtime DESC \
                     LIMIT {}", l
                )
            }
        } else {
            "SELECT e.id, e.title, COALESCE(e.content, '') AS content, COALESCE(e.file_path, '') AS file_path, \
             COALESCE(c.understanding, '') AS understanding, COALESCE(c.source, '') AS source, \
             f.mtime, e.color \
             FROM entities e \
             LEFT JOIN card_fields c ON e.id = c.entity_id \
             LEFT JOIN file_mtimes f ON e.file_path = f.filePath \
             WHERE e.kind = 'card' \
             ORDER BY f.mtime DESC".to_string()
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(CardRow {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                file_path: row.get(3)?,
                understanding: row.get(4)?,
                source: row.get(5)?,
                mtime: row.get(6)?,
                color: row.get(7)?,
            })
        })?;
        let card_rows: Vec<CardRow> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        assemble_cards(self.conn, card_rows)
    }

    fn query_by_file(&self, file_path: &str) -> Result<Vec<AtomicCard>, KeysightError> {
        let rows = query_card_rows(self.conn, "AND e.file_path = ?1", &[&file_path])?;
        assemble_cards(self.conn, rows)
    }

    fn query_by_ids(&self, ids: &[String]) -> Result<Vec<AtomicCard>, KeysightError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{i}")).collect();
        let where_clause = format!("AND e.id IN ({})", placeholders.join(", "));
        let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let rows = query_card_rows(self.conn, &where_clause, params.as_slice())?;
        let cards = assemble_cards(self.conn, rows)?;
        Ok(sort_cards_by_id_order(cards, ids))
    }

    fn count(&self) -> Result<i64, KeysightError> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM entities WHERE kind = 'card'",
            [],
            |r| r.get(0),
        )?;
        Ok(count)
    }

    fn edit_title(&self, id: &str, new_title: &str) -> Result<(), KeysightError> {
        let new_title = new_title.trim();
        if new_title.is_empty() {
            return Err(KeysightError::EmptyTitle);
        }
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("edit_title 需要 VaultFs".to_string())
        })?;

        // 查 file_path
        let file_path: String = self.conn.query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        // 读文件，替换 H1
        let content = vault_fs.read_file(&file_path)?;
        let updated = rewrite_card_title_markdown(&content, new_title);
        vault_fs.write_file(&file_path, &updated)?;

        // 更新 DB
        self.conn.execute(
            "UPDATE entities SET title = ?1 WHERE id = ?2",
            params![new_title, id],
        )?;
        Ok(())
    }

    fn edit_body(&self, id: &str, new_body: &str) -> Result<(), KeysightError> {
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("edit_body 需要 VaultFs".to_string())
        })?;
        let new_body = parser::normalize_legacy_toggle_syntax(new_body);

        let file_path: String = self.conn.query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        let content = vault_fs.read_file(&file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        // 找 H1 行的位置
        let h1_idx = lines.iter().position(|l| l.trim().starts_with("# "));
        let prefix = match h1_idx {
            Some(idx) => lines[..=idx].join("\n"),
            None => content.clone(),
        };

        let updated = format!("{prefix}\n\n{new_body}");
        vault_fs.write_file(&file_path, &updated)?;

        self.conn.execute(
            "UPDATE entities SET content = ?1 WHERE id = ?2",
            params![new_body, id],
        )?;
        Ok(())
    }

    fn update_understanding(&self, id: &str, text: &str) -> Result<(), KeysightError> {
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("update_understanding 需要 VaultFs".to_string())
        })?;

        let file_path: String = self.conn.query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        let content = vault_fs.read_file(&file_path)?;
        let updated = parser::write_frontmatter(&content, parser::FrontmatterUpdate {
            understanding: Some(text.to_string()),
            ..Default::default()
        });
        vault_fs.write_file(&file_path, &updated)?;

        self.conn.execute(
            "UPDATE card_fields SET understanding = ?1 WHERE entity_id = ?2",
            params![text, id],
        )?;
        Ok(())
    }

    fn set_color(&self, id: &str, color: &str) -> Result<(), KeysightError> {
        let vault_fs = self.vault_fs.ok_or_else(|| {
            KeysightError::FileError("set_color 需要 VaultFs".to_string())
        })?;

        let file_path: String = self.conn.query_row(
            "SELECT file_path FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KeysightError::NotFound(id.to_string()),
            other => KeysightError::Database(other),
        })?;

        // 读文件,用 serde_yaml 更新/移除 color,写回
        let content = vault_fs.read_file(&file_path)?;
        let updated = parser::write_color_frontmatter(&content, color);
        vault_fs.write_file(&file_path, &updated)?;

        // 同步到 entities.color 列(sync 的 update path 按 file_path 找到 entity)
        let next_color = if color == "default" {
            None
        } else {
            Some(color.to_string())
        };
        self.conn.execute(
            "UPDATE entities SET color = ?1 WHERE id = ?2 AND kind = 'card'",
            params![next_color, id],
        )?;
        Ok(())
    }

    fn search(&self, text: &str) -> Result<Vec<AtomicCard>, KeysightError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // 构造 FTS5 MATCH 查询：按空格拆词，每个加引号，AND 连接
        let words: Vec<String> = text
            .split_whitespace()
            .map(|w| format!("\"{}\"", w.replace('"', "")))
            .collect();
        let fts_query = words.join(" AND ");

        let mut stmt = self.conn.prepare(
            "SELECT id FROM entities_fts WHERE entities_fts MATCH ?1"
        )?;
        let ids: Vec<String> = stmt
            .query_map([&fts_query], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        if !ids.is_empty() {
            let cards = self.query_by_ids(&ids)?;
            return Ok(rank_search_results(cards, text));
        }

        // FTS 无结果 — fallback 到 LIKE
        let like_pattern = format!("%{text}%");
        let rows = query_card_rows(
            self.conn,
            "AND (e.title LIKE ?1 OR e.content LIKE ?1)",
            &[&like_pattern as &dyn rusqlite::types::ToSql],
        )?;
        let cards = assemble_cards(self.conn, rows)?;
        Ok(rank_search_results(cards, text))
    }

    fn query_links(&self, id: &str) -> Result<CardLinksResponse, KeysightError> {
        // 验证卡片存在
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM entities WHERE id = ?1 AND kind = 'card'",
            [id],
            |r| r.get::<_, i64>(0),
        )? > 0;
        if !exists {
            return Err(KeysightError::NotFound(id.to_string()));
        }

        // 出边
        let mut out_stmt = self.conn.prepare(
            "SELECT to_id, edge_type FROM edges WHERE from_id = ?1 ORDER BY rowid"
        )?;
        let out_rows = out_stmt.query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;

        let mut link_to = Vec::new();
        let mut related = Vec::new();
        let mut see_also = Vec::new();
        for r in out_rows {
            let (to_id, edge_type) = r?;
            match edge_type.as_str() {
                "link_to" => link_to.push(to_id),
                "related" => related.push(to_id),
                "see_also" => see_also.push(to_id),
                _ => {}
            }
        }

        // 入边
        let mut in_stmt = self.conn.prepare(
            "SELECT from_id, edge_type FROM edges WHERE to_id = ?1 ORDER BY rowid"
        )?;
        let in_rows = in_stmt.query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;

        let mut linked_from = Vec::new();
        let mut related_from = Vec::new();
        let mut see_also_from = Vec::new();
        for r in in_rows {
            let (from_id, edge_type) = r?;
            match edge_type.as_str() {
                "link_to" => linked_from.push(from_id),
                "related" => related_from.push(from_id),
                "see_also" => see_also_from.push(from_id),
                _ => {}
            }
        }

        Ok(CardLinksResponse {
            link_to,
            related,
            see_also,
            linked_from,
            related_from,
            see_also_from,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::keysight::db::init_db;
    use crate::modules::keysight::domain::sync;
    use crate::modules::keysight::vault_fs::MockVaultFs;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    /// 插入一张标准测试卡片。
    fn seed_card(conn: &Connection) {
        let md = "\
---
type: atomic-card
id: card_test0001
tags:
  - rust
  - ownership
linkTo:
  - card_other001
related:
  - card_other002
understanding: 测试理解
source: https://example.com
see-also:
  - card_other003
---

# 【ATC】Test Card

Body content.
";
        sync::sync_file(conn, "whiteboard/test.md", md, 1000.0).unwrap();
    }

    /// 插入第二张卡片用于批量查询测试。
    fn seed_card2(conn: &Connection) {
        let md = "\
---
type: atomic-card
id: card_test0002
tags:
  - concurrency
---

# 【ATC】Second Card

Second body.
";
        sync::sync_file(conn, "whiteboard/test2.md", md, 2000.0).unwrap();
    }

    #[test]
    fn test_get_card_by_id() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.title, "Test Card");
        assert_eq!(card.file_path, "whiteboard/test.md");
        assert_eq!(card.content, "Body content.\n");
        assert_eq!(card.tags, vec!["ownership", "rust"]); // 字母序（DB ORDER BY tag）
        assert_eq!(card.link_to, vec!["card_other001"]);
        assert_eq!(card.related, vec!["card_other002"]);
        assert_eq!(card.see_also, vec!["card_other003"]);
        assert_eq!(card.understanding, "测试理解");
        assert_eq!(card.source, "https://example.com");
        assert!(card.mtime.is_some());
    }

    #[test]
    fn test_get_card_not_found() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        let result = store.get("card_nonexist");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_query_all() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_all(None, None).unwrap();
        assert_eq!(cards.len(), 2);
        // mtime 降序 — card2(2000) 在前，card1(1000) 在后
        assert_eq!(cards[0].id, "card_test0002");
        assert_eq!(cards[1].id, "card_test0001");
    }

    #[test]
    fn test_query_all_with_limit() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_all(Some(1), None).unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn test_query_by_file() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let cards = store.query_by_file("whiteboard/test.md").unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "card_test0001");
    }

    #[test]
    fn test_query_by_file_no_match() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        let cards = store.query_by_file("nonexist.md").unwrap();
        assert!(cards.is_empty());
    }

    #[test]
    fn test_query_by_ids() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let ids = vec!["card_test0001".to_string(), "card_test0002".to_string()];
        let cards = store.query_by_ids(&ids).unwrap();
        assert_eq!(cards.len(), 2);
    }

    #[test]
    fn test_query_by_ids_preserves_input_order() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let ids = vec!["card_test0002".to_string(), "card_test0001".to_string()];
        let cards = store.query_by_ids(&ids).unwrap();

        assert_eq!(
            cards.iter().map(|card| card.id.as_str()).collect::<Vec<_>>(),
            vec!["card_test0002", "card_test0001"]
        );
    }

    #[test]
    fn test_query_by_ids_partial() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let ids = vec!["card_test0001".to_string(), "card_nonexist".to_string()];
        let cards = store.query_by_ids(&ids).unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn test_count() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn test_count_empty() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        assert_eq!(store.count().unwrap(), 0);
    }

    const CARD_FILE_CONTENT: &str = "\
---
type: atomic-card
id: card_test0001
tags:
  - rust
  - ownership
linkTo:
  - card_other001
related:
  - card_other002
understanding: 旧理解
source: https://example.com
see-also:
  - card_other003
---

# 【ATC】Test Card

Body content.
";

    const DIRTY_CARD_FILE_CONTENT: &str = "\
---
type: atomic-card
id: card_dirty0001
tags:
  - rust
understanding: 旧理解
source: https://example.com
---

# 【ATC】**Arc\\<T\\>** 原子引用计数 \\[sync\\] \\(send\\) \\| \\#

Dirty body content.
";

    fn seed_card_with_file(conn: &Connection) -> MockVaultFs {
        sync::sync_file(conn, "whiteboard/test.md", CARD_FILE_CONTENT, 1000.0).unwrap();
        MockVaultFs::new().with_file("whiteboard/test.md", CARD_FILE_CONTENT)
    }

    fn seed_dirty_card_with_file(conn: &Connection) -> MockVaultFs {
        conn.execute(
            "INSERT INTO entities (id, kind, title, whiteboard_id, file_path, content) \
             VALUES (?1, 'card', ?2, 'rust', ?3, ?4)",
            params![
                "card_dirty0001",
                "**Arc\\<T\\>** 原子引用计数 \\[sync\\] \\(send\\) \\| \\#",
                "whiteboard/dirty.md",
                "Dirty body content.\n"
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO card_fields (entity_id, understanding, source) VALUES (?1, ?2, ?3)",
            params!["card_dirty0001", "旧理解", "https://example.com"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities_fts (id, title, content) VALUES (?1, ?2, ?3)",
            params![
                "card_dirty0001",
                "**Arc\\<T\\>** 原子引用计数 \\[sync\\] \\(send\\) \\| \\#",
                "Dirty body content.\n"
            ],
        )
        .unwrap();

        MockVaultFs::new().with_file("whiteboard/dirty.md", DIRTY_CARD_FILE_CONTENT)
    }

    #[test]
    fn test_edit_title() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store.edit_title("card_test0001", "New Title").unwrap();

        // DB 更新
        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.title, "New Title");

        // 文件更新 — H1 行应包含新标题
        let file = vfs.get_file("whiteboard/test.md").unwrap();
        assert!(file.contains("# 【ATC】New Title"));
        assert!(!file.contains("# 【ATC】Test Card"));
    }

    #[test]
    fn test_edit_title_preserves_raw_markdown_characters() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store
            .edit_title("card_test0001", "**Arc<T>** 原子引用计数 [sync] (send) | #")
            .unwrap();

        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.title, "**Arc<T>** 原子引用计数 [sync] (send) | #");

        let file = vfs.get_file("whiteboard/test.md").unwrap();
        assert!(file.contains("# 【ATC】**Arc<T>** 原子引用计数 [sync] (send) | #"));
        assert!(!file.contains("\\<"));
        assert!(!file.contains("\\>"));
        assert!(!file.contains("\\["));
        assert!(!file.contains("\\]"));
        assert!(!file.contains("\\("));
        assert!(!file.contains("\\)"));
        assert!(!file.contains("\\|"));
        assert!(!file.contains("\\#"));
    }

    #[test]
    fn test_edit_title_empty_rejected() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        let result = store.edit_title("card_test0001", "  ");
        assert!(matches!(result, Err(KeysightError::EmptyTitle)));
    }

    #[test]
    fn test_edit_title_not_found() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        let result = store.edit_title("card_nonexist", "X");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_set_color_writes_frontmatter_and_db() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store.set_color("card_test0001", "#ffadad").unwrap();

        // DB 更新
        let color: Option<String> = conn
            .query_row(
                "SELECT color FROM entities WHERE id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(color, Some("#ffadad".to_string()));

        // 文件 frontmatter 更新 —— serde_yaml 会处理 hex 引号
        let file = vfs.get_file("whiteboard/test.md").unwrap();
        assert!(file.contains("color:"));
        assert!(file.contains("ffadad"));
    }

    #[test]
    fn test_set_color_default_clears_frontmatter_and_db() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        // 先设颜色
        store.set_color("card_test0001", "#ffadad").unwrap();
        // 再用 "default" 清空
        store.set_color("card_test0001", "default").unwrap();

        let color: Option<String> = conn
            .query_row(
                "SELECT color FROM entities WHERE id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(color, None);

        let file = vfs.get_file("whiteboard/test.md").unwrap();
        assert!(!file.contains("color:"));
    }

    #[test]
    fn test_set_color_not_found() {
        let conn = test_conn();
        let vfs = MockVaultFs::new();
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        let result = store.set_color("card_nonexist", "#ffadad");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_cleanup_dirty_card_title_escapes_updates_file_and_db() {
        let conn = test_conn();
        let vfs = seed_dirty_card_with_file(&conn);

        let cleaned = cleanup_dirty_card_title_escapes(&conn, &vfs).unwrap();
        assert_eq!(cleaned, 1);

        let title: String = conn
            .query_row(
                "SELECT title FROM entities WHERE id = 'card_dirty0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(title, "**Arc<T>** 原子引用计数 [sync] (send) | #");

        let fts_title: String = conn
            .query_row(
                "SELECT title FROM entities_fts WHERE id = 'card_dirty0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(fts_title, "**Arc<T>** 原子引用计数 [sync] (send) | #");

        let file = vfs.get_file("whiteboard/dirty.md").unwrap();
        assert!(file.contains("# 【ATC】**Arc<T>** 原子引用计数 [sync] (send) | #"));
        assert!(!file.contains("\\<"));
        assert!(!file.contains("\\>"));
        assert!(!file.contains("\\["));
        assert!(!file.contains("\\]"));
        assert!(!file.contains("\\("));
        assert!(!file.contains("\\)"));
        assert!(!file.contains("\\|"));
        assert!(!file.contains("\\#"));
        assert!(file.contains("Dirty body content."));
    }

    #[test]
    fn test_edit_body() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store.edit_body("card_test0001", "Brand new body.\n").unwrap();

        // DB 更新
        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.content, "Brand new body.\n");

        // 文件更新 — frontmatter 和 H1 保留，body 替换
        let file = vfs.get_file("whiteboard/test.md").unwrap();
        assert!(file.contains("Brand new body."));
        assert!(file.contains("# 【ATC】Test Card")); // H1 保留
        assert!(file.contains("type: atomic-card")); // frontmatter 保留
        assert!(!file.contains("Body content.")); // 旧 body 消失
    }

    #[test]
    fn test_edit_body_normalizes_legacy_details_summary_to_toggle_syntax() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store
            .edit_body(
                "card_test0001",
                "<details>\n<summary>折叠标题</summary>\n\n这里是详细内容\n</details>",
            )
            .unwrap();

        let card = store.get("card_test0001").unwrap();
        assert_eq!(card.content, "?>> 折叠标题\n这里是详细内容\n?<<");

        let file = vfs.get_file("whiteboard/test.md").unwrap();
        assert!(file.contains("?>> 折叠标题"));
        assert!(file.contains("?<<"));
        assert!(!file.contains("<details>"));
        assert!(!file.contains("<summary>"));
    }

    #[test]
    fn test_update_understanding() {
        let conn = test_conn();
        let vfs = seed_card_with_file(&conn);
        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);

        store.update_understanding("card_test0001", "新的理解").unwrap();

        // DB 更新
        let understanding: String = conn
            .query_row(
                "SELECT understanding FROM card_fields WHERE entity_id = 'card_test0001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(understanding, "新的理解");

        // 文件更新 — frontmatter 中 understanding 已变
        let file = vfs.get_file("whiteboard/test.md").unwrap();
        assert!(file.contains("新的理解"));
    }

    // --- search ---

    #[test]
    fn test_search_by_title() {
        let conn = test_conn();
        seed_card(&conn);
        seed_card2(&conn);
        let store = SqliteCardStore::new(&conn);

        let results = store.search("Test Card").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "card_test0001");
    }

    #[test]
    fn test_search_by_content() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let results = store.search("Body content").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_prioritizes_title_matches_over_content_matches() {
        let conn = test_conn();
        let md_title = "\
---
type: atomic-card
id: card_titlematch
---

# 【ATC】所有权速记

普通内容。
";
        let md_content = "\
---
type: atomic-card
id: card_contentmatch
---

# 【ATC】普通标题

这里讲所有权细节。
";
        sync::sync_file(&conn, "whiteboard/title.md", md_title, 1000.0).unwrap();
        sync::sync_file(&conn, "whiteboard/content.md", md_content, 2000.0).unwrap();
        let store = SqliteCardStore::new(&conn);

        let results = store.search("所有权").unwrap();

        assert_eq!(
            results.iter().map(|card| card.id.as_str()).collect::<Vec<_>>(),
            vec!["card_titlematch", "card_contentmatch"]
        );
    }

    #[test]
    fn test_search_no_results() {
        let conn = test_conn();
        seed_card(&conn);
        let store = SqliteCardStore::new(&conn);

        let results = store.search("nonexistent_xyzzy").unwrap();
        assert!(results.is_empty());
    }

    // --- query_links ---

    #[test]
    fn test_query_links() {
        let conn = test_conn();
        seed_card(&conn); // card_test0001 有 link_to/related/see_also 出边

        // 再插一张卡片，link_to card_test0001（产生入边）
        let md = "---\ntype: atomic-card\nid: card_linker1\nlinkTo:\n  - card_test0001\n---\n\n# 【ATC】Linker\n\nBody.\n";
        sync::sync_file(&conn, "whiteboard/linker.md", md, 2000.0).unwrap();

        let store = SqliteCardStore::new(&conn);
        let links = store.query_links("card_test0001").unwrap();

        // 出边
        assert_eq!(links.link_to, vec!["card_other001"]);
        assert_eq!(links.related, vec!["card_other002"]);
        assert_eq!(links.see_also, vec!["card_other003"]);

        // 入边
        assert_eq!(links.linked_from, vec!["card_linker1"]);
    }

    #[test]
    fn test_query_links_not_found() {
        let conn = test_conn();
        let store = SqliteCardStore::new(&conn);
        let result = store.query_links("card_nonexist");
        assert!(matches!(result, Err(KeysightError::NotFound(_))));
    }

    #[test]
    fn test_query_links_no_edges() {
        let conn = test_conn();
        // 无 edges 的卡片
        let md = "---\ntype: atomic-card\nid: card_lonely1\n---\n\n# 【ATC】Lonely\n\nNo links.\n";
        sync::sync_file(&conn, "whiteboard/lonely.md", md, 1000.0).unwrap();

        let store = SqliteCardStore::new(&conn);
        let links = store.query_links("card_lonely1").unwrap();
        assert!(links.link_to.is_empty());
        assert!(links.linked_from.is_empty());
    }

    #[test]
    fn test_sync_edges_to_file_rewrites_frontmatter_from_db_edges() {
        let conn = test_conn();
        seed_card(&conn);
        let vfs = seed_card_with_file(&conn);
        conn.execute(
            "INSERT INTO edges (from_id, to_id, edge_type) VALUES ('card_test0001', 'card_new_target', 'related')",
            [],
        )
        .unwrap();

        let store = SqliteCardStore::with_vault_fs(&conn, &vfs);
        store.sync_edges_to_file("card_test0001").unwrap();

        let file = vfs.get_file("whiteboard/test.md").unwrap();
        let parsed = parser::parse_entity(&file).unwrap();
        assert_eq!(parsed.related, vec!["card_other002", "card_new_target"]);
    }
}
