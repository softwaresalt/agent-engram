//! CozoDB backend — Phase 2 connection and handle management.
//!
//! `CozoHandle` is a unit-struct marker for schema unit tests.
//! `CozoDb` is the real production handle backed by `Arc<cozo::DbInstance>`.

pub mod schema;

use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::errors::{EngramError, SystemError};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ── Handle types ──────────────────────────────────────────────────────────────

/// Unit-struct CozoDB marker, used only by schema unit tests.
///
/// The test harness creates `let handle = CozoHandle;` (unit struct syntax).
/// Does NOT hold database state; use [`CozoDb`] for all production paths.
#[derive(Clone, Debug)]
pub struct CozoHandle;

/// Production CozoDB connection handle backed by a `cozo::DbInstance`.
///
/// `CozoDb: Send + Sync` because `cozo::DbInstance` uses internal `Arc<RwLock<…>>`
/// for all mutable state (CozoDB 0.7 guarantee). Do not remove the `Arc` wrapper
/// without re-verifying thread safety.
#[derive(Clone)]
pub struct CozoDb {
    pub(crate) inner: Arc<cozo::DbInstance>,
}

impl std::fmt::Debug for CozoDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CozoDb")
            .field("inner", &"<cozo::DbInstance>")
            .finish()
    }
}

/// The active database handle type for the CozoDB backend.
pub type Db = CozoDb;

type DbOpenLock = Arc<Mutex<()>>;

struct DbStartupTimings {
    process_lock: Duration,
    file_lock: Duration,
    database_open: Duration,
    schema_bootstrap: Duration,
    blocking_total: Duration,
}

// Intentionally retain one lock per concrete DB path for the daemon lifetime.
// Weak/strong-count eviction reintroduces a TOCTOU window where concurrent
// callers can race to install distinct locks for the same path.
static DB_OPEN_LOCKS: OnceLock<Mutex<HashMap<PathBuf, DbOpenLock>>> = OnceLock::new();

// ── Bootstrap trait ───────────────────────────────────────────────────────────

/// Provides a `cozo::DbInstance` to [`schema::run_schema_bootstrap`].
///
/// `CozoHandle` opens a temporary in-memory DB (validates CozoScript syntax).
/// `CozoDb` returns the persistent DB instance.
pub trait SchemaTarget {
    /// Acquire the `cozo::DbInstance` for schema bootstrap.
    ///
    /// # Errors
    /// Returns [`EngramError`] if the instance cannot be created or accessed.
    fn cozo_instance(&self) -> Result<Arc<cozo::DbInstance>, EngramError>;
}

impl SchemaTarget for CozoHandle {
    fn cozo_instance(&self) -> Result<Arc<cozo::DbInstance>, EngramError> {
        cozo::DbInstance::new("mem", "", Default::default())
            .map(Arc::new)
            .map_err(|e| map_db_err(e.to_string()))
    }
}

impl SchemaTarget for CozoDb {
    fn cozo_instance(&self) -> Result<Arc<cozo::DbInstance>, EngramError> {
        Ok(Arc::clone(&self.inner))
    }
}

// ── Connection ────────────────────────────────────────────────────────────────

