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

/// Helper: resolve a function's node ID by name within a specific file.
async fn function_id_in(q: &CodeGraphQueries, name: &str, file_path: &str) -> String {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .find(|function| function.name == name && function.file_path == file_path)
        .unwrap_or_else(|| panic!("no `{name}` function in `{file_path}`"))
        .id
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
async fn sync_retracts_symbols_and_edges_when_file_truncated_to_zero_bytes() {
    // P0-2022 (099.001-T): a previously-indexed file truncated to 0 bytes was
    // skipped as `files_unchanged` by the sync path's zero-byte guard, leaving
    // its symbols, code_file record, and calls edges stale in the graph. The
    // guard is language-agnostic, so this asserts retraction for BOTH a Python
    // file (which also owns an intra-file `direct` call edge) and a non-Python
    // (Rust) file emptied in the same sync.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let config = CodeGraphConfig::default();

    write_sample_file(
        ws,
        "pkg/mod_a.py",
        "def alpha():\n    beta()\n\n\ndef beta():\n    return 1\n",
    );
    write_sample_file(ws, "src/lib.rs", "pub fn tracked() {}\n");

    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    // Preconditions: both files contributed symbols; the Python file owns an
    // intra-file `direct` call edge (alpha -> beta).
    assert!(
        !q.get_symbol_identities_for_file("pkg/mod_a.py")
            .await
            .expect("py symbols")
            .is_empty(),
        "precondition: python file has symbols after index"
    );
    assert!(
        !q.get_symbol_identities_for_file("src/lib.rs")
            .await
            .expect("rs symbols")
            .is_empty(),
        "precondition: rust file has symbols after index"
    );
    let direct_before = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        !direct_before.is_empty(),
        "precondition: intra-file call produced a direct edge"
    );

    // Truncate BOTH tracked files to 0 bytes on disk.
    fs::write(ws.join("pkg/mod_a.py"), "").expect("truncate py");
    fs::write(ws.join("src/lib.rs"), "").expect("truncate rs");

    let result = code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync should succeed");
    assert!(
        result.errors.is_empty(),
        "emptied files must not surface as errors; got {:?}",
        result.errors
    );

    // The now-empty files must have their prior symbols, code_file records, and
    // calls edges retracted — not left stale as `files_unchanged`.
    for path in ["pkg/mod_a.py", "src/lib.rs"] {
        assert!(
            q.get_symbol_identities_for_file(path)
                .await
                .expect("stale symbols")
                .is_empty(),
            "symbols for `{path}` must be retracted after truncation to 0 bytes"
        );
        assert!(
            q.get_code_file_by_path(path)
                .await
                .expect("stale code_file")
                .is_none(),
            "code_file record for `{path}` must be retracted after truncation to 0 bytes"
        );
    }
    let direct_after = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges after");
    assert!(
        direct_after.is_empty(),
        "intra-file call edge must be retracted after the file is emptied; got {direct_after:?}"
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

/// U4.2 / T5a — a cross-file Python bare call is staged with `python_bare`
/// provenance. T5b (096.006-T) restores exact-target cross-file resolution.
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
    assert_eq!(orchestrate.len(), 1, "orchestrate must be indexed once");
    let orchestrate_id = orchestrate[0].id.clone();
    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    assert!(
        staged.iter().any(|row| row.caller_id == orchestrate_id
            && row.callee_name == "helper"
            && row.source_file == "a.py"
            && row.raw_qualifier.is_empty()
            && row.qualifier_kind == "python_bare"
            && row.enclosing_canonical_type.is_empty()),
        "cross-file call must remain staged with python_bare provenance until T5b; got {staged:?}"
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

/// U4.4 / T5a — when a Python callee name is defined in BOTH Python and Rust,
/// the Python call remains staged as `python_bare` and never binds to Rust.
/// T5b restores the positive exact-Python-target assertion.
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

    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    assert!(
        staged.iter().any(|row| row.caller_id == orchestrate_id
            && row.callee_name == "helper"
            && row.source_file == "a.py"
            && row.qualifier_kind == "python_bare"),
        "Python caller must remain staged for T5b canonical resolution; got {staged:?}"
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

// ── 096-F (T3): Python canonical_path populator ──────────────────────────────

/// T3.1 (096-F) — Python module-level defs get `canonical_path = "<module>.<name>"`:
/// a top-level module resolves to `mod.f`, and a def on a proven regular-package
/// chain (`pkg/__init__.py` present) resolves to `pkg.mod.g`.
#[test]
async fn python_module_level_def_gets_module_qualified_canonical_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // Top-level module: no ancestor dirs, resolves regardless of packages.
    write_sample_file(ws, "mod.py", "def f():\n    return 1\n");
    // Nested regular-package chain: `pkg` is a provable package (has __init__.py).
    write_sample_file(ws, "pkg/__init__.py", "# package marker\n");
    write_sample_file(ws, "pkg/mod.py", "def g():\n    return 2\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    assert_eq!(
        q.canonical_paths_for_function_name("f")
            .await
            .expect("query f"),
        vec!["mod.f".to_owned()],
        "top-level module def must canonicalise to `mod.f`"
    );
    assert_eq!(
        q.canonical_paths_for_function_name("g")
            .await
            .expect("query g"),
        vec!["pkg.mod.g".to_owned()],
        "def on a proven regular-package chain must canonicalise to `pkg.mod.g`"
    );
}

/// T3.2 (096-F, Q3/M5 fail-closed) — a def in `__init__.py`, a def under a
/// `src/` source-root, and a def in an implicit PEP 420 namespace package all
/// get `canonical_path == ""` (T1 rejects any chain with an ancestor dir lacking
/// `__init__.py`).
#[test]
async fn python_unprovable_layouts_fail_closed_to_empty_canonical() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // `__init__.py` itself is the package marker, never a resolvable module.
    write_sample_file(ws, "pkg/__init__.py", "def init_fn():\n    return 0\n");
    // `src/` source-root: `src` has no __init__.py, so `src/pkg/app.py` fails
    // closed even though `src/pkg` would otherwise be a package.
    write_sample_file(ws, "src/pkg/__init__.py", "# marker\n");
    write_sample_file(ws, "src/pkg/app.py", "def src_fn():\n    return 0\n");
    // Implicit PEP 420 namespace package: `ns` has no __init__.py.
    write_sample_file(ws, "ns/mod.py", "def ns_fn():\n    return 0\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    for name in ["init_fn", "src_fn", "ns_fn"] {
        assert_eq!(
            q.canonical_paths_for_function_name(name)
                .await
                .expect("query"),
            vec![String::new()],
            "{name} must fail closed to an empty canonical_path"
        );
    }
}

/// T3.3 (096-F, FF7DE872 non-subsumption) — two same-file defs of `f` both
/// persist their identical canonical path, preserving the duplicate multiplicity
/// the resolver later fails closed on.
#[test]
async fn python_duplicate_same_file_defs_persist_identical_canonical_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "dup.py",
        "def f():\n    return 1\n\n\ndef f():\n    return 2\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let paths = q
        .canonical_paths_for_function_name("f")
        .await
        .expect("query f");
    assert_eq!(
        paths,
        vec!["dup.f".to_owned(), "dup.f".to_owned()],
        "two same-file defs of f must both persist the identical canonical path (fail-closed multiplicity)"
    );
}

/// T3.4 (096-F, C6-1) — a Python package-topology change (an `__init__.py` added
/// or removed) recomputes descendant canonical paths PAST the sync content-hash
/// skip: adding `p/__init__.py` promotes `p/mod.py`'s def from `""` to `p.mod.cf`,
/// and removing it invalidates back to `""` — both with the descendant content
/// unchanged.
#[test]
async fn python_package_topology_change_invalidates_descendant_canonical_paths() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // Initially `p` is an implicit namespace package (no __init__.py): fail closed.
    write_sample_file(ws, "p/mod.py", "def cf():\n    return 1\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    assert_eq!(
        q.canonical_paths_for_function_name("cf")
            .await
            .expect("query cf pre"),
        vec![String::new()],
        "before __init__.py, p is a namespace package: cf must be empty (fail closed)"
    );

    // ADD p/__init__.py: `p` becomes a provable regular package. cf's content is
    // unchanged, so only C6-1 topology invalidation can refresh its canonical.
    write_sample_file(ws, "p/__init__.py", "# package marker\n");
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync after add should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    assert_eq!(
        q.canonical_paths_for_function_name("cf")
            .await
            .expect("query cf added"),
        vec!["p.mod.cf".to_owned()],
        "adding p/__init__.py must recompute the unchanged descendant to `p.mod.cf` (C6-1 add)"
    );

    // REMOVE p/__init__.py: `p` reverts to a namespace package. cf must invalidate
    // back to empty, again past the content-hash skip.
    std::fs::remove_file(ws.join("p/__init__.py")).expect("remove __init__.py");
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync after remove should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    assert_eq!(
        q.canonical_paths_for_function_name("cf")
            .await
            .expect("query cf removed"),
        vec![String::new()],
        "removing p/__init__.py must invalidate the unchanged descendant back to empty (C6-1 remove)"
    );
}

