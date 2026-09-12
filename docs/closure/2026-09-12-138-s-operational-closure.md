---
title: 138-S Generation Activation, Request Context, Startup Gate and Request Entry — Operational Closure
description: Post-merge release-readiness, monitoring, rollback, and follow-up record for shipment 138-S.
---

## Releasability

**Status: READY WITH CONDITIONS**

PR #391 merged via merge commit `81b19b0d91c79c9c456ce703dca42a4688978dfa`
(merge method: `gh pr merge 391 --merge`, operator-approved explicitly:
*"OPERATOR MERGE APPROVAL: PR #391 is explicitly approved for merge."*, for
the exact reviewed HEAD `6b39ee94`). Immediately pre-merge: full last-mile
gate re-run — headRefOid matched `6b39ee94`; CI green; 27/27 review threads
resolved; Copilot review present at the exact HEAD (P-018 satisfied);
`mergeable_state: clean`; repository settings allow merge-commit only
(squash/rebase disabled, P-009 satisfied); P-016 topology confirmed (single
worktree, correct branch). A local checkpoint commit (`1c3eb3e8`, created
during an operator-directed mid-review pause) was verified to have never
been pushed to the PR branch and was preserved instead by cherry-picking it
onto the post-merge closure branch as `ce3b2fba`, per explicit operator
instruction not to advance/re-arm review on the implementation PR.