/// Open a CozoDB SQLite handle for the given workspace branch.
///
/// Creates `{data_dir}/cozo/{branch}/engram.db` if it does not exist,
/// acquires an exclusive process-level advisory lock on
/// `{data_dir}/cozo/{branch}/engram.db.lock` to serialise concurrent opens
/// across processes, acquires an in-process mutex keyed by the concrete DB path
/// to serialise same-process callers on POSIX platforms, opens a SQLite-backed
/// `cozo::DbInstance`, bootstraps the schema idempotently (`:create` errors for
/// existing relations are silently ignored), and returns the handle — all while
/// the guards are held.
///
/// Holding both guards through schema bootstrap (not just through
/// `DbInstance::new`) prevents the `SQLITE_BUSY` unwrap panic in cozo 0.7.x
/// when two same-path callers race during open or bootstrap (U015-FLK1
/// residual, stash `C4E8F2A1`). The guards are released automatically when the
/// returned `CozoDb` handle leaves the `spawn_blocking` closure. CozoDB's own
/// SQLite WAL handles concurrent access from multiple in-process handles after
/// the initial open and bootstrap.
///
/// # Errors
///
/// Returns [`EngramError`] when the directory cannot be created, the lock
/// file cannot be opened, the advisory lock cannot be acquired within 30 s
/// (another process is opening the same DB), the database cannot be opened,
/// or schema bootstrap fails with an unexpected error.
pub async fn connect_db(data_dir: &Path, branch: &str) -> Result<Db, EngramError> {
    let connect_started = Instant::now();
    let branch_safe = branch.replace(['/', '\\', ':'], "_");
    let db_dir = data_dir.join("cozo").join(&branch_safe);
    std::fs::create_dir_all(&db_dir)
        .map_err(|e| map_db_err(format!("cannot create CozoDB dir: {e}")))?;

    let db_path = db_dir.join("engram.db");
    let lock_path = db_dir.join("engram.db.lock");
    let db_bytes = std::fs::metadata(&db_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| map_db_err("CozoDB path is not valid UTF-8"))?
        .to_owned();

    // Acquire an in-process mutex and a process-level advisory file lock before
    // opening CozoDB, then hold them through schema bootstrap to prevent two
    // variants of the SQLITE_BUSY unwrap panic in cozo 0.7.x (U015-FLK1):
    //
    //   * Multi-process variant: two daemon processes open the same SQLite
    //     file concurrently — serialised by holding the lock during
    //     `DbInstance::new`.
    //
    //   * Intra-process variant (residual): two concurrent `connect_db`
    //     calls on the same DB path can bypass POSIX advisory file locking
    //     because `fcntl` locks are process-scoped — serialised by the
    //     per-path mutex plus the file lock through bootstrap.
    //
    // `spawn_blocking` is required because all locking and DB-open work must
    // not run on the async executor. Lock order is registry -> per-path mutex
    // -> file lock. `try_write()` is used in a polling loop with a 30-second
    // deadline so the task itself enforces the timeout — there is no dangling
    // background thread after a timeout return. 50 ms polling interval keeps
    // CPU overhead negligible while bounding the worst-case latency.
    let (cozo_db, timings) =
        tokio::task::spawn_blocking(move || -> Result<(CozoDb, DbStartupTimings), EngramError> {
            let blocking_started = Instant::now();
            let process_lock_started = Instant::now();
            let open_lock = connect_db_open_lock(&db_path);
            let _open_guard = match open_lock.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let process_lock = process_lock_started.elapsed();

            let file_lock_started = Instant::now();
            let lock_file = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(&lock_path)
                .map_err(|e| map_db_err(format!("cannot open CozoDB lock file: {e}")))?;
            let mut file_lock = fd_lock::RwLock::new(lock_file);
            let deadline = Instant::now() + Duration::from_secs(30);
            // Poll with try_write so the thread respects the deadline and exits cleanly.
            let _guard = loop {
                if let Ok(guard) = file_lock.try_write() {
                    break guard;
                } else if Instant::now() >= deadline {
                    return Err(map_db_err(
                        "cannot acquire CozoDB lock: timed out after 30 s \
                     (another process is opening the same database)",
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            };
            let file_lock = file_lock_started.elapsed();
            // Bounded reopen-retry (086.002-T): the serialisation lock above prevents
            // the CONCURRENT-open panic, but a rapid SEQUENTIAL reopen can still hit a
            // transient `database is locked` (SQLITE_BUSY) when the OS releases a
            // just-closed handle's lock lazily (Windows lag). cozo 0.7.x `unwrap()`s
            // internally, so that transient surfaces as a PANIC, not an Err (U015-FLK1;
            // docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md).
            // `catch_busy_panic` converts that busy panic into a retryable error so the
            // bounded back-off (+jitter) can absorb it; non-busy panics are re-raised.
            // Interim SQLITE_BUSY mitigation — durable fix tracked as 041.002-T
            // (removable once cozo >= 0.8 handles SQLITE_BUSY gracefully).
            let db_open_started = Instant::now();
            let db = open_db_with_retry(
                || {
                    catch_busy_panic(|| {
                        cozo::DbInstance::new("sqlite", &db_path_str, Default::default())
                    })
                },
                |attempt| std::thread::sleep(reopen_backoff(attempt)),
            )?;
            let database_open = db_open_started.elapsed();
            let cozo_db = CozoDb {
                inner: Arc::new(db),
            };
            // Bootstrap runs inside the lock so schema writes are serialised
            // across concurrent callers (intra-process U015-FLK1 residual fix).
            let schema_started = Instant::now();
            schema::run_schema_bootstrap(&cozo_db)?;
            let schema_bootstrap = schema_started.elapsed();
            Ok((
                cozo_db,
                DbStartupTimings {
                    process_lock,
                    file_lock,
                    database_open,
                    schema_bootstrap,
                    blocking_total: blocking_started.elapsed(),
                },
            ))
            // `_guard` dropped here — lock released after open + bootstrap
        })
        .await
        .map_err(|join_err| map_db_err(format!("DB open task panicked: {join_err}")))??;

    tracing::info!(
        process_lock_ms = %timings.process_lock.as_millis(),
        file_lock_ms = %timings.file_lock.as_millis(),
        database_open_ms = %timings.database_open.as_millis(),
        schema_bootstrap_ms = %timings.schema_bootstrap.as_millis(),
        blocking_total_ms = %timings.blocking_total.as_millis(),
        connect_total_ms = %connect_started.elapsed().as_millis(),
        db_bytes,
        branch,
        "CozoDB startup timing"
    );
    Ok(cozo_db)
}

fn connect_db_open_lock(db_path: &Path) -> DbOpenLock {
    let registry = DB_OPEN_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = match registry.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    Arc::clone(
        locks
            .entry(db_path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    )
}

// ── Existing-generation runtime copies ────────────────────────────────────────

const RUNTIME_COPY_DB_FILE_NAME: &str = "engram.db";

/// Location of an existing published generation database on disk, validated
/// at construction time.
///
/// Construction canonicalizes `published_db_path`, confirms it resolves to an
/// existing regular file, AND confirms it is canonically contained under a
/// caller-supplied `generation_root` -- closing the gap where an arbitrary
/// readable file located anywhere else on disk (not just a non-existent or
/// non-file path) could otherwise be wrapped and handed to
/// [`open_existing_generation_via_runtime_copy`] uninspected. This module
/// cannot import `GenerationStore`/`IndexTarget` (the store-level sealing
/// primitive) without violating this task's own no-generation-service-import
/// layering rule, so this constructor performs the equivalent canonical
/// containment check locally instead, mirroring
/// `services::generations::store::GenerationStore`'s
/// canonicalize-then-`starts_with` pattern.
///
/// Construction also snapshots the validated file's length and SHA-256
/// digest. `open_existing_generation_via_runtime_copy` copies at most this
/// many bytes and rejects the copy unless its digest matches this snapshot,
/// so a source that grew, shrank, or was overwritten with different content
/// of the same length between validation and copy is detected and rejected
/// rather than silently sealed and opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingDbLocation {
    path: PathBuf,
    len: u64,
    digest: [u8; 32],
}

impl ExistingDbLocation {
    /// Wrap the published generation database path that must be opened via a
    /// private runtime copy.
    ///
    /// # Errors
    ///
    /// Returns [`EngramError`] when `generation_root` or `published_db_path`
    /// cannot be canonicalized, `generation_root` does not resolve to an
    /// existing directory, `published_db_path` does not resolve to an
    /// existing regular file, the canonicalized path escapes
    /// `generation_root`, or the file cannot be read to compute its
    /// validation digest.
    pub fn new(
        generation_root: &Path,
        published_db_path: impl Into<PathBuf>,
    ) -> Result<Self, EngramError> {
        let requested_path = published_db_path.into();
        let canonical_path = requested_path.canonicalize().map_err(|source| {
            map_runtime_copy_io_error(
                "canonicalize published database path",
                &requested_path,
                source,
            )
        })?;
        let canonical_root = generation_root.canonicalize().map_err(|source| {
            map_runtime_copy_io_error("canonicalize generation root", generation_root, source)
        })?;
        let root_metadata = std::fs::metadata(&canonical_root).map_err(|source| {
            map_runtime_copy_io_error("read metadata for generation root", &canonical_root, source)
        })?;
        if !root_metadata.is_dir() {
            return Err(map_db_err(format!(
                "generation root {} must be a directory",
                canonical_root.display()
            )));
        }
        if !canonical_path.starts_with(&canonical_root) {
            return Err(map_db_err(format!(
                "published database path {} escapes generation root {}",
                canonical_path.display(),
                canonical_root.display()
            )));
        }
        let metadata = std::fs::metadata(&canonical_path).map_err(|source| {
            map_runtime_copy_io_error(
                "read metadata for published database path",
                &canonical_path,
                source,
            )
        })?;
        if !metadata.is_file() {
            return Err(map_db_err(format!(
                "published database path {} must be a regular file",
                canonical_path.display()
            )));
        }
        let len = metadata.len();
        let digest = digest_bounded(&canonical_path, len)?;

        Ok(Self {
            path: canonical_path,
            len,
            digest,
        })
    }

    /// Return the published generation database path.
    #[must_use]
    pub fn published_db_path(&self) -> &Path {
        &self.path
    }

    /// Return the validated published database length, in bytes, captured
    /// at construction time.
    #[must_use]
    pub fn published_db_len(&self) -> u64 {
        self.len
    }
}

/// Private runtime copy path minted only by this module's open path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCopy {
    path: PathBuf,
    generation_id: String,
}

impl RuntimeCopy {
    fn new(path: PathBuf, generation_id: String) -> Self {
        Self {
            path,
            generation_id,
        }
    }

    /// Return the sealed runtime copy path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the generation identifier this runtime copy was opened for.
    ///
    /// Carried alongside the path (rather than supplied independently by a
    /// caller) so a [`GenerationReadContext`](crate::services::generations::GenerationReadContext)
    /// built from this copy cannot be paired with an unrelated identifier.
    #[must_use]
    pub fn generation_id(&self) -> &str {
        &self.generation_id
    }
}

/// Opened generation handle bound to the private runtime copy it came from.
#[derive(Clone)]
pub struct OpenedGeneration {
    db: Arc<cozo::DbInstance>,
    runtime_copy: RuntimeCopy,
}

impl std::fmt::Debug for OpenedGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenedGeneration")
            .field("db", &"<cozo::DbInstance>")
            .field("runtime_copy", &self.runtime_copy)
            .finish()
    }
}

