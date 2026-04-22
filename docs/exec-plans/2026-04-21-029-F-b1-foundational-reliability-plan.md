---
title: "029-F Shipment B1 — Foundational Daemon Reliability"
description: "Implementation plan for WS-1 version handshake, WS-3 self-healing PID/lock, WS-5 .workspace-id identity"
source_document: "docs/decisions/2026-04-21-029-F-daemon-reliability-deliberation.md"
shipment: "006-S"
covering_feature: "029-F"
requires_plan_hardening: yes
plan_review_attempts: 2
---

## Source

This plan operationalizes the deliberation at `docs/decisions/2026-04-21-029-F-daemon-reliability-deliberation.md`, Option 2 (two-phase split), Phase B1.

## Primary Objective

Eliminate the three most acute classes of "engram won't load" failures: stale daemon binary mismatch, wedged or stale PID file, and duplicate-daemon spawns from canonical-path drift. Subsequent observability and validation work (B2) ships separately.

## Implementation Units

### Unit 1 — Version handshake + auto-respawn (WS-1)

Backlog: `029.001-C` with tasks `029.001.001-T` (red-phase harness), `029.001.002-T`, `029.001.003-T`.

* **Approach**: shared version module compiled into both binaries; IPC handshake exchanges versions; mismatch triggers controlled, bounded respawn. **Respawn trigger semantics**: respawn fires on EITHER (a) typed `IpcError::VersionMismatch` response from a daemon that understands the new handshake, OR (b) ANY initial-handshake failure against an already-running daemon process — connection-reset, deserialization error, unrecognized-request error, timeout. The "stale binary" hypothesis covers both cases, so the shim treats both identically: respawn once, retry once, then surface the original error if the second attempt also fails.
* **Test posture**: red-phase task `029.001.001-T` scaffolds `tests/contract/lifecycle_test.rs` (mismatch surfaces typed error) AND `tests/integration/version_mismatch_test.rs` (end-to-end respawn) as failing harnesses; implementation tasks make them pass.
* **Touched files**: `src/shim/version.rs` (new), `src/daemon/version.rs` (new), `src/lib.rs`, `src/daemon/protocol.rs`, `src/shim/ipc_client.rs`, `src/shim/lifecycle.rs`, `src/errors/mod.rs`.
* **Error placement**: extend `EngramError` via `IpcError::VersionMismatch { expected: String, actual: String }` in `src/errors/mod.rs`. Do NOT add a new top-level variant.
* **Constants**: pipe-reachability probe timeout (100 ms) and max respawn attempts (1) live as named `pub(crate) const` in `src/shim/lifecycle.rs`, not magic literals.

### Unit 2 — Self-healing PID/lock files (WS-3)

Backlog: `029.002-C` with tasks `029.002.001-T` (red-phase harness), `029.002.002-T`, `029.002.003-T`.

* **Approach**: PID file schema gains `start_time_unix`; verify alive via PID + start-time match (defeats PID reuse); atomic temp+rename writes; pipe reachability probe before trusting a "live" PID.
* **Atomic write requirement (Windows-safe)**: temp PID file MUST be created with `tempfile::NamedTempFile::new_in(pid_dir)` (same directory as the final `engram.pid`), then `.persist()` to the target path. Cross-volume rename via the default temp dir is REJECTED — fails on Windows (`ERROR_NOT_SAME_DEVICE`). Test `029.002.003-T` must include a Windows runner case verifying same-directory persist semantics.
* **Workspace isolation AC**: PID file target path MUST be validated against bound workspace root before write; out-of-bounds target returns `WorkspaceError::PathEscape`.
* **Test posture**: red-phase task `029.002.001-T` scaffolds unit tests for liveness/reuse/atomic-write AND `tests/integration/stale_pid_recovery_test.rs` as failing harnesses.
* **Touched files**: `src/shim/pidfile.rs` (new), `src/shim/lifecycle.rs`, `src/shim/ipc_client.rs`.
* **Test helper rule**: any test helper creating a temp workspace under `.engram/` MUST return the `TempDir` handle to the caller and keep it alive through the test body — see `docs/compound/test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md`. Drop-too-early is a known nondeterministic CI flake.