/// 099.003-T (C6-1 parity, P0-1283) — the `index` path must match the sync
/// path: an `__init__.py` add/remove invalidates an unchanged descendant's
/// canonical identity (and its cross-module canonical edge) on `index` alone,
/// without deferring to a later sync. Against the pre-parity index path the
/// descendant's bytes are hash-skipped, so cf stays fail-closed and the caller's
/// canonical edge never materialises — the add assertions below fail (RED).
#[test]
async fn index_package_topology_change_invalidates_descendant_without_sync() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // `p` is initially an implicit namespace package (no __init__.py): cf fails
    // closed, and the cross-module bare call from the top-level `caller` module
    // cannot resolve to a canonical edge.
    write_sample_file(ws, "p/mod.py", "def cf():\n    return 1\n");
    write_sample_file(
        ws,
        "caller.py",
        "from p.mod import cf\n\n\ndef run():\n    cf()\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    assert_eq!(
        q.canonical_paths_for_function_name("cf")
            .await
            .expect("query cf pre"),
        vec![String::new()],
        "before __init__.py, p is a namespace package: cf must be empty (fail closed)"
    );
    let canon_pre = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges pre");
    assert!(
        canon_pre.is_empty(),
        "no canonical edge should resolve while p is a namespace package; got {canon_pre:?}"
    );

    // ADD p/__init__.py: `p` becomes a provable regular package. The descendant
    // p/mod.py is byte-unchanged, so ONLY C6-1 topology invalidation on the index
    // path can refresh cf and re-resolve the caller's canonical edge WITHOUT a
    // subsequent sync.
    write_sample_file(ws, "p/__init__.py", "# package marker\n");
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("re-index after add should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    assert_eq!(
        q.canonical_paths_for_function_name("cf")
            .await
            .expect("query cf added"),
        vec!["p.mod.cf".to_owned()],
        "adding p/__init__.py must recompute the unchanged descendant to `p.mod.cf` on index (C6-1 add)"
    );
    let run_id = function_id_in(&q, "run", "caller.py").await;
    let cf_id = function_id_in(&q, "cf", "p/mod.py").await;
    let canon_added = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges added");
    assert!(
        canon_added.contains(&(run_id, cf_id)),
        "adding p/__init__.py must materialise the cross-module canonical edge on index; got {canon_added:?}"
    );

    // REMOVE p/__init__.py: `p` reverts to a namespace package. cf must invalidate
    // back to empty and its canonical edge must be retracted, again on index alone.
    std::fs::remove_file(ws.join("p/__init__.py")).expect("remove __init__.py");
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("re-index after remove should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    assert_eq!(
        q.canonical_paths_for_function_name("cf")
            .await
            .expect("query cf removed"),
        vec![String::new()],
        "removing p/__init__.py must invalidate the unchanged descendant back to empty on index (C6-1 remove)"
    );
    let canon_removed = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges removed");
    assert!(
        canon_removed.is_empty(),
        "removing p/__init__.py must retract the cross-module canonical edge on index; got {canon_removed:?}"
    );
}

