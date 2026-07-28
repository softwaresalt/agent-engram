//! U3 (101.003-T): versioned code-graph revalidation upgrade acceptance.
//!
//! Certifies the end-to-end upgrade contract for the opt-in
//! `--revalidate-code-graph` gated backfill built in U2 (101.002-T):
//!
//! * **Upgrade round-trip** — an OLD-generation database carrying a WRONG
//!   same-file duplicate-name direct edge (persisted before the 100-F
//!   fail-closed guard) is corrected on a generation-gated revalidation sync:
//!   the wrong edge is dropped, cross-file singletons are re-materialized,
//!   recall on every unaffected edge is preserved (release blocker, H4), and the
//!   durable generation marker advances.
//! * **Partial-failure retry (A3/C7-3)** — a revalidation that hits a per-file
//!   read error still repairs the healthy edges this run but keeps the OLD
//!   marker so the next run retries; a clean retry advances the marker.
//! * **Opt-in gating (A5/H3, C12-5)** — a plain sync WITHOUT the flag over a
//!   stale generation is a strict no-op: no re-extraction, the wrong edge and
//!   the stale marker both survive (no churn on routine sync).
//!
//! Mirrors the 096-F T7 rollout acceptance shape
//! (`code_graph_test.rs::python_extraction_version_*`) and reuses the 100-F Rust
//! `cfg`-gated duplicate-name corpus (`same_file_shadowing_acceptance_test.rs`).

#![allow(clippy::doc_markdown)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

// ── Corpus (reused from the 100-F acceptance fixture) ────────────────────────

/// Rust: `plat` defined twice under mutually-exclusive `cfg` gates (a real,
/// valid same-file duplicate-name shape), called bare by `describe`. The 100-F
/// guard fails closed: a fresh index mints NO `describe -> plat` edge.
const RUST_DUP: &str = "\
#[cfg(unix)]
pub fn plat() -> u8 { 1 }

#[cfg(windows)]
pub fn plat() -> u8 { 2 }

pub fn describe() {
    let _ = plat();
}
";

/// Rust: unique-name same-file control — recall must be preserved across a
/// revalidation (`caller_unique -> helper` stays resolved).
const RUST_UNIQUE: &str = "\
pub fn helper() -> u8 { 7 }

pub fn caller_unique() {
    let _ = helper();
}
";

/// Rust: cross-file unique-name pair — the singleton post-pass must
/// re-materialize `alpha -> beta` after a force re-extraction (H1/H4: canonical
/// / singleton resolution unchanged).
const RUST_XFILE_A: &str = "pub fn alpha() {\n    beta();\n}\n";
const RUST_XFILE_B: &str = "pub fn beta() {\n    let _ = 2;\n}\n";

const CARGO_TOML: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

// ── Harness helpers ──────────────────────────────────────────────────────────

