//! Business logic services for the Engram daemon.
//!
//! Each service module contains stateless free functions that accept
//! dependencies as parameters. Modules: connection lifecycle management,
//! hydration/dehydration of `.engram/` files, embedding generation, search,
//! tree-sitter AST parsing, code graph orchestration, and Power BI indexing.

pub mod backlog_indexer;
pub mod code_graph;
pub mod config;
pub mod connection;
pub mod cozo_validation;
pub mod dehydration;
pub mod embedding;
pub mod evaluation;
pub mod file_tracker;
pub mod gate;
#[cfg(feature = "git-graph")]
pub mod git_graph;
pub mod hydration;
pub mod ingestion;
pub mod metrics;
pub mod notebook_extract;
pub mod notebook_indexer;
pub mod output;
pub mod parsing;
pub mod policy;
pub mod powerbi_extract;
pub mod powerbi_indexer;
pub mod powerbi_tmdl;
pub mod query_stats;
pub mod registry;
pub mod search;
