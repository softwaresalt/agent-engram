# Session Memory: Ship 119-S Closure — SQL CREATE PROCEDURE Grammar Fork

**Date**: 2026-08-20/21
**Agent**: Ship
**Shipment**: 119-S (feature 123-F, tasks 123.001-T-123.006-T, 11 subtasks)
**Branch (implementation)**: `feat/123-f-sql-create-procedure-grammar-fork`
**Branch (closure)**: `chore/119-s-closure` (off `origin/main` post-merge)
**Worktree**: `C:\Source\GitHub\engram\.worktrees\ship-119-s-sql-create-procedure-fork-20260820`
**PR**: <https://github.com/softwaresalt/agent-engram/pull/346> - **MERGED**
**Merge commit**: `0bc82aeb2a01ae69a231b54e9b04aa0e2ce99c4e` (merge-commit
strategy only; squash/rebase disabled at the repo level, confirmed via
`allow_squash_merge: false`, `allow_rebase_merge: false`,
`allow_merge_commit: true`)

## Continuation Context

This session resumed Ship after a prior halt at HEAD `0de0dd4c` for
operator merge-ready/blocker review. The operator responded: "You have not
yet marked the task as complete... Keep working autonomously until the
task is truly finished," which was treated as explicit authorization to
complete CI remediation and the merge/closure path once all mandatory
gates were green.

## CI Remediation (the only remaining blocker)

Root cause, fix, and rationale are fully documented in
`docs/compound/build-errors/ci-floating-stable-toolchain-drift-2026-08-20.md`.
Summary: `dtolnay/rust-toolchain@stable` (floating) drifted to Clippy 1.98.0
and failed CI with 78 `clippy::unused_async_trait_impl` errors in
pre-existing, PR-untouched files. Fixed by pinning
`dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772` (action's
`master` HEAD) with explicit `toolchain: "1.97.0"` in both `ci.yml` jobs,
both `release.yml` jobs, and `rust-toolchain.toml`'s `channel` - no Rust
source changed, no lint suppressed.

Two remediation pushes were required:
1. `35558e6a` - `ci.yml` only. CI went green (`build`,
   `start-launcher-windows` both SUCCESS). Copilot's next review posted a
   suppressed comment pointing out `release.yml` and `rust-toolchain.toml`
   still floated `stable`, so the same drift would resurface at the next
   release tag.
2. `e2ec1577` - extended the identical pin to `release.yml` (both jobs) and
   `rust-toolchain.toml`. Local `fmt`/`clippy` (CI flags, all targets)
   reverified green with the toolchain file active (rustup resolved the
   same `1.97.0 (2d8144b78 2026-07-07)` build). CI green again. Copilot's
   review at this exact HEAD raised one more suppressed comment: the
   repo-wide toolchain change had a bigger blast radius than the PR's
   stated SQL-grammar scope and should be split into its own PR or
   explicitly documented. Resolved by appending a "CI remediation addendum"
   section to the PR description (scope/evidence/risk/follow-up) rather
   than opening a second PR, to avoid a redundant review/CI cycle for an
   already-reviewed, already-gated feature. Three other suppressed
   comments (SQL test-fixture dialect nitpicks in already-reviewed,
   already-archived 123.001-T/123.006-T work) were left untouched as
   out-of-scope for Ship's CI remediation - reopening completed/archived
   feature work without operator instruction was judged a greater risk
   than leaving a low-confidence suppressed comment unaddressed.

Follow-up backlog stash `B1024A34` tracks the deliberate Rust/Clippy 1.98+
upgrade and the `clippy::unused_async_trait_impl` redesign-or-reviewed-allow
decision. Not a child of archived 123-F; independent maintenance unit; does
not block 119-S.

## Merge Confirmation Gate

1. `gh pr view 346 --json state,mergedAt,mergeCommit` -> `state: MERGED`,
   `mergedAt: 2026-08-21T00:14:47Z`,
   `mergeCommit.oid: 0bc82aeb2a01ae69a231b54e9b04aa0e2ce99c4e`.
2. `git fetch origin main` then
   `git merge-base --is-ancestor 0bc82aeb... origin/main` -> exit 0.
   Confirmed.

## Post-Merge Closure

* `backlogit shipment ship 119-S --sha 0bc82aeb... --message ... --author
  ...` succeeded (first invocation appeared to hang on the CLI's
  background update-network-check; the actual mutation had already
  completed - confirmed by re-reading `119-S` status as `shipped` with the
  commit recorded before the retry, which correctly returned "shipment
  status conflict" for an already-shipped shipment). Reran with
  `--no-update-check` afterward for all read-only verification calls.
* Final states: `119-S` = `shipped` (remains in `.backlogit/queue/119-S.md`
  by this backlogit version's design - "ship" closes the shipment record in
  place and archives the released *scope*, not the shipment artifact
  itself); `123-F` = `done`/archived; all 6 tasks = `done`/archived; all 11
  subtasks = `done`/archived. No queued/active items remained under the
  manifest - the child-expansion hazard from
  `docs/compound/workflow-issues/backlogit-ship-blocked-child-expansion-2026-04-26.md`
  did not recur because all subtasks were already terminal before shipping
  (resolved in the implementation session).
* Because `feat/123-f-sql-create-procedure-grammar-fork` was already
  merged, the shipment-ship mutation (and this memory/learning doc) were
  committed on a new branch `chore/119-s-closure` created from
  `origin/main` post-merge (not appended to the already-merged feature
  branch), then opened/merged as a small closure PR - consistent with
  "update operational closure... through an appropriate follow-up closure
  PR."
* 72-hour post-merge observation window (per
  `docs/closure/2026-08-20-sql-create-procedure-compatibility-fork-closure.md`):
  owner = release maintainer (softwaresalt, EG-1 lifecycle owner); signals
  = parse panics/errors, missing/duplicate `Function` symbols for
  `CREATE PROCEDURE`/`CREATE FUNCTION`, ABI/build failures on
  Linux/Windows/macOS; scheduled close = 2026-08-24T00:15Z (72h from merge,
  real-time, not literally blocked in-session per existing shipment
  practice).

## Root Worktree Isolation

The root `main` worktree at `C:\Source\GitHub\engram` was never read,
modified, cleaned, reset, stashed, or merged into during this session. All
git operations (fetch, checkout -b, commit, push) were performed inside the
existing Ship worktree
`C:\Source\GitHub\engram\.worktrees\ship-119-s-sql-create-procedure-fork-20260820`.
Its dirty, unrelated preserved backlog repairs remain untouched.

## Tooling Notes

* Engram CLI could not bind the worktree as a root (`.git` file, not a
  directory) - `engram daemon-status --timeout 15` failed immediately with
  "not a Git repository root." Recorded as a bounded, expected degraded
  state per session instruction; fell back to direct `git`/`view`/`gh`
  reads for all discovery in this session. Did not touch the daemon
  lifecycle.
* `backlogit sync` failed with 19 pre-existing, unrelated archive/queue
  artifact parse errors (029.x/030.x shipment files, `archived_from`
  self-reference warnings from `backlogit doctor`) that predate and are
  unrelated to 119-S/123-F. Logged `INDEX_SYNC_WARN` and proceeded with
  direct `backlogit get`/`list` reads, which succeeded independent of the
  full-index rebuild.
* `backlogit --no-update-check` is required to get responsive CLI reads in
  this environment; without it, some invocations block for minutes on a
  remote latest-release check.