// ── 096-F (T5a): Python bare-call provenance staging ─────────────────────────

/// T5a.1 — a cross-file Python bare call uses `python_bare` provenance during
/// full indexing rather than entering the legacy name-only singleton pass.
#[test]
async fn python_cross_file_bare_call_staged_python_bare_full_index() {
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

    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    assert!(
        staged.iter().any(|row| row.callee_name == "helper"
            && row.qualifier_kind == "python_bare"
            && row.source_file == "a.py"),
        "cross-file Python bare call must be staged as python_bare; got {staged:?}"
    );
}

/// T5a.2 — incremental sync applies the same `python_bare` staging contract as
/// full indexing for newly added Python files.
#[test]
async fn python_cross_file_bare_call_staged_python_bare_sync() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "keep.py", "def keep():\n    return 1\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index should succeed");
    write_sample_file(ws, "a.py", "def orchestrate():\n    helper()\n");
    write_sample_file(ws, "b.py", "def helper():\n    return 1\n");
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync should succeed");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    assert!(
        staged.iter().any(|row| row.callee_name == "helper"
            && row.qualifier_kind == "python_bare"
            && row.source_file == "a.py"),
        "sync must stage cross-file Python bare calls as python_bare; got {staged:?}"
    );
}

/// T5a.3 — the existing module-qualified Python path remains staged as
/// `qualifier_kind == "module"`.
#[test]
async fn python_module_qualified_call_staged_as_module() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "a.py", "import mod\n\n\ndef run():\n    mod.func()\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    assert!(
        staged
            .iter()
            .any(|row| row.callee_name == "func" && row.qualifier_kind == "module"),
        "module-qualified Python call must keep module provenance; got {staged:?}"
    );
}