### Unit 3 — `.workspace-id` persistent identity (WS-5)

Backlog: `029.003-C` with tasks `029.003.001-T` (red-phase harness), `029.003.002-T`, `029.003.003-T`.

* **Approach**: persist UUIDv4 to `.engram/.workspace-id` on first bind; daemon discovery key switches from path-hash to workspace-id; legacy workspaces fall back to path-hash with deprecation log; ambiguous bind (path holds a different `.workspace-id` than current daemon binding) returns typed error.
* **`ambiguous_bind` detection scope**: detection is **shim-side only** — the shim reads `.engram/.workspace-id` from disk and compares it to the daemon's currently-bound workspace-id (already known to the shim from prior `set_workspace` response). No daemon protocol change required. This keeps Unit 3 from leaking back into Unit 1's protocol surface after Unit 1's approval gate. If implementation discovers daemon-side coordination is necessary, that triggers a re-plan, NOT a quiet protocol expansion.
* **Workspace isolation AC**: `.workspace-id` write path MUST resolve through `canonicalize_workspace` and be rejected if outside workspace root.
* **Error placement**: extend `EngramError` via `WorkspaceError::AmbiguousBind { expected_id: Uuid, found_id: Uuid, path: PathBuf }` in `src/errors/mod.rs`.
* **Test posture**: red-phase task `029.003.001-T` scaffolds unit tests for stable-id-across-canonical-forms AND `tests/integration/workspace_id_drift_test.rs` as failing harnesses.
* **Touched files**: `src/db/workspace.rs`, `src/tools/lifecycle.rs`, `src/shim/lifecycle.rs`, `src/errors/mod.rs`. **NOT** `src/daemon/protocol.rs` (per ambiguous_bind scope rule above).

## Sequencing and Dependencies

1. **Unit 1** first — establishes the protocol/version foundation other units depend on for protocol negotiation.
2. **Unit 2** second — reuses the version exchange to short-circuit stale-PID handshakes.
3. **Unit 3** third — discovery key change (path-hash → workspace-id) lands after PID infrastructure is stable so the rollback surface is smaller per unit.

Within each unit, tasks are dependency-ordered: the `.001-T` red-phase harness task MUST land first (failing tests + minimum compilation stubs); `.002-T` and `.003-T` then make the harness pass.

## Constitution Check

Mapping this plan against `.github/instructions/constitution.instructions.md`:

| Principle | Compliance | Notes |
|---|---|---|
| **I. Safety-First Rust** | ✅ | No new `unsafe` blocks; all new modules inherit `#![forbid(unsafe_code)]` from `src/lib.rs`. All fallible paths return `Result<T, EngramError>` via the placement reservations below. |
| **II. Test-First Development** (NON-NEGOTIABLE) | ✅ | Each Unit's task `029.NNN.001-T` is explicitly the **red-phase harness task** — it scaffolds the failing contract/integration test before any implementation task (`.002-T`, `.003-T`) is claimed. Harvest decomposition must preserve this ordering as a hard dependency. Harness-architect skill owns the red phase per `build-feature` SKILL contract. |
| **III. Workspace Isolation** | ✅ | All new on-disk writes (`.engram/run/engram.pid`, `.engram/.workspace-id`) resolve through `canonicalize_workspace` (per repo memory; `src/db/workspace.rs:15-52`) before any create/rename. AC for Unit 2 task `029.002.002-T` and Unit 3 task `029.003.001-T` MUST include "rejects target paths outside workspace root with typed `WorkspaceError::PathEscape`". |
| **IV. CLI Workspace Containment** (NON-NEGOTIABLE) | ✅ | No file operations outside `cwd` tree; `.workspace-id` lives at `.engram/.workspace-id` relative to bound workspace root. |
| **V. Structured Observability** | ✅ | Tracing events with stable field names per "Observability checkpoints" below. |
| **VI. Single Responsibility** | ✅ | New dependency: `tempfile` (likely already in tree — confirm at task time; if so, no new dep). No other additions. |
| **VII. Destructive Command Approval** (NON-NEGOTIABLE) | ✅ | No destructive shell commands in this plan. PA-1 and PA-4 require operator PR approval per strict-safety classification. |
| **VIII. Safety Modes** | ✅ | strict-safety pack active; ProposedAction table below classifies risky actions. |
| **IX. Git-Friendly Persistence** | ✅ | `.workspace-id` is a single-line UUID file (atomic, no merge conflicts). PID file is JSON with sorted keys. |
| **X. Context Efficiency** | N/A | No new MCP tool surface in B1 (deferred to B2 WS-2 doctor command). |

