//! keysight-cli 二进制入口。
//!
//! Phase 6.1 scope:
//! - 11 live query 命令(直连 SQLite READ_ONLY,不依赖 Tauri HTTP)
//! - 2 deferred query 命令(`weak-list` / `graph get-bounds`)—— 返 `CliError::NotImplemented` → exit 2
//!
//! Phase 6.2a scope(本 phase):
//! - 9 live mutate 命令(POST /rpc,走 `HttpClient`)—— 依赖 Tauri 在线 + 正确 token + 版本匹配
//! - 2 deferred mutate 命令(`weak-add` / `weak-remove`)—— 同 weak-list structural gap
//! - `endpoint.rs`(三级优先级)+ `HttpClient`(构造即 version gate)
//!
//! Phase 6.2b scope(下一 phase):server-side mutate dispatcher + entity:changed emit

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use keysight_cli::client::{HttpClient, SqliteReadClient, CLI_EXPECTED_RPC_MAJOR};
use keysight_cli::commands;
use keysight_cli::config;
use keysight_cli::endpoint;
use keysight_cli::errors::CliError;

#[derive(Parser, Debug)]
#[command(name = "keysight-cli", version, about = "KeySight CLI (query + mutate via Tauri HTTP IPC)")]
struct Cli {
    /// 覆盖 config 文件路径(优先级:flag > env KEYSIGHT_CLI_CONFIG > app_data_dir 默认)
    #[arg(long, global = true)]
    config_file: Option<PathBuf>,

    /// 覆盖 endpoint 文件路径(仅写/flush 命令读;查询命令不读)
    #[arg(long, global = true)]
    endpoint_file: Option<PathBuf>,

    #[command(subcommand)]
    command: RootCommand,
}

/// CLI 顶层命令。Phase 6.2a 起,`QueryCommand` 重命名为 `RootCommand` —— 因为
/// 命令面已不再是纯 query(WeakAdd / WeakRemove mutate variants 扁平 sibling 级)。
#[derive(Subcommand, Debug)]
pub enum RootCommand {
    // —— Query(Phase 6.1 live + 1 deferred)——
    /// 列出全部 cards
    List,
    /// 按 title/content 搜索 cards
    Search { text: String },
    /// 按文件路径列出 cards
    File { path: String },
    /// 查询 card 的 link_to/related/see_also 出入边
    Links { query: String },
    /// 展示某 card 的 related(bidirectional projection)
    Related { id: String },
    /// 列出全部 study notes(跨 whiteboard)
    Notes,
    /// 按文件路径查 note
    Note { path: String },
    /// DB 统计
    Stats,
    /// Graph 全局 overview
    Overview,
    /// 按 whiteboard 列出 graph notes
    #[command(name = "graph-notes")]
    GraphNotes {
        #[arg(long, default_value = "wb_root")]
        wb: String,
    },
    /// 展示某 graph note
    #[command(name = "graph-note")]
    GraphNote {
        id: String,
        #[arg(long, default_value = "wb_root")]
        wb: String,
    },
    /// 列出 weak-links —— **Phase 6.1 deferred**,返 exit 2
    #[command(name = "weak-list")]
    WeakList { id: Option<String> },

    // —— Mutate 扁平(Phase 6.2a deferred,沿老 CLI 扁平形态)——
    /// 添加 weak-link —— **Phase 6.2a deferred**(schema 级 gap,同 weak-list),返 exit 2
    #[command(name = "weak-add")]
    WeakAdd {
        id: String,
        path: String,
        title: String,
        reason: String,
        anchor: Option<String>,
    },
    /// 移除 weak-link —— **Phase 6.2a deferred**,返 exit 2
    #[command(name = "weak-remove")]
    WeakRemove { id: String, path: String },

    /// 触发 Tauri 侧 `vault:flush` event,让 UI 重新 sync(Phase 6.3)
    Flush,

    /// Graph sub-commands namespace(Phase 6.1 `get-bounds` deferred;Phase 6.2a 9 live mutate)
    Graph {
        #[command(subcommand)]
        sub: GraphCommand,
    },
}

/// `graph` 命名空间子命令。Phase 6.1 仅 `get-bounds`(deferred);Phase 6.2a 加 9 live mutate。
#[derive(Subcommand, Debug)]
pub enum GraphCommand {
    /// 获取 section 边界 —— **Phase 6.1 deferred**,返 exit 2。用法:`keysight-cli graph get-bounds <SECTION>`
    #[command(name = "get-bounds")]
    GetBounds { section: String },

