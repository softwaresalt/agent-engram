---
title: "136-S post-merge operational closure"
doc_type: closure
shipment_id: "136-S"
feature_id: "142-F"
mode: post-merge
date: 2026-09-08
author: ship
verdict: "CLOSED — PR #385 merged as a merge commit under explicit operator approval ('PR 385: merge approved'), verified reachable from origin/main; shipment 136-S manually safe-closed (targeted, non-cascading) following the 133-S/134-S/135-S precedent; 142-F verified untouched and remains active for later 142-F-covering shipments."
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "pending"
pr_number: 385
merge_commit: "7632f8c03b23a5b2da064ca027ed584865cfc74b"
head_commit_merged: "fd97e8004ab01595aa0f55cfcbf59313e4f1cdf9"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: "docs/closure/136-S-2026-09-08-runtime-verification.md"
follow_up_stash:
  - "9CB60992"
  - "96A1197D"
  - "341497BC"
  - "F2A07647"
  - "9108DB24"
  - "6B624CF6"
  - "F0760EF2"
  - "959D1448"
  - "8D97190A"
  - "D1D94CF4"
  - "259D0E38"
  - "A67EEA54"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) — manual safe-close performed 2026-09-08 on post-merge/136-s-generation-domain-store-atomic-publication-and-database-open"
---

# 136-S post-merge operational closure

## Summary

`136-S` (feature `142-F`) delivered the generation domain and module
(`142.011-T`), the generation store surface with reserved-name and
path-escape guards (`142.012-T`), atomic publication with a bounded
publisher lock, orphaned-staging detection, and monotonic revision
guarding (`142.013-T` + 4 subtasks), the runtime-copy database-open path
with hashed integrity checks (`142.014-T`), and the cheap-clone generation
context handle (`142.017-T`).

PR #385 merged as a merge commit (repo policy: merge-only, squash/rebase
disabled) under explicit, PR-scoped operator approval: **"PR 385: merge
approved."** Approval was contingent on the last-mile gates still passing
for the exact reviewed HEAD (`fd97e8004ab01595aa0f55cfcbf59313e4f1cdf9`),
which this session independently re-verified immediately before merge (see
"Merge / PR Evidence" below) rather than trusting the operator's message
alone.

This closure covers `136-S`'s own 9-item manifest only. It does not
re-open, re-plan, or touch any other `142-F`-covering shipment
(`137-S`-`142-S`), and it does not archive or otherwise mutate `142-F`
itself, which remains `status: active` for those later shipments.

## Merge / PR Evidence

| Gate | Result |
|---|---|
| PR #385 state | `MERGED` at `2026-09-08T21:17:01Z` |
| Merge commit | `7632f8c03b23a5b2da064ca027ed584865cfc74b` |
| Reviewed/merged HEAD | `fd97e8004ab01595aa0f55cfcbf59313e4f1cdf9` (exact HEAD the operator approval named; unchanged through merge — no push occurred between approval and merge) |
| Ancestry | `git fetch origin main` then `git merge-base --is-ancestor 7632f8c0... origin/main` → exit `0` |
| Merge strategy | `--merge` (merge commit); squash/rebase disabled repo-wide (`allow_merge_commit: true`, `allow_squash_merge: false`, `allow_rebase_merge: false`) |
| Local review readiness at HEAD | `READY_WITH_FOLLOWUPS`, P0=0, P1=0 (PR body "Local Review Readiness" block; explicit follow-up stash IDs listed) |
| P-018 Copilot review gate | `autoharness gate copilot-review 385 --repo softwaresalt/agent-engram --enforcement auto --json` → `SATISFIED`, 0 unresolved threads, re-verified immediately before merge at the exact approved HEAD |
| Human review decision | `reviewDecision: null` (no blocking `CHANGES_REQUESTED`) |
| `mergeStateStatus` / `mergeable` (pre-merge) | `CLEAN` / `MERGEABLE` |
| CI checks | `build`: SUCCESS, `start-launcher-windows`: SUCCESS |
| Pipeline-topology gate (`--phase lifecycle`, pre-safe-close) | `exit_code: 0`, `active_shipment_invariant` passed (`active_shipment_ids: ["136-S"]`), `branch_ownership` passed (`BRANCH_CREATE_ELIGIBLE` on `main` for the post-merge branch creation step), `worktree_topology` passed (`WORKTREE_TOPOLOGY_OK`, single implementation worktree), `shipment_readiness` passed (`predecessor_ids: ["133-S", "135-S"]`) |
| Worktree topology (P-016) | Single worktree throughout (`git worktree list --porcelain` confirmed one entry before and after merge) |

