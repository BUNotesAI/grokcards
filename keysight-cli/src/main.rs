//! keysight-cli 二进制入口。
//!
//! Phase 6.1 scope:
//! - 11 query 命令(直连 SQLite READ_ONLY,不依赖 Tauri HTTP)
//! - 2 deferred 命令(weak-list / graph-get-bounds):返 `CliError::NotImplemented` → exit 2
//! - 写/flush 命令全部在 Phase 6.2+ 实现

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use keysight_cli::client::SqliteReadClient;
use keysight_cli::commands;
use keysight_cli::config;
use keysight_cli::errors::CliError;

#[derive(Parser, Debug)]
#[command(name = "keysight-cli", version, about = "KeySight query CLI (read-only; writes via Phase 6.2+)")]
struct Cli {
    /// 覆盖 config 文件路径(优先级:flag > env KEYSIGHT_CLI_CONFIG > app_data_dir 默认)
    #[arg(long, global = true)]
    config_file: Option<PathBuf>,

    /// 覆盖 endpoint 文件路径(仅写/flush 命令读;查询命令不读)
    #[arg(long, global = true)]
    endpoint_file: Option<PathBuf>,

    #[command(subcommand)]
    command: QueryCommand,
}

#[derive(Subcommand, Debug)]
pub enum QueryCommand {
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
    /// Graph sub-commands namespace(Phase 6.1 仅 `get-bounds` 一个 subcommand,deferred)
    Graph {
        #[command(subcommand)]
        sub: GraphCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum GraphCommand {
    /// 获取 section 边界 —— **Phase 6.1 deferred**,返 exit 2。
    /// 用法:`keysight-cli graph get-bounds <SECTION>`
    #[command(name = "get-bounds")]
    GetBounds { section: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::NotImplemented { command, reason }) => {
            eprintln!(
                "{}: not yet implemented ({}, see follow-up task)",
                command, reason
            );
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
    let client = SqliteReadClient::open(&config.db_path)?;
    match cli.command {
        QueryCommand::List => commands::list(&client),
        QueryCommand::Search { text } => commands::search(&client, &text),
        QueryCommand::File { path } => commands::file(&client, &path),
        QueryCommand::Links { query } => commands::links(&client, &query),
        QueryCommand::Related { id } => commands::related(&client, &id),
        QueryCommand::Notes => commands::notes(&client),
        QueryCommand::Note { path } => commands::note(&client, &path),
        QueryCommand::Stats => commands::stats(&client),
        QueryCommand::Overview => commands::overview(&client),
        QueryCommand::GraphNotes { wb } => commands::graph_notes(&client, &wb),
        QueryCommand::GraphNote { id, wb } => commands::graph_note(&client, &id, &wb),
        QueryCommand::WeakList { id } => commands::weak_list(id.as_deref()),
        QueryCommand::Graph { sub } => match sub {
            GraphCommand::GetBounds { section } => commands::graph_get_bounds(&section),
        },
    }
}
