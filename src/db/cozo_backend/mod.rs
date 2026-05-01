//! CozoDB backend — Phase 2 connection and handle management.
//!
//! `CozoHandle` is a unit-struct marker for schema unit tests.
//! `CozoDb` is the real production handle backed by `Arc<cozo::DbInstance>`.

pub mod schema;

use std::{path::Path, sync::Arc, time::Duration};

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
/// opens a SQLite-backed `cozo::DbInstance`, and bootstraps the schema
/// idempotently (`:create` errors for existing relations are silently ignored).
///
/// The file lock is held only for the duration of `DbInstance::new` and is
/// released automatically before this function returns.  `cozo`'s own SQLite
/// WAL handles concurrent access from multiple in-process handles after the
/// initial open.
///
/// # Errors
///
/// Returns [`EngramError`] when the directory cannot be created, the lock
/// file cannot be opened, the database open times out (> 5 s) waiting for
/// the lock, the database cannot be opened, or schema bootstrap fails with
/// an unexpected error.
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

    // Acquire a process-level advisory file lock before opening CozoDB to prevent
    // the concurrent-open panic in cozo 0.7.x (U015-FLK1: internal `unwrap()` on
    // SQLite lock contention).  The lock is held only during `DbInstance::new`;
    // CozoDB's own SQLite WAL handles concurrent access after the handle is open.
    //
    // `spawn_blocking` + `tokio::time::timeout` are required because
    // `fd_lock::RwLock::write()` blocks the calling thread and must not run on
    // the async executor.  This matches the plan-review R1 advisory.
    let db = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || -> Result<cozo::DbInstance, EngramError> {
            let lock_file = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(&lock_path)
                .map_err(|e| map_db_err(format!("cannot open CozoDB lock file: {e}")))?;
            let mut file_lock = fd_lock::RwLock::new(lock_file);
            // Blocks until the exclusive lock is acquired; released on `_guard` drop.
            let _guard = file_lock
                .write()
                .map_err(|e| map_db_err(format!("cannot acquire CozoDB lock: {e}")))?;
            cozo::DbInstance::new("sqlite", &db_path_str, Default::default())
                .map_err(|e| map_db_err(format!("cannot open CozoDB SQLite store: {e}")))
        }),
    )
    .await
    .map_err(|_| map_db_err("database locked by another process (5 s timeout)"))?
    .map_err(|join_err| map_db_err(format!("DB open task panicked: {join_err}")))??;

    let cozo_db = CozoDb {
        inner: Arc::new(db),
    };

    schema::run_schema_bootstrap(&cozo_db)?;

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
}
