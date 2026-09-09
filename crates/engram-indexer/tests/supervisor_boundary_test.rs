//! RED harness for the out-of-process supervisor crate boundary.

use std::fs;
use std::path::Path;

use engram::services::generations::{GenerationId, GenerationStore, IndexTargetKind};

#[tokio::test(flavor = "current_thread")]
async fn supervisor_entrypoint_accepts_a_sealed_target_from_engram() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let generation_root = tempdir.path().join("generations");
    fs::create_dir(&generation_root).expect("create generation root");
    let source_dir = generation_root.join("published");
    fs::create_dir(&source_dir).expect("create published dir");
    fs::write(source_dir.join("lib.rs"), "pub fn seeded() {}\n").expect("write source file");

    let store = GenerationStore::new(&generation_root).expect("construct generation store");
    let generation_id = GenerationId::new("generation-001").expect("valid generation id");
    let target = store
        .seal_legacy_direct(generation_id, Path::new("published").join("lib.rs"))
        .expect("seal target");
    assert_eq!(target.kind(), IndexTargetKind::LegacyDirect);

    let data_dir = tempdir.path().join(".engram");
    fs::create_dir(&data_dir).expect("create data dir");

    let _result = engram_indexer::run_for_target(&target, &data_dir)
        .await
        .expect("index sealed target");
}
