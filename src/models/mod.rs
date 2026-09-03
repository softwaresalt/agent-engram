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
pub mod lineage;
pub mod metrics;
pub mod notebook;
pub mod pbip;
pub mod policy;
pub mod powerbi;
pub mod powerbi_graph;
pub mod registry;
pub mod retrieval_eval;
pub mod watcher;

pub use backlog::{BacklogArtifacts, BacklogFile, BacklogItem, BacklogRef, ProjectManifest};
pub use backlog_graph::{
    BacklogContentRecord, BacklogEdge, BacklogEdgeType, BacklogIndexResult, BacklogNode,
};
pub use class::Class;
pub use code_edge::CodeEdge;
pub use code_file::CodeFile;
pub use commit::{ChangeRecord, ChangeType, CommitNode};
pub use config::{
    BatchConfig, CodeGraphConfig, DaemonMode, DaemonModeParseError, EmbeddingConfig, LineageConfig,
    PluginConfig, WorkspaceConfig,
};
pub use content::ContentRecord;
pub use file_hash::FileHashRecord;
pub use function::Function;
pub use graph_query::TraversalDirection;
pub use health::{HealthCheck, HealthReport, HealthStatus, ScanProgress, SmokeResult};
pub use interface::Interface;
pub use lineage::{
    CURRENT_EXTRACTOR_VERSION, DatasetKind, LINEAGE_DERIVES_FROM, LineageAuthorityContext,
    LineageEdgeCandidate, LineageEndpoint, LineageEvidence,
};
pub use metrics::MetricsConfig;
pub use notebook::NotebookIndexResult;
pub use pbip::PbipIndexResult;
pub use powerbi::PowerBiIndexResult;
pub use powerbi_graph::{
    PowerBiEdge, PowerBiEdgeType, PowerBiGraphIndexResult, PowerBiNode, PowerBiNodeKind,
};
pub use registry::{ContentSource, ContentSourceStatus, RegistryConfig};
pub use retrieval_eval::{
    GraphMetrics, RetrievalEvalConfig, RetrievalEvalReport, RetrievalEvalThresholds,
    SemanticMetrics,
};
pub use watcher::{WatchEventKind, WatcherEvent};
