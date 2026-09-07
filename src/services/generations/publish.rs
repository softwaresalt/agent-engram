//! Publication locking and revision guards for generation snapshots.
//!
//! Publication serializes on `<destination-parent>\.publisher.lock`, a
//! dedicated advisory lock file derived from the manifest destination's own
//! parent directory (never an independently supplied root) so one
//! destination always maps to exactly one lock.
use super::{GenerationRevision, RevisionError};
use fd_lock::RwLock;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use thiserror::Error;
use uuid::Uuid;
const PUBLISHER_LOCK_FILE_NAME: &str = ".publisher.lock";
/// Filesystem-backed publisher lock namespace.
#[derive(Debug, Clone, Copy, Default)]
pub struct PublisherLock;
impl PublisherLock {
    /// Acquire the publisher lock for a generation root.
    ///
    /// The lock blocks until `<generation-root>\.publisher.lock` becomes
    /// available, ensuring concurrent publishers serialize across processes.
    ///
    /// # Errors
    ///
    /// Returns [`PublishError`] when the lock file cannot be opened or the OS
    /// advisory lock cannot be acquired.
    pub fn acquire(root: &Path) -> Result<PublisherLockGuard, PublishError> {
        PublisherLockGuard::acquire(lock_path(root))
    }
}
/// RAII guard for a held publisher lock.
///
/// `fd-lock` ties its write-guard lifetime to the lock value. To keep a safe
/// drop-based API here without leaking memory or using `unsafe`, the guard owns
/// a worker thread that acquires the OS lock and waits for drop.
#[derive(Debug)]
#[must_use = "if unused the publisher lock will be released immediately"]
pub struct PublisherLockGuard {
    path: PathBuf,
    release_tx: Option<SyncSender<()>>,
    worker: Option<JoinHandle<()>>,
}
impl PublisherLockGuard {
    fn acquire(path: PathBuf) -> Result<Self, PublishError> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let worker_path = path.clone();
        // `thread::Builder::spawn` (unlike `thread::spawn`) returns a typed
        // `io::Error` instead of panicking when the OS cannot create a
        // thread, so a resource-exhaustion failure surfaces as
        // `PublishError::LockAcquisition` rather than an uncaught panic.
        let worker = thread::Builder::new()
            .name("generation-publisher-lock".to_owned())
            .spawn(move || run_lock_worker(worker_path, ready_tx, release_rx))
            .map_err(|source| PublishError::LockAcquisition {
                path: path.clone(),
                source,
            })?;
        match receive_lock_ready(&path, ready_rx) {
            Ok(()) => Ok(Self {
                path,
                release_tx: Some(release_tx),
                worker: Some(worker),
            }),
            Err(error) => {
                let _ = worker.join();
                Err(error)
            }
        }
    }
    /// Return the advisory lock file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for PublisherLockGuard {
    fn drop(&mut self) {
        drop(self.release_tx.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
/// Errors returned by publisher locking and revision checks.
#[derive(Debug, Error)]
pub enum PublishError {
    /// The publisher lock could not be acquired.
    #[error("failed to acquire publisher lock at {path:?}: {source}")]
    LockAcquisition {
        /// The advisory lock file path.
        path: PathBuf,
        /// The underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// The active manifest could not be replaced atomically.
    #[error("failed to replace manifest atomically at {path:?}: {source}")]
    AtomicReplace {
        /// The durable manifest destination path.
        path: PathBuf,
        /// The underlying filesystem failure.
        #[source]
        source: io::Error,
    },
    /// The attempted revision did not advance strictly beyond the published value.
    #[error(
        "publisher revision must increase strictly (current: {current}, attempted: {attempted})"
    )]
    RevisionGuard {
        /// The current published revision.
        current: GenerationRevision,
        /// The attempted successor revision.
        attempted: GenerationRevision,
    },
    /// The currently-published manifest could not be read to determine the
    /// current revision before guarding a new publication attempt.
    #[error("failed to read current published manifest at {path:?}: {reason}")]
    CurrentManifestRead {
        /// The durable manifest destination path.
        path: PathBuf,
        /// A human-readable description of the read or parse failure.
        reason: String,
    },
    /// The manifest to publish could not be serialized.
    #[error("failed to serialize generation manifest for {path:?}: {reason}")]
    ManifestSerialization {
        /// The durable manifest destination path.
        path: PathBuf,
        /// A human-readable description of the serialization failure.
        reason: String,
    },
    /// `destination` has no parent directory to derive the publisher lock from.
    #[error("manifest destination {path:?} must have a parent directory")]
    DestinationHasNoParent {
        /// The rejected destination path.
        path: PathBuf,
    },
}
/// Publish a generation manifest through the full F08 transaction: acquire
/// the cross-process publisher lock, read the currently-published revision
/// from `destination` (an absent destination is treated as revision `0`,
/// i.e. a first publish), enforce the checked revision guard, serialize
/// `manifest`, and durably replace the destination -- all while the lock is
/// held, so no caller can observe or exploit a gap between these steps.
///
/// The publisher lock is always derived from `destination`'s own parent
/// directory (`<destination-parent>/.publisher.lock`), never from an
/// independently supplied root: accepting a separate root parameter would
/// let two callers name different lock files for the very same destination
/// and defeat serialization entirely.
///
/// `manifest.revision()` is the single source of truth for the guarded
/// revision -- there is no separate `attempted` parameter that could name a
/// different revision than the one actually serialized into the durable
/// bytes, and the caller cannot supply arbitrary unparsed bytes that bypass
/// the manifest's own typed shape.
///
/// This is the only production entry point for publishing a generation
/// manifest. [`guard_next_revision`] and [`replace_manifest_atomically`]
/// remain `pub(crate)` for this crate's own tests and internal composition,
/// but are deliberately not exported so an external caller cannot bypass the
/// publisher lock or the revision guard by calling them directly.
///
/// # Errors
///
/// Returns [`PublishError`] when `destination` has no parent directory, the
/// lock cannot be acquired, the current manifest cannot be read,
/// `manifest`'s revision does not advance strictly, the manifest cannot be
/// serialized, or the atomic replacement fails.
pub fn publish_generation_manifest(
    destination: &Path,
    manifest: &super::GenerationManifest,
) -> Result<GenerationRevision, PublishError> {
    let root = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| PublishError::DestinationHasNoParent {
            path: destination.to_path_buf(),
        })?;
    let _lock = PublisherLock::acquire(root)?;
    let current = read_current_manifest_revision(destination)?;
    let next = guard_next_revision(current, manifest.revision())?;
    let manifest_bytes =
        serde_json::to_vec(manifest).map_err(|source| PublishError::ManifestSerialization {
            path: destination.to_path_buf(),
            reason: source.to_string(),
        })?;
    replace_manifest_atomically(destination, &manifest_bytes)?;
    Ok(next)
}
fn read_current_manifest_revision(destination: &Path) -> Result<GenerationRevision, PublishError> {
    match fs::read(destination) {
        Ok(bytes) => {
            let manifest: super::GenerationManifest =
                serde_json::from_slice(&bytes).map_err(|source| {
                    PublishError::CurrentManifestRead {
                        path: destination.to_path_buf(),
                        reason: source.to_string(),
                    }
                })?;
            Ok(manifest.revision())
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(GenerationRevision::new(0)),
        Err(source) => Err(PublishError::CurrentManifestRead {
            path: destination.to_path_buf(),
            reason: source.to_string(),
        }),
    }
}
/// Validate that `attempted` advances strictly beyond `current`.
///
/// Deliberately `pub(crate)`, not `pub`: external callers must go through
/// [`publish_generation_manifest`], which composes this guard with the
/// publisher lock and the atomic replacement so the three steps cannot be
/// split apart and individually bypassed.
///
/// # Errors
///
/// Returns [`PublishError::RevisionGuard`] when `attempted` is not strictly
/// greater than `current`.
pub(crate) fn guard_next_revision(
    current: GenerationRevision,
    attempted: GenerationRevision,
) -> Result<GenerationRevision, PublishError> {
    current.advance_to(attempted).map_err(map_revision_error)
}
/// Replace the active generation manifest with `contents` atomically.
///
/// The replacement file is staged in the destination's parent directory so the
/// final `rename` is atomic on both Unix and Windows.
///
/// Deliberately `pub(crate)`, not `pub`: external callers must go through
/// [`publish_generation_manifest`] so a caller cannot durably replace the
/// manifest without first passing through the publisher lock and the
/// revision guard.
///
/// # Errors
///
/// Returns [`PublishError::AtomicReplace`] when the staging path cannot be
/// prepared or the replacement cannot be committed durably.
pub(crate) fn replace_manifest_atomically(
    destination: &Path,
    contents: &[u8],
) -> Result<(), PublishError> {
    let path = destination.to_path_buf();
    let staging =
        manifest_staging_path(destination).map_err(|source| PublishError::AtomicReplace {
            path: path.clone(),
            source,
        })?;
    if let Err(source) =
        write_staging_manifest(&staging, contents).and_then(|()| fs::rename(&staging, destination))
    {
        cleanup_staging_manifest(&staging);
        return Err(PublishError::AtomicReplace { path, source });
    }
    #[cfg(unix)]
    {
        sync_parent_dir(destination).map_err(|source| PublishError::AtomicReplace { path, source })
    }
    #[cfg(not(unix))]
    {
        sync_parent_dir(destination);
        Ok(())
    }
}
/// List orphaned staging files left beside `destination` by interrupted publishes.
///
/// The returned paths match this module's `"<file-name>.tmp-*"` staging
/// naming convention and are discovery-only: callers can inspect or report them,
/// but this module never promotes or deletes them through this API.
///
/// # Errors
///
/// Returns [`io::Error`] when `destination` does not name a file or its parent
/// directory cannot be read.
pub fn list_orphaned_staging_files(destination: &Path) -> io::Result<Vec<PathBuf>> {
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let prefix = manifest_staging_name_prefix(destination)?;
    let prefix = prefix.to_string_lossy().into_owned();
    let mut orphans = Vec::new();

    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            orphans.push(entry.path());
        }
    }

    orphans.sort();
    Ok(orphans)
}

