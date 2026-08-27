---
title: Ship session memory — shipment 130-S (feature 137-F), pre-merge checkpoint
type: session-memory
doc_type: memory
date: 2026-08-27
status: pre-merge
shipment: 130-S
feature: 137-F
pr: https://github.com/softwaresalt/agent-engram/pull/364
---

# Ship session memory — 130-S / 137-F, pre-merge checkpoint

This is a Ship-pipeline session memory, not an RCA. The authoritative RCA is
`docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md` and is
not restated here.

## What happened this session

Executed the full Ship pipeline for queued shipment `130-S` (covering feature
`137-F`, retro-staged Stage-governed verification/release wrapper around the
already-implemented late-readiness stdio-proxy recovery diff, originating from
archived ad-hoc `136-F` / `136.001-T`).

### Isolation (P-011)

Main worktree was dirty with a pre-existing unresolved `.backlogit/stash.jsonl`
merge conflict and a pre-existing staged `.gitignore` modification, both
explicitly out of scope. Created a dedicated clean worktree/branch
(`feat/137-late-readiness-proxy-recovery-verification`, at
`.worktrees/ship-137-late-readiness-proxy-recovery-20260826`) from `main`
HEAD `19ae3160b7040e16213eba9ef7611f6573d3f4cd`, then copied only the 21
allowlisted implementation/planning/backlog files from main's working tree
into it with SHA-256 hash verification (0 mismatches). Main was re-verified
byte/status-identical to baseline both before and after every mutation in the
isolated worktree, including after the final commits. The isolated worktree's
own `.backlogit` index (checked out cleanly from HEAD) had no merge conflict,
so `backlogit sync` and `git commit` succeeded there without ever touching or
resolving the conflicted file in main.

### Verification (V1–V4, tasks 137.001-T…137.004-T)

All four independent verification/audit tasks completed and moved to `done`
(archived) with evidence recorded as backlogit comments:

* **V1** — full `contract_shim_stdio_initialize` suite (5/5), plus the
  late-readiness-recovery and disconnect-teardown cases each run 3× with zero
  flakes; recovery confirmed driven by `ENGRAM_TEST_STARTUP_DELAY_MS`, not a
  sleep race; no orphan daemon/pipe endpoint.
* **V2** — `cargo fmt`/`clippy --all-targets -D warnings -D clippy::pedantic`
  clean; `cargo dev-test` green (one transient, unrelated-file timeout flake
  on the first full run, reproducibly green on an isolated rerun and a second
  full-suite rerun); `cargo audit` exactly 14 pre-existing allowed advisories.
* **V3** — read-only audit confirmed `ENGRAM_TEST_STARTUP_DELAY_MS` is
  `#[cfg(debug_assertions)]`-gated (release-neutral) and `DbStartupTimings` /
  the `ipc_server` log-wording change are observational-only.
* **V4** — `docs/troubleshooting.md` documents `recoverable` / `retry_after_ms`
  exactly as emitted; 45 cross-references across decision/memory/plan/review
  docs resolved with zero missing; `137-F` → archived `136-F`/`136.001-T`
  links resolve.

### Review gate

An internal code-review pass (report-only) found no P0/P1 issues.

### Commit / PR (V5, task 137.005-T, still `active` — see below)

* Commit 1 `d8488a1f79be7264e1e6977ac79eb5766644b681` — the 22-file allowlisted
  change set (6 modified tracked files + 16 new files, including archived
  `136-F`/`136.001-T` and the 137 planning/backlog artifacts). No `.gitignore`,
  no `.backlogit/stash.jsonl`.
* PR #364 opened from the isolated branch against `main`.
* Real GitHub Copilot review requested via GraphQL `requestReviews(botIds:
  ["BOT_kgDOCnlnWA"])` (the REST `requested_reviewers` endpoint and `gh pr
  edit --add-reviewer` both silently no-op or 422 for this bot login in this
  environment — GraphQL `botIds` is the only mechanism that worked).
* First Copilot review (`COMMENTED`) found 4 items: a real terminal-vs-transient
  health-error classification gap in `check_health`'s boolean-only return
  (affects both the request-triggered probe and the late-readiness monitor), a
  missing concurrency test for the single-flight/cooldown behavior, and a
  plan-doc wording inaccuracy.
