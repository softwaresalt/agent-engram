//! CozoDB backend — Phase 2 connection and handle management.
//!
//! `CozoHandle` is a unit-struct marker for schema unit tests.
//! `CozoDb` is the real production handle backed by `Arc<cozo::DbInstance>`.

pub mod schema;

use std::{
    collections::HashMap,
    path::Path,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
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

type DbOpenLock = Arc<Mutex<()>>;

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
    let cozo_db = tokio::task::spawn_blocking(move || -> Result<CozoDb, EngramError> {
        let open_lock = connect_db_open_lock(&db_path);
        let _open_guard = match open_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

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
        let db = open_db_with_retry(
            || {
                catch_busy_panic(|| {
                    cozo::DbInstance::new("sqlite", &db_path_str, Default::default())
                })
            },
            |attempt| std::thread::sleep(reopen_backoff(attempt)),
        )?;
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
        MAX_REOPEN_ATTEMPTS, catch_busy_panic, is_retryable_open_error,
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
}
