//! PBIP content indexer for the Power BI project-definition layout.
//!
//! Dedicated source boundary for `.pbip` workspaces that use the newer
//! project-definition contract (split `.pbip` / `.pbir` / `.pbism` descriptors,
//! per-report/page/visual JSON under `definition/`, and `definition/**/*.tmdl`).
//!
//! This module is intentionally kept distinct from [`crate::services::powerbi_indexer`]
//! so the legacy JSON/BIM-backed `powerbi` source path remains stable while
//! the dedicated `pbip` path is built out across 062-F.
//!
//! # Build-out status
//!
//! * 062.001-T introduced the dispatch boundary so [`crate::services::ingestion`]
//!   routes `pbip` sources here.
//! * 062.003-T added the pure extractors in
//!   [`crate::services::pbip_extract`] and
//!   [`crate::services::pbip_tmdl`].
//! * 062.004-T introduced the [`collect_pbip_files`] walker and the
//!   [`compute_deleted_paths`] deletion-sweep helper.
//! * **062.002-T (this task)** wires the walker, extractors, and TMDL model
//!   merge into [`index_pbip_source`]: it assembles each `.pbip` project
//!   (workspace → reports → pages → visuals, plus the linked semantic model),
//!   emits object-level `content_type = "pbip"` content records, and builds the
//!   project graph (report→page→visual `contains`, report→model
//!   `depends_on_model`, the reused model subgraph, and visual→measure/column
//!   `uses_field` edges). Change detection re-indexes the whole source whenever
//!   any collected file's hash changes, and [`sweep_deleted_pbip_files`] prunes
//!   records and graph nodes for files removed from disk.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::EngramError;
use crate::models::content::ContentRecord;
use crate::models::pbip::{PbipBindingKind, PbipIndexResult};
use crate::models::powerbi_graph::{PowerBiEdge, PowerBiEdgeType, PowerBiNode, PowerBiNodeKind};
use crate::models::registry::ContentSource;
use crate::services::ingestion::{compute_hash, content_record_identity_seed};
use crate::services::pbip_extract::{
    parse_page, parse_page_order, parse_pbip_workspace, parse_pbir_link, parse_visual,
};
use crate::services::pbip_tmdl::merge_semantic_model_fragments;
use crate::services::powerbi_indexer::{
    build_powerbi_graph_data_from_model, compute_file_hash, extract_model_summaries_from_model,
    make_node_id,
};
use crate::services::source_traversal::collect_files_in_workspace;

/// File extensions that belong to the PBIP project-definition layout.
///
/// Mirrors the spike's enumeration: workspace entry (`.pbip`), per-artifact
/// descriptors (`.pbir`, `.pbism`), project-definition JSON under
/// `<Artifact>/definition/`, and TMDL semantic-model files under
/// `<SemanticModel>/definition/`. Matching is case-insensitive.
const PBIP_EXTENSIONS: &[&str] = &["pbip", "pbir", "pbism", "json", "tmdl"];

// ── File collection ───────────────────────────────────────────────────────

/// Collect all PBIP project-definition files under `dir` recursively.
///
/// Returns a sorted list of absolute paths to files whose extension matches
/// the PBIP project-definition layout: `.pbip`, `.pbir`, `.pbism`, `.json`,
/// or `.tmdl`. Files with other extensions are ignored even when they sit
/// alongside PBIP files.
///
/// Directory symlinks are followed only when their canonical target remains
/// within the traversal root; file symlinks are skipped. Use
/// [`collect_pbip_files_in_workspace`] when the source directory is narrower
/// than the workspace root and legitimate symlinked source directories may live
/// elsewhere inside the same workspace.
///
/// Returns an empty list if `dir` does not exist, escapes the traversal root, or
/// cannot be read.
#[must_use]
pub fn collect_pbip_files(dir: &Path) -> Vec<PathBuf> {
    collect_pbip_files_in_workspace(dir, dir)
}

/// Collect PBIP files under `dir`, traversing only symlinked directories whose
/// canonical target remains under `workspace_root`.
#[must_use]
pub fn collect_pbip_files_in_workspace(dir: &Path, workspace_root: &Path) -> Vec<PathBuf> {
    collect_files_in_workspace(dir, workspace_root, is_pbip_file)
}

fn is_pbip_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            PBIP_EXTENSIONS
                .iter()
                .any(|target| extension.eq_ignore_ascii_case(target))
        })
}

// ── Deletion sweep ────────────────────────────────────────────────────────

/// Return the subset of `workspace_relative_paths` whose files no longer
/// exist under `workspace_root`.
///
/// Each path is joined to `workspace_root` before the existence check so
/// that workspace-relative record paths (as stored in
/// `ContentRecord.file_path`) are handled correctly. Paths that attempt to
/// escape the workspace (absolute paths, `..` segments, Windows drive
/// prefixes) are rejected with a `warn!` log entry and excluded from the
/// returned set so a poisoned record cannot probe outside the workspace.
#[must_use]
pub fn compute_deleted_paths(
    workspace_relative_paths: &[String],
    workspace_root: &Path,
) -> Vec<String> {
    let Ok(canonical_root) = workspace_root.canonicalize() else {
        return workspace_relative_paths.to_vec();
    };

    workspace_relative_paths
        .iter()
        .filter_map(|rel| {
            let Some(relative_path) = workspace_relative_path(rel) else {
                warn!(
                    path = %rel,
                    "skipping PBIP deletion sweep path that escapes the workspace root"
                );
                return None;
            };
            let candidate = workspace_root.join(relative_path);
            let is_deleted = candidate
                .canonicalize()
                .map_or(true, |canonical| !canonical.starts_with(&canonical_root));
            is_deleted.then(|| rel.clone())
        })
        .collect()
}

