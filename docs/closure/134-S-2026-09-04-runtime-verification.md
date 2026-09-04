---
title: "Shipment 134-S runtime verification"
date: 2026-09-04
shipment_id: "134-S"
feature_id: "142-F"
surface: cli, api
adapter: cargo-test
verdict: FAIL
---

## Shipment 134-S Runtime Verification

### Context

`134-S` (feature `142-F`) delivers: `142.008-T` (IPC server seam extraction
into four dedicated modules — `startup_activation.rs`, `request_entry.rs`,
`error_transport.rs`, `lifecycle_policy.rs`, plus four subtasks),
`142.009-T` (`AppState` mode constructor migration), `142.010-T` (shim
restart mode propagation), `142.003-T` (the stable error envelope), and
`142.005-T` (descriptor schema / tool descriptor registry work, plus
three subtasks). This shipment directly touches the daemon IPC composition
root, request admission, error transport, tool descriptor registry, and
`AppState` construction — squarely the `api` (MCP/IPC) runtime surface, and
`cli` indirectly via daemon startup/mode resolution.

Merged via PR #379, merge commit
`760b44752a0f00704bd1a6f88fb78f91bd4e997d`, confirmed reachable from
`origin/main` via `git merge-base --is-ancestor` (exit 0).

### Validator contract

* Surface: API/IPC (daemon composition root, request admission, error
  transport, tool descriptor registry), CLI (binary version/manifest,
  daemon mode resolution).
* Adapter: `cargo check --all-targets`; `cargo build --release`;
  `cargo build` (dev profile) + `target/debug/engram.exe --version` /
  `manifest`; `cargo test --test contract_ipc_server_seam --test
  contract_tool_descriptor_registry --test contract_error_codes --test
  unit_app_state_mode --test contract_app_state_constructor_migration
  --test integration_read_server_restart`.
* Invariants: `request_entry::admit` remains the sole request-admission
  authority; the tool descriptor registry completeness/vocabulary
  contracts hold; the new 16xxx/17xxx stable error-code ranges match
  contract; `AppState::with_mode` remains the only self-returning
  constructor and no convenience constructor exists; read-server mode
  survives auto-spawn and bounded restart.

### Environment prechecks

* `cargo check --all-targets` (dev profile): **PASS** — `Finished dev
  profile [unoptimized + debuginfo]` in 1m14s, no warnings-as-errors.
* `cargo build` (dev profile): **PASS** — binary produced, 1m09s.
* `cargo build --release`: **FAIL** — see Probe outcomes / Follow-up below.
* CLI binary (`target/debug/engram.exe --version`):
  `engram 0.3.0-rc.1+g760b4475` — ok.
* `target/debug/engram.exe manifest`: full, well-formed MCP tool catalog
  returned — confirms the tool descriptor registry / capabilities changes
  did not disturb tool registration.

### Probe outcomes

| Probe | Result |
|---|---|
| `cargo check --all-targets` | ok (1m14s) |
| `cargo build` (dev) | ok (1m09s) |
| `cargo build --release` | **FAIL** — `error: unused import: \`std::time::Duration\`` at `src/daemon/startup_activation.rs:11`, `-D unused-imports` |
| `engram.exe --version` (dev binary) | ok |
| `engram.exe manifest` (dev binary) | ok (full catalog, well-formed JSON) |
| `contract_app_state_constructor_migration` (5 tests) | ok — 5/5 passed |
| `contract_error_codes` (10 tests) | ok — 10/10 passed (includes `read_server_and_activation_error_codes_match_contract`) |
| `contract_ipc_server_seam` (5 tests) | ok — 5/5 passed |
| `contract_tool_descriptor_registry` (13 tests) | ok — 13/13 passed |
| `integration_read_server_restart` (3 tests) | ok — 3/3 passed, including `read_server_mode_survives_auto_spawn_and_bounded_restart` (78.89s) |
| `unit_app_state_mode` (3 tests) | ok — 3/3 passed |

36 of 36 targeted tests passed. The one failure is a `--release`-profile-only
compile break, not a test failure.

### Root cause of the release-build failure