The condition below mirrors the identical, already-established precedent
from 135-S/137-S's post-merge closures
(`docs/closure/2026-09-05-135-s-runtime-verification.md`,
`docs/closure/2026-09-08-137-s-runtime-verification.md`, "Post-merge re-run
addendum" / "Blocked prerequisites") and is assessed non-blocking for the
same reason: the live `cli-daemon-status` probe (manifest literal `engram
status`, which does not exist as a subcommand) was not attempted against
the shared-environment daemon (PID `30528`) in this closure session. That
daemon is independently known, via direct named-pipe IPC evidence gathered
at session resume, to be in a benign but real readiness-latch defect state
(`_health` reports `starting` forever after a branch/workspace-generation
switch with no transferred successor — root-caused this session to
`src/server/state.rs::publish_workspace_generation_transition` clearing
`hydration_ready` and `src/daemon/lifecycle_policy.rs::run_watcher_driver`
never calling `set_hydration_ready_for_permit` on the no-transferred-
successor path). This defect was assessed against the 138-S P-021 C1
same-contract-surface test (below) and captured as deferred scope — not
implemented, per explicit operator instruction. None of the 14 manifest
tasks' own harnesses exercise this specific readiness-latch defect path,
and the merged commit range does not touch `run_watcher_driver` or the
no-transferred-successor branch of `publish_workspace_generation_transition`
(confirmed via `git log 81b19b0d^1..81b19b0d^2 -- src/daemon/lifecycle_policy.rs
src/server/state.rs`, which returns files touched by `142.019-T`'s
`ReadRequestContext` addition only, not the watcher/transition logic in
question).

**Condition**: the readiness-latch defect (stash `265F99BE`) should be
triaged, deliberated, and (if accepted) planned by Stage as its own
release unit — it is out of 138-S's scope per P-021 C1 (138-S owns
generation *activation*, request context, startup *gate*, and request
*entry*; it does not own the branch-switch watcher-refresh readiness
handoff in `lifecycle_policy.rs`). This condition does not block the
closure PR — it blocks upgrading releasability from `READY WITH
CONDITIONS` to an unconditional `READY` for the shared-environment daemon's
own health signal (a signal that is orthogonal to, and already known to
pre-date, 138-S's own change surface).

| Requirement | Evidence |
|---|---|
| Healthy signal | `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` clean; `cargo test --all-targets --no-fail-fast` 2457/2460 GREEN (3 confirmed pre-existing, confirmed-unrelated flakes — see runtime verification); 87/87 targeted tests across all 14 manifest tasks' own harnesses GREEN (`integration_generation_activation` 28, `unit_read_request_context` 6, `integration_read_server_startup_activation` 7, `integration_request_entry_activation` 12, `contract_read_server_dispatch_refusal` 6, `contract_mcp_tool_catalog_parity` 6, `contract_cli_tool_catalog_parity` 6, `contract_read_input_ownership_inventory` 16). |
| Review | Pre-merge: 11 rounds of Copilot review findings recorded, fixed, and re-verified across the PR lifetime (rounds 2, 3, 4, 5, 6, 7, 8, 9, 10, 11), all 27 threads resolved at the merged HEAD; 2 findings suppressed with documented rationale and captured to stash for Stage triage (`5C873386` medium, `3FFEE99B` low — both explicitly out of scope per P-021 C1, not implemented). §1.9 local review readiness + P-018 Copilot-review gate both passed for the merged HEAD, re-verified at the last-mile gate immediately before merge. |
| Runtime verification | `docs/closure/2026-09-12-138-s-runtime-verification.md` — verdict `PASS WITH FOLLOW-UP`. Build/fmt/clippy GREEN; all 14 manifest tasks' own harnesses GREEN; full-suite 2457/2460 with 3 confirmed-unrelated flakes cited/stashed (`9D313653`, `58B33C45`, `EC3BAF22`); `cli-daemon-status` probe intentionally not attempted (named condition above). |

## Invariants to preserve

- Generation activation (`src/services/generations/activation.rs`) parses
  only typed, bounds-enforced manifests; `activate_initial` resolves store
  and deadline atomically; `maybe_activate_newer` is single-flight
  (asserted by `integration_generation_activation`); the rejection cache is
  immutable with transient backoff, never silently re-parsing a previously
  rejected revision indefinitely.
- `ReadRequestContext` construction (`src/server/state.rs`) is
  mode-agnostic and does not leak `Managed`-mode-only fields into
  `ReadServer` mode or vice versa (asserted by `unit_read_request_context`).
- Startup readiness (`src/daemon/startup_activation.rs`) is gated on
  successful initial generation activation; the daemon never reports ready
  before the initial generation has activated (asserted by
  `integration_read_server_startup_activation`).
- Request entry (`src/daemon/request_entry.rs`) enforces the documented
  ordering — descriptor resolution and authorization occur before
  background activation is triggered — and background reconciliation
  spawns remain bounded (asserted by `integration_request_entry_activation`).
- The capability gate at dispatch (`src/tools/mod.rs`) rejects a
  `Managed`-mode context reaching the read-server dispatch path and
  correctly captures context for every allowed request (asserted by
  `contract_read_server_dispatch_refusal`).
- The stdio MCP tool catalog (`src/shim/tools_catalog.rs`) and the CLI
  workflow surface (`src/cli/runner.rs`) are both derived from the same
  descriptor set with parity between them (asserted by
  `contract_mcp_tool_catalog_parity` and `contract_cli_tool_catalog_parity`).
- Every input reachable from a Read-mode descriptor is enumerated and
  classified in the read-input ownership inventory; an unclassified input
  fails closed rather than silently passing through (asserted by
  `contract_read_input_ownership_inventory`).
- `#![forbid(unsafe_code)]` remains enforced workspace-wide; no `unwrap()`
  or `expect()` was introduced by any of the 14 tasks (verified by
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, which
  denies `unwrap_used`/`expect_used` at the workspace level).
- `cargo dev-test` (default features) remains the canonical local merge
  gate and must stay GREEN modulo the pre-existing documented
  `archive_verifier_runs_the_unpacked_native_binary`,
  `full_channel_branch_switch_is_acknowledged_before_following_event`, and
  `cold_start_lists_and_maps_all_three_hcl_aliases` flakes.

## Pre-deploy audits

- Confirmed the 14-task commit range (`72532bf2` through `bb3edb4e`, plus
  fix/docs commits through `6b39ee94`) is additive/corrective to the
  generation-activation, request-context, startup-gate, and request-entry
  surfaces only — no unrelated production files were touched (verified via
  `git log --stat` review across the full merge range during the pre-merge
  review cycle).
- Confirmed no data-migration or schema change is introduced: activation
  reads existing on-disk generation manifests in the already-established
  format; no new persisted schema was added.
- Confirmed `Cargo.lock` changes (if any) were regenerated via `cargo
  check`/`cargo build`, not hand-edited.
- Confirmed the readiness-latch defect discovered this session
  (`265F99BE`) is a pre-existing condition in `lifecycle_policy.rs`/
  `state.rs`, not a regression introduced by the 14 merged tasks (the
  merged range does not touch `run_watcher_driver` or the
  no-transferred-successor branch of
  `publish_workspace_generation_transition`).

## Post-deploy checks

- `engram --version` confirmed against the merged binary (via existing
  `target\debug\engram.exe`, to avoid rebuild-lock contention with
  concurrently running verification tests):
  `engram 0.3.0-rc.1+gce3b2fba-dirty`, exit 0 (the `-dirty` suffix reflects
  this closure session's own uncommitted backlog-bookkeeping files at
  capture time, not uncommitted source).
- Direct named-pipe IPC read (`_health`, `query_memory`) against the
  running shared-environment daemon (PID `30528`) succeeded and returned
  correct workspace/protocol/build identity, confirming the daemon process
  and pipe transport are healthy even while `_health.status` remains
  `starting` (the readiness-latch condition itself, tracked separately at
  stash `265F99BE`).
- Recommended follow-up (non-blocking): re-run a live `cli-daemon-status`
  probe against a freshly-created workspace/branch (not the long-lived
  shared daemon) in a quiet environment to independently confirm the 14
  merged tasks do not themselves introduce any new startup-readiness
  regression, isolated from the pre-existing watcher-refresh defect.

## Monitoring

No new default-on runtime surface was introduced beyond what 138-S's own
manifest already covers (generation activation is invoked from existing
daemon startup and request-entry paths, not a new standalone service). The
daemon's existing structured JSON logs, `get_health_report`, and
`get_daemon_status` remain the monitoring surface for activation success/
failure, startup-gate timing, and request-entry authorization outcomes. The
readiness-latch defect (`265F99BE`) is separately monitorable via
`_health.status` remaining `starting` indefinitely after a branch/workspace
switch with no transferred successor — this signal already exists and
requires no new instrumentation to observe; it is a follow-up remediation
target, not a monitoring gap.

## Failure signals

- `cargo dev-test` or `cargo ci` turning newly red on `main` after merge,
  beyond the three already-documented pre-existing flakes.
- Any operator report that generation activation accepts a manifest outside
  its documented bounds, or that the rejection cache silently re-admits a
  previously rejected revision.
- Any operator report that daemon startup reports ready before initial
  generation activation completes.
- Any operator report that request entry triggers background activation
  before descriptor resolution/authorization, or that background
  reconciliation spawns grow unbounded.
- Any operator report that a `Managed`-mode context reaches the
  read-server dispatch path, or that the MCP tool catalog and CLI workflow
  surface diverge.
- Any operator report of an unclassified read-mode input passing through
  instead of failing closed.

## Rollback trigger

Any of the failure signals above, observed on `main` after merge and
attributable to this shipment's commits (not to the three pre-existing,
independently-documented flakes, and not to the separately-tracked
readiness-latch defect at stash `265F99BE`, which pre-dates this shipment).