fn workspace_relative_path(rel_path: &str) -> Option<PathBuf> {
    let path = Path::new(rel_path);
    // `has_root()` catches both Windows-style absolute paths (`C:\...`) and
    // forward-slash absolute paths (`/etc/passwd`) on Windows, where
    // `is_absolute()` requires a drive prefix and would otherwise let
    // `/etc/passwd` slip through.
    if path.is_absolute()
        || path.has_root()
        || path.components().any(|component| {
            component == Component::ParentDir || matches!(component, Component::Prefix(_))
        })
    {
        return None;
    }
    Some(path.to_path_buf())
}

// ── Async indexer ─────────────────────────────────────────────────────────

/// In-memory snapshot of a single collected PBIP file.
#[derive(Debug, Clone)]
struct FileData {
    /// SHA-256 hash of the file bytes, for change detection.
    hash: String,
    /// UTF-8 file contents.
    content: String,
    /// File size in bytes, recorded on emitted content records.
    size: u64,
}

/// One object-level content record emitted for a PBIP project, before it is
/// persisted to the `content_record` relation.
#[derive(Debug, Clone)]
struct PbipRecord {
    /// `record_kind` discriminator, e.g. `pbip_report`, `pbip_page`.
    record_kind: String,
    /// Object display name (report name, page name, measure name, ...).
    object_name: String,
    /// Parent context string for disambiguation and search snippets.
    parent_context: String,
    /// Human-readable summary stored as the searchable record body.
    content_text: String,
    /// Workspace-relative path of the collected file the record attaches to.
    file_path: String,
}

/// The full set of entities emitted for a PBIP source: per-object content
/// records plus the project graph (nodes and edges).
#[derive(Debug, Default)]
struct PbipEmission {
    records: Vec<PbipRecord>,
    nodes: Vec<PowerBiNode>,
    edges: Vec<PowerBiEdge>,
}

/// Stateful assembler that walks a PBIP project in memory and accumulates the
/// [`PbipEmission`]. Holds the collected file snapshot plus the workspace root
/// (needed to read TMDL semantic-model definitions from disk).
struct EmissionBuilder<'a> {
    file_data: &'a BTreeMap<String, FileData>,
    source_path: &'a str,
    workspace_root: &'a Path,
    now: DateTime<Utc>,
    emission: PbipEmission,
    /// IDs of nodes already pushed, so dangling `uses_field` edges are skipped.
    node_ids: HashSet<String>,
}

impl<'a> EmissionBuilder<'a> {
    fn new(
        file_data: &'a BTreeMap<String, FileData>,
        source_path: &'a str,
        workspace_root: &'a Path,
    ) -> Self {
        Self {
            file_data,
            source_path,
            workspace_root,
            now: Utc::now(),
            emission: PbipEmission::default(),
            node_ids: HashSet::new(),
        }
    }

    fn push_node(&mut self, node: PowerBiNode) {
        self.node_ids.insert(node.id.clone());
        self.emission.nodes.push(node);
    }

    fn push_edge(&mut self, from_id: String, to_id: String, edge_type: PowerBiEdgeType) {
        self.emission.edges.push(PowerBiEdge {
            from_id,
            to_id,
            edge_type,
            source_path: self.source_path.to_owned(),
        });
    }

    fn push_record(
        &mut self,
        record_kind: &str,
        object_name: &str,
        parent_context: &str,
        content_text: String,
        file_path: &str,
    ) {
        self.emission.records.push(PbipRecord {
            record_kind: record_kind.to_owned(),
            object_name: object_name.to_owned(),
            parent_context: parent_context.to_owned(),
            content_text,
            file_path: file_path.to_owned(),
        });
    }

    /// Walk every `.pbip` workspace entry and assemble its reports.
    fn build(mut self) -> PbipEmission {
        let workspace_files: Vec<(String, FileData)> = self
            .file_data
            .iter()
            .filter(|(path, _)| has_extension(path, "pbip"))
            .map(|(path, data)| (path.clone(), data.clone()))
            .collect();

        for (pbip_path, data) in workspace_files {
            let Some(workspace) = parse_pbip_workspace(&data.content, &pbip_path) else {
                continue;
            };

            self.push_record(
                "pbip_workspace",
                &file_stem(&pbip_path),
                &pbip_path,
                format!(
                    "PBIP workspace entry. Reports: {}.",
                    workspace.report_paths.join(", ")
                ),
                &pbip_path,
            );

            let pbip_dir = parent_dir(&pbip_path);
            for report_rel in &workspace.report_paths {
                if let Some(report_folder) = join_relative(&pbip_dir, report_rel) {
                    self.build_report(&report_folder);
                }
            }
        }

        self.emission
    }

