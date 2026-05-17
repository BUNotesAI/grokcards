//! CLI 命令 handlers。
//!
//! **Query(Phase 6.1)**:`&SqliteReadClient` + keysight-core domain fn,println! 格式化输出。
//! **Mutate(Phase 6.2a)**:`&dyn WriteClient` + POST /rpc,构造 JSON params 透传给 server。
//! **Deferred**:直接返 `CliError::NotImplemented`,不构造任何 client。
//!
//! CLI 薄交互层约束(task rule 9):mutate handler **不做业务校验 / id 生成 / enum parse**,
//! 所有业务规则由 server 侧 `MutateParams` dispatcher 判定(Phase 6.2b)。

use keysight_core::domain::card::{CardStore, SqliteCardStore};
use keysight_core::domain::note::{self, NoteStore, SqliteNoteStore};
use keysight_core::domain::overview;

use crate::client::{SqliteReadClient, WriteClient};
use crate::errors::CliError;

// ========== 11 live 查询命令 ==========

pub fn list(client: &SqliteReadClient) -> Result<(), CliError> {
    let store = SqliteCardStore::new(&client.conn);
    let cards = store.query_all(None, None)?;
    println!("Total: {} cards", cards.len());
    for c in &cards {
        println!("  {} [{}]", c.title, c.id);
    }
    Ok(())
}

pub fn search(client: &SqliteReadClient, text: &str) -> Result<(), CliError> {
    let store = SqliteCardStore::new(&client.conn);
    let cards = store.search(text)?;
    println!("Found: {} results for '{}'", cards.len(), text);
    for c in &cards {
        println!("  {} [{}]", c.title, c.id);
    }
    Ok(())
}

pub fn file(client: &SqliteReadClient, path: &str) -> Result<(), CliError> {
    let store = SqliteCardStore::new(&client.conn);
    let cards = store.query_by_file(path)?;
    println!("File: {} ({} cards)", path, cards.len());
    for c in &cards {
        println!("  {} [{}]", c.title, c.id);
    }
    Ok(())
}

pub fn links(client: &SqliteReadClient, query: &str) -> Result<(), CliError> {
    let store = SqliteCardStore::new(&client.conn);
    let links = store.query_links(query)?;
    println!("Links for: {}", query);
    print_edge_group("link_to", "→", &links.link_to);
    print_edge_group("related", "→", &links.related);
    print_edge_group("see_also", "→", &links.see_also);
    print_edge_group("linked_from", "←", &links.linked_from);
    print_edge_group("related_from", "←", &links.related_from);
    print_edge_group("see_also_from", "←", &links.see_also_from);
    Ok(())
}

pub fn related(client: &SqliteReadClient, id: &str) -> Result<(), CliError> {
    let store = SqliteCardStore::new(&client.conn);
    let links = store.query_links(id)?;
    println!("Related for: {}", id);
    print_edge_group("out", "→", &links.related);
    print_edge_group("in", "←", &links.related_from);
    Ok(())
}

pub fn notes(client: &SqliteReadClient) -> Result<(), CliError> {
    let notes = note::query_all_cross_whiteboard(&client.conn)?;
    println!("Total: {} notes (cross-whiteboard)", notes.len());
    for n in &notes {
        println!("  {} [{}]", n.title, n.id);
    }
    Ok(())
}

pub fn note(client: &SqliteReadClient, path: &str) -> Result<(), CliError> {
    let store = SqliteNoteStore::new(&client.conn);
    let notes = store.query_by_file(path)?;
    if notes.is_empty() {
        println!("No note found for: {}", path);
    } else {
        for n in &notes {
            println!("Note: {} [{}]", n.title, n.id);
            println!("{}", n.content);
        }
    }
    Ok(())
}

pub fn stats(client: &SqliteReadClient) -> Result<(), CliError> {
    let s = overview::stats(&client.conn)?;
    println!("cards:     {}", s.cards);
    println!("notes:     {}", s.notes);
    println!("sections:  {}", s.sections);
    println!("aliases:   {}", s.aliases);
    println!("tasks:     {}", s.tasks);
    println!("questions: {}", s.questions);
    println!("edges:     {}", s.edges);
    Ok(())
}

