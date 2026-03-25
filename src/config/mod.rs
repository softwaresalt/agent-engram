//! Daemon configuration via CLI arguments and environment variables.
//!
//! Uses `clap` derive for parsing. All fields support both `--flag`-style
//! CLI arguments and `ENGRAM_`-prefixed environment variables.

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum StaleStrategy {
    Warn,
    Rehydrate,
    Fail,
}

/// Log output format for tracing subscriber
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

impl LogFormat {
    fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "json" => Self::Json,
            _ => Self::Pretty,
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[command(name = "engram", about = "Agent Engram MCP daemon", version)]
pub struct Config {
    /// Port for the HTTP/SSE server
    #[arg(long, env = "ENGRAM_PORT", default_value_t = 7437)]
    pub port: u16,

    /// Request timeout in milliseconds
    #[arg(long, env = "ENGRAM_REQUEST_TIMEOUT_MS", default_value_t = 60_000)]
    pub request_timeout_ms: u64,

    /// Maximum number of active workspaces
    #[arg(long, env = "ENGRAM_MAX_WORKSPACES", default_value_t = 10)]
    pub max_workspaces: usize,

    /// Behavior when workspace files are stale
    #[arg(long, env = "ENGRAM_STALE_STRATEGY", value_enum, default_value_t = StaleStrategy::Warn)]
    pub stale_strategy: StaleStrategy,

    /// Log format: json or pretty
    #[arg(long, env = "ENGRAM_LOG_FORMAT", default_value = "pretty")]
    pub log_format: String,

    /// Maximum number of events stored in the rolling event ledger per workspace
    #[arg(long, env = "ENGRAM_EVENT_LEDGER_MAX", default_value_t = 500)]
    pub event_ledger_max: usize,

    /// Allow AI agents to execute rollback_to_event operations
    #[arg(long, env = "ENGRAM_ALLOW_AGENT_ROLLBACK", default_value_t = false)]
    pub allow_agent_rollback: bool,

    /// Timeout in milliseconds for sandboxed graph queries
    #[arg(long, env = "ENGRAM_QUERY_TIMEOUT_MS", default_value_t = 50)]
    pub query_timeout_ms: u64,

    /// Maximum number of rows returned by sandboxed graph queries
    #[arg(long, env = "ENGRAM_QUERY_ROW_LIMIT", default_value_t = 1000)]
    pub query_row_limit: usize,

    /// OTLP gRPC endpoint for exporting trace spans (requires otlp-export feature)
    #[arg(long, env = "ENGRAM_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,
}

impl Config {
    pub fn parse() -> Self {
        Self::parse_from(std::env::args())
    }

    pub fn log_format(&self) -> LogFormat {
        LogFormat::from_str(&self.log_format)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            return Err("port must be > 0".into());
        }
        if self.request_timeout_ms == 0 {
            return Err("request_timeout_ms must be > 0".into());
        }
        if self.max_workspaces == 0 {
            return Err("max_workspaces must be > 0".into());
        }
        if self.event_ledger_max == 0 {
            return Err("event_ledger_max must be > 0".into());
        }
        if self.query_timeout_ms == 0 {
            return Err("query_timeout_ms must be > 0".into());
        }
        if self.query_row_limit == 0 {
            return Err("query_row_limit must be > 0".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let cfg = Config::parse_from(["engram"]);
        assert_eq!(cfg.port, 7437);
        assert!(cfg.request_timeout_ms > 0);
        assert_eq!(cfg.max_workspaces, 10);
        assert_eq!(cfg.stale_strategy, StaleStrategy::Warn);
    }
}
