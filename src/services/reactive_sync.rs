//! Verify-gated reactive markdown reingest for the daemon file-watch loop.
//!
//! The daemon's live v2 auto-sync loop classifies mutated markdown as
//! [`ServiceAction::ReingestContent`](crate::daemon::debounce::ServiceAction)
//! and drives it through the gate implemented here: a mutated markdown file is
//! ingested into the content-record table **only** when
//! [`verify_markdown`](crate::services::verify::verify_markdown) reports it
//! structurally conformant. Non-conformant mutations are skipped and logged —
//! never ingested — closing the reactive half of the Phase 1b verify gate.
//!
//! The decision logic is split so it is testable without ever spinning the
//! daemon (which would collide with the known Windows-only
//! `run_with_shutdown_v2` SQLite startup flake):
//!
//! - [`markdown_gate_decision`] — a pure, I/O-free function mapping
//!   `(path, content)` to an ingest-or-skip [`GateDecision`].
//! - [`resolve_content_source`] — a pure longest-prefix source resolver.
//! - [`verify_gated_reingest`] — a single-path orchestrator that reads the
//!   file, resolves its owning source, applies the gate, and ingests via the
//!   existing [`ingest_single_file`](crate::services::ingestion::ingest_single_file)
//!   path. It is fallible-by-`Result` and never panics, so the caller can
//!   log-and-continue on any error.
//!
//! Only the v2 consumer loop is wired to this gate. The legacy
//! `run_with_shutdown` v1 loop remains `ReindexFile`-only and is intentionally
//! not gated here; v1 parity is tracked as a separate item.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};

use crate::db::queries::CodeGraphQueries;
use crate::errors::{EngramError, IngestionError};
use crate::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
use crate::services::ingestion::{build_glob_filter, ingest_single_file};
use crate::services::verify::{VerifyFinding, verify_markdown};

/// The decision produced by the pure verify gate for a markdown document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    /// The document is conformant and should be ingested.
    Ingest,
    /// The document is non-conformant; skip ingestion. Carries the findings so
    /// the caller can emit an actionable diagnostic.
    Skip {
        /// Structural findings that blocked ingestion.
        findings: Vec<VerifyFinding>,
    },
}

/// The outcome of a verify-gated reactive markdown reingest attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReingestOutcome {
    /// Conformant markdown was ingested (or was already current on disk).
    Ingested,
    /// Markdown was non-conformant; skipped and logged, never ingested.
    SkippedNonConformant,
    /// The path is not owned by any ingestible content source; skipped.
    SkippedUnowned,
    /// The file exceeds the registry's `max_file_size_bytes`; skipped by a
    /// metadata precheck without reading the document into memory.
    SkippedOversize,
}

/// Pure verify-gate decision for a markdown document.
///
/// Given the logical `rel_path` (used only to contextualise diagnostics) and
/// the full document `content`, returns whether the document is conformant
/// enough to ingest. This performs no filesystem or database I/O and never
/// spins the daemon, so it is directly unit-testable and is the authoritative
/// gate primitive shared by [`verify_gated_reingest`].
///
/// # Errors
///
/// Propagates any [`EngramError`] surfaced by
/// [`verify_markdown`](crate::services::verify::verify_markdown). The Phase 1a
/// rule set is infallible, but the fallible contract is preserved for future
/// rules.
pub fn markdown_gate_decision(rel_path: &str, content: &str) -> Result<GateDecision, EngramError> {
    let report = verify_markdown(rel_path, content)?;
    if report.conformant {
        Ok(GateDecision::Ingest)
    } else {
        Ok(GateDecision::Skip {
            findings: report.findings,
        })
    }
}

/// Content-source types that the startup
/// [`ingest_all_sources`](crate::services::ingestion::ingest_all_sources) path
/// routes to **dedicated indexers** (code-graph, backlog, notebook, Power BI,
/// PBIP) instead of the generic markdown/content path. A `.md` mutation
/// physically under one of these sources must be skipped reactively: the startup
/// re-index would never write a generic `content_record` for it, so ingesting it
/// here would diverge the database from a fresh restart re-index (finding C-2).
///
/// This is an allowlist expressed by exclusion so it stays a faithful mirror of
/// the startup routing: every *other* type — the built-in generic types
/// (`docs`/`memory`/`spec`/`context`/`instructions`/…) **and** any custom
/// content type — falls through to the generic path in `ingest_all_sources` and
/// is therefore ingestible here. Enumerating an explicit positive allowlist
/// would wrongly skip custom generic types the startup path *does* ingest.
const DEDICATED_INDEXER_TYPES: &[&str] = &["code", "backlog", "notebook", "powerbi", "pbip"];

