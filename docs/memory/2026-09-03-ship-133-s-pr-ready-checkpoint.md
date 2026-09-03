# Ship 133-S — PR Ready for Operator Approval

* **Date**: 2026-09-03
* **Shipment**: `133-S` (read-server foundations: test-manifest registration,
  workspace membership, storage feasibility spike, mode contract)
* **Branch**: `feat/133-s-read-server-foundations-test-manifest-registration-workspace-membership-storage-spike-mode-contract`
* **PR**: [#376](https://github.com/softwaresalt/agent-engram/pull/376)
* **HEAD**: `9ccbfffa60b1d00d56af08f5ab7143cdf1901fcd`

## Status: PR ready for operator-approved merge — NOT MERGED

All pre-merge gates pass. Awaiting explicit separate operator approval per
P-014. Do not merge without it.

## Manifest items — all 10 task-level items `done`

* `142.001-T` + 5 subtasks (F00 — test-manifest registration, 49 placeholders)
* `142.002-T` (F02 — strict `DaemonMode` parsing)
* `142.004-T` (F01 — storage feasibility spike, GO verdict)
* `142.006-T` (F12a — `engram-indexer` crate stub + workspace membership)
* `142.007-T` (F03 — immutable mode-in-`AppState`)
* Covering feature `142-F` correctly left `active` (59-child multi-shipment
  feature; only this shipment's 10 items are `done`).

## Quality gates (all green at HEAD `9ccbfffa`)

* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: clean
* `cargo fmt --all -- --check`: clean
* `cargo dev-test` (full suite): 100% green, including the previously
  flaky `archive_verifier_runs_the_unpacked_native_binary` (passed clean
  this run; no code changed to affect it, existing stash `58B33C45` still
  applies if it recurs)
* CI (`build`, `start-launcher-windows`): both pass. One transient
  hosted-runner timing overrun on an unrelated PowerShell-launcher test
  self-resolved on rerun — not captured as a new stash entry (not a scope
  decision, just runner variance).

## Local review gate — READY_WITH_FOLLOWUPS

8 persona reviewers (Constitution, Rust, Correctness, Maintainability,
Learnings Researcher + conditional Security/Architecture/Scope Boundary).
Zero P0/P1. One P2 fixed in-diff (missing `required-features =
["cozo-backend"]` on F01's probe test target). Two P2 findings recommending
scope beyond the manifest captured as P-021 deferred stash entries
(`A7C0BA5F`, `5A7FBC37`) rather than fixed.

## Copilot review (P-018) — SATISFIED

Copilot reviewed at prior HEAD `4c8cc253` and flagged two real threads: the
F01 rename probe proved only the happy-path rename, not crash-point or
directory-durability behavior. Fixed in commit `9ccbfffa`
(`interrupted_rename_never_yields_a_torn_destination` +
`sync_parent_dir`), replied to and resolved both threads, Copilot
re-reviewed at the new HEAD with zero new threads.
`autoharness gate copilot-review 376 --enforcement auto` → `SATISFIED`.

## P-009 — merge-commit-only confirmed at repo level

`allow_merge_commit: true`, `allow_squash_merge: false`,
`allow_rebase_merge: false`.

## Follow-ups / blockers (none block this shipment; all require Stage
deliberation before action)

* `58B33C45` — pre-existing full-suite flaky test, unrelated to 133-S
* `7B270F79` — pre-existing `cargo ci --all-features` opentelemetry compile
  break, unrelated to 133-S, present on `main` before this PR
* `A7C0BA5F` — placeholder-tracking mechanism suggestion for F00 (out of
  F00's registration-only scope)
* `5A7FBC37` — `#[deprecated]` attribute suggestion on temporary `AppState`
  constructors (F04's scope, not F03's)
* Existing `142-F` cascade-close warning: post-merge closure for 133-S MUST
  use safe-close via `shipment-reconcile`, never `backlogit shipment ship`
  (would cascade to all 59 children of `142-F` across all shipments)

## Next steps (blocked on operator approval)

1. Present PR #376 to operator for explicit merge approval (P-014) — **do
   not auto-merge**.
2. On approval: merge via merge-commit strategy only.
3. Post-merge: Step 6 closure — create `post-merge/133-s-...` branch, safe-close
   the shipment via `shipment-reconcile` (NOT cascade `shipment ship`),
   operational-closure artifact, compound-refresh, compact-context (P-020),
   backlog index resync.

Remaining on the feature branch per Branch Management Rules until merge is
approved.
