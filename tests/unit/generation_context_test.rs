//! Unit tests for generation read context ownership and lifetime semantics.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Weak},
};

use cozo::ScriptMutability;
use engram::{
    db::cozo_backend::{
        ExistingDbLocation, OpenedGeneration, open_existing_generation_via_runtime_copy,
    },
    services::generations::{GenerationId, GenerationReadContext},
};
use tempfile::TempDir;

fn create_seeded_db(db_path: &Path) {
    {
        let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
            .expect("create db");
        db.run_default(":create probe_row {id => val}")
            .expect("create relation");
        db.run_default("?[id, val] <- [[1, 'seed']] :put probe_row {id => val}")
            .expect("seed row");
    }

    assert!(db_path.exists(), "seeded database must exist on disk");
}

fn count_probe_rows(db: &cozo::DbInstance) -> usize {
    db.run_script(
        "?[id, val] := *probe_row{id, val}",
        BTreeMap::new(),
        ScriptMutability::Immutable,
    )
    .expect("read probe rows")
    .rows
    .len()
}

fn open_generation_context(generation_id: &str) -> (GenerationReadContext, TempDir, TempDir) {
    let published_dir = tempfile::tempdir().expect("published tempdir");
    let runtime_root = tempfile::tempdir().expect("runtime tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    create_seeded_db(&published_db_path);

    let location = ExistingDbLocation::new(published_db_path);
    let opened: OpenedGeneration =
        open_existing_generation_via_runtime_copy(&location, runtime_root.path(), generation_id)
            .expect("open runtime copy");
    let generation_id = GenerationId::new(generation_id).expect("valid generation id");

    (
        GenerationReadContext::new(generation_id, opened),
        published_dir,
        runtime_root,
    )
}

#[test]
fn constructs_context_from_real_opened_generation() {
    let (context, _published_dir, runtime_root) = open_generation_context("generation-context");

    assert_eq!(context.generation_id().as_str(), "generation-context");
    assert_eq!(count_probe_rows(context.opened_generation().db()), 1);
    assert_eq!(
        context.opened_generation().runtime_copy().path(),
        runtime_root
            .path()
            .join("generation-context")
            .join("engram.db")
    );
    assert!(context.opened_generation().runtime_copy().path().exists());

    drop(context);
}

#[test]
fn cloning_is_cheap_and_shares_the_same_opened_generation() {
    let (context, _published_dir, _runtime_root) = open_generation_context("generation-clone");
    let clone_a = context.clone();
    let clone_b = context.clone();

    assert!(Arc::ptr_eq(
        context.shared_opened_generation(),
        clone_a.shared_opened_generation()
    ));
    assert!(Arc::ptr_eq(
        context.shared_opened_generation(),
        clone_b.shared_opened_generation()
    ));
    assert_eq!(Arc::strong_count(context.shared_opened_generation()), 3);
    assert_eq!(clone_a.generation_id(), context.generation_id());
    assert_eq!(clone_b.generation_id(), context.generation_id());
    assert_eq!(count_probe_rows(clone_b.opened_generation().db()), 1);

    drop(clone_b);
    drop(clone_a);
    drop(context);
}

#[test]
fn opened_generation_stays_alive_until_the_last_context_drop() {
    let (context, _published_dir, _runtime_root) = open_generation_context("generation-drop");
    let clone_a = context.clone();
    let clone_b = context.clone();
    let weak_opened: Weak<OpenedGeneration> = Arc::downgrade(context.shared_opened_generation());

    assert_eq!(Weak::strong_count(&weak_opened), 3);
    assert!(weak_opened.upgrade().is_some());

    drop(clone_a);
    assert_eq!(Weak::strong_count(&weak_opened), 2);
    assert!(weak_opened.upgrade().is_some());

    drop(clone_b);
    assert_eq!(Weak::strong_count(&weak_opened), 1);
    assert!(weak_opened.upgrade().is_some());

    drop(context);
    assert!(weak_opened.upgrade().is_none());
}