fn write_one(ws: &Path, rel: &str, content: &str) {
    let full = ws.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn write_fixture(ws: &Path) {
    write_one(ws, "Cargo.toml", CARGO_TOML);
    write_one(ws, "src/dup.rs", RUST_DUP);
    write_one(ws, "src/unique.rs", RUST_UNIQUE);
    write_one(ws, "src/xfile_a.rs", RUST_XFILE_A);
    write_one(ws, "src/xfile_b.rs", RUST_XFILE_B);
}

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

/// `id -> name` over every indexed function.
async fn id_to_name(q: &CodeGraphQueries) -> HashMap<String, String> {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .map(|f| (f.id, f.name))
        .collect()
}

/// All function ids carrying `name`, in extraction order.
async fn ids_named(q: &CodeGraphQueries, name: &str) -> Vec<String> {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .filter(|f| f.name == name)
        .map(|f| f.id)
        .collect()
}

/// Every `calls` edge across all resolution classes, mapped to `(from, to)`
/// function-name pairs (ids are re-minted by a force re-extraction).
async fn edge_name_pairs(q: &CodeGraphQueries) -> Vec<(String, String)> {
    let names = id_to_name(q).await;
    let mut pairs = Vec::new();
    for resolution in [
        "direct",
        "calls_resolved_singleton",
        "calls_resolved_canonical",
    ] {
        for (from, to) in q
            .list_calls_edges_by_resolution(resolution)
            .await
            .expect("list edges")
        {
            pairs.push((
                names.get(&from).cloned().unwrap_or_default(),
                names.get(&to).cloned().unwrap_or_default(),
            ));
        }
    }
    pairs
}

/// `calls_resolved_singleton` edges mapped to `(from, to)` name pairs.
async fn singleton_pairs(q: &CodeGraphQueries) -> Vec<(String, String)> {
    let names = id_to_name(q).await;
    q.list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons")
        .into_iter()
        .map(|(from, to)| {
            (
                names.get(&from).cloned().unwrap_or_default(),
                names.get(&to).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

// ── Acceptance scenarios ─────────────────────────────────────────────────────

/// Full upgrade round-trip (H4 release blocker). An old-generation database with
/// a persisted WRONG same-file direct edge is repaired by a generation-gated
/// revalidation sync: the wrong edge is dropped in every resolution class, the
/// cross-file singleton is re-materialized, recall on every legitimately
/// resolved edge is preserved, and the marker advances.
#[test]
async fn upgrade_revalidation_drops_wrong_edge_and_preserves_recall() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    // Ground-truth recall from a CORRECT fresh extraction (captured before we
    // corrupt the DB): the fresh 100-F guard already resolves the legit edges and
    // withholds the ambiguous same-file one.
    let baseline = edge_name_pairs(&q).await;
    assert!(
        baseline.contains(&("caller_unique".to_owned(), "helper".to_owned())),
        "fresh index resolves the unique-name same-file edge; baseline {baseline:?}"
    );
    assert!(
        baseline.contains(&("alpha".to_owned(), "beta".to_owned())),
        "fresh index resolves the cross-file singleton; baseline {baseline:?}"
    );
    assert!(
        baseline.iter().all(|(_, to)| to != "plat"),
        "fresh index withholds the ambiguous same-file edge (100-F guard); baseline {baseline:?}"
    );

    // Simulate a pre-100-F database: stale generation + a persisted WRONG
    // same-file direct edge (`describe -> shadowed first cfg-gated plat`).
    let describe_id = ids_named(&q, "describe").await.remove(0);
    let plat_ids = ids_named(&q, "plat").await;
    assert_eq!(
        plat_ids.len(),
        2,
        "cfg-gated `plat` must be extracted twice to be ambiguous"
    );
    q.create_calls_edge(&describe_id, &plat_ids[0])
        .await
        .expect("inject wrong same-file edge");
    q.set_code_graph_extraction_generation("0")
        .expect("stale generation marker");

    // Precondition: the wrong edge is persisted as a direct edge.
    let before_direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        before_direct.contains(&(describe_id.clone(), plat_ids[0].clone())),
        "precondition: injected wrong same-file edge is persisted; got {before_direct:?}"
    );

    // Opt-in generation-gated revalidation sync over unchanged bytes: force
    // re-extract every file, re-run the 100-F guard, re-materialize singletons,
    // advance the marker.
    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, false, true, None)
        .await
        .expect("gated revalidation sync should succeed");

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    let after = edge_name_pairs(&q2).await;

    // (1) Target-identity: NO calls edge, in any resolution class, targets a
    //     same-file duplicate-name def after revalidation.
    let wrong: Vec<&(String, String)> = after.iter().filter(|(_, to)| to == "plat").collect();
    assert!(
        wrong.is_empty(),
        "revalidation must drop the stale wrong same-file edge; offending {wrong:?}"
    );

    // (2) Cross-file singleton re-materialized: force re-extraction tears down
    //     each file's edges, so the singleton can only reappear via the post-pass.
    let singletons = singleton_pairs(&q2).await;
    assert!(
        singletons.contains(&("alpha".to_owned(), "beta".to_owned())),
        "revalidation must re-materialize the cross-file singleton; singletons {singletons:?}"
    );

    // (3) Recall floor (H4, RELEASE BLOCKER): every legitimately-resolved edge
    //     from the correct fresh extraction survives the revalidation.
    for legit in &baseline {
        assert!(
            after.contains(legit),
            "revalidation must not regress recall on unaffected edge {legit:?}; after {after:?}"
        );
    }

    // (4) Marker advanced to the current generation.
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "a successful revalidation advances the generation marker"
    );
}

/// Partial-failure retry (A3/C7-3). A revalidation that hits a per-file read
/// error still repairs the healthy edges this run but keeps the OLD generation
/// marker so the next run retries; a clean retry advances the marker.
#[test]
async fn revalidation_partial_failure_keeps_old_marker_then_retries() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let describe_id = ids_named(&q, "describe").await.remove(0);
    let plat_ids = ids_named(&q, "plat").await;
    q.create_calls_edge(&describe_id, &plat_ids[0])
        .await
        .expect("inject wrong same-file edge");
    q.set_code_graph_extraction_generation("0")
        .expect("stale generation marker");

    // A source file with invalid UTF-8 fails `read_to_string` during the
    // revalidation sync, landing a per-file error in the result (fail-closed
    // toward retry). Mirrors 096-F T7.3.
    fs::write(ws.join("src/broken.rs"), [0xff_u8, 0xfe, 0x00, b'x']).expect("write broken.rs");

    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, false, true, None)
        .await
        .expect("revalidation sync should succeed despite a per-file error");

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    let after = edge_name_pairs(&q2).await;

    // Healthy repair still happened this run: the wrong edge is dropped and
    // cross-file recall is preserved even though one file errored.
    assert!(
        after.iter().all(|(_, to)| to != "plat"),
        "healthy repair drops the wrong same-file edge even with a per-file error; after {after:?}"
    );
    assert!(
        after.contains(&("caller_unique".to_owned(), "helper".to_owned())),
        "healthy repair preserves unique-name recall; after {after:?}"
    );

    // The marker is KEPT at the old generation so the migration retries next run.
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("0".to_owned()),
        "a per-file error keeps the prior generation so the revalidation retries (A3/C7-3)"
    );

    // Remove the broken file: a clean retry advances the marker.
    fs::remove_file(ws.join("src/broken.rs")).expect("remove broken.rs");
    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, false, true, None)
        .await
        .expect("retry revalidation sync should succeed");

    let db3 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q3 = CodeGraphQueries::new(db3);
    assert_eq!(
        q3.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "a clean retry advances the generation marker to the current generation"
    );
    let retried = edge_name_pairs(&q3).await;
    assert!(
        retried.iter().all(|(_, to)| to != "plat"),
        "retry keeps the wrong same-file edge dropped; retried {retried:?}"
    );
}

