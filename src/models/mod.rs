//! Domain model types for Engram workspace entities.
//!
//! Provides [Task], [Spec], [Context], and [DependencyType] — the
//! core entities stored in SurrealDB and serialized to `.engram/` files.

pub mod comment;
pub mod config;
pub mod context;
pub mod graph;
pub mod label;
pub mod spec;
pub mod task;

pub use comment::Comment;
pub use config::{BatchConfig, CompactionConfig, WorkspaceConfig};
pub use context::Context;
pub use graph::DependencyType;
pub use label::Label;
pub use spec::Spec;
pub use task::{Task, TaskStatus, compute_priority_order};
