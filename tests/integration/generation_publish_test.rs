#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use engram::services::generations::{
    BranchIdentity, GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision,
    ManifestFileDigest, PublishError, PublisherLock, SealedInventory, WorkspaceIdentity,
    guard_next_revision, list_orphaned_staging_files, replace_manifest_atomically,
};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use tempfile::TempDir;

fn manifest_path(root: &Path) -> PathBuf {
    root.join("active.json")
}

fn fixture_manifest(revision: GenerationRevision, label: &str) -> GenerationManifest {
    let created_at = DateTime::parse_from_rfc3339("2026-09-07T00:00:00Z")
        .expect("fixture timestamp literal must parse")
        .with_timezone(&Utc);

    GenerationManifest::new(
        GenerationId::new(format!("gen-{label}-{}", revision.value()))
            .expect("fixture generation id must be valid"),
        revision,
        "1.0.0-test",
        BranchIdentity::new(
            "publish-harness",
            Some(format!("source-revision-{}", revision.value())),
        ),
        WorkspaceIdentity::new("workspace-publish-harness"),
        SealedInventory::new(vec![ManifestFileDigest::new(
            "active.json",
            format!("sha256:{label}-{}", revision.value()),
        )]),
        GenerationProvenance::new("integration-harness", created_at),
    )
}

fn manifest_bytes(revision: GenerationRevision, label: &str) -> Vec<u8> {
    serde_json::to_vec(&fixture_manifest(revision, label)).expect("fixture manifest must serialize")
}

fn read_manifest(path: &Path) -> GenerationManifest {
    let bytes = fs::read(path).expect("published manifest must be readable");
    serde_json::from_slice(&bytes).expect("published manifest must deserialize")
}

fn orphan_staging_path(destination: &Path, suffix: &str) -> PathBuf {
    let mut name = destination
        .file_name()
        .expect("manifest destination must name a file")
        .to_os_string();
    name.push(".tmp-");
    name.push(suffix);
    destination.with_file_name(name)
}

fn thread_count_revision(thread_count: usize) -> GenerationRevision {
    GenerationRevision::new(u64::try_from(thread_count).expect("thread count must fit in u64"))
}

fn read_current_revision(destination: &Path) -> GenerationRevision {
    match fs::read(destination) {
        Ok(bytes) => {
            let manifest: GenerationManifest =
                serde_json::from_slice(&bytes).expect("on-disk manifest must deserialize");
            manifest.revision()
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => GenerationRevision::new(0),
        Err(error) => panic!("failed to read destination manifest: {error}"),
    }
}

/// Publish a fixed candidate revision, reading "current" from the on-disk
/// manifest **under the publisher lock** rather than from any in-memory
/// oracle. This is deliberate: it gives
/// [`concurrent_publishers_with_same_candidate_choose_exactly_one_winner`]
/// real discriminating power over `PublisherLock` itself. If the lock did
/// not actually serialize the read-check-write sequence, multiple threads
/// could all read the same stale on-disk revision, all pass
/// `guard_next_revision`, and all report `Ok` -- an in-memory
/// `Mutex`-guarded oracle would mask that failure mode, since the mutex
/// alone (independent of `PublisherLock`) would already prevent it.
fn publish_fixed_candidate(
    root: &Path,
    destination: &Path,
    attempted: GenerationRevision,
    label: &str,
) -> Result<GenerationRevision, PublishError> {
    let _publisher_lock = PublisherLock::acquire(root)?;
    let current = read_current_revision(destination);
    let next = guard_next_revision(current, attempted)?;
    let contents = manifest_bytes(next, label);
    replace_manifest_atomically(destination, &contents)?;
    Ok(next)
}

fn publish_next_candidate(
    root: &Path,
    destination: &Path,
    shared_revision: &Mutex<GenerationRevision>,
    label: &str,
) -> Result<GenerationRevision, PublishError> {
    let _publisher_lock = PublisherLock::acquire(root)?;
    let next = {
        let mut current = shared_revision
            .lock()
            .expect("shared revision mutex must lock");
        let attempted = GenerationRevision::new(
            current
                .value()
                .checked_add(1)
                .expect("test revision must not overflow"),
        );
        let next = guard_next_revision(*current, attempted)?;
        *current = next;
        next
    };
    let contents = manifest_bytes(next, label);
    replace_manifest_atomically(destination, &contents)?;
    Ok(next)
}

#[test]
fn concurrent_publishers_with_same_candidate_choose_exactly_one_winner() {
    const THREADS: usize = 6;

    let temp_dir = TempDir::new().expect("temporary generation root");
    let root = temp_dir.path().to_path_buf();
    let destination = manifest_path(temp_dir.path());
    let start_gate = Arc::new(Barrier::new(THREADS));

    let handles = (0..THREADS)
        .map(|thread_index| {
            let root = root.clone();
            let destination = destination.clone();
            let start_gate = Arc::clone(&start_gate);
            thread::spawn(move || {
                start_gate.wait();
                let label = format!("winner-{thread_index}");
                publish_fixed_candidate(&root, &destination, GenerationRevision::new(1), &label)
            })
        })
        .collect::<Vec<_>>();

    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("publisher thread must not panic"))
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results.iter().filter(|result| result.is_err()).count(),
        THREADS - 1
    );
    assert!(results.iter().all(|result| {
        result.is_ok()
            || matches!(
                result,
                Err(PublishError::RevisionGuard { current, attempted })
                    if *current == GenerationRevision::new(1)
                        && *attempted == GenerationRevision::new(1)
            )
    }));
    assert_eq!(
        read_manifest(&destination).revision(),
        GenerationRevision::new(1)
    );
}