**Justified deviations**: none.

### Scope boundary clarification — partial WS-7

The integration test files named in Units 1–3 (`version_mismatch_test.rs`, `stale_pid_recovery_test.rs`, `workspace_id_drift_test.rs`) ARE the canonical WS-7 deliverables for these specific failure modes. B1 intentionally delivers a **partial WS-7 subset** scoped to the three reliability paths it ships. The remainder of WS-7 (`multi_session_resume_test.rs`, broader cross-platform smoke tests, daemon long-running stability) is deferred to B2.

This is NOT a scope leak — it is a documented partial delivery. Shipment manifest `006-S` and feature `029-F` should both note "B1 delivers WS-1, WS-3, WS-5, and the WS-7 subset bound to those paths; B2 delivers WS-7 remainder plus WS-2/4/6/8" rather than the cleaner-but-misleading "WS-7 deferred". Harvest must update the shipment manifest accordingly when this plan is decomposed.

### CI toolchain gap checkpoint

Per `docs/compound/workflow-issues/ci-rust-version-gap-clippy-lints-2026-04-20.md`, local toolchain (1.85) lags CI clippy lints. Each task author MUST run clippy against the CI-pinned toolchain (or align via `rustup override`) before PR submission to avoid avoidable fix-ci churn. Add this to the task-level definition-of-done in harvest output.

## Plan Hardening

### Hardening trigger summary

