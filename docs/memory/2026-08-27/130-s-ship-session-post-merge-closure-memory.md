---
title: Ship session memory — shipment 130-S (feature 137-F), post-merge closure
type: session-memory
doc_type: memory
date: 2026-08-27
status: post-merge
shipment: 130-S
feature: 137-F
pr: https://github.com/softwaresalt/agent-engram/pull/364
merge_commit: 2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0
supersedes: docs/memory/2026-08-27/130-s-ship-session-pre-merge-memory.md
---

# Ship session memory — 130-S / 137-F, post-merge closure

This is a Ship-pipeline session memory, not an RCA. The authoritative RCA is
`docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md` and is
not restated here. This memory picks up immediately after the pre-merge
checkpoint recorded in
`docs/memory/2026-08-27/130-s-ship-session-pre-merge-memory.md`.

## Operator approval

Explicit merge approval was given in-conversation: `PR 364: Merge approved`,
2026-08-27 10:35 PDT.

## Merge-gate recheck (immediately before merge)

Rechecked all four load-bearing gates at HEAD `db68add3...` (unchanged from
the pre-merge checkpoint) before merging:

1. Copilot review commit_id equals HEAD, paginated API + login prefix match:
   latest `copilot-pull-request-reviewer[bot]` review is at
   `db68add3514e1d85e9354fe2c93f63ec7e31c006` == current HEAD. ✓ (An earlier
   Copilot review at an older commit `d8488a1f...` is superseded by this
   later one — expected and correctly disregarded.)
2. Copilot absent from `requestRequests`: GraphQL `reviewRequests.nodes` is
   empty. ✓
3. Zero unresolved review threads: all 5 `reviewThreads.nodes[].isResolved`
   are `true`. ✓
4. `mergeStateStatus`/`mergeable`: `CLEAN` / `MERGEABLE`. ✓
5. HEAD unchanged across the whole recheck window: `db68add3...` both times. ✓

## Merge

`gh pr merge 364 --merge` (merge commit only; squash/rebase disabled
repo-wide). Result: PR `MERGED` at `2026-08-27T17:37:01Z`, merge commit
`2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0`.

**Merge Confirmation Gate**: `git fetch origin main` then `git merge-base
--is-ancestor 2e1e01cf... origin/main` → exit `0`. `git log -1 --pretty=%P`
on the merge commit shows two parents
(`19ae3160b7040e16213eba9ef7611f6573d3f4cd` +
`db68add3514e1d85e9354fe2c93f63ec7e31c006`) — confirmed a true merge commit,
not a fast-forward or squash.

## Backlog reconciliation (post-merge)

* `137.005-T` → `backlogit move 137.005-T --status done` → gate `passed`,
  `head_sha: db68add3...`. Auto-routed to `.backlogit/archive/`.
* `137-F` → `backlogit update 137-F --commit 2e1e01cf...` (records the merge
  SHA for traceability), then `backlogit move 137-F --status done` →
  succeeded at the **task-level** move gate (no children check at that
  layer) and auto-routed to archive.
* Pre-ship reconciliation (`shipment-reconcile`, manual/CLI-backed — MCP
  unavailable) at `expected_status: done`: all 6 manifest members
  (`137-F`, `137.001-T`…`137.005-T`) classified `matched` in archive; zero
  orphans (`137.006-T` mentions `130-S` only in prose body, not in
  frontmatter `shipment_id`). Recommendation `PROCEED`. Report:
  `.backlogit/reconcile/130-S-pre-20260827T104137-0700.md`.
* **`backlogit shipment ship 130-S --sha 2e1e01cf... --message "..." --author
  softwaresalt` refused (exit 6)**: `member 137.006-T is queued (not
  completed through the gate): gate blocked: 137.006-T remains active`.
  `137.006-T` is **not** a declared manifest member of `130-S` — the
  `shipment ship` command derives closure scope from the covering-feature
  parent/child relationship in addition to the declared manifest. This is
  the same class of tool behavior previously documented in
  `docs/compound/workflow-issues/backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md`
  (there it over-*archived* an excluded feature; here it over-*blocks* an
  included one). The refusal made no partial mutation (fail-closed).
