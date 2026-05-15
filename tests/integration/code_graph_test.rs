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
    let content = "pub fn f() {}\n".repeat(4); // 15 × 4 = 60 + 4 newlines = 60
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
