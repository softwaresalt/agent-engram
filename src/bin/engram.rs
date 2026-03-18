use anyhow::Result;
use clap::{Parser, Subcommand};

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
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run as MCP stdio shim (default). Connects to or spawns the workspace daemon,
    /// then proxies MCP JSON-RPC from stdin to the daemon and back to stdout.
    Shim,
    /// Run as workspace daemon. Manages workspace state, IPC server, file watching,
    /// and idle timeout. Spawned automatically by the shim; not intended for direct use.
    Daemon {
        /// Absolute path to the workspace root.
        #[arg(long)]
        workspace: String,
    },
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Shim) {
        Command::Shim => {
            engram::shim::run().await?;
        }
        Command::Daemon { workspace } => {
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
    }

    Ok(())
}
