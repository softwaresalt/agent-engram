//! RED harness for sealed-target candidate indexing service admission.
//!
//! Plan unit F10 (142.015-T) requires the indexing-service boundary to accept
//! only sealed [`IndexTarget`](engram::services::generations::IndexTarget)
//! values minted by [`GenerationStore`](engram::services::generations::GenerationStore).

use std::fs;
use std::path::{Path, PathBuf};

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph::{index_sealed_target, index_workspace};
use engram::services::generations::{GenerationId, GenerationStore, IndexTargetKind};
use tempfile::TempDir;

fn generation_store_fixture() -> (TempDir, GenerationStore, GenerationId) {
    let tempdir = tempfile::tempdir().expect("generation fixture tempdir");
    let generation_root = tempdir.path().join("generations");
    fs::create_dir(&generation_root).expect("create generation root");
    let store = GenerationStore::new(&generation_root).expect("construct generation store");
    let generation_id = GenerationId::new("generation-001").expect("valid generation id");
    (tempdir, store, generation_id)
}

fn data_dir(root: &Path) -> PathBuf {
    let data_dir = root.join("engram-data");
    fs::create_dir(&data_dir).expect("create data dir");
    data_dir
}

async fn indexed_paths(data_dir: &Path) -> Vec<String> {
    let db = connect_db(data_dir, "main")
        .await
        .expect("connect code graph DB");
    let queries = CodeGraphQueries::new(db);
    let mut paths: Vec<String> = queries
        .list_code_files()
        .await
        .expect("list indexed files")
        .into_iter()
        .map(|file| file.path)
        .collect();
    paths.sort();
    paths
}

#[tokio::test]
async fn sealed_legacy_direct_target_is_an_accepted_indexing_input() {
    let (tempdir, store, generation_id) = generation_store_fixture();
    let source_dir = tempdir.path().join("generations").join("published");
    fs::create_dir(&source_dir).expect("create source dir");
    fs::write(source_dir.join("lib.rs"), "pub fn sealed_target() {}\n").expect("write source file");
    let data_dir = data_dir(tempdir.path());

    let target = store
        .seal_legacy_direct(generation_id, Path::new("published").join("lib.rs"))
        .expect("seal legacy target");
    assert_eq!(target.kind(), IndexTargetKind::LegacyDirect);

    let result = index_sealed_target(
        &target,
        &data_dir,
        "main",
        &CodeGraphConfig::default(),
        false,
    )
    .await;
    assert!(
        result.is_ok(),
        "sealed legacy-direct targets must be accepted by the indexing service"
    );
}

#[tokio::test]
async fn sealed_candidate_target_is_an_accepted_indexing_input_without_touching_active_generation()
{
    let (tempdir, store, generation_id) = generation_store_fixture();
    fs::create_dir(tempdir.path().join("generations").join("candidates"))
        .expect("create candidates parent");
    let active_manifest_path = store.active_manifest_path();
    fs::write(&active_manifest_path, br#"{"generation":"active"}"#)
        .expect("write active generation manifest");
    let active_manifest_bytes = fs::read(&active_manifest_path).expect("read active generation");
    let data_dir = data_dir(tempdir.path());

    let target = store
        .seal_candidate(generation_id, Path::new("candidates").join("next"))
        .expect("seal candidate target");
    assert_eq!(target.kind(), IndexTargetKind::Candidate);
    fs::write(
        target.path().join("lib.rs"),
        "pub fn candidate_target() {}\n",
    )
    .expect("write candidate source file");

    let result = index_sealed_target(
        &target,
        &data_dir,
        "main",
        &CodeGraphConfig::default(),
        false,
    )
    .await;
    assert!(
        result.is_ok(),
        "sealed candidate targets must be accepted by the indexing service"
    );
    assert_eq!(
        fs::read(&active_manifest_path).expect("read active generation after candidate build"),
        active_manifest_bytes,
        "candidate indexing must not mutate the active generation bytes"
    );
}

#[tokio::test]
async fn sealed_legacy_direct_target_reindexes_a_selected_file_without_evicting_indexed_siblings() {
    let (tempdir, store, generation_id) = generation_store_fixture();
    let source_dir = tempdir.path().join("generations").join("published");
    fs::create_dir(&source_dir).expect("create source dir");
    fs::write(
        source_dir.join("alpha.rs"),
        "pub fn alpha() -> usize { 1 }\n",
    )
    .expect("write alpha source");
    fs::write(source_dir.join("beta.rs"), "pub fn beta() -> usize { 2 }\n")
        .expect("write beta source");
    let data_dir = data_dir(tempdir.path());

    index_workspace(
        &source_dir,
        &data_dir,
        "main",
        &CodeGraphConfig::default(),
        false,
    )
    .await
    .expect("seed indexed siblings");
    assert_eq!(indexed_paths(&data_dir).await, vec!["alpha.rs", "beta.rs"]);

    fs::write(
        source_dir.join("alpha.rs"),
        "pub fn alpha() -> usize { 3 }\n",
    )
    .expect("modify selected file so reindex does real work");
    let target = store
        .seal_legacy_direct(generation_id, Path::new("published").join("alpha.rs"))
        .expect("seal modified legacy target");

    let result = index_sealed_target(
        &target,
        &data_dir,
        "main",
        &CodeGraphConfig::default(),
        false,
    )
    .await
    .expect("legacy direct reindex should succeed");

    assert_eq!(
        result.files_reconciled, 0,
        "selected-file legacy direct reindex must not evict sibling indexed files"
    );
    assert_eq!(
        indexed_paths(&data_dir).await,
        vec!["alpha.rs", "beta.rs"],
        "selected-file legacy direct reindex must preserve pre-populated sibling rows"
    );
}
