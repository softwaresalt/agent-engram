//! Integration tests for graph resolution-recall eval compute (081.005-T).
//!
//! Exercises the four plan scenarios for the graph slice:
//! 1. all-local resolved → `resolution_recall` ≈ 1.0;
//! 2. cross-file drop → `resolution_recall` < 1.0;
//! 3. edge to non-existent def → `false_edge_rate` > 0;
//! 4. empty graph → zeroed report.
//!
//! Plus a denominator sanity check that the parser call-site inventory
//! (`count_call_sites`) matches the expected identifier-call count.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::models::{CodeFile, Function};
use engram::services::code_graph;
use engram::services::parsing::Language;
use engram::services::parsing::canonical::{self, CanonicalWorkspace};
use engram::services::retrieval_eval::{
    CallSiteInventory, CallSiteResolutionContext, compute_graph_metrics, count_call_sites,
    scan_call_site_inventory, scan_call_site_inventory_with_resolution, source_content_hash,
};

#[test]
fn all_local_resolved_gives_full_recall() {
    // Every visible call site resolved to a local def.
    let metrics = compute_graph_metrics(5, 5, 0);
    assert!(
        (metrics.resolution_recall - 1.0).abs() < 1e-9,
        "expected recall 1.0, got {}",
        metrics.resolution_recall
    );
    assert!((metrics.false_edge_rate - 0.0).abs() < 1e-9);
    assert_eq!(metrics.call_sites, 5);
    assert_eq!(metrics.resolved, 5);
}

#[test]
fn cross_file_calls_drop_recall_below_one() {
    // Ten visible call sites, only six resolved within-file.
    let metrics = compute_graph_metrics(10, 6, 0);
    assert!(
        (metrics.resolution_recall - 0.6).abs() < 1e-9,
        "expected recall 0.6, got {}",
        metrics.resolution_recall
    );
    assert!(
        metrics.resolution_recall < 1.0,
        "cross-file drop must lower recall below 1.0"
    );
}

#[test]
fn dangling_callee_raises_false_edge_rate() {
    // Two of eight resolved edges point at a callee with no matching def.
    let metrics = compute_graph_metrics(10, 8, 2);
    assert!(
        (metrics.false_edge_rate - 0.25).abs() < 1e-9,
        "expected false-edge rate 0.25, got {}",
        metrics.false_edge_rate
    );
    assert!(
        metrics.false_edge_rate > 0.0,
        "a dangling callee must yield a positive false-edge rate"
    );
}

#[test]
fn empty_graph_yields_zero_report() {
    let metrics = compute_graph_metrics(0, 0, 0);
    assert!((metrics.resolution_recall - 0.0).abs() < 1e-9);
    assert!((metrics.false_edge_rate - 0.0).abs() < 1e-9);
    assert_eq!(metrics.call_sites, 0);
    assert_eq!(metrics.resolved, 0);
    assert_eq!(metrics.false_edges, 0);
}

#[test]
fn count_call_sites_matches_identifier_calls() {
    // `foo` calls `bar` and `baz`; those two definitions make no calls.
    // Blocklisted helpers are excluded by the parser; method calls are excluded
    // from the denominator (see count_call_sites_excludes_method_calls).
    let source = r"
fn foo() {
    bar();
    baz();
}
fn bar() {}
fn baz() {}
";
    assert_eq!(count_call_sites(source, Language::Rust), 2);
}

#[test]
fn count_call_sites_counts_known_receiver_and_excludes_arbitrary_methods() {
    // Unit B attempts canonical resolution for known-receiver `self.method()`,
    // so it contributes to the denominator. Arbitrary `x.method()` remains
    // dropped and must not inflate recall. The free-function call also counts.
    let source = r"
fn foo() {
    self.method_one();
    x.method_two();
    free_fn();
}
fn free_fn() {}
";
    assert_eq!(count_call_sites(source, Language::Rust), 2);
}

// ── 084.002-T (88B5FAFD): denominator counts distinct (caller,callee) units ───

#[test]
fn count_call_sites_dedupes_repeated_caller_callee() {
    // The numerator counts distinct `(from,to)` call *relations* (a `calls_edge`
    // is keyed by `(from,to)`), so a caller invoking the same callee twice is ONE
    // relation, not two. The denominator must count the same unit — distinct
    // `(caller,callee)` pairs — so recall is a ratio of commensurable units and a
    // double-call does not deflate it.
    let source = r"
fn foo() {
    bar();
    bar();
}
fn bar() {}
";
    assert_eq!(
        count_call_sites(source, Language::Rust),
        1,
        "a repeated (caller,callee) call is one distinct relation, not two"
    );
}