pub fn overview(client: &SqliteReadClient) -> Result<(), CliError> {
    let o = overview::graph_overview(&client.conn)?;
    println!("Whiteboards: {}", o.whiteboards.len());
    for wb in &o.whiteboards {
        println!(
            "  {}: {} cards, {} sections, {} notes, {} aliases",
            wb.whiteboard_id, wb.cards, wb.sections, wb.notes, wb.aliases
        );
    }
    Ok(())
}

pub fn graph_notes(client: &SqliteReadClient, wb: &str) -> Result<(), CliError> {
    let store = SqliteNoteStore::new(&client.conn);
    let notes = store.query_all(wb)?;
    println!("Whiteboard {}: {} notes", wb, notes.len());
    for n in &notes {
        println!("  {} [{}]", n.title, n.id);
    }
    Ok(())
}

pub fn graph_note(client: &SqliteReadClient, id: &str, _wb: &str) -> Result<(), CliError> {
    // 注:--wb 参数冗余(entity 行自带 whiteboard_id);接受但不 validate
    let store = SqliteNoteStore::new(&client.conn);
    let n = store.get(id)?;
    println!("Note: {} [{}]", n.title, n.id);
    println!("{}", n.content);
    Ok(())
}

// ========== 9 live mutate 命令(Phase 6.2a)==========
//
// 所有 signature 用 `&dyn WriteClient`(trait-first)—— commands 层测试可 mock,
// 和 HttpClient 具体实现解耦。main.rs `dispatch_graph_mutate` 传 `&HttpClient`,
// Rust auto-coerce 到 `&dyn WriteClient`,调用侧无额外负担。

/// 创建 section — POST `/rpc` `{ method: "mutate", params: { kind: "section-create", wb, title, color? } }`
pub fn graph_section_create(
    client: &dyn WriteClient,
    wb: &str,
    title: &str,
    color: Option<&str>,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "section-create",
        "wb": wb,
        "title": title,
        "color": color,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("section-create OK: {}", result);
    Ok(())
}

/// 创建 note
pub fn graph_note_create(
    client: &dyn WriteClient,
    wb: &str,
    title: &str,
    content: Option<&str>,
    color: Option<&str>,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "note-create",
        "wb": wb,
        "title": title,
        "content": content,
        "color": color,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("note-create OK: {}", result);
    Ok(())
}

/// 更新 note
pub fn graph_note_update(
    client: &dyn WriteClient,
    id: &str,
    title: Option<&str>,
    content: Option<&str>,
    color: Option<&str>,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "note-update",
        "id": id,
        "title": title,
        "content": content,
        "color": color,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("note-update OK: {}", result);
    Ok(())
}

/// 创建 card alias
pub fn graph_alias_create(
    client: &dyn WriteClient,
    wb: &str,
    card_id: &str,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "alias-create",
        "wb": wb,
        "card_id": card_id,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("alias-create OK: {}", result);
    Ok(())
}

/// 设置 entity 位置
pub fn graph_set_pos(
    client: &dyn WriteClient,
    wb: &str,
    entity_id: &str,
    x: f64,
    y: f64,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "set-pos",
        "wb": wb,
        "entity_id": entity_id,
        "x": x,
        "y": y,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("set-pos OK: {}", result);
    Ok(())
}

/// 连接两个 entity(from → to)。Edge variant 由 server 端 `user_draw_edge`
/// 从 `EntityId::parse(from)` 的 kind 推导,wire 不传 edge_type。
pub fn graph_connect(
    client: &dyn WriteClient,
    from: &str,
    to: &str,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "connect",
        "from": from,
        "to": to,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("connect OK: {}", result);
    Ok(())
}

/// 断开两个 entity。edge_type 透传字符串(domain `EntityGraph::disconnect` 本身就是 stringly-typed)
pub fn graph_disconnect(
    client: &dyn WriteClient,
    from: &str,
    to: &str,
    edge_type: &str,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "disconnect",
        "from": from,
        "to": to,
        "edge_type": edge_type,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("disconnect OK: {}", result);
    Ok(())
}

/// 添加成员到 section
pub fn graph_section_add(
    client: &dyn WriteClient,
    section_id: &str,
    entity_id: &str,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "section-add",
        "section_id": section_id,
        "entity_id": entity_id,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("section-add OK: {}", result);
    Ok(())
}

/// 移动 section 到另一个 whiteboard
pub fn graph_section_move(
    client: &dyn WriteClient,
    section_id: &str,
    target_wb: &str,
) -> Result<(), CliError> {
    let params = serde_json::json!({
        "kind": "section-move",
        "section_id": section_id,
        "target_wb": target_wb,
    });
    let result = client.post_rpc("mutate", params)?;
    println!("section-move OK: {}", result);
    Ok(())
}