impl OpenedGeneration {
    fn new(db: Arc<cozo::DbInstance>, runtime_copy: RuntimeCopy) -> Self {
        Self { db, runtime_copy }
    }

    /// Return the opened Cozo database handle for the runtime copy.
    #[must_use]
    pub fn db(&self) -> &cozo::DbInstance {
        self.db.as_ref()
    }

    /// Return the runtime copy backing this opened generation.
    #[must_use]
    pub fn runtime_copy(&self) -> &RuntimeCopy {
        &self.runtime_copy
    }
}

/// Open an existing published generation by copying it into a private runtime
/// location and opening the copy.
///
/// Cozo 0.7.6 provides no construction path that can prove a SQLite-backed
/// database was opened read-only, so this function always follows the F01
/// runtime-copy strategy: copy the published database to
/// `<runtime_root>/<generation_id>/engram.db` via a co-located temporary file,
/// atomically rename the temporary into place, and open the final copy.
///
/// Serializes both in-process (mutex keyed by the final runtime-copy path)
/// and cross-process (advisory `fd_lock` file lock, bounded to 30 s) callers
/// across the publish-and-open sequence, mirroring [`connect_db`]'s guard
/// against the cozo 0.7.x `SQLITE_BUSY` unwrap panic (U015-FLK1): two
/// processes racing to refresh and open the same generation's runtime copy
/// hit the exact same underlying panic surface `connect_db` was hardened
/// against, so this path reuses the same lock-then-retry discipline.
///
/// # Errors
///
/// Returns [`EngramError`] when the generation identifier is not a single safe
/// path component, the runtime directory cannot be prepared, the published
/// database cannot be copied, the runtime copy cannot be renamed into place,
/// the cross-process lock cannot be acquired within 30 s, or the runtime copy
/// cannot be opened by Cozo.
pub fn open_existing_generation_via_runtime_copy(
    location: &ExistingDbLocation,
    runtime_root: &Path,
    generation_id: &str,
) -> Result<OpenedGeneration, EngramError> {
    validate_runtime_generation_id(generation_id)?;
    validate_runtime_root(runtime_root)?;

    let final_path = runtime_copy_destination_path(runtime_root, generation_id);
    // Validate that `final_path` is representable as UTF-8 (required by
    // `cozo::DbInstance::new`, which takes a `&str` path) *before* any
    // filesystem mutation below. This must happen ahead of directory
    // creation, lock-file creation, and `publish_runtime_copy` (which
    // copies the published database into place, seals it as this
    // generation's runtime `engram.db`, and removes stale sidecars) so a
    // rejected non-UTF-8 runtime root never leaves a partially- or
    // fully-published runtime copy behind (PR #385 review thread
    // `PRRT_kwDORJEduc6gJLCP`). Reused verbatim below instead of
    // re-deriving from `runtime_copy.path()` after publishing, since both
    // strings must name the exact same path.
    let final_path_str = final_path
        .to_str()
        .ok_or_else(|| map_db_err("runtime copy path is not valid UTF-8"))?
        .to_owned();
    let open_lock = connect_db_open_lock(&final_path);
    let _open_guard = match open_lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    let runtime_copy_dir = final_path.parent().ok_or_else(|| {
        map_db_err(format!(
            "runtime copy destination {} must have a parent directory",
            final_path.display()
        ))
    })?;
    std::fs::create_dir_all(runtime_copy_dir).map_err(|error| {
        map_runtime_copy_io_error("create runtime directory", runtime_copy_dir, error)
    })?;

    let lock_path = runtime_copy_lock_path(&final_path);
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| {
            map_runtime_copy_io_error("open runtime copy lock file", &lock_path, error)
        })?;
    let mut file_lock = fd_lock::RwLock::new(lock_file);
    let deadline = Instant::now() + Duration::from_secs(30);
    let _file_guard = loop {
        if let Ok(guard) = file_lock.try_write() {
            break guard;
        } else if Instant::now() >= deadline {
            return Err(map_db_err(
                "cannot acquire runtime copy lock: timed out after 30 s \
                 (another process is opening the same runtime copy)",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let runtime_copy = publish_runtime_copy(location, &final_path, generation_id)?;
    let db = open_db_with_retry(
        || {
            catch_busy_panic(|| {
                cozo::DbInstance::new("sqlite", &final_path_str, Default::default())
            })
        },
        |attempt| std::thread::sleep(reopen_backoff(attempt)),
    )?;

    Ok(OpenedGeneration::new(Arc::new(db), runtime_copy))
}

fn runtime_copy_lock_path(final_path: &Path) -> PathBuf {
    let mut lock_name = final_path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    lock_name.push(".lock");
    final_path.with_file_name(lock_name)
}

fn validate_runtime_generation_id(generation_id: &str) -> Result<(), EngramError> {
    if generation_id.is_empty() {
        return Err(map_db_err("generation ID must not be empty"));
    }

    let mut components = Path::new(generation_id).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(()),
        _ => Err(map_db_err(format!(
            "generation ID {generation_id:?} must be a single path component"
        ))),
    }
}

fn validate_runtime_root(runtime_root: &Path) -> Result<(), EngramError> {
    if runtime_root.is_absolute() {
        Ok(())
    } else {
        Err(map_db_err(format!(
            "runtime root {} must be an absolute path",
            runtime_root.display()
        )))
    }
}

fn runtime_copy_destination_path(runtime_root: &Path, generation_id: &str) -> PathBuf {
    runtime_root
        .join(generation_id)
        .join(RUNTIME_COPY_DB_FILE_NAME)
}

fn publish_runtime_copy(
    location: &ExistingDbLocation,
    final_path: &Path,
    generation_id: &str,
) -> Result<RuntimeCopy, EngramError> {
    let published_db_path = location.published_db_path();
    let parent = final_path.parent().ok_or_else(|| {
        map_db_err(format!(
            "runtime copy destination {} must have a parent directory",
            final_path.display()
        ))
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|error| map_runtime_copy_io_error("create runtime directory", parent, error))?;

    let staging_path = runtime_copy_staging_path(final_path)?;
    let copy_digest = match copy_bounded_with_digest(
        published_db_path,
        &staging_path,
        location.published_db_len(),
    ) {
        Ok(digest) => digest,
        Err(error) => {
            cleanup_runtime_copy_staging(&staging_path);
            return Err(error);
        }
    };
    if copy_digest != location.digest {
        cleanup_runtime_copy_staging(&staging_path);
        return Err(map_db_err(format!(
            "published database {} changed since it was validated: refusing to seal or \
             open an unverified runtime copy",
            published_db_path.display()
        )));
    }

    // Fsync the staged copy's content before the rename, mirroring the
    // `services::generations::publish::replace_manifest_atomically` durability
    // discipline: `fs::copy` alone does not guarantee the copied bytes are
    // durably on disk before the subsequent rename.
    if let Err(error) = sync_runtime_copy_staging(&staging_path) {
        cleanup_runtime_copy_staging(&staging_path);
        return Err(map_runtime_copy_io_error(
            "fsync staged runtime copy",
            &staging_path,
            error,
        ));
    }

    if let Err(error) = std::fs::rename(&staging_path, final_path) {
        cleanup_runtime_copy_staging(&staging_path);
        return Err(map_runtime_copy_io_error(
            "seal runtime copy",
            final_path,
            error,
        ));
    }

    // Remove any WAL/SHM/rollback-journal sidecars orphaned by a prior
    // runtime copy at this same path (e.g. a process that crashed before
    // checkpointing). SQLite's WAL replay validates a `-wal` file's own
    // internal header, not the main file's content, so a stale-but-valid
    // sidecar left over from a PRIOR generation's runtime copy could
    // otherwise be replayed against this freshly sealed copy and silently
    // reintroduce stale page state the moment it is opened. Unconditional
    // and idempotent: a fresh runtime directory with nothing to remove is a
    // no-op.
    remove_stale_runtime_copy_sidecars(final_path)?;

    #[cfg(unix)]
    sync_runtime_copy_parent_dir(final_path).map_err(|error| {
        map_runtime_copy_io_error("fsync runtime copy directory", final_path, error)
    })?;

    Ok(RuntimeCopy::new(
        final_path.to_path_buf(),
        generation_id.to_owned(),
    ))
}

/// Sidecar filename suffixes SQLite may create next to a WAL-mode database
/// (`-wal`, `-shm`) or a rollback-journal-mode database (`-journal`).
const RUNTIME_COPY_SIDECAR_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];

/// Remove stale sidecar files left next to `final_path` by a prior runtime
/// copy at this same path. See the call site in [`publish_runtime_copy`] for
/// the crash-isolation rationale. Best-effort against absence: a missing
/// sidecar is not an error.
///
/// # Errors
///
/// Returns [`EngramError`] when an existing sidecar file cannot be removed
/// for a reason other than it already being absent.
fn remove_stale_runtime_copy_sidecars(final_path: &Path) -> Result<(), EngramError> {
    let Some(file_name) = final_path.file_name() else {
        return Ok(());
    };
    for suffix in RUNTIME_COPY_SIDECAR_SUFFIXES {
        let mut sidecar_name = file_name.to_os_string();
        sidecar_name.push(suffix);
        let sidecar_path = final_path.with_file_name(sidecar_name);
        match std::fs::remove_file(&sidecar_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(map_runtime_copy_io_error(
                    "remove stale runtime copy sidecar",
                    &sidecar_path,
                    error,
                ));
            }
        }
    }
    Ok(())
}

/// Read exactly `expected_len` bytes from `source`, hashing them with
/// SHA-256, optionally tee-writing them into `sink`. Bounds both directions:
/// returns a truncation error if `source` yields fewer bytes than
/// `expected_len` (the source shrank or was deleted/replaced since
/// `expected_len` was captured), and a growth error if a probe read after
/// the bound finds any further bytes (the source grew since `expected_len`
/// was captured). `std::fs::copy` alone enforces neither bound, which is
/// what let a growing, shrinking, or concurrently rewritten published
/// database be sealed and opened unverified.
fn hash_bounded_reader(
    source: &Path,
    expected_len: u64,
    mut sink: Option<(&mut dyn Write, &Path)>,
) -> Result<[u8; 32], EngramError> {
    let mut reader = std::fs::File::open(source)
        .map_err(|error| map_runtime_copy_io_error("open published database", source, error))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    let mut copied: u64 = 0;
    while copied < expected_len {
        let remaining = expected_len - copied;
        let want = usize::try_from(remaining.min(buf.len() as u64)).unwrap_or(buf.len());
        let read = reader
            .read(&mut buf[..want])
            .map_err(|error| map_runtime_copy_io_error("read published database", source, error))?;
        if read == 0 {
            return Err(map_db_err(format!(
                "published database {} ended after {copied} byte(s) but expected \
                 {expected_len} byte(s): the source was truncated since it was validated",
                source.display()
            )));
        }
        if let Some((destination_writer, destination_path)) = sink.as_mut() {
            destination_writer
                .write_all(&buf[..read])
                .map_err(|error| {
                    map_runtime_copy_io_error("write staged runtime copy", destination_path, error)
                })?;
        }
        hasher.update(&buf[..read]);
        copied += read as u64;
    }

    let mut probe = [0u8; 1];
    let extra = reader
        .read(&mut probe)
        .map_err(|error| map_runtime_copy_io_error("probe published database", source, error))?;
    if extra != 0 {
        return Err(map_db_err(format!(
            "published database {} grew beyond the expected {expected_len} byte(s): \
             the source changed since it was validated",
            source.display()
        )));
    }

    Ok(hasher.finalize().into())
}

/// Copy at most `expected_len` bytes from `source` into a newly created
/// `destination`, returning the SHA-256 digest of the copied bytes. See
/// [`hash_bounded_reader`] for the bound/growth semantics.
///
/// # Errors
///
/// Returns [`EngramError`] when `source` cannot be opened or read,
/// `destination` cannot be created or written, `source` yields fewer bytes
/// than `expected_len`, or `source` has more than `expected_len` bytes.
fn copy_bounded_with_digest(
    source: &Path,
    destination: &Path,
    expected_len: u64,
) -> Result<[u8; 32], EngramError> {
    let mut destination_file = std::fs::File::create(destination).map_err(|error| {
        map_runtime_copy_io_error("create staged runtime copy", destination, error)
    })?;
    hash_bounded_reader(
        source,
        expected_len,
        Some((&mut destination_file, destination)),
    )
}

/// Compute the SHA-256 digest of exactly `expected_len` bytes read from
/// `source`, without writing a copy. See [`hash_bounded_reader`] for the
/// bound/growth semantics.
///
/// # Errors
///
/// Returns [`EngramError`] when `source` cannot be opened or read, `source`
/// yields fewer bytes than `expected_len`, or `source` has more than
/// `expected_len` bytes.
fn digest_bounded(source: &Path, expected_len: u64) -> Result<[u8; 32], EngramError> {
    hash_bounded_reader(source, expected_len, None)
}

fn sync_runtime_copy_staging(staging_path: &Path) -> std::io::Result<()> {
    // `write(true)` (not a read-only `File::open`) is required here: on
    // Windows, `FlushFileBuffers` (which `sync_all` calls) fails with
    // "Access is denied" against a handle opened without write access.
    std::fs::OpenOptions::new()
        .write(true)
        .open(staging_path)?
        .sync_all()
}

/// Best-effort post-rename **directory** durability step for POSIX platforms,
/// mirroring `services::generations::publish::sync_parent_dir` — see that
/// module's doc comment for the full rationale (a successful `rename`
/// guarantees atomicity but not that the updated directory entry itself
/// survives a crash without an explicit parent-directory fsync) and the
/// documented Windows asymmetry (no safe-Rust equivalent without `unsafe`
/// FFI; NTFS's transactional metadata journal bounds Windows instead).
#[cfg(unix)]
fn sync_runtime_copy_parent_dir(path: &Path) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::File::open(parent)?.sync_all()
}

