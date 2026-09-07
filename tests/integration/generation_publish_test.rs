#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use engram::services::generations::{
    BranchIdentity, GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision,
    ManifestFileDigest, PublishError, SealedInventory, WorkspaceIdentity,
    list_orphaned_staging_files, publish_generation_manifest,
};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
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

/// Retry `publish_generation_manifest` until it succeeds, advancing the
/// attempted revision from the `current` value reported by a rejected
/// attempt's own [`PublishError::RevisionGuard`]. This is the realistic
/// caller pattern for optimistic-concurrency publication: the composed
/// production function hides "current" behind the publisher lock, so a
/// caller that does not already know the exact next revision retries with
/// the value the guard itself reports.
fn publish_until_success(root: &Path, destination: &Path, label: &str) -> GenerationRevision {
    let mut attempted = GenerationRevision::new(1);
    loop {
        let contents = manifest_bytes(attempted, label);
        match publish_generation_manifest(root, destination, attempted, &contents) {
            Ok(next) => return next,
            Err(PublishError::RevisionGuard { current, .. }) => {
                attempted = GenerationRevision::new(
                    current
                        .value()
                        .checked_add(1)
                        .expect("test revision must not overflow"),
                );
            }
            Err(other) => panic!("unexpected publish error: {other}"),
        }
    }
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
                let contents = manifest_bytes(GenerationRevision::new(1), &label);
                publish_generation_manifest(
                    &root,
                    &destination,
                    GenerationRevision::new(1),
                    &contents,
                )
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

    let handles = (0..THREADS)
        .map(|thread_index| {
            let root = root.clone();
            let destination = destination.clone();
            let start_gate = Arc::clone(&start_gate);
            thread::spawn(move || {
                start_gate.wait();
                let label = format!("advance-{thread_index}");
                publish_until_success(&root, &destination, &label)
            })
        })
        .collect::<Vec<_>>();

    let mut applied_revisions = handles
        .into_iter()
        .map(|handle| handle.join().expect("publisher thread must not panic"))
        .map(GenerationRevision::value)
        .collect::<Vec<_>>();
    applied_revisions.sort_unstable();

    let expected_revisions =
        (1..=u64::try_from(THREADS).expect("thread count must fit in u64")).collect::<Vec<_>>();
    assert_eq!(applied_revisions, expected_revisions);
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

    publish_generation_manifest(
        temp_dir.path(),
        &destination,
        GenerationRevision::new(3),
        &new_bytes,
    )
    .expect("subsequent publish after interruption must succeed");

    assert_eq!(
        fs::read(&destination).expect("updated manifest must be readable"),
        new_bytes
    );
}

/// GIVEN a published manifest already exists
/// WHEN the parent directory is made non-writable (Unix: remove the write
/// bit), so the staging file `write_staging_manifest` must create in that
/// SAME directory cannot be created at all
/// THEN `publish_generation_manifest` fails at the staging-write checkpoint,
/// inside its ACTUAL production code path (not a manually crafted sibling
/// file), and the pre-existing destination is provably byte-for-byte
/// untouched afterward -- this is a real fault injection into the
/// production function, addressing the review finding that the prior
/// version of this test only ever observed a fully successful publish.
#[cfg(unix)]
#[test]
fn staging_write_failure_leaves_a_preexisting_destination_untouched() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().expect("temporary generation root");
    let destination = manifest_path(temp_dir.path());
    let old_bytes = manifest_bytes(GenerationRevision::new(5), "old");
    fs::write(&destination, &old_bytes).expect("existing manifest must be written");

    // Pre-create the publisher lock file BEFORE removing directory write
    // permission. `PublisherLock::acquire` opens this file with
    // `OpenOptions::new().create(true)...`, which only needs directory-write
    // permission when the file does not already exist; opening an EXISTING
    // file for read/write does not. Without this, lock acquisition itself
    // would fail first (a different checkpoint than the one this test names),
    // since creating a brand-new lock-file directory entry also requires
    // directory-write permission -- this was a review finding on an earlier
    // version of this test.
    fs::File::create(temp_dir.path().join(".publisher.lock"))
        .expect("publisher lock file must be pre-creatable");

    let readonly_dir_permissions = std::fs::Permissions::from_mode(0o555);
    fs::set_permissions(temp_dir.path(), readonly_dir_permissions)
        .expect("directory permissions must be restricted for this fault injection");

    let new_bytes = manifest_bytes(GenerationRevision::new(6), "new");
    let result = publish_generation_manifest(
        temp_dir.path(),
        &destination,
        GenerationRevision::new(6),
        &new_bytes,
    );

    // Restore write access before any further filesystem interaction so the
    // TempDir can clean itself up on drop.
    let writable_dir_permissions = std::fs::Permissions::from_mode(0o755);
    fs::set_permissions(temp_dir.path(), writable_dir_permissions)
        .expect("directory permissions must be restorable after the fault injection");

    assert!(
        matches!(result, Err(PublishError::AtomicReplace { .. })),
        "expected the failure to land at the staging-write/atomic-replace \
         checkpoint (not lock acquisition or the revision guard), got: {result:?}"
    );
    assert_eq!(
        fs::read(&destination).expect("pre-existing manifest must remain readable"),
        old_bytes,
        "a staging-write failure must never touch the pre-existing destination"
    );
}

