---
title: Stage session handoff - checkpoint-resolution program lock
type: session-memory
doc_type: memory
date: 2026-09-14
agent: stage
session: stage-checkpoint-resolution-program-lock-2026-09-14
outcome: program lock staged pending operator-owned publication
branch: chore/checkpoint-resolution-program-lock
---

## Scope of this file

This file records **session-only facts** and **pointers**. It is not an
authority for program structure and it is not an authority for current state.

| Question | Where the answer lives |
|---|---|
| What are the packages, boundaries, lifecycle terms, DAG, publication rules? | `docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` — immutable program structure authority |
| What is the current status of any package? What is workable next? | **The backlog.** Query it; do not read status from this file or from the decision document. |
| Where is the program discoverable from the backlog? | `026-D` — registry pointer only, not current-state authority |

Current-state query:

```text
backlogit query "SELECT id, status, title FROM items WHERE id IN
  ('027-D','028-D','029-D','030-D','031-D','032-D','033-D','034-D') ORDER BY id"
```

This file deliberately contains **no package status table, no cursor, and no
restated prohibition list**. Those were removed as duplicated mutable state.

## Immutable session facts

| Field | Value |
|---|---|
| Branch | `chore/checkpoint-resolution-program-lock` |
| Base | `origin/main` at `9ab53499f60a7afe3e215d10ee8c08a4278617b6` |
| Program lock commit | `38f4e3452bdd3f5b8efa2bde7f1bbd5fe9cfc1f1` |
| Program artifact | `docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` |
| Evidence branch (read-only) | `chore/checkpoint-resolution-ordering-restage` at `3b3edf05c4a89f61b20baeae2f2585960f3fcfc9` |
| Shipments created | none |
| Features / chores / tasks created | none |

Evidence paths cited by the decision document that exist **only** on the
evidence branch are branch-qualified there at `3b3edf05`. They do not resolve on
`main` or on this branch, and that is intentional.

## Backlog shape (structure, not status)

All program items are backlogit `deliberation` artifacts. `deliberation` is a
level-1 type with `allowed_children: []`, and the workspace has no `epic` type,
so the program is a flat set of level-1 items joined by explicit dependency
edges rather than a parent/child tree.

Structural facts fixed by this session:

* **Nine normative `blocks` edges** encode the locked DAG:
  `028<-027`, `029<-028`, `030<-029`, `031<-029`, `032<-030`, `032<-031`,
  `033<-031`, `033<-032`, `034<-033`. The `033<-031` (C -> D) edge is normative
  and directly retained.
* **No `relates_to` dependency edges to `026-D` exist.** The eight that were
  created in `38f4e345` were removed with `backlogit dep remove` and replaced
  with `related_to` **semantic links** (`backlogit link add`), which express
  association without implying a dependency.

A `blocks` edge is a sequencing signal only. It never proves `LANDED_COMPLETE`;
see the decision document's fail-closed dependency rule.

## Local state deliberately preserved

Two local files were preserved byte-for-byte and were **not** staged, committed,
reverted, or deleted. Hashes below were **recomputed read-only** during this
remediation and verified as 64-hex SHA-256 values matching current file content.

| Path | State | SHA-256 (recomputed, 64 hex) |
|---|---|---|
| `.backlogit/stash.jsonl` | modified, zero content change (LF/CRLF stat drift; `git diff --numstat` reports no changed lines) | `35b7c18e738f7b0523aec7adf8bfebf7477ec931f12eee377e7aa4ec3ffd5497` |
| `.backlogit/checkpoints/checkpoint-20260914-045836.json` | untracked, `status: resolved` | `f595dd8d5534c0d0e27abd676f7494d7d590c8071587203926591c95e723d417` |

The switch was proven safe in advance: `stash.jsonl` is byte-identical between
`HEAD` and `origin/main`, and the untracked checkpoint does not exist in
`origin/main`.

**The resolved untracked checkpoint is ignored by active-candidate recovery.**
Checkpoint recovery enumerates only checkpoints with `status: active`; a
`resolved` checkpoint is never an active candidate and never triggers a
resumption prompt. It is therefore left exactly where it is — **not deleted, not
archived, not staged**.

## Checkpoint decision

No new checkpoint was created. The program is gated on an operator-owned
publication action, not on interrupted work; the committed program lock plus
this file provide sufficient durable continuity. Creating an active checkpoint
would falsely signal an interrupted session to the recovery protocol.

## Review outcome

**Report-only review at current HEAD returned findings; it was not a clean
pass.** The findings were remediation-bearing and spanned authority separation,
lifecycle-term definition, publication gating, package eligibility, next-operation
scope, publication ownership, prohibition scope, and backlog edge correctness.

Those findings were remediated in a **subsequent** commit on this branch, not in
`38f4e345`. The earlier claim in this file that all P1 findings were remediated
before commit was inaccurate and has been corrected.

The SHA-256 length finding was re-checked directly: both recorded digests were
recomputed read-only from the preserved files and are valid 64-character
SHA-256 values matching current content. Both are restated above in normalized
lowercase hex with the recomputation recorded.

## Compound learnings cited for carry-forward

Cited as inputs to **later packages only**. None changes current program-lock
behaviour; see the decision document's carry-forward table for the mapping.

* `docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md` → Package A (`032-D`)
* `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md` → Package E (`034-D`) and the one-release-unit-per-package rule
* `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md` → Package C (`031-D`)

## Instruction for the next session

Start a **fresh** session. Do not resume this one.

1. Read the decision document first for structure; read the **backlog** for
   status. Do not infer status from either this file or the decision document.
2. The program lock is `staged-pending-publication`. Until its publication PR
   merges to `main`, every package item is `blocked` and no Stage package
   operation is authorized.
3. Publication of this branch is an **operator-owned manual GitHub action**. No
   agent role creates that PR. Agents may prepare title/body and verify
   readiness before creation and the PR after creation.
4. After the publication PR merges, an operator or authorized backlog owner
   **explicitly** transitions only `027-D` to `queued`. Nothing changes status
   automatically on merge.
