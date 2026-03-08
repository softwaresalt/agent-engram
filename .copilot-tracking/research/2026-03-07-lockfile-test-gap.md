<!-- markdownlint-disable-file -->
# Task Research: `is_process_alive` live-process integration test gap

The `is_process_alive(pid: u32) -> bool` function in `src/daemon/lockfile.rs` has three
execution branches. Current unit tests cover the dead-PID and zero-PID branches but leave
the live-process branch (`return true`) untested, creating a gap in coverage of the stale-
lockfile cleanup logic.

## Task Implementation Requests

* Add a test that exercises the `is_process_alive(std::process::id()) == true` branch (live process).
* Optionally: add an end-to-end `acquire()` test that returns `LockError::AlreadyHeld` when a live daemon holds the lock.

## Scope and Success Criteria

* Scope: `src/daemon/lockfile.rs` (inline `#[cfg(test)]`) and/or `tests/unit/lockfile_test.rs`.
* Assumptions: No trait abstraction of sysinfo is available or required; tests must compile on Windows and Linux without spawning a daemon binary.
* Success Criteria:
  * `is_process_alive(std::process::id())` returns `true` in a test.
  * `is_process_alive(0)` returns `false` in a test (the PID-0 guard).
  * `is_process_alive(99_999_999)` returns `false` in a test (dead/unreal PID).
  * All three tests pass under `cargo test` on Windows and Linux without flakiness.

## Outline

1. Function signature and implementation analysis
2. Existing test coverage inventory
3. sysinfo 0.30 API notes
4. Test-approach evaluation and selection
5. Ready-to-paste test code
6. Risks and caveats

---

## Research Executed

### File Analysis

* `src/daemon/lockfile.rs` (full file, 245 lines)
  * `is_process_alive` — private `fn`, lines 189–195
  * `acquire_inner` — private `fn`, lines 78–183, calls `is_process_alive` at line 155
  * No `#[cfg(test)]` module exists in this file (lines 1–245 contain zero test code).

* `tests/unit/lockfile_test.rs` (112 lines)
  * Four tests: `s027`, `s029`, `s030` (Unix-only), `s032`
  * All tests use `tempfile::TempDir` and no async runtime.
  * No test exercises the `is_process_alive(pid) == true` branch.

* `Cargo.toml`
  * `sysinfo = "0.30"` in `[dependencies]`
  * `tempfile = "3"` in `[dev-dependencies]`
  * No mocking framework present.

* `Cargo.lock`
  * Resolved: `sysinfo 0.30.13`

### Code Search Results

* `is_process_alive` — defined only in `src/daemon/lockfile.rs:189`, called only at line 155.
* `#[cfg(test)]` — zero occurrences in `src/daemon/lockfile.rs`.
* `refresh_process` — one occurrence at `src/daemon/lockfile.rs:194`.

### Project Conventions

* Unit tests for a source module live in `tests/unit/<module>_test.rs` (external integration-style test files), registered as `[[test]]` entries in `Cargo.toml`.
* The lockfile module has no inline `#[cfg(test)]` block. Convention in this repo favors external test files, but inline tests are also acceptable for private-function coverage.
* `tempfile::TempDir` is the standard test helper.
* No `tokio::test` is used in lockfile tests (all synchronous).

---

## Key Discoveries

### Function Signature and Behavior

```rust
// src/daemon/lockfile.rs:189-195
fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;          // Branch A: PID 0 guard → always false
    }
    let mut sys = System::new();
    sys.refresh_process(Pid::from_u32(pid))
    // Branch B: sysinfo found the process → true  (UNTESTED)
    // Branch C: sysinfo did not find the process → false
}
```

**sysinfo 0.30.13 API:**
- `System::new()` — creates an empty system snapshot (does NOT load all processes).
- `sys.refresh_process(pid: Pid) -> bool` — probes the OS for exactly one PID.
  Returns `true` if the process exists in the OS process table, `false` otherwise.
  Works on Linux (reads `/proc/<pid>/stat`) and Windows (uses `OpenProcess` / `NtQuerySystemInformation`).
- `Pid::from_u32(pid)` — infallible conversion.

### Acquire Flow (how `is_process_alive` is reached)

```
acquire(workspace)
  └─ acquire_inner(workspace, allow_retry=true)
       ├─ try_write() → Ok(guard)  → write our PID, return DaemonLock  [S027, S032]
       └─ try_write() → WouldBlock
            ├─ read_pid() → Some(pid), is_process_alive(pid) == true
            │     → warn! + return AlreadyHeld { pid }          [UNTESTED via acquire()]
            ├─ read_pid() → Some(pid), is_process_alive(pid) == false, allow_retry=true
            │     → remove stale file + recurse(allow_retry=false)       [S029]
            ├─ read_pid() → Some(pid), allow_retry=false
            │     → return AlreadyHeld { pid }                  [retry exhausted path]
            └─ read_pid() → None
                  → return AlreadyHeld { pid: 0 }              [unreadable PID path]
```

