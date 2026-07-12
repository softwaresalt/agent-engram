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
use engram::services::retrieval_eval::{compute_graph_metrics, count_call_sites};

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
