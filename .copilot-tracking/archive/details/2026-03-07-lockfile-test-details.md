<!-- markdownlint-disable-file -->
# Implementation Details: `is_process_alive` Live-Process Unit Tests

## Context Reference

Sources:
* `.copilot-tracking/research/2026-03-07-lockfile-test-gap.md` — gap analysis, API notes, risk matrix, proposed test code
* `src/daemon/lockfile.rs` (full file, 244 lines) — production source, confirmed no existing `#[cfg(test)]` module
* `.github/instructions/rust.instructions.md` — Rust coding conventions

---

## Implementation Phase 2: Write Inline Test Module

<!-- parallelizable: false -->

### Step 2.1: Append `#[cfg(test)] mod tests { ... }` to `src/daemon/lockfile.rs`

**Why inline and not external?**
`is_process_alive` is a private `fn`. External test files in `tests/` cannot reference private
items. An inline `#[cfg(test)]` block in the same source file is the idiomatic Rust approach for
testing private helpers. This does not change any production behavior — the block is gated entirely
behind `#[cfg(test)]`.

**Insertion point:**

Append the block immediately after the current end of file (line 244, which is the closing `}` of
`clean_stale_socket`). The file should end with a blank line before the new module.

Current EOF context (lines 241–244 of `src/daemon/lockfile.rs`):
```
    // On non-Unix platforms the socket concept doesn't apply; suppress the
    // unused-variable warning.
    #[cfg(not(unix))]
    let _ = run_dir;
}
```

**Exact code to append** (copy-paste ready):

```rust
#[cfg(test)]
mod tests {
    use super::is_process_alive;

    /// Verifies the live-process branch of `is_process_alive`.
    ///
    /// # Safety Note
    ///
    /// `std::process::id()` returns this process's own PID, which is guaranteed
    /// to be present in the OS process table for the entire duration of the test.
    /// No external process spawning is required, making this deterministic and
    /// cross-platform (Linux + Windows).
    ///
    /// sysinfo 0.30: `System::new()` + `refresh_process(pid)` probes the OS for
    /// exactly one PID without loading the full process table.
    #[test]
    fn is_process_alive_returns_true_for_live_process() {
        let pid = std::process::id();
        assert!(
            is_process_alive(pid),
            "is_process_alive({pid}) must return true for the running test process"
        );
    }

    /// Verifies the PID-0 guard branch of `is_process_alive`.
    ///
    /// PID 0 is reserved by the OS on all platforms and is never a user process.
    /// The function short-circuits before querying sysinfo.
    #[test]
    fn is_process_alive_returns_false_for_pid_zero() {
        assert!(
            !is_process_alive(0),
            "is_process_alive(0) must always return false (PID-0 guard)"
        );
    }

    /// Verifies the dead/nonexistent-PID branch of `is_process_alive`.
    ///
    /// PID 99_999_999 cannot exist on any real OS:
    /// * Linux: `PID_MAX` ≤ 4_194_304 (`/proc/sys/kernel/pid_max`)
    /// * Windows: PIDs are multiples of 4 up to ~4 million
    ///
    /// sysinfo 0.30 returns `false` without panicking for out-of-range PIDs.
    #[test]
    fn is_process_alive_returns_false_for_nonexistent_pid() {
        assert!(
            !is_process_alive(99_999_999),
            "is_process_alive(99_999_999) must return false — no real process has this PID"
        );
    }
}
```

**Files:**
* `src/daemon/lockfile.rs` — append the block above after line 244 (current EOF)

**Imports to add:**
* None in the production section. The test module uses `use super::is_process_alive;` (within the
  `mod tests` block) and `std::process::id()` (from std, always in scope). No new top-level `use`
  statements are needed.

**Success criteria:**
* `src/daemon/lockfile.rs` compiles with `cargo build --lib`
* `cargo test --lib daemon::lockfile` collects exactly 3 tests: `is_process_alive_returns_true_for_live_process`,
  `is_process_alive_returns_false_for_pid_zero`, `is_process_alive_returns_false_for_nonexistent_pid`
* All 3 tests pass

**Context references:**
* `.copilot-tracking/research/2026-03-07-lockfile-test-gap.md` (Lines 174–210) — proposed test code
* `src/daemon/lockfile.rs` (Lines 189–195) — `is_process_alive` signature and body
* `src/daemon/lockfile.rs` (Lines 214–244) — `clean_stale_socket`, the last function before EOF

**Dependencies:**
* No previous steps

---

## Implementation Phase 3: Validation

<!-- parallelizable: false -->

### Step 3.1: Run scoped test to verify new tests pass

```bash
cargo test --lib daemon::lockfile
```

Expected output (3 tests, all `ok`):
```
running 3 tests
test daemon::lockfile::tests::is_process_alive_returns_false_for_nonexistent_pid ... ok
test daemon::lockfile::tests::is_process_alive_returns_false_for_pid_zero ... ok
test daemon::lockfile::tests::is_process_alive_returns_true_for_live_process ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; ...
```

### Step 3.2: Run full lib test suite for regressions

```bash
cargo test --lib
```

All previously passing tests must continue to pass.

### Step 3.3: Run clippy

```bash
cargo clippy --lib -- -D warnings
```

Zero warnings. Common issues to watch:
* Dead-code warning if `is_process_alive` was somehow unused — not applicable here (called at line 155).
* Unused import — not applicable (no new imports added).

### Step 3.4: Check formatting

```bash
cargo fmt --check
```

If formatting issues:
```bash
cargo fmt
```

### Step 3.5: Blocking issue escalation

If a test fails or is flaky:

| Symptom | Probable Cause | Action |
|---|---|---|
| `is_process_alive_returns_true_for_live_process` fails | sysinfo 0.30 API regression | Pin sysinfo version; file bug |
| `is_process_alive_returns_false_for_nonexistent_pid` panics | sysinfo panics on out-of-range PID | Wrap in `std::panic::catch_unwind`; document |
| `is_process_alive_returns_false_for_nonexistent_pid` returns `true` | OS assigned PID 99_999_999 (impossible in practice) | Change PID to `u32::MAX` |

Do not attempt large-scale refactoring inline. Document and escalate.

---

## Dependencies

* Rust stable toolchain with `cargo test`, `cargo clippy`, `cargo fmt`
* `sysinfo = "0.30"` (already in `Cargo.toml` — no change required)

## Success Criteria

* `cargo test --lib daemon::lockfile` → 3 tests pass
* `cargo test --lib` → no regressions
* `cargo clippy --lib -- -D warnings` → 0 warnings
* `cargo fmt --check` → clean
