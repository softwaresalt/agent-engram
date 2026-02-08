#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]

/// Crate-level constants and shared library entrypoints for the T-Mem daemon.
pub const APP_NAME: &str = "t-mem";
pub mod config;
pub mod db;
pub mod errors;
pub mod models;
pub mod server;
pub mod tools;

use std::sync::OnceLock;

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config::LogFormat;

static TRACING_INIT: OnceLock<()> = OnceLock::new();

/// Initialize tracing subscriber in JSON or pretty mode; idempotent across calls.
pub fn init_tracing(format: LogFormat) {
    TRACING_INIT.get_or_init(|| {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("t_mem=debug,hyper=info,surrealdb=info"));

        let registry = tracing_subscriber::registry().with(env_filter);

        match format {
            LogFormat::Json => {
                registry
                    .with(fmt::layer().json().with_current_span(true))
                    .init();
            }
            LogFormat::Pretty => {
                registry.with(fmt::layer().pretty()).init();
            }
        }
    });
}