    /// Assemble a single report folder: model link, report/page/visual nodes
    /// and the edges between them.
    fn build_report(&mut self, report_folder: &str) {
        let report_name = report_display_name(report_folder);
        let report_file = format!("{report_folder}/definition/report.json");
        let pbir_file = format!("{report_folder}/definition.pbir");
        let pages_file = format!("{report_folder}/definition/pages/pages.json");

        // Resolve the linked semantic model (if any) first, so report/visual
        // edges can target its nodes.
        let mut model_node_id: Option<String> = None;
        let mut model_scope: Option<String> = None;
        if let Some(data) = self.file_data.get(&pbir_file) {
            if let Some(link) = parse_pbir_link(&data.content, &pbir_file) {
                self.push_record(
                    "pbip_report_link",
                    &report_name,
                    report_folder,
                    match &link.semantic_model_path {
                        Some(model) => {
                            format!("Report {report_name} links to semantic model {model}.")
                        }
                        None => {
                            format!("Report {report_name} links to a semantic model by connection.")
                        }
                    },
                    &pbir_file,
                );
                if let Some(model_folder) = link.semantic_model_path.clone() {
                    if let Some((id, scope)) = self.build_model(&model_folder) {
                        model_node_id = Some(id);
                        model_scope = Some(scope);
                    }
                }
            }
        }

        // Report node. Anchor its file_path to a real collected descriptor:
        // `report.json` when present, otherwise the `.pbir` descriptor. Skip the
        // report entirely when neither descriptor was collected so we never emit
        // a graph node pointing at a non-existent file with an empty hash.
        let report_anchor = if self.file_data.contains_key(&report_file) {
            report_file.clone()
        } else if self.file_data.contains_key(&pbir_file) {
            pbir_file.clone()
        } else {
            return;
        };
        let report_hash = self
            .file_data
            .get(&report_anchor)
            .map(|d| d.hash.clone())
            .unwrap_or_default();
        let report_id = make_node_id(
            self.source_path,
            report_folder,
            PowerBiNodeKind::Report,
            &report_name,
        );
        self.push_node(PowerBiNode {
            id: report_id.clone(),
            name: report_name.clone(),
            kind: PowerBiNodeKind::Report,
            file_path: report_anchor,
            source_path: self.source_path.to_owned(),
            content_hash: report_hash,
            ingested_at: self.now,
        });
        if self.file_data.contains_key(&report_file) {
            self.push_record(
                "pbip_report",
                &report_name,
                report_folder,
                format!("Power BI report {report_name}."),
                &report_file,
            );
        }
        if let Some(model_id) = &model_node_id {
            self.push_edge(
                report_id.clone(),
                model_id.clone(),
                PowerBiEdgeType::DependsOnModel,
            );
        }

        // Pages, ordered by pages.json. Emit a page-order record only when
        // pages.json actually parses. When it exists but cannot be parsed, fall
        // through to the generic `pbip_file` coverage record (added by the
        // ensure-coverage pass) instead of emitting a misleading
        // "Page order …: ." record anchored to an unparseable file.
        let page_order = self
            .file_data
            .get(&pages_file)
            .and_then(|d| parse_page_order(&d.content));
        if let Some(order) = page_order.as_ref() {
            self.push_record(
                "pbip_page_order",
                &report_name,
                report_folder,
                format!(
                    "Page order for report {report_name}: {}.",
                    order.order.join(", ")
                ),
                &pages_file,
            );
        }

        let order = page_order.map(|o| o.order).unwrap_or_default();
        for page_logical in &order {
            self.build_page(
                report_folder,
                &report_name,
                &report_id,
                page_logical,
                model_scope.as_deref(),
            );
        }
    }

    /// Assemble a single page and its visuals under a report.
    fn build_page(
        &mut self,
        report_folder: &str,
        report_name: &str,
        report_id: &str,
        page_logical: &str,
        model_scope: Option<&str>,
    ) {
        let page_file = format!("{report_folder}/definition/pages/{page_logical}/page.json");
        let Some(data) = self.file_data.get(&page_file).cloned() else {
            return;
        };
        let Some(page) = parse_page(&data.content, &page_file) else {
            return;
        };

        let page_id = make_node_id(
            self.source_path,
            report_folder,
            PowerBiNodeKind::Page,
            &format!("{report_name}/{}", page.name),
        );
        self.push_node(PowerBiNode {
            id: page_id.clone(),
            name: page.display_name.clone(),
            kind: PowerBiNodeKind::Page,
            file_path: page_file.clone(),
            source_path: self.source_path.to_owned(),
            content_hash: data.hash.clone(),
            ingested_at: self.now,
        });
        self.push_edge(
            report_id.to_owned(),
            page_id.clone(),
            PowerBiEdgeType::Contains,
        );
        self.push_record(
            "pbip_page",
            &page.name,
            report_name,
            format!(
                "Page {} ({}) in report {report_name}.",
                page.name, page.display_name
            ),
            &page_file,
        );

        // Visuals under this page (any depth beneath the `visuals/` folder).
        let visuals_prefix = format!("{report_folder}/definition/pages/{page_logical}/visuals/");
        let visual_files: Vec<(String, FileData)> = self
            .file_data
            .iter()
            .filter(|(path, _)| path.starts_with(&visuals_prefix) && path.ends_with("/visual.json"))
            .map(|(path, value)| (path.clone(), value.clone()))
            .collect();
        for (visual_file, visual_data) in visual_files {
            let Some(visual) = parse_visual(&visual_data.content, &visual_file) else {
                continue;
            };
            let visual_id = make_node_id(
                self.source_path,
                report_folder,
                PowerBiNodeKind::Visual,
                &format!("{report_name}/{}/{}", page.name, visual.name),
            );
            self.push_node(PowerBiNode {
                id: visual_id.clone(),
                name: visual.name.clone(),
                kind: PowerBiNodeKind::Visual,
                file_path: visual_file.clone(),
                source_path: self.source_path.to_owned(),
                content_hash: visual_data.hash.clone(),
                ingested_at: self.now,
            });
            self.push_edge(
                page_id.clone(),
                visual_id.clone(),
                PowerBiEdgeType::Contains,
            );

            // Visual → measure/column `uses_field` edges, only when the target
            // model node actually exists (avoids dangling edges).
            if let Some(scope) = model_scope {
                for binding in &visual.bindings {
                    let kind = match binding.kind {
                        PbipBindingKind::Measure => PowerBiNodeKind::Measure,
                        PbipBindingKind::Column => PowerBiNodeKind::Column,
                    };
                    let target = make_node_id(
                        self.source_path,
                        scope,
                        kind,
                        &format!("{}.{}", binding.entity, binding.property),
                    );
                    if self.node_ids.contains(&target) {
                        self.push_edge(visual_id.clone(), target, PowerBiEdgeType::UsesField);
                    }
                }
            }

            let binding_hint = if visual.bindings.is_empty() {
                String::new()
            } else {
                let fields: Vec<String> = visual
                    .bindings
                    .iter()
                    .map(|b| format!("{}.{}", b.entity, b.property))
                    .collect();
                format!(" Bindings: {}.", fields.join(", "))
            };
            self.push_record(
                "pbip_visual",
                &visual.name,
                &format!("{report_name}/{}", page.name),
                format!(
                    "Visual {} of type {} on page {}.{binding_hint}",
                    visual.name, visual.visual_type, page.name
                ),
                &visual_file,
            );
        }
    }