`src/daemon/startup_activation.rs` imports `std::time::Duration` at line 11.
Its only use (line 93, `tokio::time::sleep(Duration::from_millis(delay_ms))`)
is inside a block gated `#[cfg(debug_assertions)]` — a test-only startup
delay hook driven by `ENGRAM_TEST_STARTUP_DELAY_MS`. When compiled with
`debug_assertions` off (i.e. `--release`), that block — and therefore the
only use of `Duration` — is compiled out, leaving the import unused.
`-D unused-imports` (implied by `-D warnings`) turns this into a hard
compile error in release profile only. `cargo check --all-targets` and
`cargo build` (both dev profile, `debug_assertions` on) do not observe
this, which is why it was not caught by the PR's recorded build evidence
(`cargo build --all-targets`, `cargo clippy --all-targets`, `cargo fmt`,
`cargo dev-test` — all dev-profile invocations).

This file was newly created by `134-S` (`142.008-T`, IPC seam extraction),
so this is a **genuine regression introduced by this shipment**, not a
pre-existing defect. It is release-artifact-only: it does not affect
`cargo dev-test`, the dev/debug binary, or any of the behavior asserted by
the 36 targeted tests above, all of which pass in full.

### Blocked prerequisites

`engram status` / `engram health` / `engram sync` (bound-daemon probes from
the generic validator manifest) were not exercised against a live bound
workspace daemon in this verification pass — the targeted contract/
integration suite already exercises daemon startup, mode resolution, and
restart behavior directly (`integration_read_server_restart`), which is a
stronger and more specific signal than a manual CLI daemon-status probe for
this shipment's actual scope. Not silently omitted: recorded here as
narrowed-but-covered rather than skipped.

### Risky action state

No production-affecting risky action was taken during verification (build +
local dev-profile CLI commands + existing/new test suites only, on
open closure PR #380's `post-merge/*` branch, which the closure record's
own next steps identify as the source of an operator-approved merge — this
verification pass itself made no source changes).

### Follow-up (why FAIL, despite passing dev-profile checks)

* **Release build regression (new, this shipment)**: `cargo build
  --release` fails to compile due to the unused `Duration` import described
  above. `cargo build --release` is an explicit mandatory validator target
  in this shipment's own validator contract (see above), so its failure
  classifies this verification as `FAIL` per the runtime-verification
  contract — `PASS WITH FOLLOW-UP` is reserved for a usable surface that
  merely needs cleanup or monitoring, not one that fails a mandatory
  target. This blocks producing an actual release/distributable binary from
  the current `main` tip until fixed. Captured as stash `6C9AA7D3`
  (kind: bug, priority: high; source refs: task `142.008-T`, feature
  `142-F`, shipment `134-S`, PR `379`). Fixing this is a same-contract-
  surface completion of `142.008-T`'s own already-authorized seam
  extraction (a one-line import-gating fix in a file that shipment itself
  created) — in scope for a future small fix, but out of scope for this
  post-merge closure session, which is limited to non-destructive
  evidence/closure artifacts and explicitly excludes further source changes
  on `main`. Recommend Stage plan a follow-up task promptly given severity
  (blocks release packaging).
* Five pre-existing deferred-scope-expansion stash entries carried from the
  PR readiness block remain outstanding and are not re-litigated here:
  `4EE241DC`, `E12542FF`, `1918AFD2`, `F95653D1`, `AA5698E3` (all P-021 C2
  captures from the build/review cycle; see PR #379 body "Out-of-scope
  findings").

### Verdict and handoff

**FAIL**. `cargo build --release` — an explicit mandatory validator target
for this shipment — fails to compile, so this verification classifies as
`FAIL` per the runtime-verification contract, notwithstanding that
`cargo check --all-targets` and `cargo build` (dev profile) succeed and the
full targeted contract/unit/integration suite for this shipment's actual
scope (36 tests across seam extraction, tool descriptor registry,
error-code contract, `AppState` constructor migration, and read-server
mode/restart behavior) passes in full; the MCP tool catalog is unaffected.
The one genuine, narrow, release-profile-only compile regression that
drives this `FAIL` verdict was discovered and is not fixed in this session
(out of scope for a non-destructive, evidence-only closure branch) —
captured as stash `6C9AA7D3` for prompt Stage-planned remediation. Feeding
to `operational-closure` below.
