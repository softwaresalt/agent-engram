//! U3 (100.003-T): cross-language acceptance + Rust no-recall-regression for the
//! same-file duplicate-name shadowing fail-closed fix (stash FF7DE872 /
//! deliberation 014-D / 013-D no-false-edge / 082-F target-correctness).
//!
//! Target-identity gate: an adversarial same-file duplicate-name corpus (Python
//! AND Rust) must mint ZERO wrong-target `calls` edges — no edge, of any
//! resolution class, may target a definition whose name is duplicated within its
//! own file. The unique-name controls must still resolve to their exact
//! direct-edge targets (recall preserved, A6/H3), and the cross-file singleton
//! post-pass must remain unchanged (H1) — my direct-edge guard only withholds
//! the ambiguous first-match edge, it never removes a legitimate one.
//!
//! Rust reproduces the same-file duplicate-name shape via mutually-exclusive
//! `cfg`-gated definitions: tree-sitter does not evaluate `cfg`, so BOTH `plat`
//! functions are extracted into one file's symbols. (Inline `mod` bodies are not
//! descended by the extractor and two bare same-scope free functions are invalid
//! Rust, so the `cfg`-gated pair is the real, valid vector.)

#![allow(clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

// ── Adversarial corpus ───────────────────────────────────────────────────────

/// Rust: `plat` defined twice under mutually-exclusive `cfg` gates (a real,
/// valid same-file duplicate-name shape), called bare by `describe`.
const RUST_DUP: &str = "\
#[cfg(unix)]
pub fn plat() -> u8 { 1 }

#[cfg(windows)]
pub fn plat() -> u8 { 2 }

pub fn describe() {
    let _ = plat();
}
";

/// Rust: unique-name same-file control — recall must be preserved.
const RUST_UNIQUE: &str = "\
pub fn helper() -> u8 { 7 }

pub fn caller_unique() {
    let _ = helper();
}
";

/// Rust: cross-file unique-name pair — the singleton post-pass must still
/// resolve `alpha -> beta` (H1: canonical / singleton resolution unchanged).
const RUST_XFILE_A: &str = "pub fn alpha() {\n    beta();\n}\n";
const RUST_XFILE_B: &str = "pub fn beta() {\n    let _ = 2;\n}\n";

/// Python: two top-level `def parse` (last-def-wins shadow) plus a unique-name
/// control (`py_helper`).
const PY_CORPUS: &str = "\
def parse():
    return 1


def parse():
    return 2


def run():
    parse()


def py_helper():
    return 3


def py_caller():
    py_helper()
";

const FIXTURE: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    ),
    ("src/dup.rs", RUST_DUP),
    ("src/unique.rs", RUST_UNIQUE),
    ("src/xfile_a.rs", RUST_XFILE_A),
    ("src/xfile_b.rs", RUST_XFILE_B),
    ("app.py", PY_CORPUS),
];

// ── Fixture harness (mirrors calls_target_correctness_test) ──────────────────

fn write_fixture(ws: &Path) {
    for (rel, content) in FIXTURE {
        let full = ws.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create dirs");
        }
        fs::write(full, content).expect("write file");
    }
}

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

/// Index the corpus with the full-index cross-file post-pass and return a live
/// queries handle. The returned `TempDir` must be kept alive for the DB to stay
/// readable.
async fn index_corpus() -> (tempfile::TempDir, CodeGraphQueries) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    (tmp, CodeGraphQueries::new(db))
}

/// Every `calls` edge across all resolution classes, as (from_id, to_id).
async fn all_calls_edges(q: &CodeGraphQueries) -> Vec<(String, String)> {
    let mut edges = Vec::new();
    for resolution in [
        "direct",
        "calls_resolved_singleton",
        "calls_resolved_canonical",
    ] {
        edges.extend(
            q.list_calls_edges_by_resolution(resolution)
                .await
                .expect("list edges"),
        );
    }
    edges
}