    /// Merge and emit a semantic model from its `definition/` TMDL folder.
    ///
    /// Returns the model's `SemanticModel` node ID and the workspace-relative
    /// identity scope used for all of its subgraph node IDs, so callers can
    /// link reports and visuals to it. Returns `None` when no model can be
    /// extracted from disk.
    fn build_model(&mut self, model_folder: &str) -> Option<(String, String)> {
        let definition_rel = format!("{model_folder}/definition");
        // Source the semantic model from the already-filtered file snapshot
        // (max_file_size cap + UTF-8 filtering) instead of re-reading the
        // `definition/` tree from disk. Reading from disk bypassed the
        // ingestion size cap and UTF-8 filtering, so it could index files the
        // snapshot intentionally skipped and read unbounded data; sourcing from
        // the snapshot keeps extraction consistent with change detection and
        // coverage.
        let tmdl_prefix = format!("{definition_rel}/");
        // Borrow each TMDL fragment's body (`&str`) straight from the snapshot
        // instead of cloning it into `fragments`. The same content already
        // lives in `self.file_data`, so cloning doubled peak memory during a
        // rebuild of a large semantic model. Only the absolute path string is
        // owned — a small per-file allocation that `merge_semantic_model_fragments`
        // needs to derive the model identity — while the TMDL bodies stay
        // zero-copy references into the snapshot.
        let mut fragments: Vec<(String, &str)> = self
            .file_data
            .iter()
            .filter(|(path, _)| path.starts_with(&tmdl_prefix) && has_extension(path, "tmdl"))
            .map(|(rel, data)| {
                (
                    self.workspace_root.join(rel).to_string_lossy().into_owned(),
                    data.content.as_str(),
                )
            })
            .collect();
        fragments.sort_by(|a, b| a.0.cmp(&b.0));
        let model = merge_semantic_model_fragments(
            fragments
                .iter()
                .map(|(path, content)| (path.as_str(), *content)),
        )?;

        // Anchor model content records and the node file_path to a real
        // collected file: the `.pbism` descriptor, else the first TMDL file.
        let pbism_rel = format!("{model_folder}/definition.pbism");
        let anchor = if self.file_data.contains_key(&pbism_rel) {
            pbism_rel.clone()
        } else {
            self.file_data
                .keys()
                .find(|path| {
                    path.starts_with(&format!("{definition_rel}/")) && has_extension(path, "tmdl")
                })
                .cloned()
                .unwrap_or_else(|| pbism_rel.clone())
        };
        let anchor_hash = self
            .file_data
            .get(&anchor)
            .map(|d| d.hash.clone())
            .unwrap_or_default();

        // Workspace-relative scope keeps table/column/measure node IDs stable
        // across runs even though `model.id` is derived from absolute paths.
        let scope = definition_rel;
        let (nodes, edges) = build_powerbi_graph_data_from_model(
            &model,
            &scope,
            &anchor,
            self.source_path,
            &anchor_hash,
            None,
        );
        for node in nodes {
            self.push_node(node);
        }
        self.emission.edges.extend(edges);

        let model_node_id = make_node_id(
            self.source_path,
            &scope,
            PowerBiNodeKind::SemanticModel,
            &model.id,
        );

        if self.file_data.contains_key(&pbism_rel) {
            self.push_record(
                "pbip_semantic_model_descriptor",
                &model.name,
                model_folder,
                format!("Semantic model descriptor for {}.", model.name),
                &pbism_rel,
            );
        }

        for (object_kind, object_name, parent_context, content_text) in
            extract_model_summaries_from_model(&model)
        {
            let pbip_kind = object_kind.replacen("powerbi_", "pbip_", 1);
            self.push_record(
                &pbip_kind,
                &object_name,
                &parent_context,
                content_text,
                &anchor,
            );
        }

        Some((model_node_id, scope))
    }
}

