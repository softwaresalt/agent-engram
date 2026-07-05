---
date: 2026-07-05
type: impl-plan
task: 072.001-T
feature: 072-F
shipment: 074-S
title: Scope NotReady --direct hint to the daemon-startup path (add ShutdownTimeout variant)
width_domain: error-taxonomy / daemon shim lifecycle (compiled Rust)
blast_radius: low
status: reviewed (072.001-R accepted)
test_first: true
source_pr: 207
source_review: docs/closure/2026-07-04-notready-direct-hint-adversarial-review.md
---

# Impl-plan — 072.001-T: scope the NotReady `--direct` hint to startup only

## 1. Objective

073-S / PR #207 augmented `DaemonError::NotReady`'s `#[error(...)]` message with a
`--direct` escape-hatch hint. A **post-merge Copilot review on PR #207** found that
`NotReady` is a **shared** variant with two semantically different call sites:

| Call site | File / line | Meaning | `--direct` hint |
|---|---|---|---|
| `poll_until_ready` | `src/shim/lifecycle.rs:457` | genuine **startup** timeout (`timeout_ms = ready_timeout_ms()`, default 30 000) | **correct** — daemonless indexing is a valid escape |
| `wait_for_daemon_exit` | `src/shim/lifecycle.rs:392-395` | old daemon fails to **shut down** during respawn (`timeout_ms = SHUTDOWN_WAIT_TIMEOUT_MS = 2 000`) | **misleading & impossible** |

In the shutdown-wait path the hint cannot succeed: the stuck daemon still holds the
daemon lock, so `engram index --direct` immediately errors at
`src/cli/direct.rs:75-79` — `"daemon is already running (pid …); stop it before using
--direct mode"`.

**Goal:** give the shutdown-wait timeout its own message (no `--direct`), keep the
`--direct` hint only on the genuine startup timeout, and preserve the existing
`NotReady` wire contract byte-for-byte.

**Severity: LOW** — a misleading hint on a rare respawn path (only fires when the old
daemon fails to exit within 2 s during a version-mismatch / unhealthy-daemon respawn).
No data loss, no wrong action taken automatically. Real, but low-frequency.

## 2. Grounded current state (read 2026-07-05 @ branch off main `9b54694`)

| Fact | Evidence |
|---|---|
| `DaemonError` has exactly two variants: `SpawnFailed { reason }` and `NotReady { timeout_ms }`. | `src/errors/mod.rs:157-165` |
| `NotReady`'s message already carries the `--direct` / `ENGRAM_DIRECT=1` hint. | `src/errors/mod.rs:161-164` |
| `to_response` maps `Daemon(inner)` with an **exhaustive** `match inner { SpawnFailed, NotReady }`; `NotReady` → `(DAEMON_NOT_READY, "DaemonNotReady", inner.to_string(), Some({"timeout_ms": …}))`. | `src/errors/mod.rs:483-496` |
| `DAEMON_NOT_READY = 8006`; highest 8xxx IPC/daemon code in use is `WATCHER_INIT_FAILED = 8009`. Next free is **8010**. | `src/errors/codes.rs:55-64` |
| Startup call site: `poll_until_ready` returns `NotReady { timeout_ms }` after the ready deadline. **Keep as-is.** | `src/shim/lifecycle.rs:429-457` |
| Shutdown call site: `wait_for_daemon_exit(endpoint, pid_hint: Option<u32>)` returns `NotReady { timeout_ms: SHUTDOWN_WAIT_TIMEOUT_MS }` when the old daemon is still reachable/alive at the deadline. **Retarget this to the new variant.** | `src/shim/lifecycle.rs:378-400` |
| `wait_for_daemon_exit` is called only by `respawn_daemon`, which propagates via `?`. | `src/shim/lifecycle.rs:309-320` |
| No code matches on `DaemonError::NotReady` outside `to_response`; `respawn_daemon` errors flow up through `ensure_daemon_running_inner` via `?` (it only pattern-matches `IpcError` variants + a generic `Err(e)`). Adding a variant therefore breaks **only** the exhaustive `to_response` match — a **compile-enforced** reminder, not a silent drop. | `src/shim/lifecycle.rs:169-260`, `src/errors/mod.rs:483-496` |
| Existing NotReady tests pin: message contains `5000ms`/`--direct`/`ENGRAM_DIRECT=1`/brace-free, and wire `code == DAEMON_NOT_READY` + `name == "DaemonNotReady"`. | `src/errors/mod.rs:735-766` |
| `--direct` requires the daemon lock and fails if a daemon is running — confirming the hint is wrong for the shutdown path. | `src/cli/direct.rs:73-84` |