## Rollback procedure

`git revert` of this shipment's commit range on `main` (the 14 manifest
tasks' implementation commits — `72532bf2`, `6d4c75e3`, `d2dfdbc7`,
`12d01386` (142.018-T + subtasks), `02febc8a` (142.019-T), `f9b65a18`
(142.028-T), `752af9a6` (142.029-T), `1e9ec2b3` (142.030-T), `bfdd51e5`
(142.031-T), `bb3edb4e` (142.032-T), `a95599a8`/`e656fad2` (142.033-T +
subtasks), plus their associated fix commits from the 11 review rounds and
done/archive bookkeeping commits). Reverting restores the pre-shipment
generation-activation absence, the pre-shipment `ReadRequestContext`
surface, and the pre-shipment startup/request-entry/dispatch/catalog
behavior. No production or runtime data is touched by this change
(source-only additions plus backlog-state bookkeeping) — rollback carries
no data-migration risk.

## Risky action record

| Field | Value |
|---|---|
| `ProposedAction` | (1) Merge PR #391 to `main`: generation activation service, mode-agnostic request context, startup readiness gate, request entry ordering, dispatch capability gate, descriptor-derived MCP/CLI catalogs, and read-input ownership inventory — purely additive/corrective to production source. (2) Shipment safe-close bookkeeping for `138-S`: delete `.backlogit/queue/138-S.md` and author `.backlogit/archive/138-S.md` (manual safe-close, P-015 partial-feature procedure, identical pattern to 133-S/134-S/135-S/137-S). (3) Preserve the operator-directed mid-review pause checkpoint commit (`1c3eb3e8`) by cherry-picking it onto the post-merge closure branch (`ce3b2fba`) rather than the implementation PR, per explicit operator instruction not to re-arm review. |
| `ActionRisk` | (1) `moderate` — additive/corrective production source change across generation activation, request handling, startup gating, and catalog derivation; extensively covered by 11 rounds of review and 87/87 targeted tests plus a 2457/2460 full-suite run. (2) `destructive` per the strict-safety schema's literal inclusion of "deletes" — `.backlogit/queue/138-S.md` is deleted from the working tree. This is Ship-role-permitted, non-discretionary bookkeeping (Role Boundary explicitly allows "close shipments, archive completed items"; post-merge closure Step 6 is a mandatory continuation of the same operator-approved merge action); the file's full content is preserved verbatim in `.backlogit/archive/138-S.md`; the change is fully git-reversible; it lands on a dedicated closure branch/PR requiring its own separate explicit operator approval before reaching `main`. (3) `low` — a cherry-pick of an already-committed, already-approved-context checkpoint/memory file onto a branch that is not the implementation PR; no source code is touched; fully reversible. |
| `ActionResult` | (1) `applied` — merge completed cleanly via `gh pr merge 391 --merge`; ancestry verified (`git merge-base --is-ancestor 81b19b0d... origin/main`, exit 0); approval: explicit, PR-scoped operator approval for the exact reviewed HEAD `6b39ee94`, re-verified via the full last-mile gate immediately before merge. (2) `applied` — verified via live re-read (`status: active` before, `archived_status: done` after), `142-F` verified byte-for-byte unchanged (SHA-256 `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`, 802 bytes), zero orphans, no unrestored archive deletions (`git status -- ".backlogit/archive/"`). (3) `applied` — cherry-pick committed as `ce3b2fba` on `post-merge/138-s-generation-activation-request-context`; confirmed the implementation PR branch/remote HEAD never received this commit (`6b39ee94` unchanged pre- and post-cherry-pick). |