* **Corrective action**: reverted `137-F` back to `status: active` via
  `backlogit move 137-F --status active` (retaining the merge-SHA `commit`
  field for traceability). Did **not** force through `--force-gates`
  (operator-only, requires a reason, and the operator's approval this
  session covered only the PR #364 merge, not gate overrides). Did **not**
  reparent/adopt `137.006-T` away from `137-F` to work around the gate —
  that would require inventing a new placeholder parent feature (Stage's
  job) purely to satisfy a tool quirk, which risks falsifying the item's
  real relationship to `137-F`.
* `137.006-T` left completely untouched: `status: queued`,
  `parent_id: 137-F`, in `.backlogit/queue/137.006-T.md`. Not claimed
  complete; source fix not implemented. Handed off to Stage as ordinary
  queued backlog work (no separate transfer mechanism needed — it is already
  visible to Stage's next triage pass).
* Halt reconciliation report documenting the refusal and corrective action:
  `.backlogit/reconcile/130-S-halt-20260827T104502-0700.md`.
* `130-S` remains `status: active` (unshipped/unarchived) as a result. **The
  merged code itself is fully live on `origin/main` regardless of this
  backlog administrative state** — the merge is not blocked or affected by
  the shipment-ship refusal.
* `backlogit sync` run after every mutation to keep the index current.

## Runtime verification and operational closure

* `docs/closure/130-S-2026-08-27-runtime-verification.md` — verdict **PASS
  WITH FOLLOW-UP**. `cargo test --test contract_shim_stdio_initialize` 5/5
  passed, including `shim_recovers_after_timed_out_daemon_later_becomes_ready`
  (direct regression proof) and
  `shim_aborts_unresolved_startup_after_client_disconnects`. CLI smoke
  (`engram.exe --help`) returned the expected command surface.
* `docs/closure/130-S-2026-08-27-post-merge-closure.md` — verdict **READY
  WITH CONDITIONS**: shipped code is production-ready and verified; the
  backlog shipment record cannot be formally archived until `137.006-T` is
  completed, re-scoped by Stage, or an operator authorizes a force-gate
  override. Contains full invariants, rollback trigger/procedure,
  monitoring plan, and validation window (references the RCA rather than
  duplicating it).

## Root/main preservation (proof)

Root worktree (`C:\Source\GitHub\engram`) was never touched during this
session. Verified unchanged before and after all work:

* `git rev-parse HEAD` in root: `19ae3160b7040e16213eba9ef7611f6573d3f4cd`
  (unchanged throughout — the merge landed on `origin/main` via GitHub, not
  via any local push/checkout in root).
* `git status --short` in root still shows the same pre-existing conditions
  untouched: `UU .backlogit/stash.jsonl` (unresolved conflict, not edited,
  staged, or deleted) and `M .gitignore` (unrelated staged modification, not
  altered, unstaged, or committed).
* All work this session — merge-gate rechecks, the merge itself (via GitHub
  API, not a local push), backlog reconciliation, and closure-artifact
  authoring — was performed exclusively in the pre-existing isolated
  worktree `.worktrees/ship-137-late-readiness-proxy-recovery-20260826` on
  branch `feat/137-late-readiness-proxy-recovery-verification`.
* No destructive cleanup was performed: no branch deletion, no worktree
  removal, per explicit operator instruction.

## Remaining work / handoff

1. **Stage**: triage `137.006-T` (terminal-vs-transient `check_health` error
   classification gap + missing single-flight concurrency contract test).
   Either schedule it for implementation (which will naturally complete the
   task and unblock `130-S` shipment closure) or formally re-scope/re-parent
   it away from `137-F` if it should not gate this shipment's closure.
2. **Ship (future session)**: once `137.006-T` is resolved one way or the
   other, re-run `backlogit shipment ship 130-S --sha
   2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0 ...` to complete backlog closure,
   then the closure-index resync.
3. **Closure artifacts durability**: this memory file plus the two
   `docs/closure/130-S-2026-08-27-*.md` files and the two
   `.backlogit/reconcile/130-S-*.md` reports were authored in the isolated
   worktree, which already has a clean working tree relative to its own
   branch tip except for these new files. Per repository policy, durable
   persistence of post-merge closure artifacts goes through a normal PR
   (no direct push to `main`), opened from this same worktree/branch or a
   fresh follow-up branch — **not yet opened as of this memory**; see the
   session's structured result for current status. Merge of any such
   closure PR requires separate explicit operator approval; the approval
   already given covers only PR #364.
