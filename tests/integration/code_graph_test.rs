//! Integration tests for code graph indexing (US1: `index_workspace`).
//!
//! Creates a temporary workspace with sample Rust files, calls the indexing
//! service directly, and verifies that code files, functions, classes,
//! interfaces, and edges are correctly persisted.

use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

/// Helper: write a sample Rust file into the workspace.
fn write_sample_file(dir: &Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

/// Helper: derive test DB parameters from a workspace path.
fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

#[test]
async fn index_workspace_parses_rust_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(
        ws,
        "src/lib.rs",
        r#"
/// A greeter function.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

/// A helper struct.
pub struct Config {
    pub debug: bool,
}
"#,
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    assert_eq!(result.files_parsed, 1);
    assert!(
        result.functions_indexed >= 1,
        "should index at least the greet function"
    );
    assert!(
        result.classes_indexed >= 1,
        "should index at least the Config struct"
    );
    assert!(result.errors.is_empty(), "no errors expected");
}

#[test]
async fn index_workspace_skips_unchanged_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(
        ws,
        "src/lib.rs",
        "pub fn hello() -> &'static str { \"hi\" }\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    // First run: should parse the file.
    let r1 = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("first index");
    assert_eq!(r1.files_parsed, 1);

    // Second run without changes: should skip.
    let r2 = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("second index");
    assert_eq!(r2.files_parsed, 0);
    assert_eq!(r2.files_skipped, 1);
}

#[test]
async fn index_workspace_force_reindexes_unchanged_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(
        ws,
        "src/lib.rs",
        "pub fn hello() -> &'static str { \"hi\" }\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    // Index once.
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("first index");

    // Force re-index should reparse.
    let r2 = code_graph::index_workspace(ws, &data_dir, &branch, &config, true)
        .await
        .expect("force index");
    assert_eq!(r2.files_parsed, 1, "force should re-parse the file");
}

#[test]
async fn index_workspace_creates_defines_edges() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(
        ws,
        "src/main.rs",
        r"
pub fn alpha() {}
pub fn beta() {}
",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    // Each function gets a defines edge from its file.
    assert!(
        result.edges_created >= 2,
        "expected at least 2 defines edges, got {}",
        result.edges_created,
    );
}

#[test]
async fn index_workspace_applies_tiering() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    // Small function → Tier 1 (explicit_code).
    let small_fn = "pub fn tiny() { let x = 42; }\n";
    // Large function → Tier 2 (summary_pointer).
    let large_body = (0..600).fold(String::new(), |mut acc, i| {
        use std::fmt::Write;
        let _ = writeln!(acc, "    let v{i} = {i};");
        acc
    });
    let large_fn = format!("/// Big function doc.\npub fn big() {{\n{large_body}}}\n");

    write_sample_file(ws, "src/lib.rs", &format!("{small_fn}\n{large_fn}"));

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    assert!(
        result.tier1_count >= 1,
        "should have at least 1 tier-1 symbol"
    );
    assert!(
        result.tier2_count >= 1,
        "should have at least 1 tier-2 symbol"
    );
}

#[test]
async fn index_workspace_skips_unsupported_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(ws, "src/lib.rs", "pub fn go() {}\n");
    write_sample_file(ws, "src/notes.txt", "not a rust file\n");
    write_sample_file(ws, "src/script.py", "def hello(): pass\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    assert_eq!(
        result.files_parsed, 2,
        "the .rs and .py files should be parsed; .txt is skipped"
    );
}

#[test]
async fn index_workspace_collects_trait_as_interface() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(
        ws,
        "src/lib.rs",
        r"
/// A sample trait.
pub trait Greeter {
    fn greet(&self) -> String;
}
",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    assert!(
        result.interfaces_indexed >= 1,
        "should index the Greeter trait as an interface"
    );
}

#[test]
async fn index_workspace_persists_to_db() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(ws, "src/lib.rs", "pub fn persisted() { let x = 1; }\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    // Verify DB records.
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let files = q.list_code_files().await.expect("list files");
    assert!(
        !files.is_empty(),
        "should have at least one code file in DB"
    );
}