#[test]
fn concurrent_publishers_serialize_all_monotonic_advances_without_lost_updates() {
    const THREADS: usize = 6;

    let temp_dir = TempDir::new().expect("temporary generation root");
    let root = temp_dir.path().to_path_buf();
    let destination = manifest_path(temp_dir.path());
    let initial_bytes = manifest_bytes(GenerationRevision::new(0), "seed");
    fs::write(&destination, &initial_bytes).expect("seed manifest must be written");

    let start_gate = Arc::new(Barrier::new(THREADS));
    let shared_revision = Arc::new(Mutex::new(GenerationRevision::new(0)));

    let handles = (0..THREADS)
        .map(|thread_index| {
            let root = root.clone();
            let destination = destination.clone();
            let start_gate = Arc::clone(&start_gate);
            let shared_revision = Arc::clone(&shared_revision);
            thread::spawn(move || {
                start_gate.wait();
                let label = format!("advance-{thread_index}");
                publish_next_candidate(&root, &destination, &shared_revision, &label)
            })
        })
        .collect::<Vec<_>>();

    let mut applied_revisions = handles
        .into_iter()
        .map(|handle| handle.join().expect("publisher thread must not panic"))
        .map(|result| result.expect("serialized publish must succeed"))
        .map(GenerationRevision::value)
        .collect::<Vec<_>>();
    applied_revisions.sort_unstable();

    let expected_revisions =
        (1..=u64::try_from(THREADS).expect("thread count must fit in u64")).collect::<Vec<_>>();
    assert_eq!(applied_revisions, expected_revisions);
    assert_eq!(
        *shared_revision
            .lock()
            .expect("shared revision mutex must still lock"),
        thread_count_revision(THREADS)
    );
    assert_eq!(
        read_manifest(&destination).revision(),
        thread_count_revision(THREADS)
    );
}

#[test]
fn interrupted_publication_never_tears_the_destination_manifest() {
    let temp_dir = TempDir::new().expect("temporary generation root");
    let destination = manifest_path(temp_dir.path());
    let old_bytes = manifest_bytes(GenerationRevision::new(2), "old");
    let new_bytes = manifest_bytes(GenerationRevision::new(3), "new");
    let interrupted_staging = orphan_staging_path(&destination, "crash");

    fs::write(&destination, &old_bytes).expect("existing manifest must be written");
    fs::write(&interrupted_staging, &new_bytes[..new_bytes.len() / 2])
        .expect("interrupted staging sibling must be written");

    assert_eq!(
        fs::read(&destination).expect("existing manifest must be readable"),
        old_bytes
    );

    replace_manifest_atomically(&destination, &new_bytes)
        .expect("subsequent publish after interruption must succeed");

    assert_eq!(
        fs::read(&destination).expect("updated manifest must be readable"),
        new_bytes
    );
}

#[test]
fn orphaned_staging_files_are_reported_and_never_promoted() {
    let temp_dir = TempDir::new().expect("temporary generation root");
    let destination = manifest_path(temp_dir.path());
    let published_bytes = manifest_bytes(GenerationRevision::new(4), "published");
    let replacement_bytes = manifest_bytes(GenerationRevision::new(5), "replacement");
    let orphan_bytes = manifest_bytes(GenerationRevision::new(99), "orphan");
    let orphan = orphan_staging_path(&destination, "orphan");

    fs::write(&destination, &published_bytes).expect("published manifest must be written");
    fs::write(&orphan, &orphan_bytes).expect("orphan staging file must be written");

    assert_eq!(
        list_orphaned_staging_files(&destination).expect("orphan listing must succeed"),
        vec![orphan.clone()]
    );

    replace_manifest_atomically(&destination, &replacement_bytes)
        .expect("legitimate publish must ignore orphan staging siblings");

    assert_eq!(
        fs::read(&destination).expect("replacement manifest must be readable"),
        replacement_bytes
    );
    assert_eq!(
        fs::read(&orphan).expect("orphan staging file must remain readable"),
        orphan_bytes
    );
    assert_eq!(
        list_orphaned_staging_files(&destination)
            .expect("orphan listing after publish must succeed"),
        vec![orphan]
    );
}

#[test]
fn non_increasing_revisions_are_refused_with_typed_errors() {
    let current = GenerationRevision::new(7);

    for attempted in [GenerationRevision::new(7), GenerationRevision::new(6)] {
        assert!(matches!(
            guard_next_revision(current, attempted),
            Err(PublishError::RevisionGuard {
                current: observed_current,
                attempted: observed_attempted,
            }) if observed_current == current && observed_attempted == attempted
        ));
    }
}