## Owner

Ship agent (this session), on behalf of the operator who explicitly
approved PR #391 for merge ("OPERATOR MERGE APPROVAL: PR #391 is explicitly
approved for merge") for the exact reviewed HEAD.

## Validation window

Standard PR review + CI window, extended by the operator-directed
mid-review pause (checkpoint `checkpoint-20260912-020327.json`) which added
observability but no additional bake/soak requirement. The change adds a
generation activation service, request-context typing, startup gating,
request-entry ordering, a dispatch capability gate, and descriptor-derived
catalogs, with 11 rounds of Copilot review and full local test coverage
(87/87 targeted, 2457/2460 full-suite) providing direct coverage of the
change surface. The one open condition (readiness-latch defect triage) is
tracked above via stash `265F99BE` and does not gate this validation
window — it is a pre-existing condition orthogonal to this shipment's own
scope.

## Follow-up requirements (all captured to stash, none blocking this closure)

| Stash ID | Priority | Summary |
|---|---|---|
| `265F99BE` | high | **Root-caused this session**: workspace-generation readiness-latch defect — `_health` reports `starting` forever after a branch/workspace switch with no transferred successor. Root cause: `src/server/state.rs::publish_workspace_generation_transition` clears `hydration_ready`; `src/daemon/lifecycle_policy.rs::run_watcher_driver` completes its branch-refresh operation but never calls `set_hydration_ready_for_permit` when there is no transferred successor. Assessed against 138-S's P-021 C1 same-contract-surface test — 138-S owns activation/context/startup-gate/request-entry, not the watcher-refresh readiness handoff — and correctly deferred, not implemented. Awaits Stage triage/planning as its own release unit. |
| `5C873386` | medium | **Round-11 suppressed Copilot finding**: `src/services/generations/activation.rs:821` — `activate_initial` records a permanent rejection but never [truncated in original finding] — out of scope per P-021 C1 for this shipment's already-approved round-11 fix set; not implemented. |
| `3FFEE99B` | low | **Round-11 suppressed Copilot finding**: `src/errors/mod.rs:1011` — the five new `17_004`-`17_008` activation-error response branches have no dedicated `to_response`-path test coverage — cosmetic/coverage gap, out of scope per P-021 C1; not implemented. |
| `9D313653` | (cited, not re-captured) | Post-merge runtime-verification finding: `services::metrics::tests::full_channel_branch_switch_is_acknowledged_before_following_event` full-suite-only timing flake — confirmed unrelated to 138-S, passes in isolation. |
| `58B33C45` | (cited, not re-captured) | Post-merge runtime-verification finding: `hcl_indexing_test::cold_start_lists_and_maps_all_three_hcl_aliases` full-suite-only flake — confirmed unrelated to 138-S, passes in isolation; originally captured against 133-S. |
| `EC3BAF22` | low | **Post-merge closure finding**: `archive_verifier_runs_the_unpacked_native_binary` fails reproducibly with `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio` — long-documented, pre-existing, confirmed unrelated to 138-S (PR #391's hosted CI reported SUCCESS for this merge commit). Ambiguous against five prior occurrences (`58B33C45`, `4EE241DC`, `3067BC32`, `0443D844`, `7D47F30B`); a new entry was captured per the discovery-reuse protocol rather than guessing which prior entry to reuse. |
| `DA0AF326` | (cited, not re-captured) | `.autoharness/workspace-profile.yaml` runtime-validation probe commands have drifted from the actual CLI/test surface — pre-existing, unrelated to 138-S; re-confirmed again this session (`engram status` still does not exist; real subcommands are `daemon-status`, `workspace-status`, `stats`). |

All are `requires_deliberation: true` (or already so recorded) and await
Stage triage/harvest. None represent a regression introduced by, or a gap
in, 138-S's own stated scope.

## Source artifact cleanup

Checked `custom_fields.source_stash_id` and `custom_fields.source_deliberation_id`
on every item in the shipped scope (`142.018-T` and its 4 subtasks,
`142.019-T`, `142.028-T`, `142.029-T`, `142.030-T`, `142.031-T`,
`142.032-T`, `142.033-T` and its 2 subtasks — 14 items total — plus covering
feature `142-F` and the shipment record `138-S` itself) via `backlogit get
{id}`.

- Archived stash (`source_stash_id`): **none present on any shipped-scope
  item** — 0 stash entries archived.
- Archived deliberations (`source_deliberation_id`): **none present on any
  shipped-scope item** — 0 deliberation artifacts archived.
- Skipped (already archived or not found): n/a — no candidate fields were
  present to begin with.

No source-artifact retirement was performed. This is the correct, precise
outcome per the Ship Role Boundary: cleanup is scoped strictly to
manifest-derived `source_stash_id`/`source_deliberation_id` references, and
none exist for this shipment's scope. No discretionary stash edit, triage,
or archival was performed.

## Compaction status

`pending` — `compact-context --target all` to be invoked at Ship Step 8
(post-merge closure), immediately following this document's commit. This
field will be updated to `done` (or `degraded` on failure) once that
invocation completes.

## Post-merge closure record

| Field | Value |
|---|---|
| Merge SHA | `81b19b0d91c79c9c456ce703dca42a4688978dfa` |
| Merge method | merge commit (`gh pr merge 391 --merge`) |
| Merge confirmed | `MERGE_CONFIRMED` — `gh pr view 391` state `MERGED` at `2026-09-12T03:01:40Z`; `git merge-base --is-ancestor 81b19b0d... origin/main` exit 0 |
| Shipment closure | Manual safe-close (see `.backlogit/archive/138-S.md` AUDIT RATIONALE) — `backlogit shipment ship` was never attempted (established non-terminating tool defect against the shared `142-F` covering feature, per `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`); direct `move --status shipped` confirmed rejected by the CLI; manual archive-file creation used, matching the 133-S/134-S/135-S/137-S precedent for P-015-protected covering-feature scope |
| Task statuses | 142.018-T + 4 subtasks, 142.019-T, 142.028-T, 142.029-T, 142.030-T, 142.031-T, 142.032-T, 142.033-T + 2 subtasks (14 items total) — all `done`, all individually archived pre-closure (confirmed `pre-archived` in reconciliation) |
| Covering feature | `142-F` — verified `active`, byte-for-byte unchanged (SHA-256 `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`, 802 bytes; P-015 protection confirmed) |
| Reconciliation | `.backlogit/reconcile/138-S-pre-20260912-030500.md` (PROCEED), `.backlogit/reconcile/138-S-post-20260912-031100.md` (PROCEED) |
| Post-merge closure branch | `post-merge/138-s-generation-activation-request-context` |
| Post-merge closure PR | to be created via `pr-lifecycle` skill following this document's commit; title `chore: post-merge closure for 138-S — Generation activation, request context, startup gate and request entry` |
| Canonical gate-evidence file | `docs/closure/138-S-2026-09-12-post-merge-closure.md` (machine-discoverable frontmatter for the `pipeline-topology` gate's `shipment_readiness` check on later `142-F`-covering shipments) |
