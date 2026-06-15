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

use crate::models::pbip::{PbipReportLink, PbipWorkspace};
use crate::services::powerbi_extract::synthetic_id;

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

// ── Workspace entry (.pbip) ────────────────────────────────────────────────

/// Parse a `.pbip` workspace entry file into a [`PbipWorkspace`].
///
/// The `.pbip` JSON enumerates report artifacts under `artifacts[].report.path`.
/// `pbip_path` is the workspace-relative path to the `.pbip` file and is used
/// to derive the stable workspace ID.
///
/// Returns `None` when the content is not valid JSON, lacks an `artifacts`
/// array, or resolves no report artifact paths. Absence of a resolvable report
/// is treated as "this is not an indexable workspace entry" rather than
/// emitting an empty workspace.
#[must_use]
pub fn parse_pbip_workspace(content: &str, pbip_path: &str) -> Option<PbipWorkspace> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let artifacts = value
        .get("artifacts")
        .and_then(serde_json::Value::as_array)?;

    let report_paths: Vec<String> = artifacts
        .iter()
        .filter_map(|artifact| {
            artifact
                .pointer("/report/path")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect();

    if report_paths.is_empty() {
        return None;
    }

    Some(PbipWorkspace {
        id: synthetic_id(&format!("pbip_workspace:{pbip_path}")),
        path: pbip_path.to_string(),
        report_paths,
    })
}

// ── Report linkage (.pbir) ─────────────────────────────────────────────────

/// Parse a `.pbir` report descriptor into a [`PbipReportLink`].
///
/// The `.pbir` JSON declares a `datasetReference` that either points at the
/// semantic model by relative path (`byPath.path`) or by connection string
/// (`byConnection`). `pbir_path` is the workspace-relative path to the `.pbir`
/// file; its parent directory is the report folder.
///
/// Returns `None` when the content is not valid JSON or lacks a
/// `datasetReference` (and is therefore not a recognisable report descriptor).
/// A `byConnection`-only reference still yields a link, with
/// `semantic_model_path` set to `None`.
#[must_use]
pub fn parse_pbir_link(content: &str, pbir_path: &str) -> Option<PbipReportLink> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let dataset_ref = value.get("datasetReference")?;

    let report_path = report_folder_from_pbir(pbir_path);

    let semantic_model_path = dataset_ref
        .pointer("/byPath/path")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| resolve_relative(&report_path, raw));

    Some(PbipReportLink {
        id: synthetic_id(&format!("pbip_report_link:{pbir_path}")),
        path: pbir_path.to_string(),
        report_path,
        semantic_model_path,
    })
}

/// Return the report folder for a `.pbir`: the parent directory of the file,
/// normalised to forward slashes.
fn report_folder_from_pbir(pbir_path: &str) -> String {
    std::path::Path::new(pbir_path)
        .parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or("")
        .replace('\\', "/")
}

/// Resolve a `..`-relative path against a base directory, collapsing `.` and
/// `..` segments. Both inputs use either path separator. Returns `None` when
/// the relative path escapes above the base (which would leave the workspace
/// root) so a poisoned descriptor cannot reference outside the workspace.
fn resolve_relative(base_dir: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = base_dir
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();

    for segment in rel.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }

    Some(parts.join("/"))
}
