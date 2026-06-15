//! Extraction of PBIP project-definition descriptors.
//!
//! Provides parsers for `.pbism` (semantic model project descriptor) JSON files
//! from the newer Power BI project-definition layout. Kept distinct from
//! [`crate::services::powerbi_extract`] so the legacy JSON/BIM-backed `powerbi`
//! source contract and the newer `pbip` contract evolve independently.
//!
//! # 062.003-T scope
//!
//! Task 062.003-T introduces the `.pbism` descriptor parser and the TMDL
//! semantic-model walker that lives in [`crate::services::pbip_tmdl`]. Full
//! report-side extraction (`.pbip`, `.pbir`, page and visual JSON) lands in
//! later units (062.006-T, 062.007-T).
//!
//! # Why not delegate to [`crate::services::powerbi_extract`]?
//!
//! `powerbi_extract` consumes `model.bim` (a single embedded JSON model). The
//! project-definition layout splits the model into a `.pbism` container plus
//! `definition/**/*.tmdl` siblings. The two paths require different parsers
//! and produce model entities through different code paths, so a dedicated
//! `pbip_extract` module keeps the boundaries clear.

use serde::{Deserialize, Serialize};

/// A parsed `.pbism` semantic model project descriptor.
///
/// Holds the metadata Engram needs to confirm a path is a Power BI semantic
/// model project root before walking its sibling `definition/` directory for
/// TMDL extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbismDescriptor {
    /// Project-definition layout version, e.g. `"4.0"`.
    pub version: String,
}

/// Parse the JSON content of a `.pbism` descriptor file.
///
/// Returns `None` when the content is not valid JSON, not an object, or is
/// missing the required `version` string field. The intent is to be strict at
/// the boundary: ingestion treats absence as "this is not a `.pbism`" rather
/// than coercing partial structure.
#[must_use]
pub fn parse_pbism(content: &str) -> Option<PbismDescriptor> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let version = value.get("version").and_then(serde_json::Value::as_str)?;
    Some(PbismDescriptor {
        version: version.to_string(),
    })
}