/// Resolve the content source that owns `rel_path` by longest path-prefix match.
///
/// Only `Active` sources (per [`ContentSourceStatus`]) whose `content_type` uses
/// the generic markdown content path are eligible. Sources routed to dedicated
/// indexers (see [`DEDICATED_INDEXER_TYPES`]) and sources that failed validation
/// (`Missing`/`Error`/`Unknown`) are excluded, mirroring the startup
/// [`ingest_all_sources`](crate::services::ingestion::ingest_all_sources)
/// behaviour so a reactive ingest never diverges from a fresh restart re-index.
/// Among the remaining sources, the one whose declared `path` is the longest
/// prefix of `rel_path` (on a `/`-segment boundary) wins. Returns `None` when no
/// eligible source owns the path.
///
/// `rel_path` is expected workspace-relative; both it and each source path are
/// normalised so `\` separators (Windows) and trailing slashes do not affect
/// matching. A source declared at the workspace root (empty path) owns every
/// path but always yields to a more specific source.
pub fn resolve_content_source<'a>(
    rel_path: &str,
    sources: &'a [ContentSource],
) -> Option<&'a ContentSource> {
    let normalized = rel_path.replace('\\', "/");
    let mut best: Option<&ContentSource> = None;
    let mut best_len: Option<usize> = None;
    for source in sources {
        // C-3/R2: only ingest into validated, active sources. A Missing/Error
        // source is skipped exactly as the startup ingest path skips it.
        if source.status != ContentSourceStatus::Active {
            continue;
        }
        // C-2/R1: skip content types routed to dedicated indexers; ingesting a
        // `.md` under them via the generic content path would create records the
        // startup re-index never writes (DB divergence after a restart).
        if DEDICATED_INDEXER_TYPES.contains(&source.content_type.as_str()) {
            continue;
        }
        let src_owned = source.path.replace('\\', "/");
        let src = src_owned.trim_matches('/');
        let owns =
            src.is_empty() || normalized == src || normalized.starts_with(&format!("{src}/"));
        if !owns {
            continue;
        }
        let more_specific = match best_len {
            None => true,
            Some(len) => src.len() > len,
        };
        if more_specific {
            best = Some(source);
            best_len = Some(src.len());
        }
    }
    best
}

