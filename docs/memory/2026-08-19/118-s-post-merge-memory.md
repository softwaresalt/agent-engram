---
title: "Shipment 118-S post-merge closure memory"
date: 2026-08-19
doc_type: memory
shipment_id: "118-S"
feature_id: "122-F"
mode: post-merge
status: completed
verdict: RECONCILED_ARCHIVED
merge_commit_sha: "08676d341d94fd97b9d7ea3ea30562e63c5c9bba"
pr: 344
worktree: "C:\\Source\\GitHub\\engram\\.worktrees\\ship-118s-worktree-safe-engram-mcp-startup"
branch: "post-merge/118-s-worktree-safe-engram-mcp-startup"
---

## Outcome

Shipment `118-S` is closed out as shipped and archived. The merge SHA was
confirmed, the explicit manifest of 10 members matched the archive, the archive
status guard reported no deletions, and the `.backlogit/.locks/.118-S.lock`
file was released. No source, test, workflow, or backlog-scope content was
changed during this post-ship closure pass.

## Checkpoint / Resume

Resume point: the existing post-merge worktree on branch
`post-merge/118-s-worktree-safe-engram-mcp-startup` with `HEAD`
`2d331948945aeec331ef4dbcf514b9107061017d`.

Checkpoint inputs already present when this pass started:

* `.backlogit/reconcile/118-S-pre-20260819T142700-0700.md`
* `.backlogit/archive/118-S.md`
* `.backlogit/archive/122-F.md`
* `.backlogit/archive/122.002-T.md` through `.backlogit/archive/122.010-T.md`

No new checkpoint artifact was required for this short post-ship documentation
pass.

## Test / review / PR / merge / runtime / archive

* PR `#344` merged at `08676d341d94fd97b9d7ea3ea30562e63c5c9bba`.
* Exact-head review and CI are already complete from the shipped feature.
* Runtime verification for the native linked worktree reported **PASS WITH
  FOLLOW-UP** and documented the bounded startup contract.
* `src/db/workspace.rs`, `tests/integration/cli_direct_test.rs`,
  `tests/contract/shim_lifecycle_test.rs`, and
  `tests/contract/start_launcher_test.rs` carry the merged source/test proof.
* Reconciliation finished with 10 of 10 explicit manifest members matched and
  zero archive deletions.

## Degraded Engram / index / intercom observations

During closure discovery, the globally installed Engram CLI still rejected the
linked worktree as not a Git repository root; that was the old installed binary
path, not the merged-binary runtime. The merged-binary runtime contracts passed.
Backlogit index sync remains blocked by 19 unrelated malformed historical
artifacts, so the index cannot be refreshed yet. Intercom was unavailable. The
durable runtime observation remains the important one: direct `engram sync
--direct` admission works for the linked worktree, but the CLI `--timeout` is
IPC-scoped and does not bound a long direct indexing job. That is why the
launcher owns the outer wall-clock budget and must fail open to Copilot if
Engram startup stalls.

## Follow-up stash IDs

The shipment leaves the four follow-up stashes intact for later Stage work:

* `568B257C` — retained capability-rooted metadata handles
* `22DF3329` — serve static MCP initialization before daemon readiness
* `C2413934` — restore canonical development-test coverage
* `DE460A88` — independent agent-visible catalog contract

## Compact-context assessment

No additional compaction candidate was safe to archive or prune beyond this
dense memory note. The two new closure files and this memory file are the
durable post-ship summary, so context compaction is a no-op for unrelated or
active-task artifacts.
