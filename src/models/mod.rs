//! Domain model types for Engram workspace entities.
//!
//! Provides [Task], [Spec], [Context], and [DependencyType] — the
//! core entities stored in SurrealDB and serialized to `.engram/` files.
//! Also provides code graph models: [CodeFile], [Function], [Class],
//! [Interface], and [CodeEdge], and the file watcher event types
//! [`WatcherEvent`] and [`WatchEventKind`].

pub mod class;
pub mod code_edge;
pub mod code_file;
pub mod comment;
pub mod config;
pub mod context;
pub mod function;
pub mod graph;
pub mod interface;
pub mod label;
pub mod spec;
pub mod task;
pub mod watcher;

pub use class::Class;
pub use code_edge::CodeEdge;
pub use code_file::CodeFile;
pub use comment::Comment;
pub use config::{
    BatchConfig, CodeGraphConfig, CompactionConfig, EmbeddingConfig, PluginConfig, WorkspaceConfig,
};
pub use context::Context;
pub use function::Function;
pub use graph::DependencyType;
pub use interface::Interface;
pub use label::Label;
pub use spec::Spec;
pub use task::{Task, TaskStatus, compute_priority_order};
pub use watcher::{WatchEventKind, WatcherEvent};
