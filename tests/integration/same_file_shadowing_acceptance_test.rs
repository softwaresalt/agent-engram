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

/// Qualified-staging regression corpus: the canonical target stays unique while
/// caller identity varies between ambiguous and unique fixtures.
const PY_TRUSTED_TARGET: &str = "\
def trusted():
    return 7
";

const PY_DUPLICATE_CALLERS_WITHOUT_CALL: &str = "\
def dispatch():
    return 1


def dispatch():
    return 2
";

const PY_DUPLICATE_CALLERS_WITH_LOCAL_IMPORT: &str = "\
def dispatch():
    return 1


def dispatch():
    from target import trusted
    return trusted()
";

const PY_UNIQUE_CALLER_WITH_LOCAL_IMPORT: &str = "\
def dispatch():
    from target import trusted
    return trusted()
";

const PY_UNIQUE_CALLER_WITH_CHANGED_LOCAL_IMPORT_CALL: &str = "\
def dispatch():
    from target import trusted
    value = trusted()
    return value
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
        write_one(ws, rel, content);
    }
}

fn write_one(ws: &Path, rel: &str, content: &str) {
    let full = ws.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
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

/// Resolve every occurrence of a function name in one file, retaining source
/// order so duplicate IDs remain attributable to their exact definitions.
async fn function_ids_by_line(
    q: &CodeGraphQueries,
    name: &str,
    file_path: &str,
) -> Vec<(u32, String)> {
    let mut matches: Vec<_> = q
        .all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .filter(|function| function.name == name && function.file_path == file_path)
        .map(|function| (function.line_start, function.id))
        .collect();
    matches.sort_by_key(|(line_start, _)| *line_start);
    matches
}

async fn exact_function_id(q: &CodeGraphQueries, name: &str, file_path: &str) -> String {
    let matches = function_ids_by_line(q, name, file_path).await;
    assert_eq!(
        matches.len(),
        1,
        "{name} in {file_path} must identify exactly one function; got {matches:?}"
    );
    matches[0].1.clone()
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

/// Index/sync symmetry: the SECOND guarded minting site — incremental
/// `sync_workspace` — must fail closed too. A same-file duplicate-name callee
/// reached through sync (a) mints no wrong-target direct edge, (b) increments the
/// `same_file_ambiguous_dropped` counter on `SyncResult`, and (c) still mints the
/// unique-name control's direct edge (recall preserved under sync).
#[test]
async fn sync_path_fails_closed_on_same_file_duplicate_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_one(
        ws,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write_one(ws, "src/dup.rs", RUST_DUP);
    write_one(ws, "src/unique.rs", RUST_UNIQUE);

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    let result = code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync");

    assert!(
        result.same_file_ambiguous_dropped >= 1,
        "sync must count the dropped same-file ambiguous endpoint; got {}",
        result.same_file_ambiguous_dropped
    );

    let q = CodeGraphQueries::new(connect_db(&data_dir, &branch).await.expect("connect_db"));
    let (id_to_name, counts) = function_name_maps(&q).await;
    assert_eq!(
        counts.get("plat"),
        Some(&2),
        "cfg-gated `plat` must be extracted twice to be ambiguous under sync"
    );

    let ambiguous_ids: HashSet<&String> = id_to_name
        .iter()
        .filter(|(_, name)| counts.get(*name).copied().unwrap_or(0) > 1)
        .map(|(id, _)| id)
        .collect();

    let direct: Vec<(String, String)> = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        !direct.iter().any(|(_, to)| ambiguous_ids.contains(to)),
        "sync must not mint a direct edge to a same-file duplicate-name def; direct: {direct:?}"
    );

    let pairs: HashSet<(String, String)> = direct
        .into_iter()
        .map(|(from, to)| {
            (
                id_to_name.get(&from).cloned().unwrap_or_default(),
                id_to_name.get(&to).cloned().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("caller_unique".to_owned(), "helper".to_owned())),
        "sync must preserve unique-name same-file recall; pairs: {pairs:?}"
    );
}

/// 107.001-T scenario 1: a provable function-local import in the second of two
/// same-name callers must not be attributed to either duplicate. In particular,
/// first-match attribution must not mint a canonical edge from the first
/// definition to the exact trusted target.
#[test]
async fn qualified_staging_full_index_drops_ambiguous_duplicate_caller() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_one(ws, "target.py", PY_TRUSTED_TARGET);
    write_one(ws, "caller.py", PY_DUPLICATE_CALLERS_WITH_LOCAL_IMPORT);

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index qualified duplicate-caller fixture");
    let q = CodeGraphQueries::new(connect_db(&data_dir, &branch).await.expect("connect_db"));

    let duplicate_callers = function_ids_by_line(&q, "dispatch", "caller.py").await;
    assert_eq!(
        duplicate_callers.len(),
        2,
        "fixture must index both top-level dispatch definitions; got {duplicate_callers:?}"
    );
    let trusted_target_id = exact_function_id(&q, "trusted", "target.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let offending: Vec<_> = canonical
        .iter()
        .filter(|(from_id, to_id)| {
            duplicate_callers
                .iter()
                .any(|(_, caller_id)| caller_id == from_id)
                && to_id == &trusted_target_id
        })
        .cloned()
        .collect();

    assert!(
        offending.is_empty(),
        "ambiguous duplicate callers must own no canonical edge to the exact trusted target; \
         callers by source line: {duplicate_callers:?}, target: {trusted_target_id}, \
         offending wrong-origin edges: {offending:?}"
    );
}

/// 107.001-T scenario 2: incremental sync introduces the qualified call into
/// the duplicate caller file. Ambiguous caller identity must leave no staged
/// provenance key and must increment the observable ambiguity-drop counter.
#[test]
async fn qualified_staging_sync_drops_ambiguous_duplicate_caller() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_one(ws, "target.py", PY_TRUSTED_TARGET);
    write_one(ws, "caller.py", PY_DUPLICATE_CALLERS_WITHOUT_CALL);

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index sync baseline");

    write_one(ws, "caller.py", PY_DUPLICATE_CALLERS_WITH_LOCAL_IMPORT);
    let result = code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync qualified duplicate-caller change");
    let q = CodeGraphQueries::new(connect_db(&data_dir, &branch).await.expect("connect_db"));

    let duplicate_callers = function_ids_by_line(&q, "dispatch", "caller.py").await;
    assert_eq!(
        duplicate_callers.len(),
        2,
        "sync must retain both top-level dispatch definitions; got {duplicate_callers:?}"
    );
    let duplicate_ids: HashSet<&String> = duplicate_callers
        .iter()
        .map(|(_, caller_id)| caller_id)
        .collect();
    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("list staged calls");
    let rows_keyed_to_duplicates: Vec<_> = staged
        .iter()
        .filter(|row| duplicate_ids.contains(&row.caller_id))
        .collect();

    assert!(
        rows_keyed_to_duplicates.is_empty() && result.same_file_ambiguous_dropped > 0,
        "sync must drop ambiguous qualified caller provenance and count it; \
         callers by source line: {duplicate_callers:?}, \
         staged rows keyed to duplicates: {rows_keyed_to_duplicates:?}, \
         same_file_ambiguous_dropped: {}",
        result.same_file_ambiguous_dropped
    );
}

/// 107.001-T scenario 3: the unique-caller control resolves the exact canonical
/// target on full index and retains exact `python_local` provenance after the
/// corresponding incremental call change.
#[test]
async fn qualified_staging_unique_caller_preserves_exact_target() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_one(ws, "target.py", PY_TRUSTED_TARGET);
    write_one(ws, "caller.py", PY_UNIQUE_CALLER_WITH_LOCAL_IMPORT);

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index unique-caller control");
    let q = CodeGraphQueries::new(connect_db(&data_dir, &branch).await.expect("connect_db"));
    let indexed_caller_id = exact_function_id(&q, "dispatch", "caller.py").await;
    let trusted_target_id = exact_function_id(&q, "trusted", "target.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    assert!(
        canonical.contains(&(indexed_caller_id, trusted_target_id.clone())),
        "unique caller must resolve to the exact trusted target {trusted_target_id}; \
         canonical edges: {canonical:?}"
    );
    drop(q);

    write_one(
        ws,
        "caller.py",
        PY_UNIQUE_CALLER_WITH_CHANGED_LOCAL_IMPORT_CALL,
    );
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync unique-caller control change");
    let q = CodeGraphQueries::new(connect_db(&data_dir, &branch).await.expect("connect_db"));
    let synced_caller_id = exact_function_id(&q, "dispatch", "caller.py").await;
    let synced_target_id = exact_function_id(&q, "trusted", "target.py").await;
    assert_eq!(
        synced_target_id, trusted_target_id,
        "unchanged trusted target must retain its exact function ID across caller-only sync"
    );
    let staged = q
        .list_staged_calls_with_provenance()
        .await
        .expect("list staged calls");
    assert!(
        staged.iter().any(|row| {
            row.caller_id == synced_caller_id
                && row.callee_name == "trusted"
                && row.raw_qualifier == "target.trusted"
                && row.qualifier_kind == "python_local"
        }),
        "unique synced caller must stage exact target.trusted provenance; \
         caller: {synced_caller_id}, target: {trusted_target_id}, staged: {staged:?}"
    );
}