* Declined the two code-behavior findings and the test-coverage gap for
  in-place fixing — 130-S/137-F is a retro-staged, explicitly zero-source-edit
  verification-and-release wrapper; filed follow-up task **`137.006-T`**
  (parent `137-F`, queued, not part of this shipment's manifest) capturing
  both, plus a later-found stale teardown code comment (also declined in-place
  for the same scope-boundary reason, even though it was comment-only).
* Fixed the plan-doc wording inaccuracy directly (docs-only, part of our own
  shipped audit artifact) in commit 2
  `db68add3514e1d85e9354fe2c93f63ec7e31c006`, which also added `137.006-T` and
  tracked the commit SHA on `137.005-T`.
* Replied to and resolved all Copilot review threads (0 unresolved).
* Second Copilot review at HEAD `db68add3...`: **"🟢 Approval recommended"**,
  1 new (now resolved) comment, 1 explicitly non-blocking suppressed
  documentation nit (duplicate H1 vs. frontmatter `title:` in the plan doc —
  left as-is, non-blocking, optional future cleanup).
* CI (`build`, `start-launcher-windows`, `copilot-pull-request-reviewer`): all
  `success` at HEAD.
* Merge gate re-checked at HEAD `db68add3...`: Copilot review present at HEAD
  ✓, Copilot removed from `requested_reviewers` ✓, 0 unresolved threads ✓,
  `mergeable_state == clean` ✓. **PR is merge-ready.**

## What is NOT done (by design — operator gate)

* **No merge has been performed.** The operator has not given explicit
  merge approval in this session, and the pipeline's merge-approval gate is
  non-negotiable. Merge-commit only; do not squash/rebase-merge.
* `137.005-T` remains `active` (not `done`), `137-F` remains `active`, `130-S`
  remains `active` (not shipped) — by the task's own precondition text,
  completion/shipment happens only after merge.
* `.backlogit/stash.jsonl`'s pre-existing merge conflict in the **main**
  worktree was never touched, edited, or resolved — that remains an operator
  action outside this shipment. It did not need to be resolved for this PR:
  the isolated worktree's own index never entered the conflicted state.
* The pre-existing staged `.gitignore` modification in main was never
  touched.

## Rollback / monitoring signals for the operator

* This is a local MCP daemon reliability change (stdio shim + adjacent
  CozoDB/ipc_server diagnostics). No schema migration, no data format change,
  no network-facing surface change.
* Rollback is a plain `git revert` of the two PR commits — no data cleanup
  required. `src/db/cozo_backend/mod.rs` has no shim dependency and could be
  reverted independently of the shim changes if ever needed (per the plan's
  risk table).
* Runtime signal to watch post-merge: the new `tracing::info!` "CozoDB startup
  timing" log line (fields `process_lock_ms`, `file_lock_ms`,
  `database_open_ms`, `schema_bootstrap_ms`, `blocking_total_ms`,
  `connect_total_ms`, `db_bytes`, `branch`) is purely additive; no alerting
  changes needed. The shim's degraded `tools/call` responses now also carry
  `recoverable` / `retry_after_ms` — operators/agents parsing degraded
  responses should treat `recoverable: false` as the actionable failure
  signal (see `docs/troubleshooting.md`).
* Known accepted follow-up debt: `137.006-T` (parent `137-F`, queued) — the
  terminal-vs-transient health-error classification gap. Narrow blast radius
  (only affects a daemon that becomes reachable but is permanently
  protocol-incompatible after the shim's initial timeout); does not regress
  anything relative to the pre-137 baseline, where that same daemon would
  simply have stayed degraded forever with no recovery path at all.

## Next steps

1. Operator reviews PR #364 and gives explicit merge approval.
2. On approval: merge via merge-commit (not squash/rebase), then run the
   Merge Confirmation Gate (`gh pr view --json state,mergedAt,mergeCommit`,
   `git merge-base --is-ancestor`) before any post-merge closure.
3. Post-merge: move `137.001-T`…`137.005-T` and `137-F` to `done` (001–004
   already archived done from this session; `137.005-T`/`137-F` finalize
   after merge SHA is recorded), `backlogit shipment ship 130-S` with the
   merge SHA, and run the closure index resync.
4. Stage should pick up `137.006-T` in a future planning cycle.