fn runtime_copy_staging_path(final_path: &Path) -> Result<PathBuf, EngramError> {
    let file_name = final_path.file_name().ok_or_else(|| {
        map_db_err(format!(
            "runtime copy destination {} must name a file",
            final_path.display()
        ))
    })?;
    let mut staging_name = file_name.to_os_string();
    staging_name.push(".tmp-");
    staging_name.push(Uuid::new_v4().to_string());
    Ok(final_path.with_file_name(staging_name))
}

fn cleanup_runtime_copy_staging(staging_path: &Path) {
    let _ = std::fs::remove_file(staging_path);
}

fn map_runtime_copy_io_error(
    operation: &'static str,
    path: &Path,
    error: std::io::Error,
) -> EngramError {
    map_db_err(format!("cannot {operation} {}: {error}", path.display()))
}

// ── Error mapping ─────────────────────────────────────────────────────────────

/// Map any error value into an [`EngramError`] database error.
pub fn map_db_err<E: ToString>(err: E) -> EngramError {
    EngramError::from(SystemError::DatabaseError {
        reason: err.to_string(),
    })
}

// ── 086.002-T: bounded reopen-retry for transient SQLITE_BUSY ──────────────
//
// The intra-process `connect_db_open_lock` mutex plus the advisory file lock
// serialise opens WITHIN and ACROSS processes, but neither retries when the OS
// releases a just-closed handle's lock lazily (Windows lock-release lag): a
// rapid sequential reopen of the same branch DB can still surface a transient
// `database is locked` (SQLITE_BUSY) from `DbInstance::new`. A bounded reopen
// retry with capped exponential back-off + jitter absorbs that transient
// durably, giving up with a clear `EngramError` (never an unwrap panic).