## Runtime Verification

Verdict: **PASS WITH FOLLOW-UP** (full report:
[`136-S-2026-09-08-runtime-verification.md`](./136-S-2026-09-08-runtime-verification.md)) —
all mandatory validator targets pass (dev check, release build, CLI/MCP
smoke checks, 94/94 targeted contract/unit/integration/lib tests); the
`FOLLOW-UP` qualifier reflects two open, `requires_deliberation: true`
P-021 findings about the generation-identity/runtime-copy lifetime model
that must be resolved before a future shipment wires the first production
caller, not any currently-broken or unverified behavior in `136-S`'s own
shipped scope.

### Validator Evidence (structured)

| Field | Value |
|---|---|
| Surface / adapter | Internal domain/service layer (generation identity, store, atomic publish, database-open via runtime copy) — no wired production caller yet; CLI/MCP smoke checks confirm no regression to the existing bound-daemon surface |
| Verdict | `PASS WITH FOLLOW-UP` |
| Healthy signal | `cargo check --all-targets` (1m32s) and `cargo build --release` (4m27s) both green; `engram.exe --version`/`manifest` (release binary) ok; 94/94 targeted tests pass across `integration_generation_db_open` (8), `integration_generation_store` (11), `integration_generation_publish` (8), `unit_generation_domain` (3), `unit_generation_context` (4), `--lib services::generations` (13), `--lib db::cozo_backend` (39) |
| Failure signal | none observed |
| Manual checkpoint evidence | Targeted test run and CLI/MCP smoke checks executed directly against the merged `main` tip on the `post-merge/136-s-...` closure branch (same tree, no separate worktree) |
| Blocked prerequisites | none — unlike `134-S`, this shipment has no bound-daemon CLI surface (`engram status`/`health`/`sync`) to probe yet, since no caller wires the new generation domain/store/publish/db-open code into the composition root |

## Operational Closure Checklist

* **Invariants to preserve**: generation IDs remain strict
  single-component values (path-separator/traversal/Windows-drive-prefix
  rejection); revision remains strictly monotonic and checked; the store
  rejects reserved names case-insensitively and rejects path/junction
  escapes; atomic publication never tears the destination manifest and
  serializes concurrent publishers to exactly one winner; database-open
  via runtime copy validates `final_path`'s UTF-8-ness before any mutation
  and the runtime copy's bytes never diverge from the published manifest.