#[test]
async fn index_workspace_skips_oversized_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    // Create a config with a very small max file size.
    let config = CodeGraphConfig {
        max_file_size_bytes: 20,
        ..CodeGraphConfig::default()
    };

    write_sample_file(
        ws,
        "src/lib.rs",
        "pub fn this_is_definitely_longer_than_20_bytes() {}\n",
    );

    let (data_dir, branch) = test_db_params(ws);
    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    assert_eq!(
        result.files_parsed, 0,
        "oversized file should not be parsed"
    );
    assert!(result.files_skipped >= 1);
    assert!(
        result.oversized_files_skipped >= 1,
        "should track the oversized file in oversized_files_skipped"
    );
    assert!(
        result.errors.is_empty(),
        "oversized files must not appear in per-file errors"
    );
}

/// Boundary: a file whose byte count exactly equals `max_file_size_bytes`
/// must be parsed (the limit is inclusive — strictly-greater-than triggers skip).
#[test]
async fn index_workspace_oversized_boundary_exact_limit_is_parsed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    // Build content whose byte-length is exactly 64.
    let content = "pub fn f() {}\n".repeat(4); // 14-byte literal × 4 = 56 bytes before padding
    // Pad to exactly 64 bytes.
    let padding = 64usize.saturating_sub(content.len());
    let content = format!("{}{}", content, " ".repeat(padding));
    assert_eq!(content.len(), 64, "test content must be exactly 64 bytes");

    let config = CodeGraphConfig {
        max_file_size_bytes: 64,
        ..CodeGraphConfig::default()
    };

    write_sample_file(ws, "src/lib.rs", &content);

    let (data_dir, branch) = test_db_params(ws);
    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    assert_eq!(
        result.oversized_files_skipped, 0,
        "file at exactly the limit must not be counted as oversized"
    );
    assert_eq!(
        result.files_parsed, 1,
        "file at exactly the limit must be parsed"
    );
}

/// Boundary: a file one byte over `max_file_size_bytes` must be skipped and
/// counted in `oversized_files_skipped`, not in `errors`.
#[test]
async fn index_workspace_oversized_boundary_one_over_is_skipped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    let limit: u64 = 64;
    // Content that is limit+1 bytes long.
    let content = "x".repeat(usize::try_from(limit + 1).expect("limit fits usize"));

    let config = CodeGraphConfig {
        max_file_size_bytes: limit,
        ..CodeGraphConfig::default()
    };

    write_sample_file(ws, "src/lib.rs", &content);

    let (data_dir, branch) = test_db_params(ws);
    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    assert_eq!(
        result.files_parsed, 0,
        "file one byte over the limit must not be parsed"
    );
    assert!(
        result.oversized_files_skipped >= 1,
        "file one byte over the limit must increment oversized_files_skipped"
    );
    assert!(
        result.errors.is_empty(),
        "oversized files must not appear in per-file errors"
    );
}

/// Resilience: an oversized file in a mixed workspace must not prevent
/// normally-sized sibling files from being indexed.
#[test]
async fn index_workspace_oversized_file_does_not_block_siblings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    let limit: u64 = 64;
    let config = CodeGraphConfig {
        max_file_size_bytes: limit,
        ..CodeGraphConfig::default()
    };

    // One oversized file.
    write_sample_file(
        ws,
        "src/big.rs",
        &"x".repeat(usize::try_from(limit + 100).expect("limit fits usize")),
    );
    // One normal file that should be indexed successfully.
    write_sample_file(ws, "src/small.rs", "pub fn tiny() {}\n");

    let (data_dir, branch) = test_db_params(ws);
    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed even with an oversized file present");

    assert_eq!(
        result.files_parsed, 1,
        "the normal-sized sibling must be parsed despite the oversized file"
    );
    assert_eq!(
        result.oversized_files_skipped, 1,
        "the oversized file must be counted in oversized_files_skipped"
    );
    assert!(
        result.errors.is_empty(),
        "oversized files must not appear in per-file errors"
    );
}