// ========== Phase 6.3 flush ==========

/// 请求 Tauri 侧发 `vault:flush` event,让 UI 重新 sync。
///
/// spec L256:POST /rpc `{method: "flush", params: {}}`;server 侧在 Phase 6.2b
/// (commit 1e3b413)已实装为 `emitter.emit("vault:flush", Null)`,无结构化返回值。
pub fn flush(client: &dyn WriteClient) -> Result<(), CliError> {
    client.post_rpc("flush", serde_json::json!({}))?;
    println!("flush OK");
    Ok(())
}

// ========== 4 deferred 命令(Phase 6.1:weak-list / graph get-bounds;Phase 6.2a:weak-add / weak-remove)==========

pub fn weak_list(_id: Option<&str>) -> Result<(), CliError> {
    Err(CliError::NotImplemented {
        command: "weak-list",
        reason: "requires weak-link metadata schema (target_path/title/anchor/reason columns)",
    })
}

pub fn graph_get_bounds(_section: &str) -> Result<(), CliError> {
    Err(CliError::NotImplemented {
        command: "graph get-bounds",
        reason: "requires layout subsystem port (compute_bounds + estimate_card_height)",
    })
}

/// Phase 6.2a deferred —— 同 weak-list structural gap(schema 级缺失)
pub fn weak_add_deferred() -> Result<(), CliError> {
    Err(CliError::NotImplemented {
        command: "weak-add",
        reason: "requires weak-link metadata schema (target_path/title/anchor/reason columns)",
    })
}

/// Phase 6.2a deferred —— 同 weak-list structural gap
pub fn weak_remove_deferred() -> Result<(), CliError> {
    Err(CliError::NotImplemented {
        command: "weak-remove",
        reason: "requires weak-link metadata schema (target_path/title/anchor/reason columns)",
    })
}

// ========== helpers ==========

fn print_edge_group(label: &str, arrow: &str, ids: &[String]) {
    if ids.is_empty() {
        return;
    }
    println!("  {}: {}", label, ids.len());
    for id in ids {
        println!("    {} {}", arrow, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 6.1 deferred 契约锁(spec.md 未新增 scenario,plan-level 强制)
    #[test]
    fn test_weak_list_returns_not_implemented_error() {
        match weak_list(None) {
            Err(CliError::NotImplemented { command, .. }) => {
                assert_eq!(command, "weak-list");
            }
            other => panic!(
                "expected Err(CliError::NotImplemented {{ command: \"weak-list\", .. }}), got {:?}",
                other
            ),
        }
    }

    /// Phase 6.1 deferred 契约锁(spec.md 未新增 scenario,plan-level 强制)。
    /// 注:命令名 `"graph get-bounds"` 带空格 —— clap nested subcommand 语义
    /// (`keysight-cli graph get-bounds <SEC>`),对应 plan.md L473 用户可见 CLI surface。
    #[test]
    fn test_graph_get_bounds_returns_not_implemented_error() {
        match graph_get_bounds("sec_dummy01") {
            Err(CliError::NotImplemented { command, .. }) => {
                assert_eq!(command, "graph get-bounds");
            }
            other => panic!(
                "expected Err(CliError::NotImplemented {{ command: \"graph get-bounds\", .. }}), got {:?}",
                other
            ),
        }
    }

    /// Phase 6.2a deferred 契约锁(同 weak-list pattern,schema 级 structural gap)
    #[test]
    fn test_weak_add_returns_not_implemented_error() {
        match weak_add_deferred() {
            Err(CliError::NotImplemented { command, .. }) => {
                assert_eq!(command, "weak-add");
            }
            other => panic!(
                "expected Err(CliError::NotImplemented {{ command: \"weak-add\", .. }}), got {:?}",
                other
            ),
        }
    }

    /// Phase 6.2a deferred 契约锁
    #[test]
    fn test_weak_remove_returns_not_implemented_error() {
        match weak_remove_deferred() {
            Err(CliError::NotImplemented { command, .. }) => {
                assert_eq!(command, "weak-remove");
            }
            other => panic!(
                "expected Err(CliError::NotImplemented {{ command: \"weak-remove\", .. }}), got {:?}",
                other
            ),
        }
    }
}
