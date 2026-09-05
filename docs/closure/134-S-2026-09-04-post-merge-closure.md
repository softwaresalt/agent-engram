---
title: "134-S post-merge operational closure"
doc_type: closure
shipment_id: "134-S"
feature_id: "142-F"
mode: post-merge
date: 2026-09-04
author: ship
verdict: "CLOSED — evidence, non-destructive closure, and the manual shipment safe-close are all complete. The release-build regression (stash 6C9AA7D3) was fixed by PR #381 (merge c9cf8adb0eb03702a27866c35f9a4d97cc49ab91) and confirmed via cargo build --release passing; the manual shipment-record safe-close (commit attribution + done transition + archive) was performed on 2026-09-04 under explicit operator authorization on branch post-merge/134-s-manual-shipment-archival, superseding the prior session's withheld/BLOCKED state"
closure_status: "READY"
releasability: "READY"
compaction_status: "done"
pr_number: 379
closure_pr_number: 380
manual_closure_pr_number: "pending — assigned on PR creation for post-merge/134-s-manual-shipment-archival; see Manual Closure Performed This Session section for commit-level evidence"
merge_commit: "760b44752a0f00704bd1a6f88fb78f91bd4e997d"
closure_pr_merge_commit: null
head_commit_merged: "7562c29152b6f53a7551b330a1de1adaebf97084"
runtime_verification_report: "docs/closure/134-S-2026-09-04-runtime-verification.md"
follow_up_stash:
  - "6C9AA7D3"
  - "4EE241DC"
  - "E12542FF"
  - "1918AFD2"
  - "F95653D1"
  - "AA5698E3"
  - "C1EFF21F"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) — manual safe-close performed 2026-09-04 on post-merge/134-s-manual-shipment-archival"
---

# 134-S post-merge operational closure

## Summary

`134-S` (feature `142-F`) delivered IPC seam extraction (`142.008-T` + 4
subtasks: `startup_activation.rs`, `request_entry.rs`, `error_transport.rs`,
`lifecycle_policy.rs`), `AppState` mode constructor migration (`142.009-T`),
shim restart mode propagation (`142.010-T`), the stable error envelope
(`142.003-T`), and descriptor schema / tool descriptor registry work
(`142.005-T` + 3 subtasks: `capabilities.rs`, error code ranges,
tool descriptor registry updates).

PR #379 merged as a merge commit (repo policy: merge-only, squash/rebase
disabled) with explicit operator approval recorded in-session for **PR
#379 only** ("Keep working autonomously until the task is truly finished"
— per the operator's own framing, this authorizes the PR #379 merge
decision and non-destructive post-merge work; it does **not** authorize
manual shipment-record archival, which the operator explicitly withheld
for a separate approval).

**This closure was originally intentionally incomplete for two independent
reasons, neither of which involved any Ship action outside its authorized
scope. Both reasons are now resolved (retained as history):**

