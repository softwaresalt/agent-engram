//! Global CLI flags shared across all CLI subcommands.

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
    #[arg(long, global = true, value_name = "FORMAT")]
    pub format: Option<String>,

    /// Suppress non-error output.
    #[arg(long, global = true)]
    pub quiet: bool,
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
}
