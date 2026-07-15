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

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::{CodeFile, Function};
use engram::services::parsing::Language;
use engram::services::retrieval_eval::{
    CallSiteInventory, compute_graph_metrics, count_call_sites, scan_call_site_inventory,
    source_content_hash,
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
fn count_call_sites_excludes_method_calls() {
    // Method / receiver calls are extracted but never promoted to edges, so they
    // must NOT inflate the resolution-recall denominator. Only the free-function
    // call `free_fn()` counts here; `self.method_one()` and `x.method_two()` do
    // not.
    let source = r"
fn foo() {
    self.method_one();
    x.method_two();
    free_fn();
}
fn free_fn() {}
";
    assert_eq!(count_call_sites(source, Language::Rust), 1);
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

// ── 088-F: qualified-call denominator identity + deferral exclusion ────────────

#[test]
fn count_call_sites_distinguishes_qualified_paths_with_same_final_name() {
    // Two crate-rooted qualified calls with the same final segment but different
    // module paths are DISTINCT call sites — the denominator must not collapse
    // them into one entry (which, since they resolve ambiguously to no edge,
    // would shrink the denominator and inflate resolution_recall).
    let source = r"
fn foo() {
    crate::a::helper();
    crate::b::helper();
}
";
    assert_eq!(
        count_call_sites(source, Language::Rust),
        2,
        "crate::a::helper and crate::b::helper are two distinct qualified call sites"
    );
}

#[test]
fn count_call_sites_excludes_deferred_qualified_calls() {
    // Only calls the indexer attempts to resolve count. A bare (non-crate-rooted)
    // `Widget::build()` and an external `mem::swap()` are deferred (they cannot be
    // shown to reference a workspace symbol), so they must NOT inflate the
    // denominator; the crate-rooted call and the bare free call do.
    let source = r"
fn foo() {
    Widget::build();
    mem::swap();
    crate::helper();
    bare();
}
";
    assert_eq!(
        count_call_sites(source, Language::Rust),
        2,
        "only crate::helper and bare are attempted; Widget::build and mem::swap defer"
    );
}

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
