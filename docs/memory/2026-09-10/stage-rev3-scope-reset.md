# Stage memory — Revision 3 strategic scope reset (cold-start readiness)

* **Date:** 2026-09-10
* **Branch:** `chore/stage-critical-engram-readiness`
* **Agent:** Stage (planning/backlog only — no source, no commit, no branch switch, no worktree)
* **Predecessor:** `docs/memory/2026-09-10/stage-rev2-plan-review-remediation.md`
* **Plan:** `docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` (now Revision 3)

> **HISTORICAL RECORD — SUPERSEDED BY THE TERMINAL REVISION 3 PLAN-REVIEW FAIL.**
> This memory documents the Revision 3 scope-reset pass, written **before** the
> Revision 3 gate ran. The Revision 3 `plan-review` gate subsequently returned
> **FAIL (terminal)**, attempt 3 of 3, circuit breaker **OPEN**. Every assertion
> in this file about manifest executability or readiness to ship is
> **superseded**. Current state: **every member of the `143-S` and `144-S`
> manifests, including `143-F` and `144-F`, is `status: blocked`; neither
> shipment is claimable.** Authoritative terminal record:
> `docs/memory/2026-09-10/stage-rev3-terminal-plan-review-fail.md` and the plan's
> `## Plan Review — Revision 3`.

## Why this pass happened

The Revision 2 independent `plan-review` gate returned **FAIL**. The finding was
structural, not detail: the content-addressed generation-reuse work was
**under-measured and over-designed**. The operator directed a strategic scope
reset rather than another remediation pass.

Distilled Revision 2 FAIL reasons, recorded verbatim in the plan's historical
`## Plan Review — Revision 2` section: readiness-observer boundary unresolved;
dependency inversions and leaf acceptance-criteria drift; task granularity still
exceeded; generation lifecycle / catalog / producer / snapshot semantics
unresolved; agent-visible state gaps; plus an impossible heartbeat
cancel-and-join-in-`Drop` contract. Next review is **attempt 3**. Stage did not
self-declare a verdict.

## What changed

### `144-S` is now measurement-only

It implements no cold-start fix. Its deliverable is a **measured report that
returns to Stage**: durable startup attempt records, complete phase attribution,
heartbeat supervision, live CLI/MCP classification, a performance baseline, and
schema/HNSW sub-phase measurement with explicit create-vs-skip outcomes.
`144-S → 138-S` was **preserved deliberately** — measurement must be taken
against the post-138 readiness architecture.

G1/G2/G3 were converted from enforced release-blocking thresholds to **measured
baseline dimensions**, because their enforcing owners are now deferred and an
unownable gate is not a gate.

### 28 Phase 2 items blocked, not destroyed

`144.003-T` … `144.008-T` and their subtasks were set `status: blocked` with a
`blocked_reason`, removed from every shipment manifest and from queue ordering,
and stripped of all dependency edges. Their bodies were replaced with concise
blocked-scope records naming the accepted-but-deferred decision and the seven
mandatory future-plan design questions. **Destructive disposition was not
approved and was not performed** — they remain traceable under `144-F`.

The `backlogit shipment return-blocked` operation was the sanctioned mechanism:
it blocks the item, records a reason, and removes it from the manifest in one
non-destructive step. There is no `shipment remove` command.

### `144-F` split into Phase 1 and Phase 2

Title and body now distinguish Phase 1 (measurement, shipped by `144-S`) from
Phase 2 (accepted content-addressed generation direction, blocked pending
evidence and re-planning). `R-LEGACY-DB-RECLAIM` was **removed** as a `144-S`
closure condition; it belongs to the future implementation plan.

### Workstream A finalized

* `143.001-T` is GREEN-only characterization plus the deterministic TTL
  scaffold.
* A2 split into three leaves: reference-counted RAII lease with a **full idle
  TTL re-armed from the final release instant**; post-request-information
  classification with cancel/error release paths; and in-place supersession of
  the `S046`/`T049` contract text.
* The watchdog observes **one stable read-only readiness-observation seam**
  shared by managed mode and future `138-S` read-server mode. It may not write
  `ReadinessView`, drive activation, or refresh the useful-work TTL. Dual disarm
  contract tests plus a **mandatory post-138 compatibility recheck** before
  `138-S` resumes and before `144-S` is claimed.
* The hard deadline split into bounded stop/flush/drain (`143.003.002-ST`) and
  exact-process forced exit with endpoint/`fd_lock` reacquisition proof and no
  await on non-cancellable `spawn_blocking` (`143.003.006-ST`).
* **Ordering inversion corrected:** taxonomy → ledger storage → ledger
  attribution → backoff consumer → status surface. Revision 2 had the backoff
  policy before the durable counter it consumes.
* All G3/`144-S` dependencies removed from Workstream A.

### Every manifest item is executable (historical assertion — now FALSE)

> **SUPERSEDED.** The paragraph below was the pre-review Revision 3 assertion. The
> terminal Revision 3 gate FAILED and set **every** manifest member of both
> shipments to `blocked`. Finding (h) additionally rejected `143-F`/`144-F` as
> executable closure milestones and finding (g) rejected the granularity of
> multiple surviving leaves.

Parent task artifacts retained in a manifest (`143.002-T`, `143.003-T`,
`143.004-T`, `144.001-T`, `144.002-T`) were converted into **claimable
integration milestones with bounded acceptance criteria**. No prose-only
container and no blocked item remains in either manifest.

## Key technical corrections made

* **Impossible `Drop` contract removed.** Heartbeat cancel-and-join now happens
  in an **outer async finalizer**; a synchronous drop guard may only *request*
  cancellation. `Drop` cannot await.
* **Ledger before backoff**, not after.
* **Exit/wire code allocation through existing authority.** `src/errors/mod.rs`
  defines `ShimFailureClass` at exit codes 10–14 and wire codes 15001–15005; the
  new watchdog variant takes the next unused value in each contiguous range.

## Tooling notes for the next pass

* The installed backlog registry does **not** advertise `features.sizing`, so
  `size`/`complexity` are recorded as prose in each item body. Recorded
  degradation, not an omission.
* `backlogit shipment add` takes **positional** arguments and is not safe to run
  in a tight loop against the same shipment — successive invocations read a
  stale index and last-writer-wins. Edit the manifest YAML directly and then
  `backlogit sync`.
* Queue artifact files are the source of truth; direct editing plus `sync` is
  far faster than CLI round-trips, but files must be written **UTF-8 without
  BOM**.

## Carry-forward obligations

* **CP-1 conflict is unresolved and must be resolved by the operator before any
  claim.** The `138-S` backlog record reads `queued`; commit `60f0ae4d` asserts
  a claim; the most recent checkpoint record is archived and quarantined. Stage
  did not resolve it and did not modify `138-S`.
* **OD-1 blocks every `144-S` figure.** Incident daemon PID `30528` is still
  running without stop approval.
* `002-SP` remains `done`. No new feature or shipment was created.
* Exclusions honoured throughout: no change to `138-S`, any `142.*`,
  `.backlogit/stash.jsonl`, the untracked 137-S carry-forward memory, or any
  checkpoint audit file.

## Next step

A **fresh, independent Revision 3 `plan-review`** (attempt 3). Stage has not
declared and may not declare a verdict.