#[test]
async fn index_workspace_removes_stale_records_when_file_becomes_oversized() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    let limit: u64 = 64;
    let config = CodeGraphConfig {
        max_file_size_bytes: limit,
        ..CodeGraphConfig::default()
    };

    write_sample_file(ws, "src/lib.rs", "pub fn tracked() {}\n");

    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    assert!(
        q.get_code_file_by_path("src/lib.rs")
            .await
            .expect("lookup indexed file")
            .is_some(),
        "initial index should persist the file"
    );

    write_sample_file(
        ws,
        "src/lib.rs",
        &"x".repeat(usize::try_from(limit + 1).expect("limit fits usize")),
    );

    let result = code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("re-index should succeed");

    assert_eq!(
        result.oversized_files_skipped, 1,
        "oversized file should be reported as skipped"
    );
    assert!(
        result.errors.is_empty(),
        "oversized file should not surface as an error"
    );
    assert!(
        q.get_code_file_by_path("src/lib.rs")
            .await
            .expect("lookup oversized file")
            .is_none(),
        "stale code_file record should be removed once the file becomes oversized"
    );
    assert!(
        q.get_symbol_identities_for_file("src/lib.rs")
            .await
            .expect("lookup stale symbols")
            .is_empty(),
        "stale symbols should be removed once the file becomes oversized"
    );
    assert!(
        q.get_all_file_hashes()
            .await
            .expect("lookup file hashes")
            .into_iter()
            .all(|record| record.file_path != "src/lib.rs"),
        "stale file-hash metadata should be removed once the file becomes oversized"
    );
}

#[test]
async fn canonical_path_populated_and_distinct_across_modules() {
    // Option C Unit A / A6: indexing populates the additive canonical_path
    // column, and same-spelled impl methods in different modules get DISTINCT
    // identities (the RMeJ0 regression). Name-based lookups stay unchanged.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(
        ws,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    write_sample_file(ws, "src/lib.rs", "pub mod a;\npub mod b;\n");
    write_sample_file(
        ws,
        "src/a.rs",
        "pub struct Widget;\nimpl Widget {\n    pub fn build(&self) {}\n}\npub fn helper() {}\n",
    );
    write_sample_file(
        ws,
        "src/b.rs",
        "pub struct Widget;\nimpl Widget {\n    pub fn build(&self) {}\n}\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");

    let db = connect_db(&data_dir, &branch).await.expect("connect");
    let q = CodeGraphQueries::new(db);

    // RMeJ0: `Widget::build` exists in both modules with DISTINCT canonical paths.
    let mut widget = q
        .canonical_paths_for_function_name("Widget::build")
        .await
        .expect("query canonical paths");
    widget.sort();
    assert_eq!(
        widget,
        vec![
            "demo::a::Widget::build".to_owned(),
            "demo::b::Widget::build".to_owned(),
        ],
        "same-spelled impl methods in different modules must get distinct canonical identities"
    );

    // A free function canonicalises to its module path.
    let helper = q
        .canonical_paths_for_function_name("helper")
        .await
        .expect("query canonical paths");
    assert_eq!(helper, vec!["demo::a::helper".to_owned()]);
}

