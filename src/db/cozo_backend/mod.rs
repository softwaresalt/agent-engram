//! CozoDB backend — Phase 2 connection and handle management.
//!
//! `CozoHandle` is a unit-struct marker for schema unit tests.
//! `CozoDb` is the real production handle backed by `Arc<cozo::DbInstance>`.

pub mod schema;

use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::errors::{EngramError, SystemError};

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
/// `{data_dir}/cozo/{branch}/engram.db.lock` to serialise concurrent opens,
/// opens a SQLite-backed `cozo::DbInstance`, bootstraps the schema
/// idempotently (`:create` errors for existing relations are silently
/// ignored), and returns the handle — all while the lock is held.
///
/// Holding the lock through schema bootstrap (not just through
/// `DbInstance::new`) prevents the intra-process `SQLITE_BUSY` race
/// where two handles could otherwise reach schema writes concurrently
/// (U015-FLK1 residual, stash `C4E8F2A1`).  The lock is released
/// automatically when the returned `CozoDb` handle leaves the
/// `spawn_blocking` closure.  CozoDB's own SQLite WAL handles
/// concurrent access from multiple in-process handles after the
/// initial open and bootstrap.
///
/// # Errors
///
/// Returns [`EngramError`] when the directory cannot be created, the lock
/// file cannot be opened, the advisory lock cannot be acquired within 5 s
/// (another process is opening the same DB), the database cannot be opened,
/// or schema bootstrap fails with an unexpected error.
pub async fn connect_db(data_dir: &Path, branch: &str) -> Result<Db, EngramError> {
    let branch_safe = branch.replace(['/', '\\', ':'], "_");
    let db_dir = data_dir.join("cozo").join(&branch_safe);
    std::fs::create_dir_all(&db_dir)
        .map_err(|e| map_db_err(format!("cannot create CozoDB dir: {e}")))?;

    let db_path = db_dir.join("engram.db");
    let lock_path = db_dir.join("engram.db.lock");
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| map_db_err("CozoDB path is not valid UTF-8"))?
        .to_owned();

    // Acquire a process-level advisory file lock before opening CozoDB and
    // hold it through schema bootstrap to prevent two variants of the
    // SQLITE_BUSY unwrap panic in cozo 0.7.x (U015-FLK1):
    //
    //   * Multi-process variant: two daemon processes open the same SQLite
    //     file concurrently — serialised by holding the lock during
    //     `DbInstance::new`.
    //
    //   * Intra-process variant (residual): two concurrent `connect_db`
    //     calls on the same DB path both complete `DbInstance::new`, then
    //     both call `run_schema_bootstrap` concurrently — serialised by
    //     holding the lock through bootstrap (this change, stash C4E8F2A1).
    //
    // `spawn_blocking` is required because all locking and DB-open work must
    // not run on the async executor.  `try_write()` is used in a polling
    // loop with a 5-second deadline so the task itself enforces the timeout —
    // there is no dangling background thread after a timeout return.  50 ms
    // polling interval keeps CPU overhead negligible while bounding the
    // worst-case latency.
    let cozo_db = tokio::task::spawn_blocking(move || -> Result<CozoDb, EngramError> {
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| map_db_err(format!("cannot open CozoDB lock file: {e}")))?;
        let mut file_lock = fd_lock::RwLock::new(lock_file);
        let deadline = Instant::now() + Duration::from_secs(5);
        // Poll with try_write so the thread respects the deadline and exits cleanly.
        let _guard = loop {
            if let Ok(guard) = file_lock.try_write() {
                break guard;
            } else if Instant::now() >= deadline {
                return Err(map_db_err(
                    "cannot acquire CozoDB lock: timed out after 5 s \
                     (another process is opening the same database)",
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let db = cozo::DbInstance::new("sqlite", &db_path_str, Default::default())
            .map_err(|e| map_db_err(format!("cannot open CozoDB SQLite store: {e}")))?;
        let cozo_db = CozoDb {
            inner: Arc::new(db),
        };
        // Bootstrap runs inside the lock so schema writes are serialised
        // across concurrent callers (intra-process U015-FLK1 residual fix).
        schema::run_schema_bootstrap(&cozo_db)?;
        Ok(cozo_db)
        // `_guard` dropped here — lock released after open + bootstrap
    })
    .await
    .map_err(|join_err| map_db_err(format!("DB open task panicked: {join_err}")))??;

    Ok(cozo_db)
}

// ── Error mapping ─────────────────────────────────────────────────────────────

/// Map any error value into an [`EngramError`] database error.
pub fn map_db_err<E: ToString>(err: E) -> EngramError {
    EngramError::from(SystemError::DatabaseError {
        reason: err.to_string(),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "cozo-backend"))]
mod tests {
    use tempfile::TempDir;

    use super::connect_db;

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
}