/// `(id -> name)` and `name -> occurrence count` over the indexed functions.
async fn function_name_maps(
    q: &CodeGraphQueries,
) -> (HashMap<String, String>, HashMap<String, usize>) {
    let mut id_to_name = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for f in q.all_functions().await.expect("all_functions") {
        *counts.entry(f.name.clone()).or_default() += 1;
        id_to_name.insert(f.id, f.name);
    }
    (id_to_name, counts)
}

async fn edge_name_pairs(q: &CodeGraphQueries) -> HashSet<(String, String)> {
    let (id_to_name, _) = function_name_maps(q).await;
    all_calls_edges(q)
        .await
        .into_iter()
        .map(|(from, to)| {
            (
                id_to_name.get(&from).cloned().unwrap_or_default(),
                id_to_name.get(&to).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

// ── Acceptance ───────────────────────────────────────────────────────────────

/// Target-identity gate: ZERO wrong-target edges. No `calls` edge, in any
/// resolution class, may target a definition whose name is duplicated within its
/// own file (Rust `plat`, Python `parse`).
#[test]
async fn same_file_duplicate_names_mint_zero_wrong_target_edges() {
    let (_tmp, q) = index_corpus().await;
    let (id_to_name, counts) = function_name_maps(&q).await;

    assert_eq!(
        counts.get("plat"),
        Some(&2),
        "Rust `cfg`-gated `plat` must be extracted twice to be ambiguous"
    );
    assert_eq!(
        counts.get("parse"),
        Some(&2),
        "Python `parse` must be defined twice to be ambiguous"
    );

    let ambiguous_ids: HashSet<&String> = id_to_name
        .iter()
        .filter(|(_, name)| counts.get(*name).copied().unwrap_or(0) > 1)
        .map(|(id, _)| id)
        .collect();

    let wrong: Vec<(String, String)> = all_calls_edges(&q)
        .await
        .into_iter()
        .filter(|(_, to)| ambiguous_ids.contains(to))
        .collect();

    assert!(
        wrong.is_empty(),
        "no calls edge may target a same-file duplicate-name definition \
         (013-D no-false-edge / 082-F target-correctness); offending edges: {wrong:?}"
    );
}

/// Recall preserved (A6/H3): every legitimate unique-name same-file call still
/// resolves to its exact direct-edge target — a NON-ZERO set of control edges.
#[test]
async fn unique_name_same_file_controls_still_resolve() {
    let (_tmp, q) = index_corpus().await;
    let pairs = edge_name_pairs(&q).await;

    assert!(
        pairs.contains(&("caller_unique".to_owned(), "helper".to_owned())),
        "Rust unique-name same-file call must still resolve (recall); pairs: {pairs:?}"
    );
    assert!(
        pairs.contains(&("py_caller".to_owned(), "py_helper".to_owned())),
        "Python unique-name same-file call must still resolve (recall); pairs: {pairs:?}"
    );
    assert!(
        pairs.len() >= 2,
        "the recall control must produce a non-zero set of correctly-handled edges"
    );
}

/// H1: the cross-file singleton post-pass is unchanged — a legitimate
/// unique-name cross-file call (`alpha -> beta`) still resolves to a
/// `calls_resolved_singleton` edge. The same-file direct-edge guard must not
/// regress this recall path.
#[test]
async fn cross_file_singleton_resolution_unchanged() {
    let (_tmp, q) = index_corpus().await;
    let (id_to_name, _) = function_name_maps(&q).await;

    let singletons: HashSet<(String, String)> = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons")
        .into_iter()
        .map(|(from, to)| {
            (
                id_to_name.get(&from).cloned().unwrap_or_default(),
                id_to_name.get(&to).cloned().unwrap_or_default(),
            )
        })
        .collect();

    assert!(
        singletons.contains(&("alpha".to_owned(), "beta".to_owned())),
        "cross-file unique-name singleton resolution must be unchanged (H1); singletons: {singletons:?}"
    );
}