#[test]
fn count_call_sites_counts_distinct_relations() {
    // Two distinct callees from one caller = two relations; repeating one of them
    // does not add a third.
    let source = r"
fn foo() {
    bar();
    baz();
    bar();
}
fn bar() {}
fn baz() {}
";
    assert_eq!(
        count_call_sites(source, Language::Rust),
        2,
        "distinct (foo,bar) and (foo,baz); the repeat of bar adds nothing"
    );
}

// ── 084.002-T (D6F70DCC): numerator gated to configured caller languages ──────

fn make_file(path: &str, id: &str, language: &str) -> CodeFile {
    CodeFile {
        id: id.to_owned(),
        path: path.to_owned(),
        language: language.to_owned(),
        size_bytes: 0,
        content_hash: "hash".to_owned(),
        last_indexed_at: "2026-07-11T00:00:00Z".to_owned(),
    }
}

fn make_fn(id: &str, name: &str, file_path: &str) -> Function {
    Function {
        id: id.to_owned(),
        name: name.to_owned(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 2,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: String::new(),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        embedding: Vec::new(),
        summary: String::new(),
    }
}

fn write_file(ws: &Path, rel: &str, content: &str) {
    let full = ws.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write source");
}

fn write_manifest(ws: &Path) {
    write_file(
        ws,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
}

fn write_workspace_manifest_with_rename(ws: &Path) {
    write_file(
        ws,
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\", \"util\"]\nresolver = \"3\"\n",
    );
    write_file(
        ws,
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nutil = { package = \"external-util\", version = \"1\" }\n",
    );
    write_file(
        ws,
        "util/Cargo.toml",
        "[package]\nname = \"util\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
}

fn write_workspace_manifest_without_rename(ws: &Path) {
    write_file(
        ws,
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\", \"util\"]\nresolver = \"3\"\n",
    );
    write_file(
        ws,
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write_file(
        ws,
        "util/Cargo.toml",
        "[package]\nname = \"util\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
}

fn fixture_db_params(path: &Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

async fn index_fixture(ws: &Path) -> CodeGraphQueries {
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = fixture_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

async fn resolution_context(q: &CodeGraphQueries) -> CallSiteResolutionContext {
    let mut resolved_edges: HashSet<(String, String)> = HashSet::new();
    for resolution in [
        "direct",
        "calls_resolved_singleton",
        "calls_resolved_canonical",
    ] {
        resolved_edges.extend(
            q.list_calls_edges_by_resolution(resolution)
                .await
                .expect("list calls edges"),
        );
    }
    CallSiteResolutionContext::new(
        q.all_functions_for_eval()
            .await
            .expect("functions for eval"),
        resolved_edges,
        q.function_ids_by_canonical_path()
            .await
            .expect("canonical index"),
        q.load_index_canonical_workspace_snapshot()
            .await
            .expect("canonical workspace"),
    )
}

async fn indexed_rust_files(q: &CodeGraphQueries) -> Vec<CodeFile> {
    q.list_code_files()
        .await
        .expect("code files")
        .into_iter()
        .filter(|file| file.language == "rust")
        .collect()
}

// ── 091.020-T: denominator reconciles with resolved edge identity ─────────────

#[tokio::test]
async fn resolution_aware_denominator_collapses_same_target_spellings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod thing;\n");
    write_file(
        ws,
        "src/thing.rs",
        "pub fn helper() {}\npub fn caller() {\n    helper();\n    crate::thing::helper();\n}\n",
    );

    assert_eq!(
        count_call_sites(
            "pub fn helper() {}\npub fn caller() {\n    helper();\n    crate::thing::helper();\n}\n",
            Language::Rust,
        ),
        2,
        "syntactic inventory sees the two spellings before graph identity reconciliation"
    );

    let q = index_fixture(ws).await;
    let files = indexed_rust_files(&q).await;
    let context = resolution_context(&q).await;
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan");
    let resolved = q.count_calls_edges().await.expect("resolved count");
    let metrics = compute_graph_metrics(inventory.call_sites, resolved, 0);

    assert_eq!(
        inventory.call_sites, 1,
        "two spellings proven to share one (caller,target) edge are one expected edge"
    );
    assert_eq!(resolved, 1, "the graph stores one deduplicated edge");
    assert!((metrics.resolution_recall - 1.0).abs() < 1e-9);
}

#[tokio::test]
async fn resolution_aware_denominator_preserves_distinct_target_miss() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod a;\npub mod caller;\n");
    write_file(ws, "src/a.rs", "pub fn build() {}\n");
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() {\n    crate::a::build();\n    crate::b::build();\n}\n",
    );

    let q = index_fixture(ws).await;
    let files = indexed_rust_files(&q).await;
    let context = resolution_context(&q).await;
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan");
    let resolved = q.count_calls_edges().await.expect("resolved count");
    let metrics = compute_graph_metrics(inventory.call_sites, resolved, 0);

    assert_eq!(
        inventory.call_sites, 2,
        "the unresolved distinct target stays in the denominator rather than being hidden"
    );
    assert_eq!(resolved, 1, "only the existing target resolves");
    assert!((metrics.resolution_recall - 0.5).abs() < 1e-9);
}

#[tokio::test]
async fn resolution_aware_denominator_preserves_ambiguous_bare_miss() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "pub mod a;\npub mod b;\npub mod caller;\n",
    );
    write_file(ws, "src/a.rs", "pub fn build() {}\n");
    write_file(ws, "src/b.rs", "pub fn build() {}\n");
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() {\n    crate::a::build();\n    build();\n}\n",
    );

    let q = index_fixture(ws).await;
    let files = indexed_rust_files(&q).await;
    let context = resolution_context(&q).await;
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan");
    let resolved = q.count_calls_edges().await.expect("resolved count");
    let metrics = compute_graph_metrics(inventory.call_sites, resolved, 0);

    assert_eq!(
        inventory.call_sites, 2,
        "an ambiguous bare call must not inherit a same-named qualified edge"
    );
    assert_eq!(resolved, 1, "only the qualified call resolves");
    assert!((metrics.resolution_recall - 0.5).abs() < 1e-9);
}

#[tokio::test]
async fn resolution_aware_denominator_preserves_skipped_remap_miss() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "pub mod caller;\npub mod outer {\n    #[path = \"actual.rs\"]\n    pub mod inner;\n}\n",
    );
    let caller_source = "pub fn caller() {\n    helper();\n    crate::outer::inner::helper();\n}\n";
    write_file(ws, "src/caller.rs", caller_source);

    let files = vec![CodeFile {
        id: "file:caller".to_owned(),
        path: "src/caller.rs".to_owned(),
        language: "rust".to_owned(),
        size_bytes: caller_source.len() as u64,
        content_hash: source_content_hash(caller_source),
        last_indexed_at: "2026-07-17T00:00:00Z".to_owned(),
    }];
    let mut resolved_edges = HashSet::new();
    resolved_edges.insert(("fn:caller".to_owned(), "fn:helper".to_owned()));
    let mut canonical_index = HashMap::new();
    canonical_index.insert(
        "demo::outer::inner::helper".to_owned(),
        vec!["fn:helper".to_owned()],
    );
    let unsafe_prefixes = HashSet::from(["demo::outer::inner".to_owned()]);
    let canonical_workspace = CanonicalWorkspace {
        crates: canonical::discover_workspace_crates(ws),
        unsafe_prefixes,
    };
    let context = CallSiteResolutionContext::new(
        vec![
            make_fn("fn:caller", "caller", "src/caller.rs"),
            make_fn("fn:helper", "helper", "src/outer/inner.rs"),
        ],
        resolved_edges,
        canonical_index,
        Some(canonical_workspace),
    );

    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan");
    let metrics = compute_graph_metrics(inventory.call_sites, 1, 0);

    assert_eq!(
        inventory.call_sites, 2,
        "discover-only remap prefixes must preserve the qualified miss instead of sharing the singleton edge"
    );
    assert!(
        metrics.resolution_recall < 1.0,
        "the skipped mod-declaring file makes the qualified call a real miss"
    );
}

