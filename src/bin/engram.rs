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
    /// Creates `.engram/` directory structure and generates MCP configuration.
    Install,
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
        Command::Install => {
            engram::installer::install().await?;
        }
        Command::Update => {
            engram::installer::update().await?;
        }
        Command::Reinstall => {
            engram::installer::reinstall().await?;
        }
        Command::Uninstall { keep_data } => {
            engram::installer::uninstall(keep_data).await?;
        }
    }

    Ok(())
}