### Existing Test Coverage Summary

| Scenario | Test | Branch Covered |
|---|---|---|
| Fresh workspace, no PID file | `s027` | `try_write() Ok` |
| PID file with dead PID (99999999), no OS lock | `s029` | `is_process_alive == false`, stale cleanup |
| Read-only run dir (Unix) | `s030` | `try_write() AcquisitionFailed` |
| Acquire → drop → re-acquire | `s032` | `try_write() Ok` twice |
| **Live PID, OS lock held** | **MISSING** | **`is_process_alive == true`** |

The `s029` test pre-creates the file with a dead PID but **no OS-level lock** — so `try_write()` succeeds immediately and `is_process_alive` is never called. The test exercises the stale-PID cleanup but not the live-process guard.

---

## Technical Scenarios

### Scenario 1 — Direct unit test of `is_process_alive` (recommended)

**Key insight:** `is_process_alive` is a private `fn` in `src/daemon/lockfile.rs`. It can only
be called directly from tests inside the same module (`#[cfg(test)]` block in the same file).
External test files in `tests/` cannot access private functions.

**Approach:** Add an inline `#[cfg(test)]` module to `src/daemon/lockfile.rs` with three sub-tests:

1. **Live branch** — `is_process_alive(std::process::id())` must return `true`.
   `std::process::id()` always returns the current process PID, which is guaranteed to be in
   the OS process table while the test runs. No spawning, no OS-specific concerns.

2. **PID-0 guard** — `is_process_alive(0)` must return `false`.

3. **Dead PID** — `is_process_alive(99_999_999)` must return `false`.
   This mirrors the existing external test's assumption. On any real system, PID 99 million
   will not exist.

**Why not the alternatives?**

| Alternative | Assessment |
|---|---|
| **Option A: `std::process::id()` in inline test** | ✅ **Selected** — Zero dependencies, cross-platform, deterministic, no spawning. |
| **Option B: Spawn a child process** | ❌ Requires `std::process::Command`, platform-specific PID retrieval, and race conditions on Windows where PID reuse is faster. Adds complexity with no extra coverage value. |
| **Option C: Trait abstraction / mock** | ❌ Requires refactoring `is_process_alive` to accept a trait object or generic, which is out of scope and changes the production API for test-only reasons. |
| **Option D: External `acquire()` round-trip with OS lock** | ⚠️ Requires spawning a subprocess that holds the fd-lock. Complex and slow. Belongs in integration tests (the daemon lifecycle tests already cover this end-to-end at a higher level). |

### Scenario 2 — `acquire()` end-to-end test for `AlreadyHeld` (optional, integration)

The integration test `t047_s039_s040_new_daemon_starts_after_crash` in
`tests/integration/daemon_lifecycle_test.rs` covers crash recovery but not the
"live daemon rejects second spawn" path. A dedicated integration test would:

1. Spawn daemon 1 (using `DaemonHarness`).
2. Attempt `DaemonLock::acquire(workspace)` directly (or spawn daemon 2).
3. Assert `Err(EngramError::Lock(LockError::AlreadyHeld { pid }))` where `pid == daemon1.pid`.

This is valuable but requires the daemon binary and is out of scope for a pure unit test.

---

## Proposed Test Code

### Part 1 — Inline unit tests in `src/daemon/lockfile.rs` (add at end of file)

```rust
#[cfg(test)]
mod tests {
    use super::is_process_alive;

    /// S031-A: is_process_alive returns true for the current (live) process.
    ///
    /// Uses std::process::id() which is guaranteed to be in the OS process
    /// table while this test is executing.  Works on Windows and Linux.
    #[test]
    fn s031a_is_process_alive_returns_true_for_current_pid() {
        let pid = std::process::id();
        assert!(
            is_process_alive(pid),
            "is_process_alive({pid}) must return true for the running test process"
        );
    }

    /// S031-B: is_process_alive returns false for PID 0 (guard branch).
    #[test]
    fn s031b_is_process_alive_returns_false_for_pid_zero() {
        assert!(
            !is_process_alive(0),
            "is_process_alive(0) must always return false"
        );
    }

    /// S031-C: is_process_alive returns false for an astronomically high PID
    /// that cannot correspond to a running process on any real OS.
    #[test]
    fn s031c_is_process_alive_returns_false_for_dead_pid() {
        assert!(
            !is_process_alive(99_999_999),
            "is_process_alive(99_999_999) must return false — no real process has this PID"
        );
    }
}
```