    // —— Phase 6.2a live mutate(9 条)——
    /// 创建 section
    #[command(name = "section-create")]
    SectionCreate {
        title: String,
        #[arg(long, default_value = "wb_root")]
        wb: String,
        #[arg(long)]
        color: Option<String>,
    },
    /// 创建 note
    #[command(name = "note-create")]
    NoteCreate {
        title: String,
        #[arg(long, default_value = "wb_root")]
        wb: String,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
    /// 更新 note
    #[command(name = "note-update")]
    NoteUpdate {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
    /// 创建 card alias
    #[command(name = "alias-create")]
    AliasCreate {
        card_id: String,
        #[arg(long, default_value = "wb_root")]
        wb: String,
    },
    /// 设置 entity 位置
    #[command(name = "set-pos")]
    SetPos {
        entity_id: String,
        x: f64,
        y: f64,
        #[arg(long, default_value = "wb_root")]
        wb: String,
    },
    /// 连接两个 entity(from → to)。Edge variant 由 server 从 from EntityId 推导
    Connect {
        from: String,
        to: String,
    },
    /// 断开两个 entity 的连接
    Disconnect {
        from: String,
        to: String,
        #[arg(long, default_value = "link_to")]
        edge_type: String,
    },
    /// 添加成员到 section
    #[command(name = "section-add")]
    SectionAdd {
        section_id: String,
        entity_id: String,
    },
    /// 移动 section 到另一个 whiteboard
    #[command(name = "section-move")]
    SectionMove {
        section_id: String,
        target_wb: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::NotImplemented { command, reason }) => {
            eprintln!("{}: not yet implemented ({}, see follow-up task)", command, reason);
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    let config = config::load_config(cli.config_file.as_deref(), None)?;

    match cli.command {
        // —— Query 路径:直连 SQLite READ_ONLY,不 load endpoint ——
        RootCommand::List => commands::list(&SqliteReadClient::open(&config.db_path)?),
        RootCommand::Search { text } => commands::search(&SqliteReadClient::open(&config.db_path)?, &text),
        RootCommand::File { path } => commands::file(&SqliteReadClient::open(&config.db_path)?, &path),
        RootCommand::Links { query } => commands::links(&SqliteReadClient::open(&config.db_path)?, &query),
        RootCommand::Related { id } => commands::related(&SqliteReadClient::open(&config.db_path)?, &id),
        RootCommand::Notes => commands::notes(&SqliteReadClient::open(&config.db_path)?),
        RootCommand::Note { path } => commands::note(&SqliteReadClient::open(&config.db_path)?, &path),
        RootCommand::Stats => commands::stats(&SqliteReadClient::open(&config.db_path)?),
        RootCommand::Overview => commands::overview(&SqliteReadClient::open(&config.db_path)?),
        RootCommand::GraphNotes { wb } => commands::graph_notes(&SqliteReadClient::open(&config.db_path)?, &wb),
        RootCommand::GraphNote { id, wb } => commands::graph_note(&SqliteReadClient::open(&config.db_path)?, &id, &wb),

        // —— Deferred:不 load endpoint,不开 SQLite ——
        RootCommand::WeakList { id } => commands::weak_list(id.as_deref()),
        RootCommand::WeakAdd { .. } => commands::weak_add_deferred(),
        RootCommand::WeakRemove { .. } => commands::weak_remove_deferred(),
        RootCommand::Graph { sub: GraphCommand::GetBounds { section } } => commands::graph_get_bounds(&section),

        // —— Mutate 路径:load endpoint → HttpClient(构造即 version gate)→ post ——
        RootCommand::Flush => {
            let endpoint_contents = endpoint::load_endpoint(cli.endpoint_file.as_deref(), None)?;
            let http = HttpClient::new(&endpoint_contents, CLI_EXPECTED_RPC_MAJOR)?;
            commands::flush(&http)
        }
        RootCommand::Graph { sub: mutate } => {
            let endpoint_contents = endpoint::load_endpoint(cli.endpoint_file.as_deref(), None)?;
            let http = HttpClient::new(&endpoint_contents, CLI_EXPECTED_RPC_MAJOR)?;
            dispatch_graph_mutate(&http, mutate)
        }
    }
}

fn dispatch_graph_mutate(http: &HttpClient, cmd: GraphCommand) -> Result<(), CliError> {
    match cmd {
        GraphCommand::GetBounds { .. } => unreachable!("GetBounds 在 run() 里已分流到 deferred,不进 mutate 路径"),
        GraphCommand::SectionCreate { title, wb, color } => commands::graph_section_create(http, &wb, &title, color.as_deref()),
        GraphCommand::NoteCreate { title, wb, content, color } => commands::graph_note_create(http, &wb, &title, content.as_deref(), color.as_deref()),
        GraphCommand::NoteUpdate { id, title, content, color } => commands::graph_note_update(http, &id, title.as_deref(), content.as_deref(), color.as_deref()),
        GraphCommand::AliasCreate { card_id, wb } => commands::graph_alias_create(http, &wb, &card_id),
        GraphCommand::SetPos { entity_id, x, y, wb } => commands::graph_set_pos(http, &wb, &entity_id, x, y),
        GraphCommand::Connect { from, to } => commands::graph_connect(http, &from, &to),
        GraphCommand::Disconnect { from, to, edge_type } => commands::graph_disconnect(http, &from, &to, &edge_type),
        GraphCommand::SectionAdd { section_id, entity_id } => commands::graph_section_add(http, &section_id, &entity_id),
        GraphCommand::SectionMove { section_id, target_wb } => commands::graph_section_move(http, &section_id, &target_wb),
    }
}
