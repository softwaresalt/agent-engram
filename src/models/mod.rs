//! Domain model types for Engram workspace entities.
//!
//! Provides [Task], [Spec], [Context], and [DependencyType] — the
//! core entities stored in SurrealDB and serialized to `.engram/` files.
//! Also provides code graph models: [CodeFile], [Function], [Class],
//! [Interface], and [CodeEdge], the file watcher event types
//! [`WatcherEvent`] and [`WatchEventKind`], and workspace content
//! intelligence models: [`RegistryConfig`], [`ContentRecord`],
//! [`BacklogFile`], and [`CommitNode`].

pub mod backlog;
pub mod class;
pub mod code_edge;
pub mod code_file;
pub mod collection;
pub mod comment;
pub mod commit;
pub mod config;
pub mod content;
pub mod context;
pub mod event;
pub mod function;
pub mod graph;
pub mod interface;
pub mod label;
pub mod registry;
pub mod spec;
pub mod task;
pub mod watcher;

pub use backlog::{BacklogArtifacts, BacklogFile, BacklogItem, BacklogRef, ProjectManifest};
pub use class::Class;
pub use code_edge::CodeEdge;
pub use code_file::CodeFile;
pub use collection::Collection;
pub use comment::Comment;
pub use commit::{ChangeRecord, ChangeType, CommitNode};
pub use config::{
    BatchConfig, CodeGraphConfig, CompactionConfig, EmbeddingConfig, PluginConfig, WorkspaceConfig,
};
pub use content::ContentRecord;
pub use context::Context;
pub use event::{Event, EventKind};
pub use function::Function;
pub use graph::DependencyType;
pub use interface::Interface;
pub use label::Label;
pub use registry::{ContentSource, ContentSourceStatus, RegistryConfig};
pub use spec::Spec;
pub use task::{Task, TaskStatus, compute_priority_order};
pub use watcher::{WatchEventKind, WatcherEvent};
