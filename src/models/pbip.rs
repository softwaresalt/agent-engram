//! Power BI project (PBIP) ingestion models.
//!
//! Provides the result type emitted by the dedicated `pbip` indexer
//! ([`crate::services::pbip_indexer`]) that handles the newer
//! project-definition layout (`.pbip`, `.pbir`, `.pbism`, split report/page/visual
//! JSON, and `definition/**/*.tmdl`).
//!
//! Kept separate from [`crate::models::powerbi`] so the legacy JSON/BIM-backed
//! `powerbi` source contract and the newer project-definition `pbip` source
//! contract evolve independently.

/// Aggregated result from a PBIP indexer run over one content source.
#[derive(Debug, Default, Clone)]
pub struct PbipIndexResult {
    /// Number of PBIP project-definition files newly ingested or updated due to content change.
    pub ingested: usize,

    /// Number of PBIP project-definition files skipped because content hash was unchanged.
    pub unchanged: usize,

    /// Number of PBIP project-definition files removed from the index during deletion sweep.
    pub removed: usize,

    /// Total PBIP project-definition files scanned in the source path.
    pub total_files: usize,
}