/// Assert that a same-file `parse()` call is staged rather than directly bound
/// when the supplied source contains a competing lexical binding.
async fn assert_python_shadow_contest(source: &str) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "case.py", source);

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let callers = q.find_symbols_by_name("caller").await.expect("find caller");
    let parses = q.find_symbols_by_name("parse").await.expect("find parse");
    let caller_id = callers
        .iter()
        .find(|symbol| symbol.table == "function")
        .expect("caller function must be indexed")
        .id
        .clone();
    let parse_id = parses
        .iter()
        .find(|symbol| symbol.table == "function")
        .expect("parse function must be indexed")
        .id
        .clone();

    let direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        !direct.contains(&(caller_id.clone(), parse_id)),
        "contested parse must not receive a direct edge; got {direct:?}"
    );
    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    assert!(
        staged.iter().any(|row| row.caller_id == caller_id
            && row.callee_name == "parse"
            && row.qualifier_kind == "python_bare"),
        "contested parse must be staged as python_bare; got {staged:?}"
    );
}

/// T5a.4 — every coarse shadow-contest vector suppresses the same-file direct
/// edge and routes the bare call into `python_bare` provenance staging.
#[test]
async fn python_bare_shadow_contest_routing_table() {
    for source in [
        "def parse():\n    return 0\nfrom bar import parse\ndef caller():\n    parse()\n",
        "def parse():\n    return 0\nfrom n import *\ndef caller():\n    parse()\n",
        "def parse():\n    return 0\nparse = factory()\ndef caller():\n    parse()\n",
        "def parse():\n    return 0\nclass parse:\n    pass\ndef caller():\n    parse()\n",
        "def parse():\n    return 0\ndel parse\ndef caller():\n    parse()\n",
        "def parse():\n    return 0\ndef caller():\n    from bar import parse\n    parse()\n",
        "def parse():\n    return 0\nfrom .other import parse\ndef caller():\n    parse()\n",
    ] {
        assert_python_shadow_contest(source).await;
    }
}

/// T5a.5 / C9-1 — the matched definition is not its own competitor: a sole
/// same-file binding retains the direct-edge fast path.
#[test]
async fn python_sole_binding_bare_call_keeps_direct_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "sole.py",
        "def helper():\n    return 1\ndef caller():\n    helper()\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let caller_id = q.find_symbols_by_name("caller").await.expect("find caller")[0]
        .id
        .clone();
    let helper_id = q.find_symbols_by_name("helper").await.expect("find helper")[0]
        .id
        .clone();
    let direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        direct.contains(&(caller_id.clone(), helper_id)),
        "sole binding must keep its direct edge; got {direct:?}"
    );
    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    assert!(
        !staged.iter().any(|row| row.caller_id == caller_id
            && row.callee_name == "helper"
            && row.qualifier_kind == "python_bare"),
        "sole binding must not be staged as python_bare; got {staged:?}"
    );
}