#[test]
async fn duplicate_canonical_path_rows_are_not_collapsed() {
    // Copilot round-2 #1: two distinct definitions (unique ids) that share a
    // canonical_path must BOTH surface. Cozo head projection uses set semantics,
    // so projecting canonical_path alone would collapse the pair into one row and
    // hide the duplicate-definition ambiguity Unit B must fail closed on (013-D).
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );

    // connect_db bootstraps the schema, so rows can be inserted without indexing.
    let (data_dir, branch) = test_db_params(ws);
    let db = connect_db(&data_dir, &branch).await.expect("connect");
    let q = CodeGraphQueries::new(db);

    let make = |id: &str| engram::models::Function {
        id: id.to_owned(),
        name: "build".to_owned(),
        file_path: "src/a.rs".to_owned(),
        line_start: 1,
        line_end: 1,
        signature: "fn build()".to_owned(),
        docstring: None,
        body: String::new(),
        body_hash: String::new(),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        embedding: Vec::new(),
        summary: String::new(),
    };
    q.upsert_function_with_canonical(&make("function:1"), "demo::a::Widget::build")
        .await
        .expect("upsert first duplicate");
    q.upsert_function_with_canonical(&make("function:2"), "demo::a::Widget::build")
        .await
        .expect("upsert second duplicate");

    let paths = q
        .canonical_paths_for_function_name("build")
        .await
        .expect("query canonical paths");
    assert_eq!(
        paths,
        vec![
            "demo::a::Widget::build".to_owned(),
            "demo::a::Widget::build".to_owned(),
        ],
        "two rows sharing a canonical_path must not collapse to one (fail-closed multiplicity)"
    );
}

// ── 094-F (U4): Python call-graph integration + adversarial acceptance ───────

/// U4.1 — an intra-file Python bare call produces a `direct` `calls_edge`, and the
/// graph traversal used by `map_code`/`impact_analysis` (`graph_neighborhood`)
/// surfaces the caller -> callee relationship.
#[test]
async fn python_intra_file_bare_call_direct_edge_and_traversal() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "svc.py",
        "def orchestrate():\n    helper()\n\n\ndef helper():\n    return 1\n",
    );
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let orchestrate = q
        .find_symbols_by_name("orchestrate")
        .await
        .expect("find orchestrate");
    let helper = q.find_symbols_by_name("helper").await.expect("find helper");
    assert_eq!(orchestrate.len(), 1, "orchestrate must be indexed once");
    assert_eq!(helper.len(), 1, "helper must be indexed once");
    let orchestrate_id = orchestrate[0].id.clone();
    let helper_id = helper[0].id.clone();

    let direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        direct
            .iter()
            .any(|(from, to)| from == &orchestrate_id && to == &helper_id),
        "intra-file bare call must produce a direct calls_edge orchestrate->helper; got {direct:?}"
    );

    let bfs = q
        .graph_neighborhood(&orchestrate_id, 2, 50)
        .await
        .expect("graph neighborhood");
    assert!(
        bfs.edges
            .iter()
            .any(|e| e.edge_type == "calls" && e.from == orchestrate_id && e.to == helper_id),
        "map_code/impact_analysis traversal must include a calls edge orchestrate->helper; got {:?}",
        bfs.edges
    );
    assert!(
        bfs.neighbors.iter().any(|n| n.id == helper_id),
        "helper must appear as a traversal neighbor of orchestrate"
    );
}

/// U4.2 — a cross-file Python bare call resolves to the callee's EXACT id
/// (target-identity per the 082-F acceptance gate, not mere row-existence).
#[test]
async fn python_cross_file_call_resolves_to_exact_target() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "a.py", "def orchestrate():\n    helper()\n");
    write_sample_file(ws, "b.py", "def helper():\n    return 1\n");
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let orchestrate = q
        .find_symbols_by_name("orchestrate")
        .await
        .expect("find orchestrate");
    let helper = q.find_symbols_by_name("helper").await.expect("find helper");
    assert_eq!(orchestrate.len(), 1, "orchestrate must be indexed once");
    assert_eq!(
        helper.len(),
        1,
        "helper must be defined exactly once (in b.py)"
    );
    let orchestrate_id = orchestrate[0].id.clone();
    let helper_id = helper[0].id.clone();

    let singletons = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("singleton edges");
    // Target-identity: the resolved edge must point to B's EXACT helper id.
    assert!(
        singletons.contains(&(orchestrate_id.clone(), helper_id.clone())),
        "cross-file call must resolve to B's EXACT helper id (target-identity); got {singletons:?}"
    );
    // No mis-binding: every singleton originating from orchestrate targets helper.
    assert!(
        singletons
            .iter()
            .filter(|(from, _)| from == &orchestrate_id)
            .all(|(_, to)| to == &helper_id),
        "orchestrate must not resolve to any target other than B's helper; got {singletons:?}"
    );
}