/// Maximum attempts for the bounded CozoDB reopen-retry.
const MAX_REOPEN_ATTEMPTS: u32 = 10;

/// Whether a CozoDB open error is a transient `SQLITE_BUSY` worth retrying.
fn is_retryable_open_error(message: &str) -> bool {
    let m = message.to_lowercase();
    m.contains("locked") || m.contains("busy")
}

/// Whether a CAUGHT PANIC message is a transient SQLite busy/lock panic — as
/// opposed to an unrelated panic that merely mentions "busy"/"locked".
///
/// cozo 0.7.x's internal `unwrap()` panic carries the SQLite failure, e.g.
/// `SqliteFailure(DatabaseBusy, Some("database is locked"))` (SQLITE_BUSY) or the
/// SQLITE_LOCKED variant (`DatabaseLocked` / "database table is locked").
/// `catch_busy_panic` must absorb ONLY those transient panics and re-raise
/// everything else, so this matches SQLite-specific busy/lock markers rather than
/// the bare "busy"/"locked" words used for open-ERROR classification (which are
/// safe there because the source is already a CozoDB open error, but would
/// over-match arbitrary panic payloads). It mirrors the reopen policy, which
/// retries both busy AND locked outcomes.
fn is_sqlite_busy_or_locked_panic(message: &str) -> bool {
    let m = message.to_lowercase();
    // SQLITE_BUSY variants
    m.contains("database is locked")
        || m.contains("database is busy")
        || m.contains("sqlite_busy")
        || m.contains("databasebusy")
        // SQLITE_LOCKED variants
        || m.contains("database table is locked")
        || m.contains("sqlite_locked")
        || m.contains("databaselocked")
}