#[tokio::test]
async fn resolution_aware_denominator_disables_collapse_without_persisted_prefixes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() {\n    helper();\n    crate::outer::inner::helper();\n}\n",
    );
    let files = vec![CodeFile {
        id: "file:caller".to_owned(),
        path: "src/caller.rs".to_owned(),
        language: "rust".to_owned(),
        size_bytes: 0,
        content_hash: source_content_hash(
            "pub fn caller() {\n    helper();\n    crate::outer::inner::helper();\n}\n",
        ),
        last_indexed_at: "2026-07-17T00:00:00Z".to_owned(),
    }];
    let mut resolved_edges = HashSet::new();
    resolved_edges.insert(("fn:caller".to_owned(), "fn:helper".to_owned()));
    let mut canonical_index = HashMap::new();
    canonical_index.insert(
        "demo::outer::inner::helper".to_owned(),
        vec!["fn:helper".to_owned()],
    );
    let legacy_db = connect_db(&ws.join(".legacy-db"), "legacy")
        .await
        .expect("legacy db");
    let legacy_queries = CodeGraphQueries::new(legacy_db);
    let legacy_snapshot = legacy_queries
        .load_index_canonical_workspace_snapshot()
        .await
        .expect("load legacy snapshot");
    assert!(
        legacy_snapshot.is_none(),
        "a database without the snapshot relation must be distinguishable from an empty persisted set"
    );
    let context = CallSiteResolutionContext::new(
        vec![
            make_fn("fn:caller", "caller", "src/caller.rs"),
            make_fn("fn:helper", "helper", "src/outer/inner.rs"),
        ],
        resolved_edges,
        canonical_index,
        legacy_snapshot,
    );

    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan");

    assert_eq!(
        inventory.call_sites, 2,
        "missing index-time prefix snapshot must fall back to syntax-only counting"
    );
}