/// Opt-in gating / no churn (A5/H3, C12-5). A plain sync WITHOUT the
/// `--revalidate-code-graph` flag over a stale generation is a strict no-op: no
/// re-extraction, so the injected wrong edge survives with its original ids and
/// the stale marker is left untouched for the operator to act on.
#[test]
async fn plain_sync_without_flag_is_noop_on_stale_generation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    let describe_id = ids_named(&q, "describe").await.remove(0);
    let plat_ids = ids_named(&q, "plat").await;
    q.create_calls_edge(&describe_id, &plat_ids[0])
        .await
        .expect("inject wrong same-file edge");
    q.set_code_graph_extraction_generation("0")
        .expect("stale generation marker");

    // Plain sync, revalidation gate OFF: opt-in means no re-extraction, no churn.
    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, false, false, None)
        .await
        .expect("plain sync should succeed");

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);

    // Strict no-op: no re-extraction happened, so the injected edge survives with
    // its original ids (a re-extraction would have re-minted ids and the 100-F
    // guard would have withheld the edge).
    let after_direct = q2
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        after_direct.contains(&(describe_id, plat_ids[0].clone())),
        "ungated sync must not re-extract — the injected wrong edge survives; got {after_direct:?}"
    );

    // The stale marker is left untouched: advancing it requires the operator to
    // opt in to the revalidation.
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("0".to_owned()),
        "ungated sync leaves the stale generation marker for the operator to act on (H3/A5)"
    );
}
