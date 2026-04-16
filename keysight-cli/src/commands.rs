//! 13 query 命令 handler(11 live + 2 deferred)。
//!
//! 每个 live handler 接 `&SqliteReadClient` 和命令参数,调 keysight-core domain fn
//! 完成查询,然后 println! 格式化输出。deferred handler 直接返 `CliError::NotImplemented`。

use keysight_core::domain::card::{CardStore, SqliteCardStore};
use keysight_core::domain::note::{self, NoteStore, SqliteNoteStore};
use keysight_core::domain::overview;

use crate::client::SqliteReadClient;
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

// ========== 2 deferred 命令 ==========

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
}