## 3. Design (chosen approach)

Introduce a **distinct** `DaemonError` variant for the shutdown-wait timeout and give
it its own wire identity. Leave `NotReady` (and its `--direct` hint) untouched.

### 3.1 New variant — `src/errors/mod.rs:157-165`

```rust
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Failed to spawn daemon process: {reason}")]
    SpawnFailed { reason: String },
    #[error(
        "Daemon failed to reach Ready state within {timeout_ms}ms; if startup keeps timing out, run 'engram index --direct' (or set ENGRAM_DIRECT=1) to index without the daemon"
    )]
    NotReady { timeout_ms: u64 },      // ← UNCHANGED (startup path)
    #[error(
        "Daemon failed to shut down within {timeout_ms}ms during respawn; the previous daemon is still running and holds the workspace lock — stop the running engram daemon process, then retry"
    )]
    ShutdownTimeout { timeout_ms: u64 },   // ← NEW (respawn shutdown-wait path)
}
```

**thiserror brace rule:** the only `{ }` in either attribute is `{timeout_ms}`; the new
message uses `'`, `(`, `)`, `=`, `;`, `—` only — no stray braces. (Same discipline the
073-S closure verified for `NotReady`.) Deliberately no `--direct` / `ENGRAM_DIRECT`
token in the shutdown message.

### 3.2 New wire code — `src/errors/codes.rs:55-64`

```rust
pub const DAEMON_SHUTDOWN_TIMEOUT: u16 = 8010;   // next free 8xxx (8009 = WATCHER_INIT_FAILED)
```

Do **not** reuse `8006`: a distinct code (a) keeps the `NotReady` wire contract
frozen (the 073-S closure explicitly worried about renumbering 8006), and (b) gives
machine consumers an unambiguous "stuck-shutdown" signal distinct from "startup-timeout".

### 3.3 New mapping arm — `src/errors/mod.rs:483-496`

```rust
DaemonError::ShutdownTimeout { timeout_ms } => (
    DAEMON_SHUTDOWN_TIMEOUT,
    "DaemonShutdownTimeout",
    inner.to_string(),
    Some(json!({ "timeout_ms": timeout_ms })),
),
```

The `Daemon(inner)` match is exhaustive, so the compiler *requires* this arm — no risk
of a silently unmapped variant.

### 3.4 Retarget the shutdown call site — `src/shim/lifecycle.rs:392-395`

```rust
if tokio::time::Instant::now() >= deadline {
    return Err(EngramError::Daemon(DaemonError::ShutdownTimeout {
        timeout_ms: SHUTDOWN_WAIT_TIMEOUT_MS,
    }));
}
```

`poll_until_ready` (`:457`) stays on `NotReady`. `respawn_daemon` and
`ensure_daemon_running_inner` need **no** change — they propagate via `?`.

### 3.5 Files touched (≤3)

`src/errors/mod.rs` (variant + mapping arm + tests), `src/errors/codes.rs` (one const),
`src/shim/lifecycle.rs` (one call-site edit). No schema, no CLI-arg, no daemon protocol
change; the new code is purely **additive** to the wire surface.

## 4. Test-first plan (Constitution II — test before impl)

Write these in `src/errors/mod.rs` `mod tests` **before** editing production code; the
new tests must fail to compile / fail assertions pre-edit, pass post-edit.

1. **`shutdown_timeout_message_omits_direct`**
   ```text
   let msg = DaemonError::ShutdownTimeout { timeout_ms: 2000 }.to_string();
   assert!(msg.contains("2000ms"));            // interpolation preserved
   assert!(!msg.contains("--direct"));         // MUST NOT steer to --direct
   assert!(!msg.contains("ENGRAM_DIRECT"));
   assert!(msg.to_lowercase().contains("shut down"));  // is a shutdown message
   assert!(!msg.contains('{') && !msg.contains('}'));  // brace-safe
   ```