/// Verify-gate and (when conformant) reingest a single mutated markdown file.
///
/// Resolves the file's owning [`ContentSource`] via [`resolve_content_source`];
/// if unowned, returns [`ReingestOutcome::SkippedUnowned`] without reading the
/// file or touching the database. A metadata size precheck then rejects files
/// larger than `max_file_size_bytes` ([`ReingestOutcome::SkippedOversize`])
/// before the document is buffered. Otherwise the file is read and passed
/// through [`markdown_gate_decision`]: conformant documents are ingested via
/// [`ingest_single_file`](crate::services::ingestion::ingest_single_file);
/// non-conformant documents are logged at `warn` and skipped
/// ([`ReingestOutcome::SkippedNonConformant`]).
///
/// This never panics and never breaks the daemon receive loop: all fallible
/// filesystem, verify, and ingest operations surface as [`EngramError`] via
/// `?`, which the caller logs and continues past.
///
/// # Errors
///
/// Returns [`EngramError`] when the file cannot be read, when
/// [`verify_markdown`](crate::services::verify::verify_markdown) fails, or when
/// [`ingest_single_file`](crate::services::ingestion::ingest_single_file)
/// fails.
pub async fn verify_gated_reingest(
    file_path: &Path,
    workspace_root: &Path,
    config: &RegistryConfig,
    queries: &CodeGraphQueries,
) -> Result<ReingestOutcome, EngramError> {
    let rel_path = file_path
        .strip_prefix(workspace_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/");

    let Some(source) = resolve_content_source(&rel_path, &config.sources) else {
        debug!(
            path = %rel_path,
            "reactive reingest: markdown path not owned by any content source — skipping"
        );
        return Ok(ReingestOutcome::SkippedUnowned);
    };

    // C-4/R3: reject oversize files by metadata before buffering the whole
    // document, matching the `max_file_size_bytes` limit `ingest_single_file`
    // enforces (which is only reached *after* this read). Avoids pulling a large
    // file into memory just to gate it.
    let metadata = tokio::fs::metadata(file_path)
        .await
        .map_err(|e| IngestionError::Failed {
            path: rel_path.clone(),
            reason: format!("cannot stat markdown for verify gate: {e}"),
        })?;
    if metadata.len() > config.max_file_size_bytes {
        warn!(
            path = %rel_path,
            size = metadata.len(),
            max = config.max_file_size_bytes,
            "reactive reingest: skipped oversize markdown"
        );
        return Ok(ReingestOutcome::SkippedOversize);
    }

    let content =
        tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| IngestionError::Failed {
                path: rel_path.clone(),
                reason: format!("cannot read markdown for verify gate: {e}"),
            })?;

    match markdown_gate_decision(&rel_path, &content)? {
        GateDecision::Skip { findings } => {
            warn!(
                path = %rel_path,
                findings = findings.len(),
                "skipped non-conformant markdown reingest"
            );
            Ok(ReingestOutcome::SkippedNonConformant)
        }
        GateDecision::Ingest => {
            let glob_filter = build_glob_filter(source.pattern.as_deref());
            ingest_single_file(
                file_path,
                workspace_root,
                &source.content_type,
                &source.path,
                config.max_file_size_bytes,
                glob_filter.as_ref(),
                queries,
            )
            .await?;
            debug!(path = %rel_path, "reactive reingest: conformant markdown ingested");
            Ok(ReingestOutcome::Ingested)
        }
    }
}

