---
title: "Shipment 133-S runtime verification"
date: 2026-09-03
shipment_id: "133-S"
feature_id: "142-F"
surface: cli
adapter: cargo-test
verdict: PASS WITH FOLLOW-UP
---

## Shipment 133-S Runtime Verification

### Context

`133-S` (feature `142-F`) delivers **read-server foundations only**: 49
placeholder test-manifest registrations (F00), a strict `DaemonMode`
mode-contract parser (F02), an empty zero-dependency `engram-indexer` stub
crate added to workspace membership (F12a), an immutable `mode` field on
`AppState` with existing constructors forwarding unchanged (F03), and a
storage feasibility spike/decision doc (F01, no production code). Explicitly
out of scope: F04 (migrating existing `AppState::new(...)`-family call sites
onto `with_mode`), F06–F09 (real generation open/publish implementation),
F12 (real `engram-indexer` supervisor logic). **No user-facing runtime
behavior changes with this shipment** — every existing call site preserves
its current behavior unchanged, and the new crate does nothing yet.

Merged via PR #376, merge commit
`33a0a41e345cef8965b707346728d44fa5492daf` (two parents:
`c66d320ee2c...` prior `main` tip + `2005b3db94752dbe37946a98532c46dde1aad674`
feature branch HEAD), confirmed reachable from `origin/main` via
`git merge-base --is-ancestor`.

### Validator contract

Given the foundations-only, no-runtime-behavior-change scope, verification
is proportionate: confirm the release build succeeds, confirm the MCP tool
catalog and contract surfaces are unaffected by the new mode-contract/stub
crate/workspace-membership changes, and confirm no regression was introduced
in the existing contract suite. Full daemon-lifecycle probes
(`engram daemon-status`, `engram sync`) from the generic validator manifest
require a bound, running workspace daemon and are **not applicable** here —
this shipment does not touch daemon bind/sync/lifecycle behavior at all
(that surface is untouched pending F04/F06–F09/F12 in later shipments).

* Surface: CLI (release build + local, no-daemon-required commands), MCP
  tool catalog (`manifest`), and the existing MCP/CLI contract test suite.
* Adapter: `cargo build --release`; `target/release/engram.exe --version`;
  `target/release/engram.exe manifest`; `cargo test --test
  contract_shim_stdio_initialize --test contract_mcp_catalog_oracle --test
  contract_mcp_tool_catalog_parity --test contract_mcp_envelope --test
  contract_read_server_cli_mcp_parity --release`.
* Invariants: the MCP tool catalog remains schema-stable and drift-free; the
  stdio shim/daemon IPC contract suite shows no new failures introduced by
  this shipment; the new placeholder harnesses for F00/F02/F03 remain inert
  (`placeholder_registered`) and do not alter existing behavior.

### Environment prechecks

* Build: `cargo build --release` completed cleanly in 5m15s, no errors, at
  merge commit `33a0a41e`.
* CLI binary: `target/release/engram.exe --version` →
  `engram 0.3.0-rc.1+g33a0a41e-dirty`.
* `target/release/engram.exe manifest` (local, no-daemon-required MCP
  catalog dump) returned the full, well-formed tool list unchanged —
  confirms the new `DaemonMode`/`AppState::mode` plumbing did not disturb
  tool registration.

### Probe outcomes

| Probe | Result |
|---|---|
| `cargo build --release` | ok (5m15s) |
| `engram.exe --version` | ok |
| `engram.exe manifest` | ok (full catalog, well-formed JSON) |
| `contract_mcp_catalog_oracle` (9 tests) | ok — 9 passed, 0 failed |
| `contract_mcp_envelope` (F00 placeholder) | ok — 1 passed |
| `contract_mcp_tool_catalog_parity` (F00 placeholder) | ok — 1 passed |
| `contract_read_server_cli_mcp_parity` (F00 placeholder) | ok — 1 passed |
| `contract_shim_stdio_initialize` (19 tests) | 18 passed, 1 failed — see below |

`shim_aborts_unresolved_startup_after_client_disconnects` failed both in
isolation and alongside the rest of the suite. **Regression check**:
reproduced identically (same assertion failure, same panic signature) in an
isolated temporary worktree checked out at the pre-merge `main` tip
(`c66d320ee2ce8b0aab90e73bc07d4f81c3059862`, PR #375's merge commit) using
the same `cargo test --test contract_shim_stdio_initialize --release`
command. **Confirmed pre-existing on `main`, unrelated to 133-S** — this
shipment's manifest (F00/F01/F02/F03/F12a) never touches
`src/shim/`, `src/daemon/`, or `tests/contract/shim_stdio_initialize_test.rs`
(unchanged by the merge diff). The diagnostic worktree was removed
immediately after the comparison; no state or artifact from it is part of
this shipment's closure.

`daemon-status`, `workspace-status`, and `sync_workspace` probes from the
generic validator manifest were attempted and found **not applicable**: they
require a bound, running daemon session, which is out of scope for a
foundations-only shipment that does not touch daemon lifecycle behavior.

### Risky action state

No production-affecting risky action was taken during verification
(build + local no-daemon CLI commands + existing test suite only). The one
observed test failure is pre-existing and unrelated; no new risk was
introduced by this shipment's actual scope.

### Follow-up (why PASS WITH FOLLOW-UP, not plain PASS)

* `shim_aborts_unresolved_startup_after_client_disconnects` remains failing
  on `main` (pre-existing, confirmed unrelated to 133-S). Not a 133-S defect;
  no new stash entry captured for it since it predates this shipment and is
  already outside 133-S's manifest scope. Recommend a separate Stage-planned
  investigation.
* F01's storage-feasibility spike documents an accepted, unverified residual
  risk: Windows directory-entry durability for the selected
  replace-existing rename primitive is not independently proven by an
  executable probe (POSIX is proven via explicit parent-directory fsync;
  Windows has no equivalent safe-Rust primitive available). Already captured
  as deferred stash `F2E84E15` (source refs: task `142.004-T`, feature
  `142-F`, shipment `133-S`, PR #376). Required next action: F07/F08
  implementers (later shipments) must explicitly re-review this residual
  risk before treating Windows publication as crash-durable equivalent to
  POSIX.
* Four additional deferred-scope-expansion stash entries from PR readiness
  remain outstanding and unrelated to runtime behavior: `A7C0BA5F`
  (F00 placeholder-tracking mechanism), `5A7FBC37` (`#[deprecated]`
  attributes on temporary `AppState` constructors — F04's job),
  `58B33C45` (pre-existing full-suite flakiness under parallel execution),
  `7B270F79` (pre-existing `cargo ci`/`--all-features` opentelemetry compile
  break, present on `main` before this PR).

### Verdict and handoff

**PASS WITH FOLLOW-UP**. The release build succeeds, the MCP tool catalog
and contract surfaces are unaffected, and the one observed test failure is
confirmed pre-existing on `main` and unrelated to this shipment's scope.
This shipment introduces no new runtime behavior (mode contract and
`AppState::mode` are additive/inert pending F04's call-site migration; the
new crate is an empty stub pending F12), so there is no new production
surface to monitor beyond the existing invariants already tracked for the
daemon/shim/MCP surfaces. Feeding to `operational-closure` below.