#[tokio::test]
async fn resolution_aware_denominator_uses_persisted_prefixes_after_remap_removed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "pub mod caller;\npub mod outer {\n    #[path = \"actual.rs\"]\n    pub mod inner;\n}\n",
    );
    let caller_source = "pub fn caller() {\n    helper();\n    crate::outer::inner::helper();\n}\n";
    write_file(ws, "src/caller.rs", caller_source);
    write_file(ws, "src/outer/inner.rs", "pub fn helper() {}\n");

    let q = index_fixture(ws).await;
    let resolved = q.count_calls_edges().await.expect("resolved count");
    assert_eq!(
        resolved, 1,
        "the singleton edge is resolved while the unsafe qualified call is not"
    );
    let snapshot = q
        .load_index_canonical_workspace_snapshot()
        .await
        .expect("load snapshot")
        .expect("persisted canonical workspace");
    assert!(
        snapshot.unsafe_prefixes.contains("demo::outer::inner"),
        "index-time remap prefix must be persisted with the edge snapshot"
    );

    write_file(
        ws,
        "src/lib.rs",
        "pub mod caller;\npub mod outer {\n    pub mod inner;\n}\n",
    );
    let files = indexed_rust_files(&q).await;
    let base_context = resolution_context(&q).await;
    let caller_id = base_context
        .functions
        .iter()
        .find(|function| function.name == "caller")
        .expect("caller function")
        .id
        .clone();
    let helper_id = base_context
        .functions
        .iter()
        .find(|function| function.name == "helper")
        .expect("helper function")
        .id
        .clone();
    assert!(
        base_context
            .resolved_edges
            .contains(&(caller_id.clone(), helper_id.clone())),
        "the coincidental singleton edge exists in the numerator identity space"
    );
    let mut stale_canonical_index = base_context.canonical_index.clone();
    stale_canonical_index.insert("demo::outer::inner::helper".to_owned(), vec![helper_id]);
    let context = CallSiteResolutionContext::new(
        base_context.functions.clone(),
        base_context.resolved_edges.clone(),
        stale_canonical_index.clone(),
        Some(snapshot),
    );
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan");
    let metrics = compute_graph_metrics(inventory.call_sites, resolved, 0);

    assert_eq!(
        inventory.call_sites, 2,
        "the persisted index-time prefix preserves the qualified miss even when current disk lacks the remap declaration"
    );
    assert!(
        inventory.index_stale,
        "the changed declaring file remains visible as stale inventory"
    );
    assert!(metrics.resolution_recall < 1.0);

    let shrunken_prefix_context = CallSiteResolutionContext::new(
        base_context.functions,
        base_context.resolved_edges,
        stale_canonical_index,
        Some(CanonicalWorkspace {
            crates: canonical::discover_workspace_crates(ws),
            unsafe_prefixes: HashSet::new(),
        }),
    );
    let collapsed_inventory = scan_call_site_inventory_with_resolution(
        ws,
        &files,
        &["rust".to_owned()],
        &shrunken_prefix_context,
    )
    .await
    .expect("scan with shrunken prefixes");
    assert_eq!(
        collapsed_inventory.call_sites, 1,
        "non-vacuous guard: a current-disk recompute that lost the prefix would collapse the miss"
    );
}

