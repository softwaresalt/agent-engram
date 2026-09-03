---
title: "133-S post-merge operational closure"
doc_type: closure
shipment_id: "133-S"
feature_id: "142-F"
mode: post-merge
date: 2026-09-03
author: ship
verdict: "BLOCKED — shipment record archival pending Stage manifest correction"
closure_status: "BLOCKED"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
pr_number: 376
merge_commit: "33a0a41e345cef8965b707346728d44fa5492daf"
head_commit_merged: "2005b3db94752dbe37946a98532c46dde1aad674"
runtime_verification_report: "docs/closure/133-S-2026-09-03-runtime-verification.md"
follow_up_stash:
  - "A7C0BA5F"
  - "5A7FBC37"
  - "58B33C45"
  - "7B270F79"
  - "F2E84E15"
  - "F9D1C495"
blocking_stash: "F9D1C495"
---

# 133-S post-merge operational closure

## Summary

`133-S` (feature `142-F`) delivered read-server foundations: F00 (49
placeholder test-manifest registrations), F01 (storage feasibility spike,
GO verdict, accepted Windows durability residual risk), F02 (strict
`DaemonMode` mode-contract parser), F03 (immutable `mode` field on
`AppState`), and F12a (`crates/engram-indexer` empty stub crate + workspace
membership). No user-facing runtime behavior changed with this shipment.

PR #376 merged as a merge commit (repo policy: merge-only, squash/rebase
disabled) with explicit operator approval recorded in-session ("Keep
working autonomously until the task is truly finished" — treated as a
one-time approval scoped to PR #376 only, per the operator's own
instruction, not a blanket future-PR authorization).

**This closure is BLOCKED at the shipment-record-archival step.** All task-
level manifest items are done and archived; the shipment record itself
(`133-S`) cannot currently be safely transitioned to `shipped`/archived by
any available backlogit 1.10.1 CLI path without violating this workspace's
own P-015 policy. See "Blocking Finding" below for full evidence. This is a
genuine tooling/policy conflict, not a Ship execution error, and is
recorded as high-priority follow-up stash `F9D1C495` for Stage resolution.

## Merge / PR Evidence

