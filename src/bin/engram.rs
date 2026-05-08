use anyhow::Result;
use clap::{Parser, Subcommand};

use engram::cli::commands::{indexing, lifecycle, manifest, report, search};
use engram::cli::flags::GlobalFlags;
use engram::cli::output::OutputFormatter;

/// Engram workspace-local MCP plugin.
///
/// Manages per-workspace daemon processes that serve MCP tool calls via stdio.
/// The shim subcommand (default) is the MCP client entry point; the daemon
/// subcommand is spawned automatically by the shim.
#[derive(Debug, Parser)]
#[command(name = "engram", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Debug, Subcommand)]
enum Command {
    // ── Internal commands (existing) ────────────────────────────────────────
    /// Run as MCP stdio shim (default). Connects to or spawns the workspace daemon,
    /// then proxies MCP JSON-RPC from stdin to the daemon and back to stdout.
    Shim,

    /// Run as workspace daemon. Manages workspace state, IPC server, file watching,
    /// and idle timeout. Spawned automatically by the shim; not intended for direct use.
    Daemon,

    /// Install the engram plugin into the current workspace.
    /// Creates `.engram/` directory structure, generates MCP configuration,
    /// and writes agent hook files for GitHub Copilot, Claude Code, and Cursor.
    Install {
        /// Generate only agent hook files; skip `.engram/` data file creation.
        #[arg(long)]
        hooks_only: bool,
        /// Skip agent hook file generation.
        #[arg(long)]
        no_hooks: bool,
        /// MCP HTTP endpoint port to embed in hook file URLs.
        #[arg(long, default_value_t = engram::installer::DEFAULT_PORT)]
        port: u16,
    },

    /// Update the engram plugin runtime artifacts (binary references, config templates).
    /// Preserves existing workspace data files.
    Update,

    /// Reinstall the engram plugin, cleaning runtime artifacts while preserving data.
    Reinstall,

    /// Remove the engram plugin from the workspace.
    /// Stops any running daemon and removes plugin artifacts.
    Uninstall {
        /// Keep workspace data files (tasks.md, config.toml, etc.) after uninstall.
        #[arg(long)]
        keep_data: bool,
    },

    // ── CLI parity subcommands ───────────────────────────────────────────────
    /// Bind the daemon to a workspace directory (`set_workspace`).
    /// Defaults to the current working directory when <path> is omitted.
    Bind {
        /// Workspace root path. Defaults to cwd.
        path: Option<String>,
    },

    /// Return runtime metrics for the running daemon (`get_daemon_status`).
    #[command(name = "daemon-status")]
    DaemonStatus,

    /// Return current workspace status and code-graph statistics (`get_workspace_status`).
    #[command(name = "workspace-status")]
    WorkspaceStatus,

    /// Persist in-memory workspace state to disk (`flush_state`).
    Flush,

    /// Incrementally synchronize changed source files into the code graph (`sync_workspace`).
    /// Use --full to force a complete re-index.
    Sync {
        /// Force full re-index instead of incremental sync.
        #[arg(long)]
        full: bool,
    },

    /// Parse and index all workspace source files into the code graph (`index_workspace`).
    /// Equivalent to `engram sync --full`.
    Index,

    /// List MCP tools registered in the compile-time catalog (local, no daemon required).
    Manifest,

    /// Search across tasks, context records, and code symbols (`unified_search`).
    Search {
        /// Natural language search query.
        query: String,
        /// Limit search to a specific region (tasks, context, code).
        #[arg(long)]
        region: Option<String>,
        /// Maximum number of results (default 20).
        #[arg(long)]
        limit: Option<u32>,
        /// Filter by content type.
        #[arg(long)]
        content_type: Option<String>,
        /// Scope search to callers/callees of this symbol.
        #[arg(long)]
        scope_to: Option<String>,
    },

    /// Search workspace context records with a natural language query (`query_memory`).
    #[command(name = "query-memory")]
    QueryMemory {
        /// Natural language search query.
        query: String,
        /// Maximum number of results (default 10).
        #[arg(long)]
        limit: Option<u32>,
        /// Filter by content type.
        #[arg(long)]
        content_type: Option<String>,
    },

    /// List indexed symbols with optional filters (`list_symbols`).
    Symbols {
        /// Filter to symbols in this file path.
        #[arg(long)]
        file: Option<String>,
        /// Filter by symbol kind (function, struct, enum, trait, impl, …).
        #[arg(long = "type", value_name = "KIND")]
        node_type: Option<String>,
        /// Filter to symbols whose name starts with this prefix.
        #[arg(long)]
        prefix: Option<String>,
        /// Maximum number of results (default 50).
        #[arg(long)]
        limit: Option<u32>,
        /// Pagination offset.
        #[arg(long)]
        offset: Option<u32>,
    },

    /// Return the call graph and usages for a named symbol (`map_code`).
    #[command(name = "map-code")]
    MapCode {
        /// Name of the symbol to map.
        symbol: String,
        /// Maximum traversal depth (default 1).
        #[arg(long)]
        depth: Option<u32>,
        /// Maximum number of graph nodes.
        #[arg(long)]
        max_nodes: Option<u32>,
    },

