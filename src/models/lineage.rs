//! Shared data-lineage value types (095-F, Unit U1a).
//!
//! These are the cross-cutting carriers every lineage emitter and writer shares,
//! defined once here so no unit invents its own shape:
//!
//! * [`DatasetKind`] — `table` | `path` (no `view` in v1).
//! * [`LineageEndpoint`] — a resolved, authority-bound dataset reference.
//! * [`LineageEdgeCandidate`] — a statement-grouped, directional edge carrier
//!   (one target derives from N sources) produced by the Python (U2b) and SQL
//!   (U3) paths and consumed/flattened by the notebook write path (U4).
//! * [`LineageEvidence`] — a per-(edge, notebook, cell) provenance row.
//! * [`LineageAuthorityContext`] — the trusted-authority handle the U2/U3
//!   extractors consume to bind a literal to a canonical, authority-embedded id
//!   (fail-closed on any unresolvable authority; AR-01).
//!
//! All resolution fails **closed** (013-D / A5 precision floor): an unmapped
//! catalog, a 1-/2-part name, a relative literal, or an untrusted storage
//! authority yields `None` — never a guessed endpoint.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The single v1 lineage edge type (AR-05).
///
/// Oriented `from_id` = the written target dataset → `to_id` = the read source
/// dataset: data flows source→target, but the edge encodes derives-from (the
/// target derives from the source).
pub const LINEAGE_DERIVES_FROM: &str = "lineage_derives_from";

/// The kind of a resolved lineage dataset endpoint.
///
/// v1 recognizes tables and paths only. Permanent (catalog) views are recorded
/// as [`DatasetKind::Table`] because v1 has no signal to distinguish them from a
/// table (both are a 3-part name + authority); temp views are unrepresentable
/// and fail closed (they never reach this type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DatasetKind {
    /// A 3-part `catalog.schema.table` bound to a trusted metastore authority.
    Table,
    /// An absolute URI bound to a trusted storage authority.
    Path,
}

impl DatasetKind {
    /// Return the canonical persisted string for this kind (`"table"`/`"path"`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DatasetKind::Table => "table",
            DatasetKind::Path => "path",
        }
    }
}

/// A resolved, authority-bound dataset reference.
///
/// The `id` is the canonical key that **embeds** the resolved authority, so two
/// distinct metastores sharing the same `catalog.schema.table` produce distinct
/// endpoints and never collide (AR-01).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineageEndpoint {
    /// Canonical, authority-embedded dataset id (the `dataset_node.id` key).
    pub id: String,
    /// Human-readable dataset name (the 3-part table name or the absolute URI).
    pub name: String,
    /// Whether this endpoint is a table or a path.
    pub kind: DatasetKind,
}

/// A statement-grouped, directional lineage edge candidate.
///
/// Each candidate pairs one written/`target` endpoint with the set of read
/// `sources` for a single statement or dataflow, so a CTAS with several source
/// tables — or several statements in one cell — keeps its target↔source pairing
/// (a flat endpoint list would lose which target each source belongs to;
/// cycle-5 F3/F4). The notebook write path (U4) flattens each candidate to one
/// directional `lineage_derives_from` edge per `(target, source)` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageEdgeCandidate {
    /// The written dataset — the `from_id` of every emitted derives-from edge.
    pub target: LineageEndpoint,
    /// The read datasets — one `to_id` per emitted derives-from edge.
    pub sources: Vec<LineageEndpoint>,
}

/// A per-(edge, notebook, cell) lineage provenance observation.
///
/// One row is written per observation; `chunk_index` is part of the evidence key
/// so the same edge seen in two cells of one notebook yields two rows (cycle-4
/// E1 / AR-20). Notebook-scope deletion removes every row matching
/// `notebook_path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageEvidence {
    /// The written target dataset id (the edge `from_id`).
    pub from_id: String,
    /// The read source dataset id (the edge `to_id`).
    pub to_id: String,
    /// Workspace-relative notebook path this observation came from.
    pub notebook_path: String,
    /// One-based source-order cell ordinal within the notebook.
    pub chunk_index: u32,
    /// Content hash of the observing notebook (provenance/freshness aid).
    pub content_hash: String,
}

/// The trusted-authority handle the U2/U3 extractors consume.
///
/// Carries the catalog→authority mapping (each configured catalog maps to a
/// stable metastore authority id) and the trusted storage-authority allowlist.
/// Construction from engram config + live propagation into the indexer is U1b.
/// An **empty** context resolves nothing, so an absent/empty authority config
/// makes every table/path identity unresolved and no edge is emitted
/// (fail-closed; 013-D, A5 precision floor).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineageAuthorityContext {
    /// Catalog name → stable metastore authority id. A catalog absent from this
    /// map is **unmapped** and fails closed (AR-01).
    catalog_authority: BTreeMap<String, String>,
    /// Trusted storage-authority prefixes (e.g. `s3://bucket`,
    /// `abfss://container@account.dfs.core.windows.net`). A path whose
    /// `scheme://authority` matches none of these fails closed.
    storage_authorities: Vec<String>,
}