#[tokio::test]
async fn canonical_snapshot_removed_after_incremental_context_drift() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "pub mod caller;\npub mod outer {\n    #[path = \"actual.rs\"]\n    pub mod inner;\n}\n",
    );
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() {\n    helper();\n    crate::outer::inner::helper();\n}\n",
    );
    write_file(ws, "src/outer/inner.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = fixture_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("full index");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let q = CodeGraphQueries::new(db);
    assert!(
        q.load_index_canonical_workspace_snapshot()
            .await
            .expect("load full snapshot")
            .is_some(),
        "full index establishes the baseline snapshot"
    );

    write_file(
        ws,
        "src/lib.rs",
        "pub mod caller;\npub mod outer {\n    pub mod inner;\n}\n",
    );
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("incremental sync");
    let snapshot = q
        .load_index_canonical_workspace_snapshot()
        .await
        .expect("load drift snapshot");
    assert!(
        snapshot.is_none(),
        "incremental context drift must leave canonical collapse disabled"
    );

    let files = indexed_rust_files(&q).await;
    let base_context = resolution_context(&q).await;
    let helper_id = base_context
        .functions
        .iter()
        .find(|function| function.name == "helper")
        .expect("helper function")
        .id
        .clone();
    let mut stale_canonical_index = base_context.canonical_index.clone();
    stale_canonical_index.insert("demo::outer::inner::helper".to_owned(), vec![helper_id]);
    let context = CallSiteResolutionContext::new(
        base_context.functions.clone(),
        base_context.resolved_edges.clone(),
        stale_canonical_index.clone(),
        snapshot,
    );
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan with drift-disabled snapshot");
    assert_eq!(
        inventory.call_sites, 2,
        "drift-disabled collapse preserves the qualified miss"
    );

    let shrunken_context = CallSiteResolutionContext::new(
        base_context.functions,
        base_context.resolved_edges,
        stale_canonical_index,
        Some(CanonicalWorkspace {
            crates: canonical::discover_workspace_crates(ws),
            unsafe_prefixes: HashSet::new(),
        }),
    );
    let collapsed = scan_call_site_inventory_with_resolution(
        ws,
        &files,
        &["rust".to_owned()],
        &shrunken_context,
    )
    .await
    .expect("scan with shrunken snapshot");
    assert_eq!(
        collapsed.call_sites, 1,
        "control: replacing the snapshot with the shrunken context would collapse the miss"
    );
}

#[tokio::test]
async fn incremental_sync_without_prior_snapshot_keeps_collapse_disabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "pub fn helper() {}\npub fn caller() {\n    helper();\n    crate::helper();\n}\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = fixture_db_params(ws);
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("baseline-less sync");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let q = CodeGraphQueries::new(db);
    let snapshot = q
        .load_index_canonical_workspace_snapshot()
        .await
        .expect("load baseline-less snapshot");
    assert!(
        snapshot.is_none(),
        "incremental sync without a prior full-index baseline must not enable collapse"
    );

    let files = indexed_rust_files(&q).await;
    let context = resolution_context(&q).await;
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan baseline-less sync");
    assert_eq!(
        inventory.call_sites, 2,
        "baseline-less sync falls back to syntax-only counting"
    );
}

#[tokio::test]
async fn canonical_snapshot_uses_persisted_dependency_renames() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_workspace_manifest_with_rename(ws);
    write_file(
        ws,
        "app/src/lib.rs",
        "pub fn caller() {\n    helper();\n    util::thing::helper();\n}\n",
    );
    write_file(ws, "util/src/lib.rs", "pub mod thing;\n");
    write_file(ws, "util/src/thing.rs", "pub fn helper() {}\n");

    let q = index_fixture(ws).await;
    let resolved = q.count_calls_edges().await.expect("resolved count");
    assert_eq!(
        resolved, 1,
        "the bare singleton edge exists while the dependency-renamed qualified call is missed"
    );

    write_workspace_manifest_without_rename(ws);
    let files = indexed_rust_files(&q).await;
    let context = resolution_context(&q).await;
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan persisted rename");
    let metrics = compute_graph_metrics(inventory.call_sites, resolved, 0);
    assert_eq!(
        inventory.call_sites, 2,
        "eval must use the index-time dependency rename context, not the live manifest"
    );
    assert!(metrics.resolution_recall < 1.0);

    let live_prefix_context = CallSiteResolutionContext::new(
        context.functions,
        context.resolved_edges,
        context.canonical_index,
        Some(CanonicalWorkspace {
            crates: canonical::discover_workspace_crates(ws),
            unsafe_prefixes: HashSet::new(),
        }),
    );
    let collapsed = scan_call_site_inventory_with_resolution(
        ws,
        &files,
        &["rust".to_owned()],
        &live_prefix_context,
    )
    .await
    .expect("scan with live manifest context");
    assert_eq!(
        collapsed.call_sites, 1,
        "control: rebuilding crates from the live manifest would collapse the miss"
    );
}