* **Pre-deploy audits**: none required — no config schema, migration, or
  access-control surface changed. No CLI/MCP-facing surface changed either
  (this shipment's new code has no wired caller yet).
* **Deployment / rollout path**: merge-only to `main` (repo policy:
  squash/rebase disabled, merge commit only). `engram` is a locally-run
  daemon/CLI binary distributed via release artifact; the next
  `release.yml` build is the next rollout checkpoint and is confirmed
  unblocked (`cargo build --release` green, see Runtime Verification).
* **Post-deploy checks**: `cargo check --all-targets` and
  `cargo build --release` green; CLI/MCP smoke checks (`--version`,
  `manifest`) pass against the release binary; 94/94 targeted tests pass.
* **Risky action record**: `ProposedAction`: merge PR #385 to `main`
  (new internal generation domain/store/publish/db-open surface,
  additive-only, no existing behavior removed or changed). `ActionRisk`:
  high per the task framing (large new subsystem, generation-identity and
  atomic-publication invariants are safety-critical for future
  read-server correctness) but non-destructive at merge time — no wired
  caller exists yet, so no production behavior changes as a direct result
  of this merge. Approval path: explicit, PR-scoped operator approval
  ("PR 385: merge approved"), independently re-verified against the exact
  approved HEAD and all last-mile gates immediately before executing the
  merge (see Merge / PR Evidence). `ActionResult`: success — merge
  completed cleanly via `gh pr merge 385 --merge`, ancestry verified
  (`git merge-base --is-ancestor`), CI green pre-merge, 0 unresolved
  Copilot threads, no `CHANGES_REQUESTED`. A second, separate,
  Ship-role-permitted (not independently destructive-approval-gated)
  action was the shipment safe-close itself (`backlogit move 136-S
  --status done` → `backlogit archive 136-S`), performed under the
  established 133-S/134-S/135-S manual safe-close precedent because the
  covering feature `142-F` is a shared partial-covering root (59 total
  roster children; this shipment's manifest covers only 9, with no
  feature member in `items`) — the cascade `backlogit shipment ship`
  operation is P-015-forbidden for this shipment family. `ActionResult`:
  success — verified via live re-read (`status: done` before archive,
  `archived_status: done` after), `142-F` verified unchanged
  (`status: active`), zero orphans, no unrestored archive deletions
  (`git status -- ".backlogit/archive/"`).
* **Healthy signals**: see Validator Evidence table above — dev
  check/build, CLI smoke checks, and full targeted suite all green.
* **Failure signals**: a future `cargo build --release` failure on `main`
  touching `src/services/generations/*` or `src/db/cozo_backend/mod.rs`,
  or a regression in the targeted contract/unit/integration suite for
  this shipment's scope, would each indicate rollback or hotfix
  intervention is needed.
* **Monitoring plan**: no live dashboards/alerts apply to this
  locally-run developer tool; the operative monitoring signal is the next
  scheduled/triggered `release.yml` build and the targeted test suite run
  on subsequent shipments (F17/F18) that wire this generation surface into
  the daemon composition root — those shipments are the first point at
  which the two deferred design-gap findings below become directly
  observable in running behavior.
* **Rollback trigger**: a `release.yml` build failure, or a reported
  regression traced to the generation domain, store, atomic-publication,
  or database-open modules introduced by this shipment.
* **Rollback procedure**: fix-forward is preferred; if not viable, revert
  the merge commit (`7632f8c0`) on `main` and re-open `142-F`'s affected
  tasks (`142.011-T`, `142.012-T`, `142.013-T` + 4 subtasks, `142.014-T`,
  `142.017-T`).
* **Validation window**: through the next `release.yml` run and the next
  `142-F`-covering shipment (F17/F18) that wires this generation surface
  into a real caller — that is the point at which the two deferred
  design-gap findings must be resolved via Stage deliberation before
  proceeding.
* **Owner**: the Ship agent / operator executing the next release cut, or
  Stage when deliberating the F17/F18 wiring decision that depends on
  resolving `9108DB24` and `A67EEA54`.

### Releasability Evidence (structured)

| Requirement | Status |
|---|---|
| Dev build/check | Satisfied — `cargo check --all-targets` green |
| Release-artifact build | Satisfied — `cargo build --release` green |
| Targeted test suite | Satisfied — 94/94 green |
| CI required checks (PR #385) | Satisfied — `build`, `start-launcher-windows` both SUCCESS |
| Local review readiness (PR #385) | Satisfied — `READY_WITH_FOLLOWUPS`, P0=0, P1=0, explicit follow-up stash IDs recorded |
| P-018 Copilot review gate (PR #385) | Satisfied — `SATISFIED`, 0 unresolved threads at merge |
| Human review decision | Satisfied — no blocking `CHANGES_REQUESTED` |
| Rollback path defined | Satisfied — fix-forward primary, hard-revert fallback documented above |
| Monitoring plan defined | Satisfied — see Monitoring plan above |
| Shipment-record archival (manual safe-close) | Satisfied — performed 2026-09-08 on `post-merge/136-s-generation-domain-store-atomic-publication-and-database-open`; `136-S` is `status: archived`, `archived_status: done` |
| Generation-identity/runtime-copy lifetime design gap | **Condition** — `9108DB24` and `A67EEA54` (both `requires_deliberation: true`) must be resolved via Stage deliberation before F17/F18 wire the first production caller of this shipment's new surface |

**Overall**: `READY_WITH_CONDITIONS`. `136-S`'s own scope is fully closed
and verified; the single named condition is a forward-looking design
decision gating a *future* shipment (F17/F18), not a defect in what was
actually shipped here — no production caller of the affected code exists
today.

## Reconciliation

* **Pre-mode** (`.backlogit/reconcile/136-S-pre-20260908T141854Z.md`,
  `expected_status: done`): all 9 manifest items classified
  `pre-archived` (already archived with `archived_status: done` during
  the build phase, on the merged feature branch itself, before this
  closure session began). No `missing`, `status-mismatch`, or `orphan`
  items. `recommendation: PROCEED`.
* **Safe-close mutation applied**: `backlogit update 136-S --section
  description=<audit rationale>` (P-015 cascade-ineligibility rationale
  and precedent citation) → `backlogit move 136-S --status done`
  (live-verified `status: "done"`) → `backlogit archive 136-S`
  (live-verified `archived_status: "done"`, relocated from
  `.backlogit/queue/136-S.md` to `.backlogit/archive/136-S.md`). No
  manifest item's `status`, `parent_id`, or `id` field was touched by this
  session (all 9 were already archived prior to this session).
* **Post-mode** (`.backlogit/reconcile/136-S-post-20260908T142130Z.md`):
  all 9 manifest items confirmed present in `.backlogit/archive/`; the
  shipment record itself confirmed present in `.backlogit/archive/136-S.md`.
  `git status -- ".backlogit/archive/"` reported only the new untracked
  `136-S.md` file — no deletions, no `git restore` required.
  `recommendation: PROCEED`.
* **Covering feature verification**: `142-F` confirmed `status: active`
  (unchanged) via `backlogit get 142-F`. `142-F`'s `size_composition`
  roster (59 children) is untouched by this session — no mutation
  targeted `142-F` or any item outside `136-S`'s own 9-item manifest.
* **Orphan scan**: grep across `.backlogit/queue/*.md` for the literal
  token `136-S` finds only `137-S.md` and `138-S.md`, both of which
  legitimately list `136-S` as a `dependencies:` entry in their own
  frontmatter (not manifest membership) — no orphans introduced.
* **P-015 cascade classification**: `136-S`'s manifest contains only
  task/subtask items — no feature member (`142-F` absent from `items`).
  `142-F` has 59 total roster children while this manifest covers only 9;
  the verified-fully-covered-root exception does not apply. The cascade
  close path (`backlogit shipment ship`) was correctly never invoked;
  targeted manual safe-close (as used for `133-S`, `134-S`, and `135-S`)
  was the only valid path, and is exactly what was performed.
* **Source artifact cleanup**: none of `136-S`'s 9 manifest items carry a
  `source_stash_id` or `source_deliberation_id` custom field — no
  source-artifact cleanup performed or required.

## Source artifact cleanup

- Archived stash (`source_stash_id`): none — no manifest item carries a
  `source_stash_id` custom field.
- Archived deliberations (`source_deliberation_id`): none — no manifest
  item carries a `source_deliberation_id` custom field.
- Skipped (already archived or not found): none — no candidate fields
  existed to act on.

## Preserved Scope (142-F descendants)

No mutation performed by this session touched `142-F`, any of its
direct/nested descendants outside `136-S`'s own 9-item manifest, or any
other covering shipment's manifest (`125-S`-`131-S`, `137-S`-`142-S`).
`backlogit shipment ship` was never invoked. `142-F` was never archived.
Future shipments `137-S` and `138-S` (which declare `136-S` as a
dependency) were not claimed, edited, or otherwise touched.

## Follow-up disposition (P-021, carried forward, not re-actioned)

All follow-up findings below were already captured with full P-021
traceability on PR #385 prior to this closure session. This closure
session did not edit, re-prioritize, or re-classify any of them (single-write
invariant preserved); it only carries their IDs forward for Stage triage:

| Stash ID | Priority | Requires deliberation | Note |
|---|---|---|---|
| `9CB60992` | medium | no | Pre-existing, from original F06–F09/F16a implementation |
| `96A1197D` | low | no | Pre-existing, from original F06–F09/F16a implementation |
| `341497BC` | medium | no | Pre-existing, from original F06–F09/F16a implementation |
| `F2A07647` | low | no | Pre-existing, from original F06–F09/F16a implementation |
| `9108DB24` | high | **yes** | Runtime-copy stable path vs. live `OpenedGeneration` handle — pre-integration blocker for F17/F18 |
| `6B624CF6` | low | no | Informational correction (stray `\r` JSON-escape in an earlier stash entry's text) |
| `F0760EF2` | low | no | `sync_runtime_copy_parent_dir` error-attribution finding (same family as an already-fixed bug) |
| `959D1448` | medium | **yes** | `GenerationRevision::new(0)` sentinel-collision finding |
| `8D97190A` | medium | no | Superseded/corrected by `D1D94CF4` — do not action directly |
| `D1D94CF4` | low | no | Missing regression test for `GenerationManifest` deserialize-time `GenerationId` validation |
| `259D0E38` | medium | no | Timeout-cleanup race risk in unrelated lock-acquisition test/cleanup code |
| `A67EEA54` | high | **yes** | `generation_id` not bound to `ExistingDbLocation` — identity-binding design gap, likely related to `9108DB24` |

No new follow-up was identified by this closure session beyond what PR
#385 already captured; nothing new was stashed.

## Documentation / knowledge-graduation evaluation

* `docs/ARCHITECTURE.md`: no update made. The new generation domain/store/
  publish/db-open modules (`src/services/generations/*`,
  `src/db/cozo_backend/mod.rs` additions) have no wired caller in the
  daemon composition root yet (F17/F18 remain future shipments in the
  `142-F` roster). Consistent with `133-S`/`134-S`/`135-S` precedent (none
  of which added an `ARCHITECTURE.md` section for their own intermediate,
  not-yet-wired `142-F` roster units), architecture documentation for this
  subsystem is deferred until F17/F18 actually wire it into the read-server
  composition root, at which point the description will be accurate and
  complete rather than a snapshot of an interim internal-only state.
* `AGENTS.md`: no update — no agent or skill behavior changed by this
  shipment.
* `docs/research/`: no update — no new design decision was graduated by
  this shipment; the two open design questions (`9108DB24`, `A67EEA54`)
  are explicitly *not yet decided* and are tracked as P-021 deferred
  findings pending Stage deliberation, not as graduated research.
* `docs/compound/`: no `compound-refresh` invocation. The one compound
  artifact touched by this shipment's build phase
  (`docs/compound/best-practices/proactive-copilot-review-pattern-checklist-2026-09-08.md`)
  was authored fresh during the build session itself (not before it), so
  no prior compound entry was superseded, duplicated, or invalidated by
  this shipment's work. No evidence of staleness in any other
  `docs/compound/` entry was surfaced during this closure session.

## Compaction status (P-020)

**`pending`** at the time this document was created by `operational-closure`
(Step 6 item 2). Finalized to `done` or `degraded` by Ship Step 6 item 8
(`compact-context --target all`) — see the "Compaction status" update
below, appended after that invocation completes.
