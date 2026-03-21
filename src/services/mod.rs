//! Business logic services for the Engram daemon.
//!
//! Each service module contains stateless free functions that accept
//! dependencies as parameters. Modules: connection lifecycle management,
//! hydration/dehydration of `.engram/` files, embedding generation, search,
//! tree-sitter AST parsing, and code graph orchestration.

pub mod code_graph;
pub mod config;
pub mod connection;
pub mod dehydration;
pub mod embedding;
pub mod gate;
#[cfg(feature = "git-graph")]
pub mod git_graph;
pub mod hydration;
pub mod ingestion;
pub mod output;
pub mod parsing;
pub mod registry;
pub mod search;