#[tokio::test]
async fn resolution_aware_denominator_preserves_use_graph_drift_miss() {
    let original_source = "use external::thing as Alias;\n\npub fn caller() {\n    helper();\n    Alias::helper();\n}\n";
    let drifted_source =
        "use crate::thing as Alias;\n\npub fn caller() {\n    helper();\n    Alias::helper();\n}\n";

    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod caller;\npub mod thing;\n");
    write_file(ws, "src/thing.rs", "pub fn helper() {}\n");
    write_file(ws, "src/caller.rs", original_source);

    let q = index_fixture(ws).await;
    assert!(
        q.load_index_canonical_workspace_snapshot()
            .await
            .expect("load snapshot")
            .is_some(),
        "full index must publish a canonical workspace snapshot"
    );
    let resolved = q.count_calls_edges().await.expect("resolved count");
    assert_eq!(
        resolved, 1,
        "index-time external alias keeps the qualified call missed while the bare singleton resolves"
    );

    write_file(ws, "src/caller.rs", drifted_source);
    let files = indexed_rust_files(&q).await;
    let context = resolution_context(&q).await;
    let inventory =
        scan_call_site_inventory_with_resolution(ws, &files, &["rust".to_owned()], &context)
            .await
            .expect("scan drifted use graph");
    let metrics = compute_graph_metrics(inventory.call_sites, resolved, 0);
    assert!(
        inventory.index_stale,
        "the caller file hash must reveal that eval reparsed a non-index-time use graph"
    );
    assert_eq!(
        inventory.call_sites, 2,
        "stale caller files must use syntax-only counting so the index-time alias miss is preserved"
    );
    assert!(metrics.resolution_recall < 1.0);

    let fresh_tmp = tempfile::tempdir().expect("fresh tempdir");
    let fresh_ws = fresh_tmp.path();
    write_manifest(fresh_ws);
    write_file(fresh_ws, "src/lib.rs", "pub mod caller;\npub mod thing;\n");
    write_file(fresh_ws, "src/thing.rs", "pub fn helper() {}\n");
    write_file(fresh_ws, "src/caller.rs", drifted_source);
    let fresh_q = index_fixture(fresh_ws).await;
    let fresh_files = indexed_rust_files(&fresh_q).await;
    let fresh_context = resolution_context(&fresh_q).await;
    let fresh_inventory = scan_call_site_inventory_with_resolution(
        fresh_ws,
        &fresh_files,
        &["rust".to_owned()],
        &fresh_context,
    )
    .await
    .expect("scan fresh use graph");
    let fresh_resolved = fresh_q.count_calls_edges().await.expect("fresh resolved");
    let fresh_metrics = compute_graph_metrics(fresh_inventory.call_sites, fresh_resolved, 0);
    assert!(
        !fresh_inventory.index_stale,
        "fresh control must keep resolution-aware collapse enabled"
    );
    assert_eq!(
        fresh_inventory.call_sites, 1,
        "fresh alias and bare call proven to share one edge still collapse"
    );
    assert!((fresh_metrics.resolution_recall - 1.0).abs() < 1e-9);
}

/// Build a graph with two resolved edges: one whose caller lives in a Rust file
/// and one whose caller lives in a TypeScript file, both calling the same target.
async fn graph_with_multilang_edges() -> (tempfile::TempDir, CodeGraphQueries) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db = connect_db(tmp.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    queries
        .upsert_code_file(&make_file("src/a.rs", "file:a", "rust"))
        .await
        .expect("file a");
    queries
        .upsert_code_file(&make_file("src/b.ts", "file:b", "typescript"))
        .await
        .expect("file b");

    queries
        .upsert_function(&make_fn("fn:caller_rs", "caller_rs", "src/a.rs"))
        .await
        .expect("caller_rs");
    queries
        .upsert_function(&make_fn("fn:caller_ts", "caller_ts", "src/b.ts"))
        .await
        .expect("caller_ts");
    queries
        .upsert_function(&make_fn("fn:target", "target", "src/a.rs"))
        .await
        .expect("target");

    queries
        .create_calls_edge("fn:caller_rs", "fn:target")
        .await
        .expect("edge rs");
    queries
        .create_calls_edge("fn:caller_ts", "fn:target")
        .await
        .expect("edge ts");

    (tmp, queries)
}

#[tokio::test]
async fn numerator_gated_to_configured_language_excludes_others() {
    // Ungated, both edges count. Gated to ["rust"], only the edge whose caller
    // resides in a Rust file counts — the TypeScript-caller edge is excluded so
    // the numerator matches the language-gated call-site denominator.
    let (_tmp, queries) = graph_with_multilang_edges().await;
    assert_eq!(
        queries.count_calls_edges().await.expect("ungated count"),
        2,
        "sanity: two resolved edges exist in total"
    );
    let rust_only = queries
        .count_calls_edges_in_languages(&["rust".to_owned()])
        .await
        .expect("gated count");
    assert_eq!(
        rust_only, 1,
        "only the Rust-file caller's edge is in scope; the TypeScript edge is excluded"
    );
}

