---
type: stage-session-memory
timestamp: 2026-09-02T23:58:00Z
agent: stage
session_id: stage-separate-indexer-rev6-20260902
phase: confirmation-failed-harvest-blocked
supersedes: docs/memory/2026-09-02/separate-indexer-plan-review-circuit-breaker.md
---

# Stage Session — Separate Indexer Revision 6 Confirmation

## Operator direction

The operator explicitly reset the plan-review circuit breaker and authorized
continuation. Directed remediations:

1. Module separation (not mere ordering) for the co-owned
   `src/daemon/ipc_server.rs`: dedicated startup activation, request entry,
   error transport, and lifecycle policy modules with an unambiguous
   integration seam.
2. Fix the Rust confirmation issue by establishing minimal workspace membership
   before the supervisor crate RED harness can run.

Frozen architecture (not reopened): separately packaged `engram-indexer`;
immutable generations; no live generation control endpoint; initial activation
gates readiness; post-start activation background/single-flight and never
blocking reads; pinned request context across all read inputs; Managed mode
compatibility; direct IPC/CLI/stdio MCP only; retired HTTP/SSE removed; trusted
same-user threat model; no destructive generation cleanup.

## Work performed

* Ran Step 0.0 tool gate. `TOOL_OK: backlogit CLI 1.10.1`. MCP tools not exposed
  in this invocation, so operated in `DEGRADED_MODE` on registry-declared CLI
  fallbacks. `INDEX_SYNC_OK` (1173 artifacts). Hook queue empty.
* Checkpoint enumeration surfaced pre-existing debris (see Anomalies below).
  None relates to this work.
* Authored `## Remediation Revision 6 — Operator-Directed Module Separation`
  in `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`.
  Boundary/ownership changes only; no design rewrite.
* Ran ONE bounded confirmation (Rust Reviewer + Architecture Strategist, both
  claude-opus-5) scoped to the final executable roster only.
* Recorded `## Plan Review — Revision 6` with the FAIL verdict.

## Revision 6 content

**Section A — IPC server module separation.** New unit F04a extracts a
composition-root seam plus `startup_activation.rs`, `request_entry.rs`,
`error_transport.rs`, `lifecycle_policy.rs`. Behavior-preserving; each module
ships pass-through so the tree is GREEN at extraction. Seam signatures declared
up front so no downstream unit edits the seam. Single-admission rule:
`request_entry::admit` is sole admission authority; F18 only publishes
`ReadinessView`. `ipc_server.rs` ownership reduced from five unordered owners to
`F04a -> F04`.

**Section B — Workspace membership first.** New unit F12a creates the minimal
stub crate and adds `crates/engram-indexer` to root `[workspace] members`.
F12 amended to depend on F12a; F13 no longer edits root `Cargo.toml`, so the
Revision 5 `F47 -> F13` edge is dropped.

**Section C — Residual serialization.** `F25 -> F26` (`src/tools/read.rs`),
`F27 -> F45` (`src/tools/lifecycle.rs`), `F23 -> F42` (`src/cli/runner.rs`).

## Confirmation result: FAIL — harvest blocked

Both operator-directed findings CONFIRMED RESOLVED. Graph confirmed acyclic;
`server -> generation activation/context -> db` layering holds; no
declared-ownership file lacks a total order.

Three new blocking findings:

* **P0-1** New RED harnesses are never compiled. Repo has zero top-level
  `tests/*.rs` and 218 explicit `[[test]]` blocks in root `Cargo.toml`; files
  under `tests/<subdir>/` are not auto-discovered. ~30+ new harnesses would be
  inert, producing false GREEN. Root `Cargo.toml` is thereby implicitly
  co-owned by ~30 units — the same defect Revision 6 removed from
  `ipc_server.rs`, relocated to the manifest. Independently verified.
* **P1-1** F12's RED harness cannot compile: needs
  `crates/engram-indexer/Cargo.toml` dependency section in its file list.
* **P1-2** Missing edge `F16 -> F20`; F20 captures `Arc<ReadRequestContext>`
  which F16 defines.

Per operator instruction ("if any P0/P1 remains, halt and report; do not
harvest"), Stage halted. No feature, task, subtask, or shipment was created.

## Anomalies surfaced (pre-existing, unrelated)

Seven backlogit checkpoints have missing `agent`/`status` fields
(`checkpoint-20260806-182205`, `-20260808-025905`, `-20260801-065720`,
`-20260715-065126`, `-20260521-043100`, `-20260517-162840`, `-20260515-064853`).
Five further stage/ship checkpoints remain `active` from May-Aug 2026. All
verified unrelated to the indexer work. Flagged for operator hygiene; not
treated as a blocker because the operator explicitly supplied this session's
resumption artifact and scope.

## Next step

Revision 7 applying exactly three mechanical corrections:

1. Extend F12a to register every new `[[test]]` target with committed
   placeholder harness files; add `F12a -> F01` and `F12a -> {each unit adding a
   test file}`.
2. Add `crates/engram-indexer/Cargo.toml` (dependency section only) to F12.
3. Amend `F02 + F17 + F38 -> F20` to `F02 + F16 + F17 + F38 -> F20`.

Then a mechanical confirmation of those three corrections only — no further
multi-persona cycle is required — followed by harvest and shipment assembly.

## Boundary compliance

No production source modified. No branch created. No build run. No shipment
claimed. No PR. Nothing pushed. Planning artifacts only.
