//! Publication locking and revision guards for generation snapshots.
//!
//! Publication serializes on `<generation-root>\.publisher.lock`, a dedicated
//! advisory lock file separate from the manifest paths that later publish
//! stages will replace atomically.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

use fd_lock::RwLock;
use thiserror::Error;

use super::{GenerationRevision, RevisionError};

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
}

/// Validate that `attempted` advances strictly beyond `current`.
///
/// # Errors
///
/// Returns [`PublishError::RevisionGuard`] when `attempted` is not strictly
/// greater than `current`.
pub fn guard_next_revision(
    current: GenerationRevision,
    attempted: GenerationRevision,
) -> Result<GenerationRevision, PublishError> {
    current.advance_to(attempted).map_err(map_revision_error)
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
    use std::fs::{self, OpenOptions};
    use std::io;

    use fd_lock::RwLock;
    use tempfile::TempDir;

    use super::{PublishError, PublisherLock, guard_next_revision, lock_path};
    use crate::services::generations::GenerationRevision;

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
}
