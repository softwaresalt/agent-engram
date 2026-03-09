//! Daemon configuration via CLI arguments and environment variables.
//!
//! Uses `clap` derive for parsing. All fields support both `--flag`-style
//! CLI arguments and `ENGRAM_`-prefixed environment variables.

use std::path::{Path, PathBuf};

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

    /// Data directory for embedded database and models
    #[arg(long, env = "ENGRAM_DATA_DIR", default_value_os_t = default_data_dir())]
    pub data_dir: PathBuf,

    /// Behavior when workspace files are stale
    #[arg(long, env = "ENGRAM_STALE_STRATEGY", value_enum, default_value_t = StaleStrategy::Warn)]
    pub stale_strategy: StaleStrategy,

    /// Log format: json or pretty
    #[arg(long, env = "ENGRAM_LOG_FORMAT", default_value = "pretty")]
    pub log_format: String,
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
        if self.data_dir.as_os_str().is_empty() {
            return Err("data_dir must not be empty".into());
        }
        Ok(())
    }

    pub fn ensure_data_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }
}

fn default_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".local")
        .join("share")
        .join("engram")
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