**File location:** Append to `src/daemon/lockfile.rs` after line 244 (the last `}` of
`clean_stale_socket`).

### Part 2 — Optional external `acquire()` test (add to `tests/unit/lockfile_test.rs`)

This tests `acquire()` when a stale PID file exists with the *current* process PID and no OS
lock. Because `is_process_alive(std::process::id())` returns `true`, `acquire()` must return
`AlreadyHeld` without retrying.

```rust
/// S031-D: acquire() returns AlreadyHeld when PID file contains a live PID
/// and no OS lock is held.
///
/// Note: This tests the code path at lockfile.rs:155-157.
/// Because try_write() succeeds (no OS lock on the file), this test actually
/// reaches the Ok(guard) branch and overwrites the PID.  To reach the
/// is_process_alive == true branch via acquire(), the OS lock must be held
/// simultaneously — which requires a concurrent file handle.
///
/// Verdict: this scenario CANNOT be exercised via DaemonLock::acquire() alone
/// in an external test without spawning a child process.  Use the inline
/// #[cfg(test)] approach (Part 1) instead.
///
/// This test is included as documentation of the limitation.
#[test]
#[ignore = "demonstrates acquire() limitation; use inline is_process_alive tests instead"]
fn s031d_acquire_with_live_pid_in_unlocked_file_succeeds_not_fails() {
    // When no fd-lock is held, try_write() succeeds regardless of the PID in the file.
    // The is_process_alive check is never reached through acquire() in this scenario.
    let dir = TempDir::new().expect("tempdir");
    let workspace = dir.path();
    let run_dir = workspace.join(".engram").join("run");
    fs::create_dir_all(&run_dir).expect("create run dir");
    let pid_path = run_dir.join("engram.pid");
    let mut f = fs::File::create(&pid_path).expect("create pid file");
    // Write the current (live) process PID — but no fd-lock held.
    writeln!(f, "{}", std::process::id()).expect("write live pid");
    drop(f);

    // acquire() sees WouldBlock=false (try_write succeeds), so is_process_alive
    // is never called.  The lock succeeds and overwrites the PID.
    let lock = DaemonLock::acquire(workspace).expect("should succeed without OS lock");
    assert_eq!(lock.pid(), std::process::id());
}
```

---

## Risks and Caveats

1. **PID 99_999_999 on Linux** — Linux PID_MAX is typically 4_194_304 (`/proc/sys/kernel/pid_max`).
   PID 99 million is safely above this. On Windows, PIDs are multiples of 4 up to ~4 million.
   The value 99_999_999 is safe on all platforms (sysinfo will not find it).

2. **`is_process_alive` is private** — It cannot be called from `tests/unit/lockfile_test.rs`.
   Inline `#[cfg(test)]` in the source file is the only way to unit-test this function directly.
   This is idiomatic Rust for testing private helpers.

3. **`System::new()` vs `System::new_all()`** — The implementation correctly uses `System::new()`
   (no global process load) followed by `refresh_process()` for a targeted single-PID probe.
   Tests depend on this being correct for the current process — verified: sysinfo 0.30 documents
   that `refresh_process` queries the OS for exactly one PID without needing a prior `new_all()`.

4. **Windows fd-lock behavior** — On Windows, `LockFileEx` locks are per-file-handle per-process.
   Two `RwLock` instances opened on the same file within the **same process** may or may not
   produce `WouldBlock` depending on Windows' sharing semantics. This is why testing the
   `is_process_alive == true` branch through `acquire()` end-to-end requires a second **process**,
   not just a second thread or file handle.

5. **No `#[cfg(test)]` module currently exists** in `src/daemon/lockfile.rs`. Adding one does not
   change any production behavior — it is gated behind `#[cfg(test)]`.

6. **Scenario numbering** — The existing tests use S027, S029, S030, S032. The new tests should
   be numbered consistently with project conventions. `S031` appears unused; `s031a/b/c` is
   suggested, or the team may prefer a fresh scenario number (e.g., S033+).

---

## Actionable Next Steps for Implementation

1. **Open** `src/daemon/lockfile.rs`.
2. **Append** the `#[cfg(test)] mod tests { ... }` block from Part 1 at the end of the file (after line 244).
3. **Run** `cargo test --test unit_lockfile` to confirm existing tests still pass.
4. **Run** `cargo test -p engram --lib` to run the new inline tests.
5. **(Optional)** Add the `#[ignore]`-tagged external test to `tests/unit/lockfile_test.rs` as
   documented evidence of why end-to-end coverage of this branch requires a spawned subprocess.