1. **Procedural — RESOLVED (2026-09-04, separate operator authorization)**:
   the shipment-record archival sequence (commit attribution on 12
   manifest items + status transition + archive) was withheld pending
   separate, explicit operator approval when this closure record was
   first drafted (P-010/Role Boundary: destructive-action approval
   required). The operator subsequently gave that separate, explicit
   authorization ("Perform closure operations as needed to return to a
   clean state on the main branch"), and the manual safe-close was
   performed on branch `post-merge/134-s-manual-shipment-archival` — see
   "Manual Closure Performed This Session" below for the full command
   log and post-mutation verification.
2. **Substantive — RESOLVED**: `cargo build --release` failed to compile
   on the merged `main` tip (`760b4475`) due to an unused-import lint
   (`Duration` in `src/daemon/startup_activation.rs`, only referenced
   inside a `#[cfg(debug_assertions)]` block — see runtime-verification
   report for full root cause). This was a genuine regression introduced
   by `134-S` itself (the file is new to this shipment), not a
   pre-existing issue, and was not fixed on the original evidence-only
   closure branch/PR (fixing source on `main` was out of scope for that
   non-destructive session). It was subsequently fixed by PR #381 (merge
   `c9cf8adb0eb03702a27866c35f9a4d97cc49ab91`), and this closure branch
   was updated to merge `origin/main` (which includes that fix) —
   `cargo build --release` now passes when re-run directly on this
   branch. Captured historically as stash `6C9AA7D3` (already resolved by
   PR #381; no longer a live blocker).


## Merge / PR Evidence

| Gate | Result |
|---|---|
| PR #379 state | `MERGED` at `2026-09-04T18:23:40Z` |
| Merge commit | `760b44752a0f00704bd1a6f88fb78f91bd4e997d` |
| Reviewed/merged HEAD | `7562c29152b6f53a7551b330a1de1adaebf97084` (exact HEAD the operator/session specified, unchanged through merge) |
| Ancestry | `git fetch origin main` then `git merge-base --is-ancestor 760b4475... origin/main` → exit `0` |
| Merge strategy | `--merge` (merge commit); squash/rebase disabled repo-wide (`allow_merge_commit: true`, `allow_squash_merge: false`, `allow_rebase_merge: false`) |
| Local review readiness at HEAD | `READY`, P0=0, P1=0 (PR body "Local Review Readiness" block) |
| P-018 Copilot review gate | `autoharness gate copilot-review 379 --enforcement auto --max-wait 900 --json` → `SATISFIED`, 0 unresolved threads |
| Unresolved review threads at merge | 0 of 15 (`isResolved: true` on all, independently verified via GraphQL) |
| `mergeStateStatus` / `mergeable` (pre-merge) | `CLEAN` / `MERGEABLE` |
| CI checks | `build`: SUCCESS, `start-launcher-windows`: SUCCESS |
| Pipeline-topology lifecycle gate (pre-merge, pre-closure re-checks) | passed all invocations |

## Runtime Verification

Verdict: **FAIL** (full report:
[`134-S-2026-09-04-runtime-verification.md`](./134-S-2026-09-04-runtime-verification.md)) —
`cargo build --release` is an explicit mandatory validator target and it
fails to compile; per the runtime-verification contract this classifies as
`FAIL`, not `PASS WITH FOLLOW-UP` (which is reserved for a usable surface
needing cleanup/monitoring, not one that fails a mandatory validator
target).

### Validator Evidence (structured)

| Field | Value |
|---|---|
| Surface / adapter | API/IPC (daemon composition root, admission, error transport, descriptor registry); CLI (binary version/manifest, mode resolution) |
| Verdict | `FAIL` |
| Healthy signal | `cargo check --all-targets` (1m14s) and `cargo build` (dev, 1m09s) succeed; `engram.exe --version`/`manifest` ok; 39/39 targeted contract/unit/integration tests pass (seam extraction, tool descriptor registry, error-code contract, `AppState` constructor migration, read-server mode/restart) |
| Failure signal | `cargo build --release` fails to compile (`-D unused-imports`, `src/daemon/startup_activation.rs:11`) — release-artifact-only, does not affect dev/test behavior |
| Manual checkpoint evidence | Targeted test run listed above executed directly against the merged `main` tip on the `post-merge/*` closure branch (no worktree needed — same tree) |
| Blocked prerequisites | Bound-daemon CLI probes (`engram status`/`health`/`sync`) not separately exercised; superseded by the more specific `integration_read_server_restart` suite already covering daemon startup/mode/restart behavior for this shipment's scope |

## Operational Closure Checklist

* **Invariants to preserve**: IPC seam boundaries (`request_entry.rs`,
  `error_transport.rs`, `lifecycle_policy.rs`, `startup_activation.rs`)
  must continue enforcing the same admission/error-transport contracts as
  before extraction; the stable error envelope (`142.003-T`) must remain
  wire-compatible for existing clients; `AppState`'s constructor migration
  (`142.009-T`) must not change externally observable daemon startup
  behavior; the tool descriptor registry (`142.005-T`) must continue
  serving the same descriptor schema/error-code ranges consumed
  downstream.
* **Pre-deploy audits**: none required — no config schema, migration, or
  access-control surface changed. The only pre-deploy-relevant item was
  the release-build regression (resolved; see Runtime Verification).
* **Deployment / rollout path**: merge-only to `main` (repo policy:
  squash/rebase disabled, merge commit only). The actual `134-S` code
  change already completed this path via PR #379's merge (`760b4475`);
  this PR (#380) is evidence-only and carries no further rollout of its
  own. `engram` is a locally-run daemon/CLI binary distributed via release
  artifact (no server-side canary/phased-rollout mechanism); the next
  release-artifact build (`release.yml`, `cargo build --release`) is the
  next rollout checkpoint and is confirmed unblocked (see Runtime
  Verification).
* **Post-deploy checks**: `cargo check --all-targets` and `cargo build`
  (dev) green; `engram.exe --version`/`manifest` smoke checks pass; 39/39
  targeted contract/unit/integration tests pass (seam extraction, tool
  descriptor registry, error-code contract, `AppState` constructor
  migration, read-server mode/restart); `cargo build --release` now passes
  (confirmed directly on this branch after merging PR #381's fix).
* **Risky action record**: `ProposedAction`: merge PR #379 to `main`
  (daemon startup/IPC composition-root change). `ActionRisk`: medium
  (affects daemon startup and inter-process error transport, but is
  additive/refactor-only — no removed capability). Approval path:
  explicit operator approval recorded in-session, scoped to PR #379's
  merge. `ActionResult`: success — merge completed cleanly, ancestry
  verified (`merge-base --is-ancestor`), CI green pre-merge. A second,
  smaller risky action occurred during this closure PR's own remediation:
  merging `origin/main` into the closure branch to absorb PR #381's fix;
  `ActionRisk`: low (docs/backlog-only branch, one resolved JSONL
  append-conflict, source-tree diff against `main` remains empty);
  `ActionResult`: success, verified via `cargo build --release` re-run.
* **Healthy signals**: see Validator Evidence table above — dev
  build/check, smoke CLI checks, and full targeted suite all green, and
  the release-profile build now also green.
* **Failure signals**: a future `cargo build --release` failure on `main`,
  a daemon startup/health-probe regression, or an IPC error-transport
  contract break for existing clients would each indicate rollback or
  hotfix intervention is needed (the release-build case already
  materialized once for `134-S` and was hotfixed via PR #381 — this is
  the concrete, exercised failure-signal-to-response path for this
  shipment).
* **Monitoring plan**: no live dashboards/alerts apply to this
  locally-run developer tool; the operative monitoring signal is the next
  scheduled/triggered `release.yml` build (which exercises
  `cargo build --release` directly) and the existing targeted test suite
  run on subsequent PRs touching the same daemon/IPC surfaces.
* **Rollback trigger**: a `release.yml` build failure, or a reported
  daemon startup/IPC regression traced to one of the four extracted seam
  files or the `AppState` constructor migration.
* **Rollback procedure**: fix-forward is preferred over reverting
  `760b4475` (as already demonstrated by PR #381's hotfix), since a hard
  revert would also remove subsequently-layered work; if fix-forward is
  not viable, revert the merge commit on `main` and re-open `142-F`'s
  affected tasks.
* **Validation window**: through the next `release.yml` run and the next
  shipment's own build/test cycle touching daemon/IPC code (no fixed
  calendar duration — this is a locally-triggered CI/release tool, not a
  continuously-monitored production service).
* **Owner**: the Ship agent / operator executing the next release cut or
  the next shipment touching daemon/IPC surfaces.

### Releasability Evidence (structured)

| Requirement | Status |
|---|---|
| Dev build/check | Satisfied — `cargo check --all-targets`, `cargo build` green |
| Targeted test suite | Satisfied — 39/39 green |
| Release-artifact build | Satisfied — `cargo build --release` green (post PR #381 merge-in) |
| CI required checks (PR #379) | Satisfied — `build`, `start-launcher-windows` both SUCCESS |
| Local review readiness (PR #379) | Satisfied — `READY`, P0=0, P1=0 |
| P-018 Copilot review gate (PR #379) | Satisfied — `SATISFIED`, 0 unresolved threads at merge |
| Rollback path defined | Satisfied — fix-forward primary, hard-revert fallback documented above |
| Monitoring plan defined | Satisfied — see Monitoring plan above (release-build + subsequent-PR test signal) |
| Shipment-record archival (manual safe-close) | Satisfied — performed 2026-09-04 on branch `post-merge/134-s-manual-shipment-archival`; `134-S` is `status: archived`, `archived_status: done` |

**Overall**: `READY`. Both previously-open items are now resolved: the
release-artifact build passes (PR #381), and the shipment-record manual
safe-close is complete (`134-S` archived with `archived_status: done`,
commit attribution on all 12 manifest items, audit rationale recorded).
`closure_status` is updated to `READY` accordingly.

## Reconciliation (post-mutation, this session)

* **Pre-mutation check (before any write)**: all 12 manifest items
  (`142.008-T` + 4 subtasks, `142.009-T`, `142.010-T`, `142.003-T`,
  `142.005-T` + 3 subtasks) were confirmed `status: done` and already
  physically present in `.backlogit/archive/` (classified `pre-archived`
  per the `shipment-reconcile` schema). `134-S` itself was confirmed
  `status: active` in `.backlogit/queue/`. `142-F` was confirmed
  `status: active`, not a manifest member of `134-S` (task-only manifest,
  consistent with the `097-S` precedent). Closure PR #380's merge
  (`c50abc2d...`) and PR #381's merge (`c9cf8adb...`) were both confirmed
  ancestors of `origin/main` via `git merge-base --is-ancestor`.
* **Mutation applied**: all 12 items received a `commit:
  760b44752a0f00704bd1a6f88fb78f91bd4e997d` frontmatter field via
  `backlogit update --commit` (the official update seam); none of their
  `status`, `parent_id`, or `id` fields changed. `134-S` received an
  appended `description` rationale section, then transitioned
  `active → done` (live-verified) then archived (live-verified
  `archived_status: done`).
  Their pre-existing lack of canonical `archived_status`/`archived_from`
  stamping (mirrors `133-S`'s stash `B761AFA7`) was **not** normalized —
  that remains out of scope per stash `C1EFF21F` and the operator's
  explicit instruction not to normalize archive-convention debt beyond
  what this safe-close required.
* **Post-mutation verification**: `backlogit sync` re-indexed 1305
  artifacts (unchanged from the pre-mutation count — no artifacts lost or
  orphaned by the mutation). `142-F` remains `status: active`, untouched.
  All 59 direct children of `142-F` retain `parent_id: 142-F` (49 queue +
  10 archive). Zero orphans found across all 87 `142.*` items (every
  non-`142-F` item resolves its `parent_id` to `142-F` or to another valid
  `142.*` task). All 65 items belonging to future shipments `135-S`
  through `142-S` remain `status: queued`; those 8 shipment manifests are
  unchanged (item lists/counts verified identical to the pre-mutation
  baseline: 4, 9, 6, 14, 6, 7, 7, 12 — total 65). `133-S` (already
  archived) was not touched.
* **Orphan scan (repeat)**: grep across `.backlogit/queue/*.md` for the
  literal token `134-S` still finds only `138-S.md` and `141-S.md` (both
  reference `134-S` as a dependency in their own frontmatter, unaffected
  by `134-S`'s archival) — no orphans introduced by this session's
  archival.
* **P-015 cascade classification (unchanged)**: `134-S`'s manifest
  contained only task/subtask items — no feature member (`142-F` absent
  from `items`). Per the P-015 verified-fully-covered-root exception, the
  cascade close path (`backlogit shipment ship`) was correctly never
  eligible for this shipment; safe-close was the only valid path, and
  safe-close is exactly what was performed.
* No `source_stash_id` or `source_deliberation_id` custom fields exist on
  `134-S` — no source-artifact cleanup performed or required.

## Source artifact cleanup

- Archived stash (`source_stash_id`): none — `134-S` carries no
  `source_stash_id` custom field.
- Archived deliberations (`source_deliberation_id`): none — `134-S` carries
  no `source_deliberation_id` custom field.
- Skipped (already archived or not found): none — no candidate fields
  existed to act on.

## Manual Closure Performed This Session (2026-09-04, operator-authorized)

The operator explicitly authorized this session to "Perform closure
operations as needed to return to a clean state on the main branch,"
which was treated as approval for the already-documented high/destructive
targeted manual safe-close of `134-S` specifically (not as blanket
authorization for `backlogit shipment ship`, manifest edits, claiming
`135-S`, or merging a new PR without its own separate P-014 approval).
Work was performed on dedicated branch
`post-merge/134-s-manual-shipment-archival`, created from a freshly
pulled `origin/main` (which already contains PR #380's merge `c50abc2d...`
and PR #381's merge `c9cf8adb...`). All 15 previously-specified mutation
commands were executed via official `backlogit` CLI seams only — no
direct file edits, no `backlogit shipment ship`:

1. `backlogit update 142.008-T --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
2. `backlogit update 142.008.001-ST --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
3. `backlogit update 142.008.002-ST --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
4. `backlogit update 142.008.003-ST --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
5. `backlogit update 142.008.004-ST --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
6. `backlogit update 142.009-T --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
7. `backlogit update 142.010-T --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
8. `backlogit update 142.003-T --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
9. `backlogit update 142.005-T --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
10. `backlogit update 142.005.001-ST --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
11. `backlogit update 142.005.002-ST --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
12. `backlogit update 142.005.003-ST --commit 760b44752a0f00704bd1a6f88fb78f91bd4e997d` — done
13. `backlogit update 134-S --section description=<audit rationale>` — appended rationale citing PR #379/#380/#381 provenance and the P-015 shared-parent cascade hazard (142-F covers 8 additional queued shipments, `135-S`–`142-S`, 65 remaining items, plus already-archived `133-S`), explaining why manual safe-close (not `backlogit shipment ship`) was required.
14. `backlogit update 134-S --status done` — live-verified `status: "done"` via `backlogit get 134-S --format json` before archival.
15. `backlogit archive 134-S` — live-verified `archived_status: "done"`, `status: "archived"`, no longer present in `.backlogit/queue/` (relocated to `.backlogit/archive/134-S.md`).

**Every mutation was scoped to exactly the 12 manifest items plus the
`134-S` shipment record itself.** Post-mutation verification (`backlogit
sync` re-indexed 1305 artifacts, unchanged from pre-mutation count):
`142-F` remains `status: active` in `.backlogit/queue/`, untouched; all 59
direct children of `142-F` retain `parent_id: 142-F` (49 in queue + 10 in
archive); zero orphans across all 87 `142.*` items; all 65 items belonging
to future shipments `135-S`–`142-S` remain `status: queued`, and those 8
shipment manifests are byte-for-byte unchanged (item lists and counts
verified identical to pre-mutation baseline: 4, 9, 6, 14, 6, 7, 7, 12).
`backlogit shipment ship` was never invoked. `142-F` was never archived.

## Preserved Scope (142-F descendants)

Across both this closure record's originating session (PR #380,
evidence-only) and this session's manual safe-close mutation (2026-09-04,
`post-merge/134-s-manual-shipment-archival`), no mutation ever touched
`142-F`, any of its direct/nested descendants outside `134-S`'s own
12-item manifest, or any other covering shipment's manifest (`135-S`
through `142-S`). The originating session's reconciliation was read-only
(`backlogit get`, filesystem enumeration, grep) plus two non-destructive
stash entries (`6C9AA7D3`, `C1EFF21F`). This session's mutations were the
15 commands listed under "Manual Closure Performed This Session" above,
scoped exactly to the 12 manifest items plus the `134-S` shipment record
itself — verified by the post-mutation invariant checks in the
Reconciliation section above (142-F active/untouched, 59 children retain
parent_id, zero orphans, all 65 future items unchanged).

## Follow-ups Stashed This Session

| Stash ID | Kind | Priority | Summary |
|---|---|---|---|
| `6C9AA7D3` | bug | high | `cargo build --release` fails — unused `Duration` import in `src/daemon/startup_activation.rs`, release-profile-only regression introduced by `134-S`; blocks release-artifact packaging until fixed |
| `C1EFF21F` | bug | medium | 134-S manifest's 12 already-archived task/subtask items lack canonical `archived_status`/`archived_from` metadata (mirrors `133-S`'s `B761AFA7`); captured during PR #380 review remediation |

Pre-existing stash entries carried forward from the PR #379 readiness block
(not created this session, listed for closure traceability only):
`4EE241DC`, `E12542FF`, `1918AFD2`, `F95653D1`, `AA5698E3`.

## Compaction

`compact-context` (`target: all`) was invoked as a mandatory (P-020) step
of this closure sequence. Outcome: **done**.

* Candidates identified: 3 files (all `134-S`/PR #379 historical session
  reports — `ready-for-operator-merge-decision`,
  `final-readiness-9th-finding-resolved`, `final-report` — each
  self-described as a stale/historical point-in-time snapshot, eligible
  under the completed-work rule since 134-S's manifest is 12/12 `done`).
* Compacted into: `docs/memory/compacted/2026-09-04-134-S-compacted.md`.
* Verbose originals moved to `docs/archive/memory/` (not deleted).
* Space recovered: ~14.3 KB (20,594 → 5,992 bytes for the consolidated
  record).
* Active checkpoint preserved: this session's own live memory file,
  `docs/memory/2026-09-04-ship-134-s-pr-379-merge-and-closure.md`, was
  intentionally **not** compacted (most-recent-checkpoint preservation
  rule) — it remains the current, non-stale session record.
* No `docs/exec-plans/` or other `docs/closure/` artifacts were compacted
  this pass (the governing plan document is shared across `134-S`..`142-S`
  and is not yet fully consumed; other closure records belong to different,
  already-closed shipments and were left untouched).

## Remaining Blockers (for operator visibility)

1. **Release-build regression — RESOLVED**: `cargo build --release` (and
   `release.yml`'s actual release build step) failed on `main` at
   `760b4475` (the exact merged HEAD for this shipment) due to an
   unused-import lint. This was fixed by PR #381 (merge
   `c9cf8adb0eb03702a27866c35f9a4d97cc49ab91`, gating the `Duration`
   import behind `#[cfg(debug_assertions)]` in
   `src/daemon/startup_activation.rs`). Confirmed passing after the fix.
   Tracked historically as stash `6C9AA7D3` (resolved).
2. **Shipment-record closure — RESOLVED**: the 15-step manual mutation
   set was executed on 2026-09-04 under explicit operator authorization
   ("Perform closure operations as needed to return to a clean state on
   the main branch") on branch `post-merge/134-s-manual-shipment-archival`.
   `134-S` is now `status: archived`, `archived_status: done`.
3. **P-001 release-closure gate — RESOLVED**: with item 2 complete,
   `134-S` is no longer treated as an active release unit for P-001
   purposes. The pipeline-topology `pre_claim` gate for `135-S` was
   re-checked after this closure (see Post-Closure Gate Check below) and
   advances past the `134-S` predecessor-closure requirement. `135-S`
   itself was **not** claimed this session — claiming remains a separate,
   future action outside this session's authorization.
4. **Non-blocking follow-ups (unchanged, not triaged this session)**: the
   pre-existing stash entries `4EE241DC`, `E12542FF`, `1918AFD2`,
   `F95653D1`, `AA5698E3`, and `C1EFF21F` (archive-convention debt on the
   12 pre-archived items — see Reconciliation) remain open for Stage's
   disposition. This session did not triage, re-prioritize, or normalize
   any of them, per explicit operator scope.

## Post-Closure Gate Check (this session)

`autoharness gate pipeline-topology --mode agent --shipment 135-S --phase
pre_claim --json` was re-run after the `134-S` archival above. Result:
`exit_code: 0`, `blocked: false`, `message: "topology gate pass"`.
`active_shipment_invariant` check reports `active_shipment_ids: []` (no
top-level release unit active — consistent with `134-S` no longer being
`active`). `shipment_readiness` passed for `135-S` with
`predecessor_ids: ["133-S", "134-S"]`, confirming the gate now recognizes
both predecessors as closed. `branch_ownership` reports
`BRANCH_POST_MERGE_CLOSURE_ELIGIBLE` (expected — this session's branch is
a post-merge closure branch, not a `135-S` feature/chore branch).
`135-S` was **not** claimed this session — this check verifies eligibility
only, per the operator's explicit scope limitation.