/// Load the workspace registry, open the content database, and drive each
/// pending mutated markdown path through [`verify_gated_reingest`].
///
/// This is the daemon-loop-facing orchestrator invoked by the live v2 auto-sync
/// consumer. It is intentionally *not* unit-tested — it performs registry and
/// content-database I/O keyed on the daemon's data directory, which would
/// require spinning daemon-adjacent state. The gate correctness it relies on is
/// covered by direct tests of [`verify_gated_reingest`],
/// [`markdown_gate_decision`], and [`resolve_content_source`].
///
/// It is fail-safe by construction: a missing registry, a database-connect
/// failure, or a per-file error is logged and the function returns normally, so
/// the daemon receive loop is never broken. `data_dir` and `branch` mirror the
/// values used by the code-graph sync path so both open the same workspace DB.
pub async fn reingest_pending_markdown(
    workspace_root: &Path,
    data_dir: &Path,
    branch: &str,
    pending: &BTreeSet<PathBuf>,
) {
    if pending.is_empty() {
        return;
    }

    let registry_path = workspace_root.join(".engram").join("registry.yaml");
    let mut config = match crate::services::registry::load_registry(&registry_path) {
        Ok(Some(config)) => config,
        Ok(None) => {
            debug!("reactive reingest: no registry.yaml — skipping markdown reingest");
            return;
        }
        Err(e) => {
            warn!(error = %e, "reactive reingest: registry load failed — skipping");
            return;
        }
    };
    // Best-effort source hydration; a validation failure still leaves usable
    // sources (mirrors the startup ingestion path).
    let _ = crate::services::registry::validate_sources(&mut config, workspace_root);

    let db = match crate::db::connect_db(data_dir, branch).await {
        Ok(db) => db,
        Err(e) => {
            warn!(error = %e, "reactive reingest: failed to connect to database — skipping");
            return;
        }
    };
    let queries = CodeGraphQueries::new(db);

    let mut ingested = 0usize;
    let mut skipped = 0usize;
    for rel in pending {
        let file_path = workspace_root.join(rel);
        match verify_gated_reingest(&file_path, workspace_root, &config, &queries).await {
            Ok(ReingestOutcome::Ingested) => ingested += 1,
            Ok(
                ReingestOutcome::SkippedNonConformant
                | ReingestOutcome::SkippedUnowned
                | ReingestOutcome::SkippedOversize,
            ) => {
                skipped += 1;
            }
            Err(e) => {
                warn!(error = %e, path = ?rel, "reactive reingest: markdown reingest failed");
            }
        }
    }

    if ingested > 0 || skipped > 0 {
        info!(ingested, skipped, "reactive markdown reingest complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::registry::{ContentSource, ContentSourceStatus};
    use tempfile::TempDir;

    fn docs_source() -> ContentSource {
        ContentSource {
            content_type: "docs".to_string(),
            language: None,
            path: "docs".to_string(),
            pattern: None,
            optional: false,
            status: ContentSourceStatus::Active,
        }
    }

    fn config_with(sources: Vec<ContentSource>) -> RegistryConfig {
        RegistryConfig {
            sources,
            max_file_size_bytes: 1_048_576,
            batch_size: 50,
            ..RegistryConfig::default()
        }
    }

    // ── Pure gate decision (no I/O, no DB) ────────────────────────────────────

    #[test]
    fn gate_decision_conformant_markdown_is_ingest() {
        let decision =
            markdown_gate_decision("docs/notes.md", "# Title\n\nA conformant body.\n").unwrap();
        assert_eq!(decision, GateDecision::Ingest);
    }

    #[test]
    fn gate_decision_malformed_frontmatter_is_skip() {
        let content = "---\nkey: [unterminated\n---\n\nBody text.\n";
        let decision = markdown_gate_decision("docs/bad.md", content).unwrap();
        match decision {
            GateDecision::Skip { findings } => {
                assert!(
                    findings.iter().any(|f| f.rule == "frontmatter.malformed"),
                    "expected a frontmatter.malformed finding, got {findings:?}"
                );
            }
            GateDecision::Ingest => panic!("malformed frontmatter must not be Ingest"),
        }
    }

    #[test]
    fn gate_decision_empty_body_is_skip() {
        let content = "---\ntitle: Only Frontmatter\n---\n";
        let decision = markdown_gate_decision("docs/empty.md", content).unwrap();
        assert!(matches!(decision, GateDecision::Skip { .. }));
    }

    #[test]
    fn gate_decision_unresolved_template_is_skip() {
        let content = "# Title\n\nValue is {{PLACEHOLDER}}.\n";
        let decision = markdown_gate_decision("docs/tmpl.md", content).unwrap();
        assert!(matches!(decision, GateDecision::Skip { .. }));
    }

    // ── Pure source resolution ────────────────────────────────────────────────

    #[test]
    fn resolve_owned_path_returns_source() {
        let sources = vec![docs_source()];
        let resolved = resolve_content_source("docs/notes.md", &sources);
        assert_eq!(resolved.map(|s| s.path.as_str()), Some("docs"));
    }

    #[test]
    fn resolve_unowned_path_returns_none() {
        let sources = vec![docs_source()];
        assert!(resolve_content_source("other/notes.md", &sources).is_none());
    }

    #[test]
    fn resolve_prefers_longest_prefix() {
        let general = ContentSource {
            content_type: "docs".to_string(),
            language: None,
            path: "docs".to_string(),
            pattern: None,
            optional: false,
            status: ContentSourceStatus::Active,
        };
        let specific = ContentSource {
            content_type: "spec".to_string(),
            language: None,
            path: "docs/specs".to_string(),
            pattern: None,
            optional: false,
            status: ContentSourceStatus::Active,
        };
        let sources = vec![general, specific];
        let resolved = resolve_content_source("docs/specs/api.md", &sources);
        assert_eq!(resolved.map(|s| s.content_type.as_str()), Some("spec"));
    }

    #[test]
    fn resolve_excludes_code_and_backlog_sources() {
        let code = ContentSource {
            content_type: "code".to_string(),
            language: Some("rust".to_string()),
            path: "src".to_string(),
            pattern: None,
            optional: false,
            status: ContentSourceStatus::Active,
        };
        let backlog = ContentSource {
            content_type: "backlog".to_string(),
            language: None,
            path: ".backlogit".to_string(),
            pattern: None,
            optional: false,
            status: ContentSourceStatus::Active,
        };
        let sources = vec![code, backlog];
        // A markdown file under a code source is not owned by the content path.
        assert!(resolve_content_source("src/README.md", &sources).is_none());
        assert!(resolve_content_source(".backlogit/queue/x.md", &sources).is_none());
    }

    #[test]
    fn resolve_excludes_dedicated_indexer_sources() {
        // `.md` physically under a dedicated-indexer source must not resolve to
        // the generic content path: startup routes these to their own indexers,
        // so a reactive generic ingest would diverge the DB from a restart
        // re-index (finding C-2/R1).
        for content_type in ["powerbi", "notebook", "pbip", "code", "backlog"] {
            let source = ContentSource {
                content_type: content_type.to_string(),
                language: None,
                path: "reports".to_string(),
                pattern: None,
                optional: false,
                status: ContentSourceStatus::Active,
            };
            assert!(
                resolve_content_source("reports/readme.md", std::slice::from_ref(&source))
                    .is_none(),
                "a .md under a `{content_type}` source must not resolve to the generic path"
            );
        }
    }

    #[test]
    fn resolve_skips_inactive_source() {
        // A source that failed validation (Missing/Error) or is unhydrated
        // (Unknown) is skipped, mirroring the startup ingest path (C-3/R2).
        for status in [
            ContentSourceStatus::Missing,
            ContentSourceStatus::Error,
            ContentSourceStatus::Unknown,
        ] {
            let source = ContentSource {
                content_type: "docs".to_string(),
                language: None,
                path: "docs".to_string(),
                pattern: None,
                optional: false,
                status,
            };
            assert!(
                resolve_content_source("docs/notes.md", std::slice::from_ref(&source)).is_none(),
                "a non-Active ({status:?}) source must not own any path"
            );
        }
    }

    #[test]
    fn resolve_normalizes_backslash_separators() {
        let sources = vec![docs_source()];
        let resolved = resolve_content_source("docs\\deep\\notes.md", &sources);
        assert_eq!(resolved.map(|s| s.path.as_str()), Some("docs"));
    }

    // ── End-to-end gate + ingest against a temporary CozoDB (no daemon) ────────

    async fn setup_db(db_dir: &TempDir, branch: &str) -> CodeGraphQueries {
        let db = crate::db::connect_db(db_dir.path(), branch)
            .await
            .expect("open temp cozo db");
        CodeGraphQueries::new(db)
    }

    fn write_markdown(workspace: &Path, rel: &str, content: &str) -> std::path::PathBuf {
        let path = workspace.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        // std::fs::write flushes fully before returning (avoids the tokio::fs
        // File flush-before-read landmine).
        std::fs::write(&path, content).expect("write markdown fixture");
        path
    }

    #[tokio::test]
    async fn valid_markdown_is_ingested() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let queries = setup_db(&db_dir, "valid-md").await;
        let config = config_with(vec![docs_source()]);

        let file = write_markdown(
            workspace.path(),
            "docs/guide.md",
            "# Guide\n\nThis is a conformant markdown document.\n",
        );

        let outcome = verify_gated_reingest(&file, workspace.path(), &config, &queries)
            .await
            .expect("gate should not error on conformant markdown");
        assert_eq!(outcome, ReingestOutcome::Ingested);

        let records = queries
            .select_content_records(Some("docs"))
            .await
            .expect("select docs content records");
        assert!(
            !records.is_empty(),
            "a conformant markdown mutation must write at least one content record"
        );
    }

    #[tokio::test]
    async fn malformed_frontmatter_markdown_is_skipped_and_logged() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let queries = setup_db(&db_dir, "malformed-md").await;
        let config = config_with(vec![docs_source()]);

        let file = write_markdown(
            workspace.path(),
            "docs/broken.md",
            "---\nkey: [unterminated\n---\n\nBody text that would otherwise ingest.\n",
        );

        let outcome = verify_gated_reingest(&file, workspace.path(), &config, &queries)
            .await
            .expect("gate should not error on non-conformant markdown");
        assert_eq!(outcome, ReingestOutcome::SkippedNonConformant);

        let records = queries
            .select_content_records(Some("docs"))
            .await
            .expect("select docs content records");
        assert!(
            records.is_empty(),
            "non-conformant markdown must not write any content record"
        );
    }

    #[tokio::test]
    async fn empty_body_markdown_is_skipped() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let queries = setup_db(&db_dir, "empty-md").await;
        let config = config_with(vec![docs_source()]);

        let file = write_markdown(
            workspace.path(),
            "docs/empty.md",
            "---\ntitle: Only Frontmatter\n---\n",
        );

        let outcome = verify_gated_reingest(&file, workspace.path(), &config, &queries)
            .await
            .expect("gate should not error");
        assert_eq!(outcome, ReingestOutcome::SkippedNonConformant);

        let records = queries
            .select_content_records(Some("docs"))
            .await
            .expect("select docs content records");
        assert!(records.is_empty(), "empty-body markdown must not ingest");
    }

    #[tokio::test]
    async fn unresolved_template_var_markdown_is_skipped() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let queries = setup_db(&db_dir, "template-md").await;
        let config = config_with(vec![docs_source()]);

        let file = write_markdown(
            workspace.path(),
            "docs/tmpl.md",
            "# Title\n\nUnresolved {{PLACEHOLDER}} remains.\n",
        );

        let outcome = verify_gated_reingest(&file, workspace.path(), &config, &queries)
            .await
            .expect("gate should not error");
        assert_eq!(outcome, ReingestOutcome::SkippedNonConformant);

        let records = queries
            .select_content_records(Some("docs"))
            .await
            .expect("select docs content records");
        assert!(
            records.is_empty(),
            "markdown with unresolved template variables must not ingest"
        );
    }

    #[tokio::test]
    async fn unowned_path_is_skipped() {
        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let queries = setup_db(&db_dir, "unowned-md").await;
        let config = config_with(vec![docs_source()]);

        // File lives outside every declared source directory.
        let file = write_markdown(
            workspace.path(),
            "other/notes.md",
            "# Notes\n\nConformant, but unowned.\n",
        );

        let outcome = verify_gated_reingest(&file, workspace.path(), &config, &queries)
            .await
            .expect("gate should not error");
        assert_eq!(outcome, ReingestOutcome::SkippedUnowned);

        let docs = queries
            .select_content_records(Some("docs"))
            .await
            .expect("select docs content records");
        assert!(
            docs.is_empty(),
            "unowned path must not ingest into any source"
        );
    }

    #[tokio::test]
    async fn markdown_under_dedicated_indexer_source_is_not_ingested() {
        // Regression for C-2/R1: a conformant `.md` physically under a powerbi
        // source must be skipped (as unowned by the generic path), never
        // ingested — otherwise the reactive path writes a generic content_record
        // the startup powerbi indexer would never create, diverging the DB.
        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let queries = setup_db(&db_dir, "powerbi-md").await;
        let powerbi = ContentSource {
            content_type: "powerbi".to_string(),
            language: None,
            path: "reports".to_string(),
            pattern: None,
            optional: false,
            status: ContentSourceStatus::Active,
        };
        let config = config_with(vec![powerbi]);

        let file = write_markdown(
            workspace.path(),
            "reports/model.md",
            "# Model\n\nConformant, but under a dedicated-indexer source.\n",
        );

        let outcome = verify_gated_reingest(&file, workspace.path(), &config, &queries)
            .await
            .expect("gate should not error");
        assert_eq!(outcome, ReingestOutcome::SkippedUnowned);

        let records = queries
            .select_content_records(Some("powerbi"))
            .await
            .expect("select powerbi content records");
        assert!(
            records.is_empty(),
            "a .md under a powerbi source must not create a generic content record"
        );
    }

    #[tokio::test]
    async fn oversize_markdown_is_skipped_by_metadata_precheck() {
        // C-4/R3: an otherwise-conformant file larger than max_file_size_bytes
        // is rejected by the metadata precheck without ingesting.
        let workspace = TempDir::new().expect("workspace tempdir");
        let db_dir = TempDir::new().expect("db tempdir");
        let queries = setup_db(&db_dir, "oversize-md").await;
        let config = RegistryConfig {
            sources: vec![docs_source()],
            max_file_size_bytes: 8, // tiny limit; the document below exceeds it
            batch_size: 50,
            ..RegistryConfig::default()
        };

        let file = write_markdown(
            workspace.path(),
            "docs/big.md",
            "# Big\n\nThis conformant document exceeds the tiny size limit.\n",
        );

        let outcome = verify_gated_reingest(&file, workspace.path(), &config, &queries)
            .await
            .expect("gate should not error on oversize markdown");
        assert_eq!(outcome, ReingestOutcome::SkippedOversize);

        let records = queries
            .select_content_records(Some("docs"))
            .await
            .expect("select docs content records");
        assert!(
            records.is_empty(),
            "oversize markdown must not write any content record"
        );
    }
}