Triggers that flagged this plan for hardening: (a) IPC handshake protocol change (public contract between shim and daemon binaries); (b) on-disk schema change to PID file format; (c) daemon discovery key migration (path-hash → workspace-id) affecting every existing workspace; (d) introduction of an automatic shim respawn path (lifecycle-altering behavior on the agent's critical startup path).

### Reinforcing context consulted

* `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` — drove the per-phase shipment shape (B1 only this round).
* `docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md` — irrelevant to this plan but confirmed shipment-reconcile gate remains in force for the eventual ship.
* Repo memory: tree-sitter ABI + `canonicalize_workspace` Windows test convention — applies to the integration tests under Unit 1/2/3.
* `.github/instructions/strict-safety.instructions.md` — drives the ProposedAction classification below.
* `.github/instructions/release-observability.instructions.md` — drives the observation-window and rollback-trigger discipline below.

### Risky actions (strict-safety classification)

| ProposedAction | change_kind | targets | ActionRisk | approval_required | rollback |
|---|---|---|---|---|---|
| **PA-1** Introduce `version` field in IPC handshake; respawn shim on mismatch | protocol change + lifecycle change | `src/daemon/protocol.rs`, `src/shim/lifecycle.rs`, `src/shim/ipc_client.rs` | **high** | Yes — operator review at PR | Revert chore `029.001-C`; pre-change behavior is "opaque error on mismatch" (annoying but not destructive) |
| **PA-2** Change PID file schema to include `start_time_unix`; treat absent field as "stale, reclaim" | on-disk schema change | `src/shim/pidfile.rs`, `.engram/run/engram.pid` | **moderate** | No (PR review only) — schema is internal, no external consumers, fallback is force-reclaim which matches today's behavior | Revert chore `029.002-C`; old PID file format works with reverted code |
| **PA-3** Atomic temp+rename writes to PID file | implementation change | `src/shim/pidfile.rs` | **low** | No | Revert PA-3 alone — replaces direct writes with `tempfile::NamedTempFile::persist` |
| **PA-4** Persist UUIDv4 to `.engram/.workspace-id` on first bind; switch daemon discovery key from path-hash to workspace-id | on-disk artifact + discovery semantics change | `src/db/workspace.rs`, `src/tools/lifecycle.rs`, `src/shim/lifecycle.rs`, every existing `.engram/` workspace directory | **high** | Yes — operator review at PR; runtime verification on at least one staged workspace before merge | Revert chore `029.003-C`; legacy `.workspace-id` files become orphaned but harmless until manual cleanup |
| **PA-5** Pipe reachability probe before trusting a "live" PID (adds startup latency) | lifecycle behavior | `src/shim/ipc_client.rs` | **low** | No — bounded by 100ms timeout (tested in 029.002.003-T) | Revert PA-5 alone |

ActionResult tracking begins as `planned`; each PA moves to `applied` when its task lands and the chore PR merges. PA-1 and PA-4 must reach `applied` only after explicit operator approval per the approval_required column.

### Rollback triggers

| Trigger | Threshold | Action |
|---|---|---|
| `set_workspace` p95 latency | > 1500 ms | Revert Unit 3 (workspace-id); inspect path resolution cost |
| Shim respawn rate | > 2/hour per workspace | Revert Unit 1; investigate version-detection oscillation |
| Stale-PID false positives in CI | any | Revert Unit 2; investigate atomic-write race |
| Daemon discovery returns stale handle | any | Revert Unit 3; treat as data integrity bug |

### Observability checkpoints

* **Structured event logging (B1 substitute for telemetry counters)**: emit `tracing::info!` events with stable field names — `event_type`, `workspace_id`, `outcome`, `latency_ms` — for `respawn`, `pid_recovered`, `workspace_id_created`, `ambiguous_bind`, `version_mismatch_detected`. Operators can grep `.engram/logs/` and pipe to `jq` for ad-hoc analysis until B2 WS-8 lands proper counters.
* **CI matrix**: integration tests in this plan MUST run on both `windows-latest` and `ubuntu-latest` runners — pipe primitives, atomic-rename semantics, and PID liveness checks differ.
* **Manual smoke test before merge** (since `tests/integration/multi_session_resume_test.rs` is deferred to B2 WS-7): on a real workspace, kill the daemon mid-session, verify shim recovers via PID staleness path; bind a second workspace to the same path with cleared `.workspace-id`, verify `ambiguous_bind` error surfaces (Unit 3). Document outcome in PR description.

### Runtime verification (pre-deploy / pre-merge)

| Scenario | Environment | Expected result | Owner |
|---|---|---|---|
| Fresh shim against fresh daemon (matching versions) | dev workstation | normal `set_workspace` succeeds; no respawn event | task author |
| Shim against stale daemon (version skew injected) | dev workstation | one respawn event, then success | task author |
| Two shims racing to claim same workspace | dev workstation | second shim observes existing PID, reuses it | task author |
| Workspace bound at canonical path A then accessed via symlink path A' | dev workstation | both shims resolve to the same `.workspace-id`; daemon serves both | task author |
| `.engram/.workspace-id` exists with a UUID different from the daemon's bound UUID | dev workstation | `ambiguous_bind` typed error returned; shim does NOT auto-rebind | task author |

### Post-deploy / post-merge observation window

* **Window**: 7 calendar days after the shipment merges.
* **Owner**: shipment author (named in 006-S PR description).
* **Signals to watch**: any `version_mismatch_detected` event followed by repeated respawns within 60s on the same workspace; any `ambiguous_bind` error in operator logs; any `set_workspace` latency above the rollback threshold.
* **Reporting**: operator records observation-window outcome in `docs/closure/2026-MM-DD-006-S-closure.md` (created by Ship's operational-closure skill at merge).

### Backout plan

Each Unit lives behind a single chore. Reverting `029.001-C` reverts to opaque-error-on-mismatch behavior (no worse than today). Reverting `029.002-C` reverts to today's PID race window. Reverting `029.003-C` reverts to path-hash discovery (today's duplicate-daemon risk). Order of revert: Unit 3 → Unit 2 → Unit 1 (reverse of land order).

**Revert procedure**:

1. Identify the merge commit for the chore in question via `git log --oneline --grep '029\.NNN-C'`.
2. `git revert -m 1 <merge_sha>` on a hotfix branch.
3. Restore archived backlog files (if archived by Ship): `git restore .backlogit/archive/`.
4. Push hotfix branch, open PR with explicit "REVERT: chore 029.NNN-C — reason" title, request operator approval.
5. After merge, run the manual smoke test from the runtime verification table to confirm the revert restored prior behavior.

**Forward-fix vs revert decision rule**: if a rollback trigger fires within 24h of merge, prefer revert. Beyond 24h with downstream commits depending on the change, prefer forward-fix unless the trigger is a data-integrity bug (PA-4 stale-handle case) — that is always revert.

### Migration considerations

* **Legacy workspaces without `.workspace-id`** (every existing workspace today): Unit 3 includes a "create on first bind" path. No data migration required. First bind after the upgrade writes `.workspace-id` and uses it from then on. Daemon discovery uses path-hash as fallback when `.workspace-id` is absent (logged at INFO with `event_type=workspace_id_fallback`); deprecation warning logged after 30 days of fallback use per workspace.
* **Legacy PID files** written by old shims have no `start_time_unix`. Unit 2 treats absent field as "stale, reclaim" — equivalent to current force-reclaim behavior on shim startup. No data loss; worst case is one extra daemon restart on the first post-upgrade shim invocation per workspace.
* **No database migration**: SurrealDB schema is unchanged.
* **No breaking IPC change for in-flight sessions**: a daemon already running an old protocol version receives a version-mismatch from a new shim and the new shim respawns (the daemon stays). Old shims connecting to a new daemon: new daemon also recognizes the mismatch and rejects the request with a typed `protocol_version_mismatch` error (handled by 029.001.002-T contract test) — old shim surfaces the error to the operator who must upgrade.

### Operator checkpoints

* **Before merging Unit 1 PR**: operator reviews PA-1 ProposedAction; explicit approval required.
* **Before merging Unit 3 PR**: operator reviews PA-4 ProposedAction; explicit approval required; manual smoke test executed and outcome recorded in PR.
* **At T+24h post-merge of each unit**: shipment owner checks observation signals; if any rollback trigger fires, escalate per the "Forward-fix vs revert decision rule" above.
* **At T+7d post-merge of the full shipment**: observation window closes; outcome recorded in `docs/closure/`.

### Unresolved operator decisions

* Should `engram doctor` (B2 WS-2) be partially implemented in B1 to surface the new structured events through a CLI rather than requiring `grep` of log files? **Recommendation**: defer — doctor belongs cohesively to B2.
* Should the `version_mismatch_detected` event also fire on minor-version skew, or only on protocol-incompatible skew? **Recommendation**: only protocol-incompatible (defined as: any change in handshake message shape). Minor patch-version differences that share the protocol are silent. This is a Unit 1 implementation decision — operator may override during Unit 1 review.

## Self-Review Against Plan-Review Criteria

* **Source document referenced**: yes (`docs/decisions/2026-04-21-029-F-daemon-reliability-deliberation.md`).
* **Acceptance criteria traceable**: yes — every task in 029.001/002/003 has explicit AC items mapped to 029-F's top-level AC.
* **Plan Hardening section present**: yes (above).
* **2-hour rule**: each task scoped to ≤2 files, ≤2 functions, ≤3 test scenarios.
* **Width isolation**: each task is single-domain (code OR test infrastructure). No mixed code+docs tasks.
* **Atomic milestones**: each task produces a passing test as verifiable outcome.
* **Out-of-scope explicit**: WS-2/4/6/7/8 deferred to Shipment B2; cross-platform telemetry counters deferred to WS-8.

**Self-review verdict**: PASS (with hardening section satisfied inline).

## Requires plan hardening

yes — embedded above.

## Plan Review

**Conducted by**: Stage agent (rubber-duck personas: Constitution / Rust / Scope Boundary / Learnings Researcher)
**Attempts**: 2 (max 2 per skill)

### Attempt 1 — GATE: FAIL

P1 findings raised:

1. **Constitution Reviewer**: missing mandatory `## Constitution Check` section.
2. **Constitution Reviewer**: test-first posture not encoded strongly enough — task ordering permitted implementation before harness.
3. **Rust Reviewer**: migration story unsound — old daemon does not return typed `IpcError::VersionMismatch`; only the new daemon does. Respawn trigger must catch ANY initial-handshake failure.
4. **Rust Reviewer**: atomic temp+rename underspecified for Windows — `tempfile::NamedTempFile` MUST be created with `new_in(pid_dir)` to avoid `ERROR_NOT_SAME_DEVICE` on cross-volume rename.
5. **Scope Boundary Auditor**: WS-7 boundary ambiguity — plan says WS-7 deferred but Units 1-3 each ship canonical WS-7 test files.

P2 findings raised: workspace-isolation AC for new write paths; error-shape placement convention; `ambiguous_bind` shim-side-only declaration; TempDir lifetime compound reference; CI clippy toolchain gap checkpoint.

### Attempt 2 — Fix-up applied

All five P1s addressed in this plan:

1. ✅ `## Constitution Check` section added with per-principle compliance table (above).
2. ✅ Each Unit's `029.NNN.001-T` task explicitly redesignated as **red-phase harness** task. Task descriptions and AC rewritten in `.backlogit/queue/029.001.001-T.md`, `029.002.001-T.md`, `029.003.001-T.md` to scaffold failing tests + minimum compilation stubs only. Implementation moved to `.002-T` and `.003-T` siblings. Plan sequencing line updated. Shipment manifest `006-S.md` notes hard dependency.
3. ✅ Unit 1 "Respawn trigger semantics" added: respawn fires on EITHER typed mismatch OR any initial-handshake failure (connection-reset / deserialization / unrecognized-request / timeout) against an already-running daemon. Bounded to one retry.
4. ✅ Unit 2 "Atomic write requirement (Windows-safe)" added: `tempfile::NamedTempFile::new_in(pid_dir)` then `.persist()`. Cross-volume rename via default temp dir explicitly REJECTED. Windows runner test required.
5. ✅ "Scope boundary clarification — partial WS-7" section added explicitly framing B1 as a partial WS-7 delivery (3 of 5 tests). `006-S.md` description and `029-F.md` AC table both updated to mark each AC as B1 or B2 scope.

All P2s also addressed: workspace-isolation AC requires `WorkspaceError::PathEscape`; error placements named (`IpcError::VersionMismatch`, `WorkspaceError::AmbiguousBind`); `ambiguous_bind` declared shim-side only with re-plan trigger if daemon coordination needed; TempDir lifetime compound learning cited in Unit 2 + harness task descriptions; CI toolchain-gap checkpoint added.

### Final GATE: PASS

No remaining P0/P1 findings. The plan is harvest-ready.

**Open advisory items (P2/P3, not blocking)**:

* `engram doctor` partial-implementation question (defer to B2 — recommended)
* Minor-version skew handling in `version_mismatch_detected` event (Unit 1 implementation decision; default = protocol-incompatible only)
* The plan does not yet account for what happens if the post-deploy 7-day observation window finds NO events at all (silent success vs degraded telemetry path) — operator may want to add a "minimum signal threshold" check during closure.

These are recorded for shipment author awareness; harvest may proceed.