/// GIVEN a published manifest already exists and is marked read-only
/// WHEN `publish_generation_manifest` is attempted with fresh content
/// THEN the staging file is written and fsynced successfully (it is a new,
/// writable file), but the final `fs::rename` onto the read-only
/// destination fails (Windows respects the read-only attribute for a
/// rename-replace), injecting a real failure at the RENAME checkpoint
/// specifically -- the complementary checkpoint to the Unix
/// staging-write-failure test above -- and the pre-existing destination
/// must remain byte-for-byte untouched.
#[cfg(windows)]
#[test]
fn rename_failure_onto_a_readonly_destination_leaves_it_untouched() {
    let temp_dir = TempDir::new().expect("temporary generation root");
    let destination = manifest_path(temp_dir.path());
    let old_bytes = manifest_bytes(GenerationRevision::new(5), "old");
    fs::write(&destination, &old_bytes).expect("existing manifest must be written");

    let mut readonly_permissions = fs::metadata(&destination)
        .expect("destination metadata must be readable")
        .permissions();
    readonly_permissions.set_readonly(true);
    fs::set_permissions(&destination, readonly_permissions)
        .expect("destination must be markable read-only for this fault injection");

    let new_bytes = manifest_bytes(GenerationRevision::new(6), "new");
    let result = publish_generation_manifest(
        temp_dir.path(),
        &destination,
        GenerationRevision::new(6),
        &new_bytes,
    );

    // Restore write access before any further filesystem interaction so the
    // TempDir can clean itself up on drop. This function is `#[cfg(windows)]`
    // only, so clippy's Unix "world-writable" concern for `set_readonly(false)`
    // does not apply here: Windows has no equivalent permission-bits leak.
    let mut writable_permissions = fs::metadata(&destination)
        .expect("destination metadata must be readable")
        .permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    writable_permissions.set_readonly(false);
    fs::set_permissions(&destination, writable_permissions)
        .expect("destination permissions must be restorable after the fault injection");

    assert!(
        result.is_err(),
        "publish must fail when the destination cannot be replaced"
    );
    assert_eq!(
        fs::read(&destination).expect("pre-existing manifest must remain readable"),
        old_bytes,
        "a rename failure must never leave a torn or partially replaced destination"
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

    publish_generation_manifest(
        temp_dir.path(),
        &destination,
        GenerationRevision::new(5),
        &replacement_bytes,
    )
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
    let temp_dir = TempDir::new().expect("temporary generation root");
    let destination = manifest_path(temp_dir.path());
    let seed_bytes = manifest_bytes(GenerationRevision::new(7), "seed");
    fs::write(&destination, &seed_bytes).expect("seed manifest must be written");

    for attempted in [GenerationRevision::new(7), GenerationRevision::new(6)] {
        let contents = manifest_bytes(attempted, "rejected");
        assert!(matches!(
            publish_generation_manifest(temp_dir.path(), &destination, attempted, &contents),
            Err(PublishError::RevisionGuard {
                current,
                attempted: observed_attempted,
            }) if current == GenerationRevision::new(7) && observed_attempted == attempted
        ));
    }
}