2. **`not_ready_message_points_at_direct`** — *keep the existing test verbatim*: the
   startup message still contains `5000ms`, `--direct`, `ENGRAM_DIRECT=1`, brace-free.
   This is the paired positive assertion proving the hint stays on the startup path.
3. **`not_ready_wire_contract_unchanged`** — *keep*: `code == DAEMON_NOT_READY`,
   `name == "DaemonNotReady"`. (Optional, closes closure-F1: also
   `assert_eq!(payload.error.code, 8006);` to pin the literal external number.)
4. **`shutdown_timeout_wire_contract`** (new):
   ```text
   let payload = EngramError::from(DaemonError::ShutdownTimeout { timeout_ms: 2000 }).to_response();
   assert_eq!(payload.error.code, DAEMON_SHUTDOWN_TIMEOUT);
   assert_eq!(payload.error.name, "DaemonShutdownTimeout");
   assert_eq!(payload.error.details, Some(json!({ "timeout_ms": 2000 })));
   ```

**Optional (nice-to-have, may defer):** a lifecycle-level test that
`wait_for_daemon_exit` against a still-reachable endpoint returns `ShutdownTimeout`
(not `NotReady`). This needs a fake endpoint that stays reachable past the 2 s deadline
(an integration-style harness); the message + wire unit tests above are the primary gate
and are sufficient for a LOW-blast change. Ship may skip the lifecycle test if the
harness is disproportionate.

## 5. Blast radius — LOW

Additive new error variant + code + one retargeted call site + tests, all inside one
crate. The `NotReady` wire contract (code/name/details) is frozen; the new code `8010`
is additive and matched exhaustively (compile-enforced). No daemon protocol / IPC schema
/ CLI-arg / distribution change. **`plan-harden` NOT warranted** — this is not a
schema-evolution, CLI-distribution, or multi-family change; it is a contained, additive
error-taxonomy split with a compiler-verified mapping.

## 6. Open questions (operator / Ship decision)

- **Q1 (recommend defer):** Should `ShutdownTimeout` embed the concrete stuck **pid**
  in its message? `wait_for_daemon_exit` has `pid_hint: Option<u32>`, but thiserror's
  `#[error(...)]` is a single format string and cannot conditionally render an
  `Option` cleanly (`{pid:?}` prints `Some(1234)`/`None`). Preferred: keep the variant
  `{ timeout_ms }`-only with a generic "stop the running engram daemon process" phrasing
  (clean, brace-safe). Threading the pid (`ShutdownTimeout { timeout_ms, pid: Option<u32> }`
  + a small `Display` adapter) is a follow-up nicety, not required for the fix.
- **Q2 (recommend include):** Pin the literal `8006` in `not_ready_wire_contract_unchanged`
  (closure finding **F1**, advisory) while we are already editing the wire tests — cheap
  hardening of the external numeric contract. Keep it a one-line add; do **not** expand
  into a repo-wide contract-pin task here (that stays a separate hygiene item).
- **Q3 (confirm):** New code number `8010` — confirmed free (`codes.rs` uses 8001-8009).
  If the operator prefers a reserved block for daemon-lifecycle codes, reassign, but
  8010 is the natural next value.

## 7. Definition of Done

- §4 tests written first (new ones fail pre-edit); all green post-edit.
- Shutdown-wait message contains **no** `--direct` / `ENGRAM_DIRECT`; startup message
  still contains both; both brace-free.
- `NotReady` wire contract (`8006` / `DaemonNotReady` / `{timeout_ms}`) unchanged;
  `ShutdownTimeout` maps to `8010` / `DaemonShutdownTimeout` / `{timeout_ms}`.
- `≤3` files touched (`errors/mod.rs`, `errors/codes.rs`, `shim/lifecycle.rs`).
- Quality gates green: `cargo fmt --all -- --check`;
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`;
  `cargo test --all-targets`; `cargo audit`.
- Out of scope (untouched): broader error-taxonomy refactor, `IpcError::Timeout`
  wording, `src/bin/engram.rs` help enrichment.