/// Random jitter in `0..cap_ms` used to de-synchronise concurrent reopeners so
/// competing processes do not retry in lock-step (thundering herd).
fn open_retry_jitter(cap_ms: u64) -> u64 {
    use std::hash::{BuildHasher, Hasher};
    if cap_ms == 0 {
        return 0;
    }
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    // Fold in a high-resolution timestamp so successive draws differ even within
    // the same process seed epoch. A clock error simply yields the seed-only
    // value; it never panics.
    if let Ok(dur) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        hasher.write_u128(dur.as_nanos());
    }
    hasher.finish() % cap_ms
}

/// Capped exponential back-off (20 ms → 250 ms) plus up to 50% jitter for reopen
/// `attempt`. Over [`MAX_REOPEN_ATTEMPTS`] this gives ≈ 1.6–2.4 s of headroom —
/// comfortably longer than the Windows lock-release lag that motivates it while
/// staying well inside the 30 s advisory-lock deadline already held by the caller.
fn reopen_backoff(attempt: u32) -> Duration {
    let base = std::cmp::min(20u64 << attempt.min(4), 250);
    Duration::from_millis(base + open_retry_jitter(base / 2))
}

/// Bounded reopen-retry driver around a fallible CozoDB open.
///
/// Retries only transient `SQLITE_BUSY` ("database is locked") outcomes, up to
/// [`MAX_REOPEN_ATTEMPTS`], invoking `sleep` with the attempt index between
/// tries. Any non-retryable error surfaces immediately; an exhausted budget
/// surfaces the last busy error as an [`EngramError`] — never an unwrap panic.
fn open_db_with_retry<T, E, F, S>(mut open: F, mut sleep: S) -> Result<T, EngramError>
where
    E: std::fmt::Display,
    F: FnMut() -> Result<T, E>,
    S: FnMut(u32),
{
    for attempt in 0..MAX_REOPEN_ATTEMPTS {
        match open() {
            Ok(db) => return Ok(db),
            Err(e) => {
                if is_retryable_open_error(&e.to_string()) && attempt + 1 < MAX_REOPEN_ATTEMPTS {
                    sleep(attempt);
                } else {
                    return Err(map_db_err(format!("cannot open CozoDB SQLite store: {e}")));
                }
            }
        }
    }
    // The final attempt always returns via the match above, so this is provably
    // unreachable; surface an error rather than panic to keep the fn total (F3).
    Err(map_db_err(
        "cannot open CozoDB SQLite store: reopen retry budget exhausted",
    ))
}

/// Extract a human-readable message from a caught panic payload.
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        String::new()
    }
}

/// Run a CozoDB open, converting a transient `SQLITE_BUSY` PANIC into a
/// retryable `Err` so the bounded reopen-retry can absorb it.
///
/// cozo 0.7.x calls `unwrap()` internally on the SQLite open (U015-FLK1), so a
/// transient "database is locked" at a rapid sequential reopen surfaces as a
/// PANIC, not an `Err` (see
/// `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`).
/// This catches that panic and, when its message matches an SQLite-specific busy
/// or locked marker (via [`is_sqlite_busy_or_locked_panic`], NOT the broader
/// open-error predicate), returns `Err(message)`. Any OTHER panic is re-raised
/// unchanged so a genuine bug still propagates (ultimately contained by the
/// caller's `spawn_blocking`).
fn catch_busy_panic<T, E, F>(open: F) -> Result<T, String>
where
    E: std::fmt::Display,
    F: FnOnce() -> Result<T, E>,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(open)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(e.to_string()),
        Err(payload) => {
            let message = panic_payload_message(payload.as_ref());
            if is_sqlite_busy_or_locked_panic(&message) {
                Err(message)
            } else {
                std::panic::resume_unwind(payload);
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "cozo-backend"))]
mod tests {
    use std::{
        path::PathBuf,
        sync::{Arc, mpsc},
        time::Duration,
    };

    use tempfile::TempDir;

    use super::{
        MAX_REOPEN_ATTEMPTS, catch_busy_panic, hash_bounded_reader, is_retryable_open_error,
        is_sqlite_busy_or_locked_panic, open_db_with_retry, open_retry_jitter, reopen_backoff,
    };
    use super::{connect_db, connect_db_open_lock};
    use crate::errors::EngramError;

    /// Verify two concurrent `connect_db` calls to the same path do not panic.
    ///
    /// Regression test for U015-FLK1: `cozo` 0.7.x panics via an internal
    /// `unwrap()` when two processes open the same SQLite file concurrently.
    ///
    /// The process-level advisory file lock in `connect_db` serialises the
    /// `DbInstance::new` calls, ensuring both succeed rather than one panicking.
    ///
    /// # RED phase
    /// Before the fix: running two concurrent opens races on `DbInstance::new`
    /// and may panic.
    ///
    /// # GREEN phase
    /// After the fix: both calls succeed because the fd-lock serialises the
    /// `DbInstance::new` invocations.
    #[tokio::test]
    async fn concurrent_connect_db_does_not_panic() {
        let tmpdir = TempDir::new().expect("tempdir");
        let dir1 = tmpdir.path().to_path_buf();
        let dir2 = tmpdir.path().to_path_buf();

        // Issue two concurrent connect_db calls to the same branch path.
        let (r1, r2) = tokio::join!(
            connect_db(&dir1, "test-branch"),
            connect_db(&dir2, "test-branch")
        );

        // Neither call must panic.  Both must succeed because the lock
        // serialises the opens — the second waits for the first to complete.
        assert!(r1.is_ok(), "first concurrent connect_db failed: {r1:?}");
        assert!(r2.is_ok(), "second concurrent connect_db failed: {r2:?}");
    }

    /// Verify that schema bootstrap is also covered by the fd-lock, preventing
    /// the intra-process `SQLITE_BUSY` race (U015-FLK1 residual).
    ///
    /// When multiple callers race on `connect_db` for the same DB path, both
    /// `DbInstance::new` AND `run_schema_bootstrap` must be serialised by the
    /// advisory file lock.  Without this, two handles can reach schema
    /// bootstrap concurrently and trigger the cozo 0.7.x unwrap panic on
    /// `SQLITE_BUSY`.
    ///
    /// This test exercises higher concurrency (four simultaneous callers) to
    /// stress the bootstrap window, and verifies that every handle is usable
    /// (i.e., schema is consistent) after all opens complete.
    ///
    /// # RED phase
    /// Before the fix: `run_schema_bootstrap` runs outside the lock; four
    /// concurrent callers can race on schema writes and panic.
    ///
    /// # GREEN phase
    /// After the fix: `run_schema_bootstrap` runs inside the `spawn_blocking`
    /// closure while the lock is held; callers queue up and all succeed.
    #[tokio::test]
    async fn concurrent_connect_db_schema_bootstrap_does_not_race() {
        let tmpdir = TempDir::new().expect("tempdir");
        let base = tmpdir.path().to_path_buf();

        // Four concurrent callers on the same branch/path.
        let (r1, r2, r3, r4) = tokio::join!(
            connect_db(&base, "schema-race-branch"),
            connect_db(&base, "schema-race-branch"),
            connect_db(&base, "schema-race-branch"),
            connect_db(&base, "schema-race-branch"),
        );

        assert!(r1.is_ok(), "caller 1 failed: {r1:?}");
        assert!(r2.is_ok(), "caller 2 failed: {r2:?}");
        assert!(r3.is_ok(), "caller 3 failed: {r3:?}");
        assert!(r4.is_ok(), "caller 4 failed: {r4:?}");
    }