#[tokio::test]
async fn numerator_language_gate_is_case_insensitive() {
    // Config languages are matched case-insensitively (mirrors the call-site
    // denominator gate), so "Rust" still selects the rust-file caller's edge.
    let (_tmp, queries) = graph_with_multilang_edges().await;
    let rust_only = queries
        .count_calls_edges_in_languages(&["Rust".to_owned()])
        .await
        .expect("gated count");
    assert_eq!(rust_only, 1);
}

#[tokio::test]
async fn numerator_empty_languages_counts_all_edges() {
    // An empty language list disables the gate (opt-in), counting every edge —
    // parity with the call-site denominator's empty-gate behavior.
    let (_tmp, queries) = graph_with_multilang_edges().await;
    let all = queries
        .count_calls_edges_in_languages(&[])
        .await
        .expect("ungated-via-empty count");
    assert_eq!(all, 2, "empty languages disables the numerator gate");
}

// ── 084.008-T (Thread-8): dangling (false-edge) numerator shares the gate ─────

/// Build a graph with two DANGLING edges (callee id matches no definition): one
/// whose caller lives in a Rust file, one whose caller lives in a TypeScript
/// file. Each caller has a real `function_meta`; only the callee is missing.
async fn graph_with_multilang_dangling_edges() -> (tempfile::TempDir, CodeGraphQueries) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db = connect_db(tmp.path(), "dangling-lang-gate")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    queries
        .upsert_code_file(&make_file("src/a.rs", "file:a", "rust"))
        .await
        .expect("file a");
    queries
        .upsert_code_file(&make_file("src/b.ts", "file:b", "typescript"))
        .await
        .expect("file b");
    queries
        .upsert_function(&make_fn("fn:caller_rs", "caller_rs", "src/a.rs"))
        .await
        .expect("caller_rs");
    queries
        .upsert_function(&make_fn("fn:caller_ts", "caller_ts", "src/b.ts"))
        .await
        .expect("caller_ts");

    // Dangling: the callee ids have NO `function_meta` row (stale / unresolved),
    // but each caller is a real in-language function.
    queries
        .create_calls_edge("fn:caller_rs", "fn:ghost_rs")
        .await
        .expect("dangling rs");
    queries
        .create_calls_edge("fn:caller_ts", "fn:ghost_ts")
        .await
        .expect("dangling ts");

    (tmp, queries)
}

#[tokio::test]
async fn dangling_count_gated_to_configured_language_excludes_others() {
    // Both dangling edges count ungated; gated to ["rust"], only the dangling
    // edge whose caller resides in a Rust file counts — so `false_edge_rate`'s
    // numerator is scoped identically to its resolved denominator and a stale
    // TypeScript edge cannot inflate (or clamp to 1.0) a Rust-only run's
    // false-edge rate (084.008-T / Thread-8).
    let (_tmp, queries) = graph_with_multilang_dangling_edges().await;
    assert_eq!(
        queries
            .count_dangling_calls_edges()
            .await
            .expect("ungated dangling"),
        2,
        "sanity: two dangling edges exist in total"
    );
    let rust_only = queries
        .count_dangling_calls_edges_in_languages(&["rust".to_owned()])
        .await
        .expect("gated dangling");
    assert_eq!(
        rust_only, 1,
        "only the Rust-file caller's dangling edge is in scope; the TypeScript one is excluded"
    );
    // Case-insensitive, matching the resolved-numerator gate.
    let rust_ci = queries
        .count_dangling_calls_edges_in_languages(&["Rust".to_owned()])
        .await
        .expect("gated dangling ci");
    assert_eq!(rust_ci, 1, "language match is case-insensitive");
    // Empty languages disables the gate (opt-in parity with the resolved count).
    let all = queries
        .count_dangling_calls_edges_in_languages(&[])
        .await
        .expect("empty-gate dangling");
    assert_eq!(all, 2, "empty languages disables the dangling gate");
}

// ── 084.003-T (54848E3D): index/generation consistency gate ──────────────────

const RS_SOURCE: &str = "fn helper() {}\nfn caller() {\n    helper();\n}\n";

fn code_file(path: &str, language: &str, content_hash: &str) -> CodeFile {
    CodeFile {
        id: format!("code_file:{path}"),
        path: path.to_owned(),
        language: language.to_owned(),
        size_bytes: 0,
        content_hash: content_hash.to_owned(),
        last_indexed_at: "2026-07-11T00:00:00Z".to_owned(),
    }
}

