//! Business logic services for the Engram daemon.
//!
//! Each service module contains stateless free functions that accept
//! dependencies as parameters. Modules: connection lifecycle management,
//! hydration/dehydration of `.engram/` files, embedding generation, search,
//! tree-sitter AST parsing, and code graph orchestration.

pub mod code_graph;
pub mod compaction;
pub mod config;
pub mod connection;
pub mod dehydration;
pub mod embedding;
pub mod event_ledger;
pub mod gate;
pub mod hydration;
pub mod output;
pub mod parsing;
pub mod search;