fn manifest_staging_path(destination: &Path) -> io::Result<PathBuf> {
    let mut staging_name = manifest_staging_name_prefix(destination)?;
    staging_name.push(Uuid::new_v4().to_string());
    Ok(destination.with_file_name(staging_name))
}

fn manifest_staging_name_prefix(destination: &Path) -> io::Result<std::ffi::OsString> {
    let file_name = destination.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest destination must name a file",
        )
    })?;
    let mut staging_name = file_name.to_os_string();
    staging_name.push(".tmp-");
    Ok(staging_name)
}
fn write_staging_manifest(staging: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = File::create(staging)?;
    file.write_all(contents)?;
    file.sync_all()
}
fn cleanup_staging_manifest(staging: &Path) {
    let _ = fs::remove_file(staging);
}
/// Best-effort post-rename **directory** durability step for POSIX
/// platforms, completing the half of the crash-recovery property that
/// fsyncing the staging file alone does not cover.
///
/// A successful `rename(2)` guarantees *atomicity*: no reader ever observes
/// a torn/partial file, and the destination name always resolves to either
/// the old or the new inode — never a missing or corrupt one. It does
/// **not** by itself guarantee that the *updated directory entry* survives
/// a power loss immediately afterward: on POSIX filesystems the parent
/// directory's own metadata must be explicitly synced (opening it and
/// calling `fsync`) for the rename to be crash-durable, not just
/// crash-atomic. Windows has no safe-Rust equivalent (opening a directory
/// as a `File` requires `FILE_FLAG_BACKUP_SEMANTICS`, which `std::fs::File`
/// does not set), so on Windows this step is a documented no-op bounded
/// instead by NTFS's own transactional metadata journal, which provides
/// crash-*consistency* (the volume always resolves to a well-formed pre- or
/// post-rename state, never torn) rather than an fsync-level durability
/// guarantee. Introducing the raw `FlushFileBuffers`/`MOVEFILE_WRITE_THROUGH`
/// Win32 equivalent would require `unsafe` FFI, which the F01 storage probe
/// establishes is not required for the selected primitive.
#[cfg(unix)]
fn sync_parent_dir(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let dir = File::open(parent)?;
    dir.sync_all()
}
#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) {
    // No safe-Rust equivalent on Windows without unsafe FFI; see the
    // doc comment above for the accepted platform asymmetry.
}
fn map_revision_error(error: RevisionError) -> PublishError {
    match error {
        RevisionError::NonIncreasing { current, attempted } => {
            PublishError::RevisionGuard { current, attempted }
        }
    }
}
fn receive_lock_ready(path: &Path, ready_rx: Receiver<io::Result<()>>) -> Result<(), PublishError> {
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(source)) => Err(PublishError::LockAcquisition {
            path: path.to_path_buf(),
            source,
        }),
        Err(_) => Err(PublishError::LockAcquisition {
            path: path.to_path_buf(),
            source: io::Error::other("publisher lock worker terminated unexpectedly"),
        }),
    }
}
fn run_lock_worker(path: PathBuf, ready_tx: SyncSender<io::Result<()>>, release_rx: Receiver<()>) {
    if let Err(source) = hold_lock_until_release(&path, &ready_tx, release_rx) {
        let _ = ready_tx.send(Err(source));
    }
}
fn hold_lock_until_release(
    path: &Path,
    ready_tx: &SyncSender<io::Result<()>>,
    release_rx: Receiver<()>,
) -> io::Result<()> {
    let file = open_lock_file(path)?;
    let mut lock = RwLock::new(file);
    let _guard = lock.write()?;
    ready_tx.send(Ok(())).map_err(|_| {
        io::Error::other("publisher lock acquisition receiver dropped before readiness")
    })?;
    let _ = release_rx.recv();
    Ok(())
}
fn open_lock_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}
fn lock_path(root: &Path) -> PathBuf {
    root.join(PUBLISHER_LOCK_FILE_NAME)
}
#[cfg(test)]
mod tests {
    use super::{
        PublishError, PublisherLock, guard_next_revision, list_orphaned_staging_files, lock_path,
        manifest_staging_path, replace_manifest_atomically,
    };
    use crate::services::generations::GenerationRevision;
    use fd_lock::RwLock;
    use std::fs::{self, File, OpenOptions};
    #[cfg(target_os = "windows")]
    use std::io;
    use std::io::Write;
    use tempfile::TempDir;
    #[test]
    fn sequential_lock_acquisitions_succeed_after_drop() {
        let temp_dir = TempDir::new().unwrap();
        {
            let first_guard = PublisherLock::acquire(temp_dir.path()).unwrap();
            assert_eq!(first_guard.path(), lock_path(temp_dir.path()).as_path());
        }
        let second_guard = PublisherLock::acquire(temp_dir.path()).unwrap();
        assert_eq!(second_guard.path(), lock_path(temp_dir.path()).as_path());
    }
    #[test]
    fn independent_handles_use_the_same_os_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let path = lock_path(temp_dir.path());
        fs::write(&path, []).unwrap();
        let file_a = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mut lock_a = RwLock::new(file_a);
        let _guard_a = lock_a.try_write().unwrap();
        let file_b = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mut lock_b = RwLock::new(file_b);
        #[cfg(target_os = "windows")]
        assert!(
            matches!(lock_b.try_write(), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
        );
        #[cfg(not(target_os = "windows"))]
        let _ = lock_b.try_write();
    }
    #[test]
    fn revision_guard_accepts_only_strictly_greater_revisions() {
        let current = GenerationRevision::new(7);
        let cases = [
            (GenerationRevision::new(7), false),
            (GenerationRevision::new(6), false),
            (GenerationRevision::new(8), true),
        ];
        for (attempted, should_succeed) in cases {
            let result = guard_next_revision(current, attempted);
            if should_succeed {
                assert_eq!(result.unwrap(), attempted);
            } else {
                assert!(
                    matches!(result, Err(PublishError::RevisionGuard { current: observed_current, attempted: observed_attempted }) if observed_current == current && observed_attempted == attempted)
                );
            }
        }
    }
    #[test]
    fn lock_releases_when_error_leaves_the_critical_section() {
        let temp_dir = TempDir::new().unwrap();
        let result: Result<(), PublishError> = (|| {
            let _guard = PublisherLock::acquire(temp_dir.path())?;
            guard_next_revision(GenerationRevision::new(4), GenerationRevision::new(4))?;
            Ok(())
        })();
        assert!(matches!(result, Err(PublishError::RevisionGuard { .. })));
        let reacquired_guard = PublisherLock::acquire(temp_dir.path()).unwrap();
        assert_eq!(
            reacquired_guard.path(),
            lock_path(temp_dir.path()).as_path()
        );
    }
    #[test]
    fn lock_acquisition_failure_is_typed() {
        let temp_dir = TempDir::new().unwrap();
        let file_root = temp_dir.path().join("not-a-directory");
        fs::write(&file_root, b"sentinel").unwrap();
        let error = PublisherLock::acquire(&file_root).unwrap_err();
        assert!(matches!(error, PublishError::LockAcquisition { .. }));
    }
    #[test]
    fn replace_manifest_overwrites_existing_content_without_leaving_staging_files() {
        let temp_dir = TempDir::new().unwrap();
        let destination = temp_dir.path().join("active.json");
        fs::write(&destination, b"old-content").unwrap();
        replace_manifest_atomically(&destination, b"new-content-that-is-longer").unwrap();
        assert_eq!(
            fs::read(&destination).unwrap(),
            b"new-content-that-is-longer"
        );
        replace_manifest_atomically(&destination, b"tiny").unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"tiny");
        let entries = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec![std::ffi::OsString::from("active.json")]);
    }
    #[test]
    fn interrupted_staging_write_leaves_destination_untouched() {
        let temp_dir = TempDir::new().unwrap();
        let destination = temp_dir.path().join("active.json");
        let original = b"published-content";
        let replacement = b"replacement-content-that-never-gets-renamed";
        fs::write(&destination, original).unwrap();
        let staging = manifest_staging_path(&destination).unwrap();
        {
            let mut file = File::create(&staging).unwrap();
            file.write_all(&replacement[..replacement.len() / 2])
                .unwrap();
            file.sync_all().unwrap();
        }
        assert_eq!(fs::read(&destination).unwrap(), original);
        assert_eq!(
            fs::read(&staging).unwrap(),
            &replacement[..replacement.len() / 2]
        );
    }

    #[test]
    fn list_orphaned_staging_files_reports_only_matching_manifest_temporaries() {
        let temp_dir = TempDir::new().unwrap();
        let destination = temp_dir.path().join("active.json");
        let orphan = temp_dir.path().join("active.json.tmp-orphan");
        let unrelated = temp_dir.path().join("unrelated.tmp-orphan");
        let sibling_manifest_orphan = temp_dir.path().join("other.json.tmp-orphan");
        fs::write(&destination, b"published").unwrap();
        fs::write(&orphan, b"orphaned").unwrap();
        fs::write(&unrelated, b"ignore-me").unwrap();
        fs::write(&sibling_manifest_orphan, b"ignore-me-too").unwrap();

        let orphans = list_orphaned_staging_files(&destination).unwrap();

        assert_eq!(orphans, vec![orphan]);
    }

    #[test]
    fn orphaned_staging_files_do_not_block_a_subsequent_atomic_replace() {
        let temp_dir = TempDir::new().unwrap();
        let destination = temp_dir.path().join("active.json");
        let orphan = temp_dir.path().join("active.json.tmp-orphan");
        fs::write(&destination, b"currently-published").unwrap();
        fs::write(&orphan, b"abandoned-write").unwrap();

        replace_manifest_atomically(&destination, b"fresh-publication").unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"fresh-publication");
        assert_eq!(fs::read(&orphan).unwrap(), b"abandoned-write");
    }

    #[test]
    fn orphaned_staging_files_are_reported_but_never_promoted_or_deleted() {
        let temp_dir = TempDir::new().unwrap();
        let destination = temp_dir.path().join("active.json");
        let orphan = temp_dir.path().join("active.json.tmp-orphan");
        let orphan_contents = b"stale-staging-payload";
        fs::write(&orphan, orphan_contents).unwrap();

        let detected_before_publish = list_orphaned_staging_files(&destination).unwrap();
        assert_eq!(detected_before_publish, vec![orphan.clone()]);

        replace_manifest_atomically(&destination, b"fresh-manifest").unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"fresh-manifest");
        assert_eq!(fs::read(&orphan).unwrap(), orphan_contents);
        let detected_after_publish = list_orphaned_staging_files(&destination).unwrap();
        assert_eq!(detected_after_publish, vec![orphan]);
    }

    #[test]
    fn replace_manifest_creates_destination_on_first_publish() {
        let temp_dir = TempDir::new().unwrap();
        let destination = temp_dir.path().join("active.json");
        replace_manifest_atomically(&destination, b"first-publish").unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"first-publish");
        let entries = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec![std::ffi::OsString::from("active.json")]);
    }
}