| Gate | Result |
|---|---|
| PR #376 state | `MERGED` at `2026-09-03T17:54:11Z` |
| Merge commit | `33a0a41e345cef8965b707346728d44fa5492daf` |
| Reviewed/merged HEAD | `2005b3db94752dbe37946a98532c46dde1aad674` (unchanged from the operator's specified HEAD through merge) |
| Ancestry | `git fetch origin main` then `git merge-base --is-ancestor 33a0a41e... origin/main` → exit `0` |
| Merge strategy | `--merge` (merge commit); squash/rebase disabled repo-wide (`allow_merge_commit: true`, `allow_squash_merge: false`, `allow_rebase_merge: false`) |
| Local review readiness at HEAD | `READY_WITH_FOLLOWUPS`, P0=0, P1=0 |
| P-018 Copilot review gate | `autoharness gate copilot-review 376 --enforcement auto --max-wait 0 --json` → `SATISFIED` |
| Unresolved review threads at merge | 0 of 5 (`isResolved: true` on all, verified via GraphQL) |
| Last Copilot review vs HEAD | `commit_id` == HEAD; state `COMMENTED` (not `CHANGES_REQUESTED`); 0 new comments, only previously-suppressed items cited |
| `mergeStateStatus` / `mergeable` (pre-merge) | `CLEAN` / `MERGEABLE` |
| CI checks | `build`: SUCCESS, `start-launcher-windows`: SUCCESS |
| Pipeline-topology lifecycle gate (pre-merge, last-mile re-check) | passed both times |

## Runtime Verification

Verdict: **PASS WITH FOLLOW-UP** (see
[`133-S-2026-09-03-runtime-verification.md`](./133-S-2026-09-03-runtime-verification.md)).
Release build succeeds; MCP tool catalog and existing MCP/CLI contract
suites unaffected; one contract-suite failure
(`shim_aborts_unresolved_startup_after_client_disconnects`) confirmed
pre-existing and unrelated via isolated-worktree reproduction against the
pre-merge `main` tip. No new runtime behavior is introduced by this
shipment (F04's call-site migration and F06–F09/F12's real logic are
explicitly deferred to later shipments).

## Reconciliation

* **Pre-archive reconciliation**: manual verification (this backlogit
  version's `shipment-reconcile pre` mode assumptions about a direct
  `move --status shipped` path are now stale — see Blocking Finding) showed
  all 10 task-level manifest items (`142.001-T` + 5 subtasks, `142.002-T`,
  `142.004-T`, `142.006-T`, `142.007-T`) present in `.backlogit/archive/`
  with `status: done`. Covering feature `142-F` correctly remains in
  `.backlogit/queue/` at `status: active` (59 total subtasks, only 10 in
  this shipment's manifest — not fully covered, correctly untouched by the
  merged build work).
* Orphan scan (grep for `shipment_id: 133-S` across queue/archive):
  no matches — expected, this backlogit version does not store
  shipment back-references on task records.
* No `source_stash_id` or `source_deliberation_id` custom fields exist on
  `133-S` or `142-F` — no source-artifact cleanup required.

## Blocking Finding: Shipment Record Cannot Be Safely Archived

**Evidence chain** (full detail; Ship independently reproduced every claim
below against the live `.backlogit/` state and the installed `backlogit`
binary in this session):

1. `backlogit move 133-S --status shipped` → **fails** (exit 9):
   `"shipment must be shipped via ShipShipment, not a direct status
   update"`. This error is produced by an unconditional guard
   (`ErrShipmentShippedRequiresEnvelope`, introduced by backlogit feature
   `144-F`, described in that repo's own code comments as intentionally
   unbypassable — "no legitimate caller... even an operator `--force`").
   `backlogit shipment ship --help` confirms no scope-limiting flag exists
   (`--author`, `--message`, `--sha` only).
2. The only remaining CLI path to `status: shipped` is
   `backlogit shipment ship 133-S`, which performs a **cascade**: for every
   ancestor feature reachable from the shipment's manifest items
   (`featureScopeRoots`), every non-manifest, non-terminal descendant is
   force-set to `status: queued` with `parent_id` cleared (detached), and
   if the feature itself is an explicit manifest member, it is
   **unconditionally** marked `done` regardless of actual completion.
3. `133-S`'s manifest lists `142-F` (the covering feature) as an explicit
   item, alongside only 10 of its 59 total subtasks. Invoking
   `backlogit shipment ship 133-S` would therefore force-mark `142-F`
   `done` (58 of 59 subtasks still incomplete) and detach roughly 49
   sibling subtasks from their parent.
4. This workspace's own P-015 policy permits the cascade close path
   **only** when every feature member of a shipment manifest is a root and
   fully covered (100% of its live children are manifest members) —
   `142-F` fails this decisively. **Invoking the cascade here would be a
   P-015 policy violation**, not merely an inconvenient side effect.
5. backlogit's own design-decision record
   (`docs/decisions/2026-07-31-shipshipment-partial-feature-archive-cascade-deliberation.md`
   in the `backlogit` repository) confirms this is the *intended* design:
   "the covering feature closes/archives iff it is an explicit manifest
   member" — meaning a multi-shipment covering feature is expected to be
   **omitted** from a partial shipment's `items` in the first place
   (relying on `parent_id` resolution, the same task-only-manifest pattern
   already used elsewhere in this workspace, e.g. `097-S`).

**Conclusion**: `133-S`'s manifest was assembled with `142-F` present as an
explicit item despite the shipment covering only a fraction of `142-F`'s
scope. This is a manifest-assembly correctness issue, not a Ship-side
execution defect, and correcting shipment planning fields (`custom_fields.items`)
is outside Ship's role boundary (Stage-only). **Recorded as follow-up stash
`F9D1C495`** (priority: high) with the recommended remediation: Stage
removes `142-F` from `133-S`'s `custom_fields.items`, after which
`backlogit shipment ship 133-S` becomes safe to invoke (or a corrected
safe-close path becomes available).

**Independent safety net**: the `pipeline-topology` gate's predecessor
check (`_is_shipped_terminal`) already fails closed on `133-S`'s current
`active` status ahead of even reading this closure artifact's
`closure_status` field — so `134-S` cannot be claimed regardless of this
document's content. `134-S` must not be claimed until `133-S` reaches a
genuinely shipped/archived terminal state.

## Invariants to Preserve

* The strict `DaemonMode` parser must continue to hard-error on any value
  outside `managed`/`strict` — no silent fallback.
* Existing `AppState` construction call sites (`new`,
  `with_stale_strategy`, `with_options`) must continue to forward to
  `with_mode(DaemonMode::Managed, ...)` unchanged until F04 migrates them.
* The new `engram-indexer` crate must remain inert (no wired daemon
  participation) until its real supervisor logic ships under a later,
  reviewed shipment (F12).

## Pre-Deploy Audits

* No schema, migration, or config-flag changes ship with observable
  runtime effect — the new `DaemonMode`/`mode` field is additive and
  defaults to preserving current (`Managed`) behavior at every existing
  call site.
* No new external dependency, port, or credential surface introduced.
* Scope confirmed via merge-diff to be limited to the F00/F01/F02/F03/F12a
  files described in `docs/ARCHITECTURE.md`'s updated Module boundaries
  section, plus test/backlog/documentation artifacts.

## Deployment / Rollout Path

Merge-only. `engram` is distributed as a per-workspace binary/plugin; there
is no separate deploy or canary step beyond the merge landing on
`origin/main` and downstream consumers picking up the next build. No
maintenance window required.

## Post-Deploy Checks

* MCP tool catalog (`engram manifest`) continues to return the full,
  well-formed catalog on `main` (already verified this session at the
  merge commit).
* `contract_mcp_catalog_oracle` and related F00-placeholder contract tests
  continue to pass on CI.

## Healthy Signals

* Release build continues to succeed on `main`.
* No new failures introduced in the existing MCP/CLI/shim contract test
  suites beyond the confirmed pre-existing, unrelated
  `shim_aborts_unresolved_startup_after_client_disconnects` failure.

## Failure Signals (Rollback Trigger)

* A CI regression in `contract_mcp_catalog_oracle`,
  `contract_mcp_tool_catalog_parity`, `contract_mcp_envelope`, or
  `contract_read_server_cli_mcp_parity` attributable to this shipment's
  changes.
* Any report of an existing `AppState` construction call site behaving
  differently post-merge (would indicate the `with_mode` forwarding
  default is broken).

## Rollback Procedure

Revert merge commit `33a0a41e345cef8965b707346728d44fa5492daf` on `main`
via a standard `git revert -m 1` PR (merge-commit revert, preserving
history), gated through the same PR review/CI/approval pipeline as any
other change. No data migration or external state requires separate
rollback.

## Validation Window & Monitoring Plan

No external metrics/APM backend exists for this CLI/daemon tool.
Monitoring is CI/build-based:

| SLI | Source | Baseline | Threshold (escalate) | Owner |
|---|---|---|---|---|
| Existing contract suite stability | CI `build` check on every `main` push | 100% pass except the known pre-existing `shim_aborts_unresolved_startup_after_client_disconnects` gap | Any *new* contract-suite failure attributable to this shipment's files | Repository maintainer (`softwaresalt`) |
| Windows generation-publish durability (residual risk, stash `F2E84E15`) | Manual re-review by F07/F08 implementers before treating Windows publication as crash-durable equivalent to POSIX | N/A (accepted, unverified residual risk) | Any field/support report of a torn or missing generation directory on Windows after a crash during publish | Repository maintainer (`softwaresalt`), F07/F08 implementers |

Observation window: through the next shipment that touches
`src/db/cozo_backend/` or the generation-publish path (expected in a later
F06–F09 shipment).

## Owner

Ship agent / repository maintainer (`softwaresalt`) for monitoring; Stage
owns the manifest-correction remediation for the blocking finding above.

## Compaction Status (P-020)

`compact-context --target all` was invoked this session (mandatory,
unconditional on merge). Result: **done**. The just-closed release unit's
own session memory (three files:
`2026-09-03-ship-pr-372-stage-133-s-merge-closure.md`,
`2026-09-03-ship-133-s-mid-session-checkpoint.md`,
`2026-09-03-ship-133-s-pr-ready-checkpoint.md`) was consolidated into
`docs/memory/compacted/2026-09-03-133-s-read-server-foundations-compacted.md`
and the verbose originals moved to `docs/archive/memory/`. `docs/exec-plans/`
and `docs/closure/` were scanned for 133-S/142-F-specific candidates: the
one related exec-plan
(`2026-09-02-separate-indexer-read-server-plan.md`) governs feature `142-F`
as a whole, which remains open across multiple future shipments, so it does
not meet the "feature/chore complete" compaction precondition and was
correctly left uncompacted (scan-only, no-op for that artifact).

## Verdict

**BLOCKED (shipment-record archival only)**. The code was merged, verified,
and is production-ready; runtime verification is PASS WITH FOLLOW-UP; all
task-level manifest items are done and archived. The shipment record
`133-S` itself remains `active` (not `shipped`/archived) because backlogit
1.10.1 provides no CLI path to close it without either (a) an unconditional,
unbypassable guard rejecting direct status update, or (b) a cascade that
would violate this workspace's own P-015 policy against `142-F`'s partial
coverage. This is recorded as high-priority follow-up stash `F9D1C495` for
Stage to resolve (recommended: remove `142-F` from `133-S`'s
`custom_fields.items`). **`134-S` must not be claimed until `133-S` reaches
a genuinely shipped/archived terminal state** — independently enforced by
the `pipeline-topology` gate's predecessor check regardless of this
document's content.