/// T5a.6 regression — non-Python cross-file bare calls retain the legacy
/// name-only staging marker.
#[test]
async fn rust_cross_file_bare_call_still_name_only_staged() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write_sample_file(ws, "src/lib.rs", "pub mod a;\npub mod b;\n");
    write_sample_file(ws, "src/a.rs", "pub fn orchestrate() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("indexing should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("staged calls");
    let helper = staged
        .iter()
        .find(|row| row.callee_name == "helper")
        .unwrap_or_else(|| panic!("Rust helper call must remain staged: {staged:?}"));
    assert_eq!(helper.qualifier_kind, "");
    assert_ne!(helper.qualifier_kind, "python_bare");
}

// ── 096-F (T7): versioned re-extraction + gated canonical backfill ───────────

/// T7.1 (Q2) — a gated backfill sync after an extraction-version bump restores
/// the resolved cross-module canonical edge on unchanged-hash `.py` files and
/// advances the durable marker to the current version.
#[test]
async fn python_extraction_version_gated_backfill_restores_canonical_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "app.py",
        "from helper import compute\n\n\ndef run():\n    compute()\n",
    );
    write_sample_file(ws, "helper.py", "def compute():\n    return 1\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    // Fresh full index: resolves the canonical edge and advances the marker.
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let run_id = function_id_in(&q, "run", "app.py").await;
    let compute_id = function_id_in(&q, "compute", "helper.py").await;
    let fresh = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges");
    assert!(
        fresh.contains(&(run_id.clone(), compute_id.clone())),
        "fresh index must resolve the cross-module canonical edge; got {fresh:?}"
    );
    assert_eq!(
        q.python_extraction_version().expect("read marker"),
        Some("1".to_owned()),
        "fresh index advances the extraction-version marker"
    );

    // Simulate a pre-upgrade index: older extraction produced no canonical edge
    // and the marker sits at an earlier version.
    q.retract_all_calls_resolved_canonical_edges()
        .await
        .expect("retract canonical edges");
    q.set_python_extraction_version("0").expect("stale marker");
    let before = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges");
    assert!(
        !before.contains(&(run_id.clone(), compute_id.clone())),
        "precondition: canonical edge retracted before backfill; got {before:?}"
    );

    // Gated backfill sync over unchanged `.py` bytes re-extracts and re-resolves.
    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, true, false, None)
        .await
        .expect("gated backfill sync should succeed");
    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    // Force re-extraction assigns fresh node IDs, so re-resolve by name/file.
    let run_id = function_id_in(&q2, "run", "app.py").await;
    let compute_id = function_id_in(&q2, "compute", "helper.py").await;
    let after = q2
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges");
    assert!(
        after.contains(&(run_id, compute_id)),
        "gated backfill must restore the canonical edge on unchanged files; got {after:?}"
    );
    assert_eq!(
        q2.python_extraction_version().expect("read marker"),
        Some("1".to_owned()),
        "successful backfill advances the marker to the current version"
    );
}

/// T7.2 (C12-5) — without the `--backfill-python-canonical` gate, a stale marker
/// is a no-op deferral: routine sync must not re-extract or churn canonical edges.
#[test]
async fn python_extraction_version_ungated_sync_defers_without_churn() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "app.py",
        "from helper import compute\n\n\ndef run():\n    compute()\n",
    );
    write_sample_file(ws, "helper.py", "def compute():\n    return 1\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let run_id = function_id_in(&q, "run", "app.py").await;
    let compute_id = function_id_in(&q, "compute", "helper.py").await;

    // Stale pre-upgrade state: no canonical edge, older marker.
    q.retract_all_calls_resolved_canonical_edges()
        .await
        .expect("retract canonical edges");
    q.set_python_extraction_version("0").expect("stale marker");

    // Plain sync (backfill gate OFF): must defer — no re-extraction, no churn.
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("ungated sync should succeed");
    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    let after = q2
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges");
    assert!(
        !after.contains(&(run_id, compute_id)),
        "ungated sync must not backfill the canonical edge; got {after:?}"
    );
    assert_eq!(
        q2.python_extraction_version().expect("read marker"),
        Some("0".to_owned()),
        "ungated sync leaves the stale marker untouched (deferral)"
    );
}

