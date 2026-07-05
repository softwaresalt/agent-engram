---
date: 2026-07-04
type: impl-plan
task: 065.004-T
feature: 065-F
shipment: 073-S
title: Point daemon-startup-timeout (NotReady) error at --direct / ENGRAM_DIRECT
width_domain: error-message / CLI help (compiled Rust)
blast_radius: low
status: reviewed (073.001-R accepted)
test_first: true
---

# Impl-plan — 065.004-T: NotReady error points at `--direct`

## 1. Objective

When daemon startup times out, the `DaemonError::NotReady` error message should
point the user at the daemonless escape hatch (`engram index --direct`, or
`ENGRAM_DIRECT=1`), so a repeatedly-timing-out daemon does not dead-end the user.

Acceptance criteria (verbatim from 065.004-T):

- `NotReady` error message includes an actionable `--direct` / `ENGRAM_DIRECT=1` hint.
- A unit/contract test asserts the augmented message (test-first).
- Quality gates green: `cargo fmt --all -- --check`;
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`;
  `cargo test --all-targets`; `cargo audit`.
- CLI help snapshot/contract tests updated **if** the top-level help summary changed.

## 2. Grounded current state (read 2026-07-04 @ main 237595b)

| Fact | Evidence |
|---|---|
| `DaemonError::NotReady { timeout_ms: u64 }` with `#[error("Daemon failed to reach Ready state within {timeout_ms}ms")]`. | `src/errors/mod.rs:161-162` |
| The `to_response` mapper emits `code=DAEMON_NOT_READY`, `name="DaemonNotReady"`, payload `{ "timeout_ms": ... }` — **string-independent**, so message text changes do not affect the wire contract. | `src/errors/mod.rs:488-492` |
| Precedent for a longer, actionable `#[error]` message already exists. | `IpcError::VersionMismatch` @ `src/errors/mod.rs:151-154` ("...Restart the daemon or rerun the shim...") |
| The **IPC** timeout (`IpcError::Timeout`, "IPC request timed out after {timeout_ms}ms") is a **different** error and is explicitly **out of scope**. | `src/errors/mod.rs:149-150` |
| An `#[cfg(test)] mod tests` block already exists in this file (test home). | `src/errors/mod.rs:719-732` |
| Top-level clap doc-comments live in `bin/engram.rs`; `#[command(name="engram", version, about)]`. | `src/bin/engram.rs:8-14`, subcommand docs `:26-56` |

## 3. Design

Augment the `NotReady` `#[error(...)]` string to append the escape-hatch hint:

```rust
#[error("Daemon failed to reach Ready state within {timeout_ms}ms; \
if startup keeps timing out, run 'engram index --direct' \
(or set ENGRAM_DIRECT=1) to index without the daemon")]
NotReady { timeout_ms: u64 },
```

### Caveats (from the task — non-negotiable)

- **`thiserror` brace rule:** the only `{ }` permitted in the string is the
  existing `{timeout_ms}` interpolation. The appended hint text must contain
  **no** `{` or `}` (they would be parsed as interpolation and fail to compile).
  → The hint uses `--direct`, `ENGRAM_DIRECT=1`, single quotes — no braces.
- **Do NOT** modify the `ENGRAM_DIRECT` `BoolishValueParser` (see
  `docs/compound/clap-bool-env-var-boolish-value-parser-2026-05-08.md`).
- Scope target is the daemon-**startup** timeout (`NotReady`). Re-wording the IPC
  timeout (`IpcError::Timeout`) is an optional follow-up, **not** in this task.

### Optional (stretch, default OFF — see §6)

Enrich the `Index`/`Sync` clap doc-comment in `bin/engram.rs` so `engram --help`
signposts `--direct`. Recommend **deferring** this to avoid churning any CLI
help snapshot/contract tests for a low-priority nicety; keep the change to
`errors/mod.rs` + its test (≤2 files) unless the operator wants the help hint.

## 4. Test-first plan (Constitution II — test before impl)

Add to `src/errors/mod.rs` `mod tests` (`:719`):

- `not_ready_message_points_at_direct`:
  ```text
  let msg = DaemonError::NotReady { timeout_ms: 5000 }.to_string();
  assert!(msg.contains("5000ms"));       // interpolation preserved
  assert!(msg.contains("--direct"));
  assert!(msg.contains("ENGRAM_DIRECT=1"));
  assert!(!msg.contains('{'));           // no stray thiserror braces
  ```
- `not_ready_wire_contract_unchanged` (regression guard): build the response via
  `EngramError::from(DaemonError::NotReady { timeout_ms: 5000 }).to_response()`
  and assert `code == DAEMON_NOT_READY` and `name == "DaemonNotReady"` — proves
  the message change did not disturb the machine-readable contract.

Write both **before** editing the `#[error]` string; the first test must fail on
the current string, then pass after the edit.

## 5. Blast radius — LOW

Single error-string edit + two unit tests, self-contained, **no daemon runtime**,
no async, no DB, no IPC. `to_response` mapping is string-independent, so no wire
contract moves. `plan-harden` **not** warranted. Width domain: compiled-Rust
error/CLI text only — deliberately isolated from the daemon event-loop work in
064.004-T (different shipment 072-S).

## 6. Open question (operator decision)

- **Help-hint stretch (Q1):** include the optional `bin/engram.rs` help
  enrichment, or keep it out? Recommend **out** (default) — the error-message hint
  fully satisfies the acceptance criteria, and touching top-level help risks
  churning `--help` snapshot/contract tests for a `low`-priority item. If
  included, the DoD's "update CLI help snapshot/contract tests if help changed"
  clause becomes active.

## 7. Definition of Done

- Both §4 tests written first; first fails pre-edit, both green post-edit.
- `cargo fmt` / `clippy -D warnings -D clippy::pedantic` / `cargo test
  --all-targets` / `cargo audit` all green.
- `≤3` files touched (`src/errors/mod.rs` + optional `src/bin/engram.rs`).
- `ENGRAM_DIRECT` `BoolishValueParser` untouched; hint text brace-free.