/// U4.3 (ADVERSARIAL) — a Python bare call `parse()` whose only workspace-global
/// `parse` definition is a Rust `fn parse` must NOT bind to it, proving the
/// language-scoped resolver (Unit 3) blocks cross-language mis-binding (013-D
/// no-false-edge invariant, 082-F target-correctness gate).
#[test]
async fn python_bare_call_does_not_bind_to_rust_definition() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "caller.py", "def run():\n    parse()\n");
    write_sample_file(ws, "engine.rs", "pub fn parse() {\n    let _ = 1;\n}\n");
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let run = q.find_symbols_by_name("run").await.expect("find run");
    let parse = q.find_symbols_by_name("parse").await.expect("find parse");
    assert_eq!(run.len(), 1, "run must be indexed once");
    assert_eq!(
        parse.len(),
        1,
        "exactly one parse (the Rust fn) must exist workspace-global"
    );
    let run_id = run[0].id.clone();
    let parse_id = parse[0].id.clone();

    for resolution in [
        "direct",
        "calls_resolved_singleton",
        "calls_resolved_canonical",
    ] {
        let edges = q
            .list_calls_edges_by_resolution(resolution)
            .await
            .expect("list edges");
        assert!(
            !edges
                .iter()
                .any(|(from, to)| from == &run_id && to == &parse_id),
            "Python run() must NOT bind to the Rust parse fn via {resolution}; got {edges:?}"
        );
    }
}

/// U4.4 (ordering proof) — when a Python callee name is defined in BOTH Python
/// and Rust, a Python caller must resolve to the PYTHON target. This is the
/// mixed-language positive case: it fails unless language filtering happens
/// BEFORE the singleton unambiguity check (a global-singleton-first ordering
/// would see two `helper` candidates, deem them ambiguous, and create NO edge).
#[test]
async fn python_cross_file_call_resolves_to_python_target_amid_same_name_rust_def() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "a.py", "def orchestrate():\n    helper()\n");
    write_sample_file(ws, "b.py", "def helper():\n    return 1\n");
    write_sample_file(ws, "r.rs", "pub fn helper() {\n    let _ = 1;\n}\n");
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let orchestrate = q
        .find_symbols_by_name("orchestrate")
        .await
        .expect("find orchestrate");
    assert_eq!(orchestrate.len(), 1, "orchestrate must be indexed once");
    let orchestrate_id = orchestrate[0].id.clone();

    let helpers = q.find_symbols_by_name("helper").await.expect("find helper");
    let py_helper = helpers
        .iter()
        .find(|s| s.file_path.replace('\\', "/").contains("b.py"))
        .expect("Python helper (b.py) must be indexed");
    let rust_helper = helpers
        .iter()
        .find(|s| s.file_path.replace('\\', "/").contains("r.rs"))
        .expect("Rust helper (r.rs) must be indexed");
    assert_ne!(
        py_helper.id, rust_helper.id,
        "the two same-name helpers must be distinct symbols"
    );

    let singletons = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("singleton edges");
    // Positive: resolves to the PYTHON helper (proves filter-before-singleton).
    assert!(
        singletons.contains(&(orchestrate_id.clone(), py_helper.id.clone())),
        "Python caller must resolve to the Python helper in b.py; got {singletons:?}"
    );
    // Negative: must never bind to the same-named Rust helper via any resolution.
    for resolution in [
        "direct",
        "calls_resolved_singleton",
        "calls_resolved_canonical",
    ] {
        let edges = q
            .list_calls_edges_by_resolution(resolution)
            .await
            .expect("list edges");
        assert!(
            !edges
                .iter()
                .any(|(from, to)| from == &orchestrate_id && to == &rust_helper.id),
            "Python caller must NOT bind to the Rust helper via {resolution}; got {edges:?}"
        );
    }
}