/// T7.3 (C7-3) — a gated backfill that hits a `.py` read error still resolves the
/// healthy edges this run but keeps the prior marker so the next sync retries the
/// migration (fail-closed toward retry).
#[test]
async fn python_extraction_version_backfill_keeps_marker_on_py_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "app.py",
        "from helper import compute\n\n\ndef run():\n    compute()\n",
    );
    write_sample_file(ws, "helper.py", "def compute():\n    return 1\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    // Pre-upgrade state.
    q.retract_all_calls_resolved_canonical_edges()
        .await
        .expect("retract canonical edges");
    q.set_python_extraction_version("0").expect("stale marker");

    // A `.py` file with invalid UTF-8 fails `read_to_string` during the sync,
    // landing a per-file error in `SyncResult.errors` (has_python_file_errors).
    fs::write(ws.join("broken.py"), [0xff_u8, 0xfe, 0x00, b'x']).expect("write broken.py");

    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, true, false, None)
        .await
        .expect("gated backfill sync should succeed despite a .py error");
    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    // Force re-extraction assigns fresh node IDs, so re-resolve by name/file.
    let run_id = function_id_in(&q2, "run", "app.py").await;
    let compute_id = function_id_in(&q2, "compute", "helper.py").await;
    let after = q2
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges");
    assert!(
        after.contains(&(run_id, compute_id)),
        "backfill still resolves the healthy edge on this run; got {after:?}"
    );
    assert_eq!(
        q2.python_extraction_version().expect("read marker"),
        Some("0".to_owned()),
        "a `.py` error keeps the prior marker so the migration retries (C7-3)"
    );

    // Remove the broken file: the next gated backfill completes and advances.
    fs::remove_file(ws.join("broken.py")).expect("remove broken.py");
    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, true, false, None)
        .await
        .expect("retry backfill sync should succeed");
    let db3 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q3 = CodeGraphQueries::new(db3);
    assert_eq!(
        q3.python_extraction_version().expect("read marker"),
        Some("1".to_owned()),
        "a clean retry sync advances the marker to the current version"
    );
    let restored = q3
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("canonical edges");
    let run_id = function_id_in(&q3, "run", "app.py").await;
    let compute_id = function_id_in(&q3, "compute", "helper.py").await;
    assert!(
        restored.contains(&(run_id, compute_id)),
        "retry keeps the canonical edge resolved; got {restored:?}"
    );
}

/// T7.4 (R1) — the extraction-version marker lives in schema metadata, never in a
/// file's `content_hash`, so the raw-SHA content hash and unchanged-file fast path
/// are unaffected; a current-version file is skipped and Rust is untouched.
#[test]
async fn python_extraction_version_marker_preserves_content_hash_fast_path() {
    use sha2::{Digest, Sha256};

    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let app_src = "from helper import compute\n\n\ndef run():\n    compute()\n";
    write_sample_file(ws, "app.py", app_src);
    write_sample_file(ws, "helper.py", "def compute():\n    return 1\n");
    write_sample_file(
        ws,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write_sample_file(ws, "src/lib.rs", "pub fn only() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    // R1: content_hash is the raw source SHA-256, uncontaminated by the marker.
    let files = q.list_code_files().await.expect("list code files");
    let app = files
        .iter()
        .find(|f| f.path == "app.py")
        .expect("app.py indexed");
    let expected = format!("{:x}", Sha256::digest(app_src.as_bytes()));
    assert_eq!(
        app.content_hash, expected,
        "content_hash must stay the raw source SHA, not encode the extraction-version marker"
    );
    assert_eq!(
        q.python_extraction_version().expect("read marker"),
        Some("1".to_owned())
    );

    // Fast path: a plain sync with unchanged bytes skips the .py (hash match) and
    // never re-runs backfill — the marker is already current.
    let result = code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("no-op sync should succeed");
    assert!(
        result.files_unchanged >= 2,
        "unchanged files must hit the content-hash fast path; got {} unchanged",
        result.files_unchanged
    );
    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    assert_eq!(
        q2.python_extraction_version().expect("read marker"),
        Some("1".to_owned()),
        "a current-version no-op sync leaves the marker at the current version"
    );
}