    /// Verify same-path callers reuse a shared in-process mutex.
    #[test]
    fn connect_db_open_lock_reuses_mutex_per_db_path() {
        let path = PathBuf::from("same-path");
        let other = PathBuf::from("other-path");

        let first = connect_db_open_lock(&path);
        let second = connect_db_open_lock(&path);
        let third = connect_db_open_lock(&other);

        assert!(
            Arc::ptr_eq(&first, &second),
            "same DB path must reuse the same in-process mutex"
        );
        assert!(
            !Arc::ptr_eq(&first, &third),
            "different DB paths must not share the same in-process mutex"
        );
    }

    // ── 086.002-T: bounded reopen-retry driver ──────────────────────────────

    // Only transient busy/locked errors are retryable; other errors are not.
    #[test]
    fn is_retryable_open_error_matches_busy_and_locked() {
        assert!(is_retryable_open_error(
            "Cannot open store: database is locked"
        ));
        assert!(is_retryable_open_error(
            "SQLITE_BUSY: the database file is busy"
        ));
        assert!(!is_retryable_open_error("no such table: calls_edge"));
        assert!(!is_retryable_open_error("disk I/O error"));
    }

    // The back-off stays within the documented capped-exponential envelope.
    #[test]
    fn reopen_backoff_is_bounded_and_capped() {
        for attempt in 0..MAX_REOPEN_ATTEMPTS {
            let ms = u64::try_from(reopen_backoff(attempt).as_millis()).unwrap_or(u64::MAX);
            assert!(
                ms >= 20,
                "attempt {attempt}: back-off must be at least the 20ms base floor: {ms}"
            );
            assert!(
                ms <= 375,
                "attempt {attempt}: back-off must be capped: {ms}"
            );
        }
    }

    // Jitter is bounded by its cap and produces variation (never a constant).
    #[test]
    fn open_retry_jitter_is_bounded_and_varies() {
        assert_eq!(open_retry_jitter(0), 0, "a zero cap must yield zero jitter");
        let mut any_nonzero = false;
        for _ in 0..200 {
            let j = open_retry_jitter(50);
            assert!(j < 50, "jitter must stay below the cap: {j}");
            if j > 0 {
                any_nonzero = true;
            }
        }
        assert!(any_nonzero, "jitter must introduce real variation");
    }