/// Index all PBIP project-definition files from a single content source.
///
/// Walks the source with [`collect_pbip_files`], hashes each collected file,
/// and compares the per-file hash set against the existing `content_type =
/// "pbip"` records for change detection. Because PBIP is inherently cross-file
/// (a changed visual reshapes the report graph), any hash change triggers a
/// full rebuild of the source: existing records and graph nodes are deleted,
/// then the project is re-assembled into object-level content records and a
/// report→page→visual / report→model / model-subgraph / visual→field graph.
///
/// # Errors
///
/// Returns `Err` when an underlying database query fails unrecoverably.
pub async fn index_pbip_source(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
    max_file_size: u64,
) -> Result<PbipIndexResult, EngramError> {
    let mut result = PbipIndexResult::default();

    let source_dir = workspace_root.join(&source.path);
    if !source_dir.exists() {
        debug!(
            path = %source.path,
            "PBIP source directory does not exist — skipping"
        );
        return Ok(result);
    }

    let files = collect_pbip_files_in_workspace(&source_dir, workspace_root);
    result.total_files = files.len();

    // Snapshot every collected, in-bounds, UTF-8 file.
    let mut file_data: BTreeMap<String, FileData> = BTreeMap::new();
    for path in &files {
        let Ok(metadata) = path.metadata() else {
            continue;
        };
        if metadata.len() > max_file_size {
            debug!(path = %path.display(), "PBIP file exceeds max_file_size — skipping");
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let Some(rel_path) = snapshot_relative_path(path, workspace_root) else {
            warn!(
                path = %path.display(),
                "skipping PBIP file that cannot be made workspace-relative"
            );
            continue;
        };
        file_data.insert(
            rel_path,
            FileData {
                hash: compute_file_hash(&bytes),
                content: text.to_owned(),
                size: metadata.len(),
            },
        );
    }

    // Existing per-file hashes for this source, for change detection.
    let existing: HashMap<String, String> = queries
        .select_content_records(Some("pbip"))
        .await?
        .into_iter()
        .filter(|record| record.source_path == source.path)
        .map(|record| (record.file_path, record.content_hash))
        .collect();

    let current: HashMap<String, String> = file_data
        .iter()
        .map(|(path, data)| (path.clone(), data.hash.clone()))
        .collect();

    if current == existing {
        // `unchanged` counts files that were actually considered for hashing —
        // i.e. the snapshotted set (`file_data`), not `total_files`, which also
        // includes files skipped during snapshotting (oversized / non-UTF-8 /
        // unreadable). This keeps the field consistent with its doc comment and
        // with the other indexers.
        result.unchanged = file_data.len();
        debug!(
            path = %source.path,
            considered = result.unchanged,
            total_files = result.total_files,
            "PBIP source unchanged — skipping re-index"
        );
        return Ok(result);
    }

    // Full rebuild: clear existing records and graph for this source. Scope the
    // graph deletion to the PBIP-owned file paths (every PBIP node anchors to a
    // collected file that carries a `pbip` content record) rather than deleting
    // every `powerbi_node` for the registry path. A legacy `powerbi` source
    // registered at the same path writes nodes under the same `source_path`, so
    // a blanket source-scoped delete would erase that independent legacy graph.
    for path in existing.keys() {
        queries
            .delete_content_records_by_scope(path, "pbip", &source.path)
            .await?;
        queries.delete_powerbi_nodes_by_file_path(path).await?;
    }

    let emission = EmissionBuilder::new(&file_data, &source.path, workspace_root).build();

    // Persist object-level records, tracking which files are covered.
    let mut covered: HashSet<String> = HashSet::new();
    for record in &emission.records {
        let Some(data) = file_data.get(&record.file_path) else {
            continue;
        };
        let chunk_id = format!("{}:{}", record.object_name, record.record_kind);
        let identity_seed = content_record_identity_seed(
            &record.file_path,
            "pbip",
            &source.path,
            Some(&format!("{}:{chunk_id}", record.parent_context)),
        );
        let row = ContentRecord {
            id: format!("cr_{}", compute_hash(identity_seed.as_bytes())),
            content_type: "pbip".to_string(),
            file_path: record.file_path.clone(),
            content_hash: data.hash.clone(),
            content: format!(
                "Kind: {}. Name: {}. Context: {}. {}",
                record.record_kind, record.object_name, record.parent_context, record.content_text
            ),
            embedding: None,
            source_path: source.path.clone(),
            file_size_bytes: data.size,
            ingested_at: Utc::now(),
            record_kind: record.record_kind.clone(),
            chunk_id: Some(chunk_id),
            chunk_index: None,
            heading_path: Vec::new(),
            line_start: None,
            line_end: None,
            fallback_reason: None,
            lint_summary: None,
            suggestions: Vec::new(),
        };
        queries.upsert_content_record(&row).await?;
        covered.insert(record.file_path.clone());
    }

    // Ensure-coverage fallback: every collected file must own at least one
    // record so the next run's change-detection set matches the disk set.
    for (rel_path, data) in &file_data {
        if covered.contains(rel_path) {
            continue;
        }
        let identity_seed =
            content_record_identity_seed(rel_path, "pbip", &source.path, Some("file"));
        let row = ContentRecord {
            id: format!("cr_{}", compute_hash(identity_seed.as_bytes())),
            content_type: "pbip".to_string(),
            file_path: rel_path.clone(),
            content_hash: data.hash.clone(),
            content: format!("PBIP project-definition file {rel_path}."),
            embedding: None,
            source_path: source.path.clone(),
            file_size_bytes: data.size,
            ingested_at: Utc::now(),
            record_kind: "pbip_file".to_string(),
            chunk_id: Some("file".to_string()),
            chunk_index: None,
            heading_path: Vec::new(),
            line_start: None,
            line_end: None,
            fallback_reason: None,
            lint_summary: None,
            suggestions: Vec::new(),
        };
        queries.upsert_content_record(&row).await?;
    }

    if !emission.nodes.is_empty() {
        queries.upsert_powerbi_nodes(&emission.nodes).await?;
    }
    if !emission.edges.is_empty() {
        queries.upsert_powerbi_edges(&emission.edges).await?;
    }

    // `ingested` reports the files actually snapshotted and re-indexed this run
    // (the considered set), not `total_files`, which over-reports because it
    // still includes files skipped during snapshotting (oversized / non-UTF-8 /
    // unreadable). Matches the meaning used by the other indexers.
    result.ingested = file_data.len();
    info!(
        ingested = result.ingested,
        records = emission.records.len() + (file_data.len().saturating_sub(covered.len())),
        nodes = emission.nodes.len(),
        edges = emission.edges.len(),
        source = %source.path,
        "PBIP indexing complete"
    );

    Ok(result)
}

/// Sweep deleted PBIP project-definition files from the index.
///
/// Queries all `content_type = "pbip"` records for the source, checks each
/// file path against the filesystem via [`compute_deleted_paths`], and deletes
/// the records and graph nodes for files that no longer exist on disk.
///
/// # Errors
///
/// Returns `Err` when an underlying database query fails unrecoverably.
pub async fn sweep_deleted_pbip_files(
    source: &ContentSource,
    workspace_root: &Path,
    queries: &CodeGraphQueries,
) -> Result<usize, EngramError> {
    let records = queries.select_content_records(Some("pbip")).await?;

    let known_paths: Vec<String> = records
        .into_iter()
        .filter(|record| record.source_path == source.path)
        .map(|record| record.file_path)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let deleted = compute_deleted_paths(&known_paths, workspace_root);
    let mut removed = 0_usize;

    for path in &deleted {
        queries
            .delete_content_records_by_scope(path, "pbip", &source.path)
            .await?;
        queries.delete_powerbi_nodes_by_file_path(path).await?;
        removed += 1;
    }

    Ok(removed)
}

// ── Path helpers ──────────────────────────────────────────────────────────

/// Convert an absolute collected path into a forward-slashed,
/// workspace-relative string.
///
/// Returns `None` when `path` is not under `workspace_root`. Downstream
/// subsystems (the deletion sweep, path-escape guards) assume
/// [`ContentRecord::file_path`] is workspace-relative, so a collected path that
/// cannot be made relative is skipped rather than stored as an absolute path
/// that could later escape the workspace boundary.
fn snapshot_relative_path(path: &Path, workspace_root: &Path) -> Option<String> {
    path.strip_prefix(workspace_root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

/// Whether `path` ends with the given extension (case-insensitive).
fn has_extension(path: &str, ext: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(ext))
}

/// The file stem of a workspace-relative path (filename without extension).
fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_string()
}

/// The parent directory of a workspace-relative path, forward-slashed. Returns
/// an empty string when the path has no parent.
fn parent_dir(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .replace('\\', "/")
}

/// The human-readable report name from a report folder, dropping a trailing
/// `.Report` suffix (case-insensitive) when present.
fn report_display_name(report_folder: &str) -> String {
    let leaf = report_folder.rsplit('/').next().unwrap_or(report_folder);
    // Trim a trailing ".report" (7 bytes, ASCII) only at a valid char boundary
    // so a non-ASCII leaf derived from untrusted `.pbip` content cannot panic.
    match leaf.len().checked_sub(7) {
        Some(idx) if leaf.is_char_boundary(idx) && leaf[idx..].eq_ignore_ascii_case(".report") => {
            leaf[..idx].to_string()
        }
        _ => leaf.to_string(),
    }
}

/// Resolve a `.`/`..`-relative `rel` segment against a workspace-relative
/// `base_dir`, collapsing segments and forward-slashing the result. Returns
/// `None` when the relative path escapes above the workspace root.
fn join_relative(base_dir: &str, rel: &str) -> Option<String> {
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

// ── Unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a [`FileData`] snapshot from raw text.
    fn fd(content: &str) -> FileData {
        FileData {
            hash: compute_file_hash(content.as_bytes()),
            content: content.to_string(),
            size: content.len() as u64,
        }
    }

    /// A minimal one-report / one-page / one-visual project (no `.pbir`, so no
    /// semantic-model linkage) keyed by workspace-relative path.
    fn sample_project(visual_json: &str) -> BTreeMap<String, FileData> {
        let mut files = BTreeMap::new();
        files.insert(
            "proj/My.pbip".to_string(),
            fd(r#"{"version":"1.0","artifacts":[{"report":{"path":"My.Report"}}]}"#),
        );
        files.insert(
            "proj/My.Report/definition/report.json".to_string(),
            fd("{}"),
        );
        files.insert(
            "proj/My.Report/definition/pages/pages.json".to_string(),
            fd(r#"{"pageOrder":["Page1"],"activePageName":"Page1"}"#),
        );
        files.insert(
            "proj/My.Report/definition/pages/Page1/page.json".to_string(),
            fd(r#"{"name":"Page1","displayName":"First Page"}"#),
        );
        files.insert(
            "proj/My.Report/definition/pages/Page1/visuals/v1/visual.json".to_string(),
            fd(visual_json),
        );
        files
    }

    fn build(files: &BTreeMap<String, FileData>) -> PbipEmission {
        EmissionBuilder::new(files, "proj", Path::new(".")).build()
    }

    /// S-PIDX-01: report→page→visual contains edges are emitted, and the node
    /// kinds/names reflect the parsed descriptors.
    #[test]
    fn builds_report_page_visual_contains_graph() {
        let files = sample_project(r#"{"name":"v1","visual":{"visualType":"card"}}"#);
        let emission = build(&files);

        let report = emission
            .nodes
            .iter()
            .find(|n| n.kind == PowerBiNodeKind::Report)
            .expect("report node present");
        assert_eq!(report.name, "My", "report name strips .Report suffix");

        let page = emission
            .nodes
            .iter()
            .find(|n| n.kind == PowerBiNodeKind::Page)
            .expect("page node present");
        assert_eq!(page.name, "First Page", "page node uses displayName");

        let visual = emission
            .nodes
            .iter()
            .find(|n| n.kind == PowerBiNodeKind::Visual)
            .expect("visual node present");
        assert_eq!(visual.name, "v1");

        let has_edge = |from: &str, to: &str| {
            emission.edges.iter().any(|e| {
                e.from_id == from && e.to_id == to && e.edge_type == PowerBiEdgeType::Contains
            })
        };
        assert!(has_edge(&report.id, &page.id), "report should contain page");
        assert!(has_edge(&page.id, &visual.id), "page should contain visual");
    }

    /// S-PIDX-02: each project object emits a content record with the expected
    /// `record_kind`, and every emitted record attaches to a collected file.
    #[test]
    fn emits_object_records_for_each_entity() {
        let files = sample_project(r#"{"name":"v1","visual":{"visualType":"card"}}"#);
        let emission = build(&files);

        let kinds: Vec<&str> = emission
            .records
            .iter()
            .map(|r| r.record_kind.as_str())
            .collect();
        for expected in [
            "pbip_workspace",
            "pbip_report",
            "pbip_page_order",
            "pbip_page",
            "pbip_visual",
        ] {
            assert!(
                kinds.contains(&expected),
                "expected a {expected} record, got {kinds:?}"
            );
        }

        for record in &emission.records {
            assert!(
                files.contains_key(&record.file_path),
                "record {} attaches to uncollected file {}",
                record.record_kind,
                record.file_path
            );
        }
    }

    /// S-PIDX-03: visual semantic bindings are summarised in the visual record,
    /// but produce no `uses_field` edge when the model is absent (no dangling
    /// edges).
    #[test]
    fn records_visual_bindings_without_dangling_edges() {
        let visual = r#"{
            "name":"v1",
            "visual":{
                "visualType":"card",
                "query":{"queryState":{"Values":{"projections":[
                    {"field":{"Measure":{"Expression":{"SourceRef":{"Entity":"Sales"}},"Property":"Total"}}}
                ]}}}
            }
        }"#;
        let files = sample_project(visual);
        let emission = build(&files);

        let visual_record = emission
            .records
            .iter()
            .find(|r| r.record_kind == "pbip_visual")
            .expect("visual record present");
        assert!(
            visual_record.content_text.contains("Sales.Total"),
            "binding hint should appear in record: {}",
            visual_record.content_text
        );

        assert!(
            !emission
                .edges
                .iter()
                .any(|e| e.edge_type == PowerBiEdgeType::UsesField),
            "no uses_field edge should be emitted without a resolved model"
        );
    }

    /// S-PIDX-04: `report_display_name` trims a trailing `.Report` suffix
    /// case-insensitively but leaves other names intact.
    #[test]
    fn report_display_name_trims_suffix() {
        assert_eq!(report_display_name("proj/Sales.Report"), "Sales");
        assert_eq!(report_display_name("proj/Sales.REPORT"), "Sales");
        assert_eq!(report_display_name("proj/Dashboard"), "Dashboard");
        // Non-ASCII leaf derived from untrusted `.pbip` content must not panic
        // on the byte-length suffix check (regression for char-boundary slice).
        assert_eq!(report_display_name("proj/éabcdef"), "éabcdef");
        assert_eq!(report_display_name("proj/Ventés.Report"), "Ventés");
    }

    /// S-PIDX-05: `join_relative` collapses `..` and rejects escapes above the
    /// workspace root.
    #[test]
    fn join_relative_resolves_and_guards() {
        assert_eq!(
            join_relative("proj", "My.Report").as_deref(),
            Some("proj/My.Report")
        );
        assert_eq!(
            join_relative("proj/My.Report", "../My.SemanticModel").as_deref(),
            Some("proj/My.SemanticModel")
        );
        assert_eq!(join_relative("proj", "../../etc"), None);
    }

    /// S-PIDX-06 (Issue B regression): a report folder whose descriptor files
    /// were never collected must not emit an orphaned report node anchored to a
    /// non-existent `definition.pbir` with an empty content hash.
    #[test]
    fn report_without_descriptor_emits_no_orphan_node() {
        let mut files = BTreeMap::new();
        files.insert(
            "proj/My.pbip".to_string(),
            fd(r#"{"version":"1.0","artifacts":[{"report":{"path":"My.Report"}}]}"#),
        );
        // Deliberately collect neither report.json nor definition.pbir for the
        // referenced My.Report folder.
        let emission = build(&files);

        assert!(
            !emission
                .nodes
                .iter()
                .any(|n| n.kind == PowerBiNodeKind::Report),
            "no report node should be emitted without a collected descriptor"
        );
        assert!(
            !emission
                .records
                .iter()
                .any(|r| r.record_kind == "pbip_report" || r.record_kind == "pbip_report_link"),
            "no report records should be emitted without a collected descriptor"
        );
        // Every emitted record must still attach to a collected file.
        for record in &emission.records {
            assert!(
                files.contains_key(&record.file_path),
                "record {} attaches to uncollected file {}",
                record.record_kind,
                record.file_path
            );
        }
    }

    /// S-PIDX-07 (Issue D regression): a `pages.json` that exists but cannot be
    /// parsed must not emit a misleading `pbip_page_order` record.
    #[test]
    fn unparseable_page_order_emits_no_page_order_record() {
        let mut files = sample_project(r#"{"name":"v1","visual":{"visualType":"card"}}"#);
        // Corrupt pages.json so parse_page_order returns None.
        files.insert(
            "proj/My.Report/definition/pages/pages.json".to_string(),
            fd("{ this is not valid json"),
        );
        let emission = build(&files);

        assert!(
            !emission
                .records
                .iter()
                .any(|r| r.record_kind == "pbip_page_order"),
            "no page-order record should be emitted when pages.json cannot be parsed"
        );
    }

    /// S-PIDX-08 (Issue E regression): a collected path outside the workspace
    /// root is rejected rather than stored as an absolute `file_path`.
    #[test]
    fn snapshot_relative_path_rejects_paths_outside_workspace() {
        let root = Path::new("ws").join("root");
        let inside = root.join("a").join("b.tmdl");
        assert_eq!(
            snapshot_relative_path(&inside, &root).as_deref(),
            Some("a/b.tmdl"),
            "a path under the workspace root is made forward-slashed relative"
        );

        let outside = Path::new("other").join("x.tmdl");
        assert_eq!(
            snapshot_relative_path(&outside, &root),
            None,
            "a path outside the workspace root is rejected"
        );
    }

    /// Copilot PR #177 regression: files collected by the walker but skipped
    /// during snapshotting (oversized / non-UTF-8 / unreadable) must NOT inflate
    /// `ingested` or `unchanged`. Those counters reflect the considered set
    /// (`file_data`), while `total_files` still reflects every collected file.
    #[tokio::test]
    async fn ingested_and_unchanged_exclude_skipped_files() {
        use tempfile::TempDir;

        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let root = workspace.path();

        let report_def = root.join("proj").join("My.Report").join("definition");
        let pages = report_def.join("pages");
        let visual_dir = pages.join("Page1").join("visuals").join("v1");
        std::fs::create_dir_all(&visual_dir).expect("create project tree");

        // Five valid, in-bounds, UTF-8 PBIP files.
        std::fs::write(
            root.join("proj").join("My.pbip"),
            r#"{"version":"1.0","artifacts":[{"report":{"path":"My.Report"}}]}"#,
        )
        .expect("write .pbip");
        std::fs::write(report_def.join("report.json"), "{}").expect("write report.json");
        std::fs::write(
            pages.join("pages.json"),
            r#"{"pageOrder":["Page1"],"activePageName":"Page1"}"#,
        )
        .expect("write pages.json");
        std::fs::write(
            pages.join("Page1").join("page.json"),
            r#"{"name":"Page1","displayName":"First Page"}"#,
        )
        .expect("write page.json");
        std::fs::write(visual_dir.join("visual.json"), "{}").expect("write visual.json");

        // One collected-but-skipped file: a `.json` (a PBIP extension, so the
        // walker collects it) whose bytes are not valid UTF-8, so the snapshot
        // step drops it before hashing/indexing.
        let invalid_utf8: [u8; 4] = [0xff, 0xfe, 0xff, 0x00];
        std::fs::write(report_def.join("invalid.json"), invalid_utf8)
            .expect("write non-UTF-8 json");

        let source = ContentSource {
            content_type: "pbip".to_string(),
            language: None,
            path: "proj".to_string(),
            pattern: None,
            optional: false,
            status: crate::models::registry::ContentSourceStatus::default(),
        };

        let db = crate::db::connect_db(db_dir.path(), "pbip-count-test")
            .await
            .expect("open test db");
        let queries = CodeGraphQueries::new(db);

        // First run performs a full rebuild: `ingested` is the considered set
        // (5), not `total_files` (6, which includes the skipped non-UTF-8 file).
        let first = index_pbip_source(&source, root, &queries, 1_048_576)
            .await
            .expect("first index run");
        assert_eq!(
            first.total_files, 6,
            "total_files counts every collected file, including the skipped one"
        );
        assert_eq!(
            first.ingested, 5,
            "ingested excludes the collected-but-skipped non-UTF-8 file"
        );
        assert_eq!(
            first.unchanged, 0,
            "a rebuild leaves nothing in the unchanged count"
        );

        // Second run with no on-disk changes takes the unchanged branch:
        // `unchanged` is likewise the considered set (5), not `total_files`.
        let second = index_pbip_source(&source, root, &queries, 1_048_576)
            .await
            .expect("second index run");
        assert_eq!(second.total_files, 6, "total_files is stable across runs");
        assert_eq!(
            second.unchanged, 5,
            "unchanged excludes the collected-but-skipped non-UTF-8 file"
        );
        assert_eq!(
            second.ingested, 0,
            "the unchanged branch performs no ingestion"
        );
    }
}
