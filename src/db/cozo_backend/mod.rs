//! CozoDB backend — Phase 2 connection and handle management.
//!
//! `CozoHandle` is a unit-struct marker for schema unit tests.
//! `CozoDb` is the real production handle backed by `Arc<cozo::DbInstance>`.

pub mod schema;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::errors::{EngramError, SystemError};
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

/// Sealed location of an existing published generation database on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingDbLocation {
    published_db_path: PathBuf,
}

impl ExistingDbLocation {
    /// Wrap the published generation database path that must be opened via a
    /// private runtime copy.
    #[must_use]
    pub fn new(published_db_path: impl Into<PathBuf>) -> Self {
        Self {
            published_db_path: published_db_path.into(),
        }
    }

    /// Return the published generation database path.
    #[must_use]
    pub fn published_db_path(&self) -> &Path {
        &self.published_db_path
    }
}

/// Private runtime copy path minted only by this module's open path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCopy {
    path: PathBuf,
}

impl RuntimeCopy {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Return the sealed runtime copy path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
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

    let runtime_copy = publish_runtime_copy(location.published_db_path(), &final_path)?;
    let final_path_str = runtime_copy
        .path()
        .to_str()
        .ok_or_else(|| map_db_err("runtime copy path is not valid UTF-8"))?;
    let db = open_db_with_retry(
        || catch_busy_panic(|| cozo::DbInstance::new("sqlite", final_path_str, Default::default())),
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
    published_db_path: &Path,
    final_path: &Path,
) -> Result<RuntimeCopy, EngramError> {
    let parent = final_path.parent().ok_or_else(|| {
        map_db_err(format!(
            "runtime copy destination {} must have a parent directory",
            final_path.display()
        ))
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|error| map_runtime_copy_io_error("create runtime directory", parent, error))?;

    let staging_path = runtime_copy_staging_path(final_path)?;
    if let Err(error) = std::fs::copy(published_db_path, &staging_path) {
        cleanup_runtime_copy_staging(&staging_path);
        return Err(map_runtime_copy_io_error(
            "copy published database to staging",
            &staging_path,
            error,
        ));
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

    #[cfg(unix)]
    sync_runtime_copy_parent_dir(final_path).map_err(|error| {
        map_runtime_copy_io_error("fsync runtime copy directory", final_path, error)
    })?;

    Ok(RuntimeCopy::new(final_path.to_path_buf()))
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
    m.contains("database is locked")
        || m.contains("database is busy")
        || m.contains("sqlite_busy")
        || m.contains("databasebusy")
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
    use std::{path::PathBuf, sync::Arc};

    use tempfile::TempDir;

    use super::{
        MAX_REOPEN_ATTEMPTS, catch_busy_panic, is_retryable_open_error,
        is_sqlite_busy_or_locked_panic, open_db_with_retry, open_retry_jitter,
        panic_payload_message, reopen_backoff,
    };
    use super::{connect_db, connect_db_open_lock};
    use crate::errors::EngramError;

    /// Verify two concurrent `connect_db` calls to the same path do not panic.
    #[tokio::test]
    async fn concurrent_connect_db_does_not_panic() {
        let tmpdir = TempDir::new().expect("tempdir");
        let dir1 = tmpdir.path().to_path_buf();
        let dir2 = tmpdir.path().to_path_buf();

        let (r1, r2) = tokio::join!(
            connect_db(&dir1, "test-branch"),
            connect_db(&dir2, "test-branch")
        );

        assert!(r1.is_ok(), "first concurrent connect_db failed: {r1:?}");
        assert!(r2.is_ok(), "second concurrent connect_db failed: {r2:?}");
    }

    /// Verify that schema bootstrap is also covered by the fd-lock, preventing
    /// the intra-process `SQLITE_BUSY` race (U015-FLK1 residual).
    #[tokio::test]
    async fn concurrent_connect_db_schema_bootstrap_does_not_race() {
        let tmpdir = TempDir::new().expect("tempdir");
        let base = tmpdir.path().to_path_buf();

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
    }

    #[test]
    fn panic_payload_message_extracts_string_forms() {
        let static_payload = Box::new("static panic") as Box<dyn std::any::Any + Send>;
        assert_eq!(
            panic_payload_message(static_payload.as_ref()),
            "static panic"
        );

        let owned_payload = Box::new(String::from("owned panic")) as Box<dyn std::any::Any + Send>;
        assert_eq!(panic_payload_message(owned_payload.as_ref()), "owned panic");
    }

    #[test]
    fn sqlite_busy_panic_classifier_matches_only_sqlite_busy_or_locked_markers() {
        assert!(is_sqlite_busy_or_locked_panic(
            "SqliteFailure(DatabaseBusy, Some(\"database is locked\"))"
        ));
        assert!(is_sqlite_busy_or_locked_panic("SQLITE_BUSY"));
        assert!(is_sqlite_busy_or_locked_panic(
            "SqliteFailure(DatabaseLocked, Some(\"database table is locked\"))"
        ));
        assert!(is_sqlite_busy_or_locked_panic("SQLITE_LOCKED"));
    }

    #[test]
    fn sqlite_busy_panic_classifier_does_not_match_non_sqlite_lock_panics() {
        assert!(
            !is_sqlite_busy_or_locked_panic("the connection mutex is locked"),
            "must not classify arbitrary lock wording as SQLite busy/locked"
        );
    }
}
