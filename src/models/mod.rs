//! Domain model types for Engram workspace entities.
//!
//! Provides code graph models: [`CodeFile`], [`Function`], [`Class`],
//! [`Interface`], and [`CodeEdge`]; the file watcher event types
//! [`WatcherEvent`] and [`WatchEventKind`]; and workspace content
//! intelligence models: [`RegistryConfig`], [`ContentRecord`],
//! [`BacklogFile`], and [`CommitNode`].

pub mod backlog;
pub mod class;
pub mod code_edge;
pub mod code_file;
pub mod commit;
pub mod config;
pub mod content;
pub mod file_hash;
pub mod function;
pub mod interface;
pub mod metrics;
pub mod registry;
pub mod watcher;

pub use backlog::{BacklogArtifacts, BacklogFile, BacklogItem, BacklogRef, ProjectManifest};
pub use class::Class;
pub use code_edge::CodeEdge;
pub use code_file::CodeFile;
pub use commit::{ChangeRecord, ChangeType, CommitNode};
pub use config::{BatchConfig, CodeGraphConfig, EmbeddingConfig, PluginConfig, WorkspaceConfig};
pub use content::ContentRecord;
pub use file_hash::FileHashRecord;
pub use function::Function;
pub use interface::Interface;
pub use metrics::MetricsConfig;
pub use registry::{ContentSource, ContentSourceStatus, RegistryConfig};
pub use watcher::{WatchEventKind, WatcherEvent};
