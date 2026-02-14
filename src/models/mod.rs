//! Domain model types for T-Mem workspace entities.
//!
//! Provides [Task], [Spec], [Context], and [DependencyType] — the
//! core entities stored in SurrealDB and serialized to `.tmem/` files.

pub mod context;
pub mod graph;
pub mod spec;
pub mod task;

pub use context::Context;
pub use graph::DependencyType;
pub use spec::Spec;
pub use task::{Task, TaskStatus};
