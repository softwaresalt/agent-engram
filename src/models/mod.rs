//! Domain model types for Engram workspace entities.
//!
//! Provides code graph models: [`CodeFile`], [`Function`], [`Class`],
//! [`Interface`], and [`CodeEdge`]; the file watcher event types
//! [`WatcherEvent`] and [`WatchEventKind`]; workspace content
//! intelligence models: [`RegistryConfig`], [`ContentRecord`],
//! [`BacklogFile`], and [`CommitNode`]; Power BI project entity models
//! [`PowerBiReport`], [`PowerBiSemanticModel`], and [`PowerBiIndexResult`];
//! Power BI graph models [`PowerBiNode`], [`PowerBiEdge`], and
//! [`PowerBiEdgeType`]; and graph query types [`TraversalDirection`].

pub mod backlog;
pub mod backlog_graph;
pub mod class;
pub mod code_edge;
pub mod code_file;
pub mod commit;
pub mod config;
pub mod content;
pub mod evaluation;
pub mod file_hash;
pub mod function;
pub mod graph_query;
pub mod health;
pub mod interface;
pub mod metrics;
pub mod notebook;
pub mod policy;
pub mod powerbi;
pub mod powerbi_graph;
pub mod registry;
pub mod watcher;

pub use backlog::{BacklogArtifacts, BacklogFile, BacklogItem, BacklogRef, ProjectManifest};
pub use backlog_graph::{
    BacklogContentRecord, BacklogEdge, BacklogEdgeType, BacklogIndexResult, BacklogNode,
};
pub use class::Class;
pub use code_edge::CodeEdge;
pub use code_file::CodeFile;
pub use commit::{ChangeRecord, ChangeType, CommitNode};
pub use config::{BatchConfig, CodeGraphConfig, EmbeddingConfig, PluginConfig, WorkspaceConfig};
pub use content::ContentRecord;
pub use file_hash::FileHashRecord;
pub use function::Function;
pub use graph_query::TraversalDirection;
pub use health::{HealthCheck, HealthReport, HealthStatus, ScanProgress, SmokeResult};
pub use interface::Interface;
pub use metrics::MetricsConfig;
pub use notebook::NotebookIndexResult;
pub use powerbi::PowerBiIndexResult;
pub use powerbi_graph::{
    PowerBiEdge, PowerBiEdgeType, PowerBiGraphIndexResult, PowerBiNode, PowerBiNodeKind,
};
pub use registry::{ContentSource, ContentSourceStatus, RegistryConfig};
pub use watcher::{WatchEventKind, WatcherEvent};