    /// Identify code and context affected by changes to a symbol (`impact_analysis`).
    Impact {
        /// Name of the changed symbol.
        symbol: String,
        /// Call-graph traversal depth (default 1).
        #[arg(long)]
        depth: Option<u32>,
        /// Maximum number of graph nodes.
        #[arg(long)]
        max_nodes: Option<u32>,
        /// Conceptual scope hint to narrow the analysis.
        #[arg(long)]
        concept: Option<String>,
    },

    /// Execute a read-only Datalog query against the workspace graph (`query_graph`).
    ///
    /// **Note**: this subcommand is not yet implemented. It always returns an error.
    /// Included here to complete the CLI surface for future activation.
    #[command(name = "query-graph")]
    QueryGraph {
        /// Datalog query string (`CozoScript`).
        query: String,
    },

    /// Return workspace statistics: task counts, label distribution (`get_workspace_statistics`).
    Stats,

    /// Return daemon health metrics and latency percentiles (`get_health_report`).
    Health,

    /// Return branch metrics or compare two branches (`get_branch_metrics`).
    #[command(name = "branch-metrics")]
    BranchMetrics {
        /// Branch to summarize; defaults to current branch.
        #[arg(long)]
        branch: Option<String>,
        /// Optional second branch to compare against.
        #[arg(long)]
        compare: Option<String>,
    },

    /// Generate report subcommands (token-savings, eval, retry-metrics).
    Report {
        #[command(subcommand)]
        subcommand: ReportCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ReportCommand {
    /// Token delivery summary for the current branch (`get_token_savings_report`).
    #[command(name = "token-savings")]
    TokenSavings,
    /// Agent efficiency evaluation report (`get_evaluation_report`).
    Eval,
    /// Retry metrics for mutable script executions (`get_mutable_script_retry_metrics`).
    #[command(name = "retry-metrics")]
    RetryMetrics,
}

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let flags = cli.flags;

    match cli.command.unwrap_or(Command::Shim) {
        // ── Internal commands ─────────────────────────────────────────────────
        Command::Shim => {
            engram::shim::run(flags.workspace.as_deref()).await?;
        }
        Command::Daemon => {
            engram::init_tracing(engram::config::LogFormat::Pretty);
            let workspace = flags
                .workspace
                .clone()
                .ok_or_else(|| anyhow::anyhow!("daemon requires --workspace <path>"))?;
            engram::daemon::run(&workspace).await?;
        }
        Command::Install {
            hooks_only,
            no_hooks,
            port,
        } => {
            let workspace = std::env::current_dir()?;
            let opts = engram::installer::InstallOptions {
                hooks_only,
                no_hooks,
                port,
            };
            engram::installer::install(&workspace, &opts).await?;
        }
        Command::Update => {
            let workspace = std::env::current_dir()?;
            engram::installer::update(&workspace).await?;
        }
        Command::Reinstall => {
            let workspace = std::env::current_dir()?;
            engram::installer::reinstall(&workspace).await?;
        }
        Command::Uninstall { keep_data } => {
            let workspace = std::env::current_dir()?;
            engram::installer::uninstall(&workspace, keep_data).await?;
        }

        // ── CLI parity commands ───────────────────────────────────────────────
        Command::Bind { path } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = lifecycle::run_bind(path, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::DaemonStatus => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = lifecycle::run_daemon_status(&flags, &fmt).await;
            std::process::exit(code);
        }
        Command::WorkspaceStatus => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = lifecycle::run_workspace_status(&flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Flush => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = lifecycle::run_flush(&flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Sync { full } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = indexing::run_sync(full, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Index => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = indexing::run_index(&flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Manifest => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = manifest::run_manifest(&flags, &fmt);
            std::process::exit(code);
        }
        Command::Search {
            query,
            region,
            limit,
            content_type,
            scope_to,
        } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code =
                search::run_search(query, region, limit, content_type, scope_to, &flags, &fmt)
                    .await;
            std::process::exit(code);
        }
        Command::QueryMemory {
            query,
            limit,
            content_type,
        } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = search::run_query_memory(query, limit, content_type, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Symbols {
            file,
            node_type,
            prefix,
            limit,
            offset,
        } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code =
                search::run_symbols(file, node_type, prefix, limit, offset, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::MapCode {
            symbol,
            depth,
            max_nodes,
        } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = search::run_map_code(symbol, depth, max_nodes, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Impact {
            symbol,
            depth,
            max_nodes,
            concept,
        } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = search::run_impact(symbol, depth, max_nodes, concept, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::QueryGraph { query } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = search::run_query_graph(query, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Stats => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = report::run_stats(&flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Health => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = report::run_health(&flags, &fmt).await;
            std::process::exit(code);
        }
        Command::BranchMetrics { branch, compare } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = report::run_branch_metrics(branch, compare, &flags, &fmt).await;
            std::process::exit(code);
        }
        Command::Report { subcommand } => {
            let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
            let code = match subcommand {
                ReportCommand::TokenSavings => report::run_token_savings(&flags, &fmt).await,
                ReportCommand::Eval => report::run_eval(&flags, &fmt).await,
                ReportCommand::RetryMetrics => report::run_retry_metrics(&flags, &fmt).await,
            };
            std::process::exit(code);
        }
    }

    Ok(())
}