#[tokio::test]
async fn scan_clean_index_is_not_stale_and_counts_denominator() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("src")).expect("mkdir");
    std::fs::write(ws.join("src/thing.rs"), RS_SOURCE).expect("write");
    // Recorded hash matches the on-disk content, as it would at index time.
    let files = vec![code_file(
        "src/thing.rs",
        "rust",
        &source_content_hash(RS_SOURCE),
    )];

    let inv = scan_call_site_inventory(ws, &files, &["rust".to_owned()])
        .await
        .expect("scan");
    assert!(
        !inv.index_stale,
        "an unmodified tree must not be flagged stale"
    );
    assert_eq!(inv.unreadable_files, 0, "every indexed file is readable");
    assert_eq!(
        inv.call_sites, 1,
        "the single (caller,helper) relation is the denominator"
    );
}

#[tokio::test]
async fn scan_flags_stale_when_file_edited_after_index() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("src")).expect("mkdir");
    std::fs::write(ws.join("src/thing.rs"), RS_SOURCE).expect("write");
    // Record the ORIGINAL content's hash, then edit the file on disk without
    // re-indexing: the on-disk content now diverges from the recorded hash.
    let files = vec![code_file(
        "src/thing.rs",
        "rust",
        &source_content_hash(RS_SOURCE),
    )];
    std::fs::write(
        ws.join("src/thing.rs"),
        "fn helper() {}\nfn caller() {\n    helper();\n    helper();\n}\n",
    )
    .expect("rewrite");

    let inv = scan_call_site_inventory(ws, &files, &["rust".to_owned()])
        .await
        .expect("scan");
    assert!(
        inv.index_stale,
        "a file edited after indexing must flag index_stale, not be silently clamped"
    );
}

#[tokio::test]
async fn scan_accounts_unreadable_indexed_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // An indexed file that no longer exists on disk (deleted since index time).
    let files = vec![code_file("src/gone.rs", "rust", "somehash")];

    let inv = scan_call_site_inventory(tmp.path(), &files, &["rust".to_owned()])
        .await
        .expect("scan");
    assert_eq!(
        inv.unreadable_files, 1,
        "a deleted-but-indexed file must be accounted, not silently dropped"
    );
    assert_eq!(
        inv.call_sites, 0,
        "an unreadable file contributes no call sites"
    );
    assert!(
        !inv.index_stale,
        "a missing file is accounted as unreadable, not conflated with content drift"
    );
}

#[tokio::test]
async fn scan_empty_inventory_is_zeroed_and_not_stale() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let inv = scan_call_site_inventory(tmp.path(), &[], &["rust".to_owned()])
        .await
        .expect("scan");
    assert_eq!(inv, CallSiteInventory::default());
}

#[tokio::test]
async fn scan_language_gate_excludes_nonconfigured_files() {
    // A file outside the configured languages is skipped by the denominator scan
    // (no read, no staleness/unreadable accounting), matching the numerator gate.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // No file on disk: were it scanned, it would be counted unreadable. It must
    // instead be gated out entirely because its language is not configured.
    let files = vec![code_file("src/app.ts", "typescript", "h")];

    let inv = scan_call_site_inventory(ws, &files, &["rust".to_owned()])
        .await
        .expect("scan");
    assert_eq!(
        inv,
        CallSiteInventory::default(),
        "non-configured-language files are gated out before any read"
    );
}

// ── 084.011-T (CA401F5F): bounded-batch parsing preserves the denominator ─────

#[tokio::test]
async fn scan_counts_are_invariant_across_batch_boundaries() {
    // The denominator scan parses files in bounded batches so peak memory stays
    // bounded by one batch rather than the whole corpus. This exercises a corpus
    // far larger than the internal batch size (many full batches plus a partial
    // final batch) and asserts the total is exactly the per-file sum — a dropped
    // final partial batch or a double-counted batch boundary would fail here.
    const FILE_COUNT: usize = 210;
    const PER_FILE_RELATIONS: usize = 2; // (caller,helper_a) and (caller,helper_b)
    let src =
        "fn caller() {\n    helper_a();\n    helper_b();\n}\nfn helper_a() {}\nfn helper_b() {}\n";
    let hash = source_content_hash(src);

    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("src")).expect("mkdir");

    let mut files = Vec::with_capacity(FILE_COUNT);
    for i in 0..FILE_COUNT {
        let path = format!("src/f{i}.rs");
        std::fs::write(ws.join(&path), src).expect("write source");
        files.push(code_file(&path, "rust", &hash));
    }

    let inv = scan_call_site_inventory(ws, &files, &["rust".to_owned()])
        .await
        .expect("scan");

    assert_eq!(
        inv.call_sites,
        FILE_COUNT * PER_FILE_RELATIONS,
        "batched parsing must sum every file's call sites, including the final partial batch"
    );
    assert_eq!(
        inv.unreadable_files, 0,
        "every fixture file is readable across all batches"
    );
    assert!(
        !inv.index_stale,
        "every recorded hash matches its on-disk content across all batches"
    );
}