impl LineageAuthorityContext {
    /// Build a context from a catalog→authority mapping and a storage allowlist.
    #[must_use]
    pub fn new(
        catalog_authority: BTreeMap<String, String>,
        storage_authorities: Vec<String>,
    ) -> Self {
        Self {
            catalog_authority,
            storage_authorities,
        }
    }

    /// An empty context that resolves nothing (the fail-closed default).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return `true` when no authority is configured (resolves nothing).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.catalog_authority.is_empty() && self.storage_authorities.is_empty()
    }

    /// Look up the stable authority id bound to `catalog`, if mapped.
    #[must_use]
    pub fn catalog_authority_id(&self, catalog: &str) -> Option<&str> {
        self.catalog_authority.get(catalog).map(String::as_str)
    }

    /// Resolve a 3-part `catalog.schema.table` literal to a canonical,
    /// authority-embedded [`LineageEndpoint`], or `None` (fail closed).
    ///
    /// Returns `None` for anything that is not exactly three non-empty
    /// dot-separated parts, or whose catalog is not mapped to a trusted
    /// metastore authority (AR-01). The resolved authority id is embedded in the
    /// `id` so two metastores sharing `catalog.schema.table` never collide.
    #[must_use]
    pub fn resolve_table(&self, literal: &str) -> Option<LineageEndpoint> {
        let parts: Vec<&str> = literal.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
            return None;
        }
        let catalog = parts[0];
        let authority = self.catalog_authority.get(catalog)?;
        let name = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
        let id = format!("table::{authority}::{name}");
        Some(LineageEndpoint {
            id,
            name,
            kind: DatasetKind::Table,
        })
    }

    /// Resolve an absolute storage URI to a canonical [`LineageEndpoint`], or
    /// `None` (fail closed).
    ///
    /// Returns `None` unless `literal` is an absolute `scheme://authority/...`
    /// URI whose `scheme://authority` matches a trusted storage-authority prefix
    /// in the allowlist. Relative literals, config/widget-derived values, and
    /// untrusted authorities all fail closed.
    #[must_use]
    pub fn resolve_path(&self, literal: &str) -> Option<LineageEndpoint> {
        // Must be an absolute URI: `scheme://authority[/path]`.
        let scheme_end = literal.find("://")?;
        if scheme_end == 0 {
            return None;
        }
        let trusted = self
            .storage_authorities
            .iter()
            .any(|prefix| uri_matches_authority(literal, prefix));
        if !trusted {
            return None;
        }
        Some(LineageEndpoint {
            id: format!("path::{literal}"),
            name: literal.to_owned(),
            kind: DatasetKind::Path,
        })
    }
}

/// Return `true` when `uri` sits under the trusted `prefix` authority.
///
/// The URI must either equal the prefix exactly or continue with a path
/// separator, so `s3://bucket` never matches `s3://bucket-other/...`.
fn uri_matches_authority(uri: &str, prefix: &str) -> bool {
    if !uri.starts_with(prefix) {
        return false;
    }
    match uri.as_bytes().get(prefix.len()) {
        None | Some(b'/') => true,
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> LineageAuthorityContext {
        let mut catalogs = BTreeMap::new();
        catalogs.insert("cat".to_owned(), "prod-metastore".to_owned());
        LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
    }

    #[test]
    fn dataset_kind_persisted_strings() {
        assert_eq!(DatasetKind::Table.as_str(), "table");
        assert_eq!(DatasetKind::Path.as_str(), "path");
    }

    #[test]
    fn endpoint_and_candidate_construct_with_target_source_grouping() {
        let target = LineageEndpoint {
            id: "table::a::c.s.t".to_owned(),
            name: "c.s.t".to_owned(),
            kind: DatasetKind::Table,
        };
        let source = LineageEndpoint {
            id: "table::a::c.s.src".to_owned(),
            name: "c.s.src".to_owned(),
            kind: DatasetKind::Table,
        };
        let candidate = LineageEdgeCandidate {
            target: target.clone(),
            sources: vec![source.clone()],
        };
        assert_eq!(candidate.target, target);
        assert_eq!(candidate.sources, vec![source]);
    }

    #[test]
    fn empty_context_resolves_nothing() {
        let empty = LineageAuthorityContext::empty();
        assert!(empty.is_empty());
        assert!(empty.resolve_table("cat.sch.t").is_none());
        assert!(empty.resolve_path("s3://bucket/p").is_none());
    }

    #[test]
    fn context_carries_catalog_authority_mapping() {
        let c = ctx();
        assert!(!c.is_empty());
        assert_eq!(c.catalog_authority_id("cat"), Some("prod-metastore"));
        assert_eq!(c.catalog_authority_id("other"), None);
    }
}
