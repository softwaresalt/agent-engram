//! Global CLI flags shared across all CLI subcommands.

use std::time::Duration;

use clap::Args;

/// Global flags available on every CLI subcommand.
#[derive(Debug, Clone, Args)]
pub struct GlobalFlags {
    /// Workspace root path. Defaults to cwd or ENGRAM_WORKSPACE env var.
    #[arg(long, global = true, env = "ENGRAM_WORKSPACE")]
    pub workspace: Option<String>,

    /// JSON-RPC 2.0 request ID echoed in the response.
    #[arg(long, global = true)]
    pub id: Option<String>,

    /// Force JSON-RPC 2.0 output (overrides --format).
    #[arg(long, global = true)]
    pub json: bool,

    /// Output format: "json" or "text". Defaults to "json" on non-TTY.
    #[arg(long, global = true, value_name = "FORMAT", value_parser = ["json", "text"])]
    pub format: Option<String>,

    /// Suppress non-error output.
    #[arg(long, global = true)]
    pub quiet: bool,

    /// IPC request timeout in seconds. Overrides the per-command default.
    /// Set higher for long-running operations such as full index on large workspaces.
    /// Env: ENGRAM_CLI_TIMEOUT
    #[arg(long, global = true, value_name = "SECS", env = "ENGRAM_CLI_TIMEOUT")]
    pub timeout: Option<u64>,

    /// Caller-supplied correlation id stamped onto emitted usage-telemetry
    /// records (dual-source with MCP `_meta.correlation_id`). Precedence:
    /// `--correlation-id` flag > `ENGRAM_CORRELATION_ID` env > unset. Rejected
    /// when it contains control characters or exceeds 128 characters.
    #[arg(
        long,
        global = true,
        value_name = "ID",
        env = "ENGRAM_CORRELATION_ID"
    )]
    pub correlation_id: Option<String>,
}

impl GlobalFlags {
    /// Resolve the request ID as a `serde_json::Value`.
    pub fn id_value(&self) -> Option<serde_json::Value> {
        self.id.as_deref().map(|s| {
            // Try numeric first, fall back to string.
            if let Ok(n) = s.parse::<i64>() {
                serde_json::Value::Number(n.into())
            } else {
                serde_json::Value::String(s.to_owned())
            }
        })
    }

    /// Resolve the IPC timeout, giving precedence to an explicit `--timeout` flag
    /// or `ENGRAM_CLI_TIMEOUT` env var, falling back to `command_default_secs`.
    pub fn ipc_timeout(&self, command_default_secs: u64) -> Duration {
        Duration::from_secs(self.timeout.unwrap_or(command_default_secs))
    }

    /// Resolve the workspace path: flag → env var → cwd.
    ///
    /// # Errors
    ///
    /// Returns an error string if cwd cannot be determined.
    pub fn resolve_workspace(&self) -> Result<std::path::PathBuf, String> {
        if let Some(ws) = &self.workspace {
            return Ok(std::path::PathBuf::from(ws));
        }
        std::env::current_dir().map_err(|e| format!("cannot determine workspace: {e}"))
    }

    /// Resolve and validate the caller-supplied correlation id.
    ///
    /// clap already applies the flag → `ENGRAM_CORRELATION_ID` precedence. This
    /// applies the strict CLI/direct policy (reject control chars / over-128),
    /// treating an empty value as unset.
    ///
    /// # Errors
    ///
    /// Returns an error string when the id contains control characters or
    /// exceeds the length cap.
    pub fn resolve_correlation_id(&self) -> Result<Option<String>, String> {
        match self.correlation_id.as_deref() {
            Some(raw) => crate::models::metrics::validate_correlation_id(raw),
            None => Ok(None),
        }
    }
}
