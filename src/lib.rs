//! Engram: a local-first MCP daemon providing persistent task memory and
//! semantic search for AI coding assistants.
//!
//! This crate exposes library modules used by the `engram` binary. The daemon
//! binds to `127.0.0.1` via axum, accepts MCP JSON-RPC over SSE, and stores
//! workspace state in an embedded SurrealDB instance backed by `.engram/` files.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::new_without_default)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::bool_to_int_with_if)]
#![allow(clippy::format_push_string)]
#![allow(clippy::implicit_hasher)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::unnecessary_filter_map)]
#![allow(clippy::trivially_copy_pass_by_ref)]

/// Crate-level constants and shared library entrypoints for the Engram daemon.
pub const APP_NAME: &str = "engram";
pub mod config;
pub mod db;
pub mod errors;
pub mod models;
pub mod server;
pub mod services;
pub mod tools;

use std::sync::OnceLock;

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config::LogFormat;

static TRACING_INIT: OnceLock<()> = OnceLock::new();

/// Initialize tracing subscriber in JSON or pretty mode; idempotent across calls.
pub fn init_tracing(format: LogFormat) {
    TRACING_INIT.get_or_init(|| {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("engram=debug,hyper=info,surrealdb=info"));

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