    // A transient busy is retried until the open succeeds, within budget.
    #[test]
    fn open_db_with_retry_succeeds_after_transient_busy() {
        let mut attempts = 0u32;
        let mut sleeps = 0u32;
        let result: Result<u32, EngramError> = open_db_with_retry(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err("database is locked")
                } else {
                    Ok(7u32)
                }
            },
            |_attempt| sleeps += 1,
        );
        assert_eq!(result.expect("must open within the retry budget"), 7);
        assert_eq!(attempts, 3, "must retry twice then succeed");
        assert_eq!(sleeps, 2, "must back off before each retry");
    }

    // A persistent busy is bounded and surfaces a clear EngramError (no panic).
    #[test]
    fn open_db_with_retry_gives_up_after_max_attempts() {
        let mut attempts = 0u32;
        let result: Result<u32, EngramError> = open_db_with_retry::<u32, &str, _, _>(
            || {
                attempts += 1;
                Err("database is locked")
            },
            |_attempt| {},
        );
        assert!(
            result.is_err(),
            "persistent busy must give up with an error"
        );
        assert_eq!(
            attempts, MAX_REOPEN_ATTEMPTS,
            "retry budget must be bounded by MAX_REOPEN_ATTEMPTS"
        );
    }

    // A non-busy open error is surfaced immediately without retrying.
    #[test]
    fn open_db_with_retry_surfaces_non_busy_error_immediately() {
        let mut attempts = 0u32;
        let result: Result<u32, EngramError> = open_db_with_retry::<u32, &str, _, _>(
            || {
                attempts += 1;
                Err("disk I/O error")
            },
            |_attempt| {},
        );
        assert!(result.is_err(), "a fatal open error must surface");
        assert_eq!(attempts, 1, "a non-retryable error must not retry");
    }

    // cozo 0.7.x unwraps internally on a transient reopen busy: the "database is
    // locked" surfaces as a PANIC. catch_busy_panic must convert it to a
    // retryable Err so the reopen-retry can absorb it (086.002-T F2).
    #[test]
    fn catch_busy_panic_converts_busy_panic_to_retryable_err() {
        let result: Result<u32, String> = catch_busy_panic(|| -> Result<u32, String> {
            panic!(
                "{}",
                "called `Result::unwrap()` on an `Err` value: \
                 SqliteFailure(DatabaseBusy, Some(\"database is locked\"))"
            );
        });
        let message = result.expect_err("a busy panic must become an Err");
        assert!(
            is_sqlite_busy_or_locked_panic(&message),
            "the converted error must classify as an SQLite busy/locked panic: {message}"
        );
    }

    // A transient SQLITE_LOCKED panic (distinct from SQLITE_BUSY) must ALSO be
    // converted to a retryable Err so it follows the bounded reopen-retry rather
    // than surfacing as a startup panic (Copilot PR#249).
    #[test]
    fn catch_busy_panic_converts_locked_panic_to_retryable_err() {
        let result: Result<u32, String> = catch_busy_panic(|| -> Result<u32, String> {
            panic!(
                "{}",
                "called `Result::unwrap()` on an `Err` value: \
                 SqliteFailure(DatabaseLocked, Some(\"database table is locked\"))"
            );
        });
        let message = result.expect_err("a locked panic must become an Err");
        assert!(
            is_sqlite_busy_or_locked_panic(&message),
            "the converted error must classify as an SQLite busy/locked panic: {message}"
        );
        assert!(
            is_retryable_open_error(&message),
            "the converted locked error must follow the bounded reopen-retry path: {message}"
        );
    }

    // Ok and non-panic Err values pass through catch_busy_panic unchanged.
    #[test]
    fn catch_busy_panic_passes_through_results() {
        let ok: Result<u32, String> = catch_busy_panic(|| Ok::<u32, String>(9));
        assert_eq!(ok.expect("Ok must pass through"), 9);
        let err: Result<u32, String> =
            catch_busy_panic(|| Err::<u32, String>("disk full".to_owned()));
        assert!(err.is_err(), "a non-panic Err must pass through");
    }

    // A NON-busy panic must be re-raised unchanged, never swallowed.
    #[test]
    #[should_panic(expected = "unrelated invariant")]
    fn catch_busy_panic_reraises_non_busy_panic() {
        let _: Result<u32, String> = catch_busy_panic(|| -> Result<u32, String> {
            panic!("{}", "unrelated invariant violated");
        });
    }

    // Copilot review (PR #249): an unrelated panic that merely MENTIONS "busy"
    // or "locked" must still be RE-RAISED — not misclassified as a retryable
    // SQLite busy and swallowed, which would mask a genuine bug.
    #[test]
    #[should_panic(expected = "worker is busy")]
    fn catch_busy_panic_reraises_unrelated_busy_panic() {
        let _: Result<u32, String> = catch_busy_panic(|| -> Result<u32, String> {
            panic!("{}", "worker is busy after invariant failure");
        });
    }

    // is_sqlite_busy_or_locked_panic matches SQLite-specific busy/locked markers
    // only, so an unrelated "busy"/"locked" panic message is not misclassified.
    #[test]
    fn is_sqlite_busy_or_locked_panic_matches_sqlite_markers_only() {
        // SQLITE_BUSY variants
        assert!(is_sqlite_busy_or_locked_panic(
            "called `Result::unwrap()` on an `Err` value: \
             SqliteFailure(DatabaseBusy, Some(\"database is locked\"))"
        ));
        assert!(is_sqlite_busy_or_locked_panic(
            "SQLITE_BUSY: the database file is busy"
        ));
        // SQLITE_LOCKED variants
        assert!(is_sqlite_busy_or_locked_panic(
            "SqliteFailure(DatabaseLocked, Some(\"database table is locked\"))"
        ));
        assert!(is_sqlite_busy_or_locked_panic("error code SQLITE_LOCKED"));
        // Unrelated panics that merely mention busy/locked must NOT match.
        assert!(
            !is_sqlite_busy_or_locked_panic("worker is busy after invariant failure"),
            "an unrelated busy panic must not match"
        );
        assert!(
            !is_sqlite_busy_or_locked_panic("the connection mutex is locked"),
            "an unrelated locked panic must not match"
        );
    }

    /// Verify the in-process mutex serializes same-path open attempts.
    #[tokio::test]
    async fn connect_db_open_lock_serializes_same_path_callers() {
        let path = PathBuf::from("serialized-path");
        let first_lock = connect_db_open_lock(&path);
        let second_lock = connect_db_open_lock(&path);
        let (first_acquired_tx, first_acquired_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (second_attempt_tx, second_attempt_rx) = mpsc::channel();
        let (second_acquired_tx, second_acquired_rx) = mpsc::channel();

        let hold_guard = tokio::task::spawn_blocking(move || {
            let _guard = match first_lock.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let _ = first_acquired_tx.send(());
            let _ = release_rx.recv();
        });

        assert!(
            first_acquired_rx
                .recv_timeout(Duration::from_secs(1))
                .is_ok(),
            "first caller must report that it holds the lock"
        );

        let wait_guard = tokio::task::spawn_blocking(move || {
            let _ = second_attempt_tx.send(());
            let _guard = match second_lock.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let _ = second_acquired_tx.send(());
        });

        assert!(
            second_attempt_rx
                .recv_timeout(Duration::from_secs(1))
                .is_ok(),
            "second caller must report that it is attempting the lock"
        );

        assert!(
            matches!(
                second_acquired_rx.recv_timeout(Duration::from_millis(250)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ),
            "second caller must remain blocked until the first guard is released"
        );

        assert!(
            release_tx.send(()).is_ok(),
            "test must be able to release the first caller"
        );

        assert!(
            second_acquired_rx
                .recv_timeout(Duration::from_secs(1))
                .is_ok(),
            "second caller must acquire the lock after the first releases it"
        );

        let first_result = hold_guard.await;
        assert!(
            first_result.is_ok(),
            "first blocking task must complete without panic: {first_result:?}"
        );

        let second_result = wait_guard.await;
        assert!(
            second_result.is_ok(),
            "second blocking task must complete without panic: {second_result:?}"
        );
    }

    /// Regression test for PR #385 review thread `PRRT_kwDORJEduc6gFv5l`
    /// (stash `1C8F1150`): a destination-write failure inside
    /// `hash_bounded_reader` must be attributed to the actual staging/
    /// destination path being WRITTEN, never to `source` (the file being
    /// READ) -- the pre-fix code passed `source` into the write-error
    /// mapping unconditionally. Uses a synthetic always-failing writer so a
    /// genuine write failure is forced deterministically and portably,
    /// without relying on OS-specific tricks (permission bits do not
    /// re-check on already-open handles, and disk-full devices like
    /// `/dev/full` are Linux-only).
    #[test]
    fn hash_bounded_reader_attributes_write_failures_to_the_destination_not_the_source() {
        struct AlwaysFailWriter;
        impl std::io::Write for AlwaysFailWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("synthetic destination write failure"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let source_dir = TempDir::new().expect("tempdir");
        let source_path = source_dir.path().join("published.db");
        std::fs::write(&source_path, b"some published bytes").expect("write source file");
        let expected_len = std::fs::metadata(&source_path)
            .expect("read source metadata")
            .len();
        let destination_path = PathBuf::from("staged-destination-should-be-named.tmp");
        let mut writer = AlwaysFailWriter;

        let error = hash_bounded_reader(
            &source_path,
            expected_len,
            Some((&mut writer, destination_path.as_path())),
        )
        .expect_err("a destination write failure must surface as an error");

        let message = error.to_string();
        assert!(
            message.contains("staged-destination-should-be-named.tmp"),
            "expected the error to name the destination path, got: {message}"
        );
        assert!(
            !message.contains(&*source_path.to_string_lossy()),
            "expected the error NOT to name the source path being read, got: {message}"
        );
    }
}
