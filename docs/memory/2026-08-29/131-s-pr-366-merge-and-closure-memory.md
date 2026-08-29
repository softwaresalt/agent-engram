---
title: "131-S / PR #366 merge and post-merge closure"
date: 2026-08-29
doc_type: memory
agent: ship
shipment_id: "131-S"
feature_id: "138-F"
pr: 366
merge_commit: dd0ba6116a39c54f8c25ff033c72211041b2a65f
---

## Completed Work

- Recovered the originally dirty/conflicted primary checkout without loss.
- Preserved the exact primary snapshot on pushed branch
  `rescue/primary-pre-sync-20260828` at
  `9ea2938d66c3052d145d6c5c0c6a03a2f348afe0`.
- Preserved raw `.backlogit/stash.jsonl` conflict forms as pushed tags
  `rescue-primary-stash-stage2-20260828` and
  `rescue-primary-stash-stage3-20260828`.
- Preserved Stage planning work on pushed branch
  `rescue/stage-138-planning-worktree-20260828` at trace tip
  `78b0b379a127ae4ee847ac299bab4e5a2e4cdaab`.
- Preserved a later primary backlog-link change on
  `rescue/primary-backlog-138-link-20260829` at
  `37a589bb2b231d1361725d59dbbab07442025c13`.
- Removed two large worktrees only after explicit approval and pushed
  preservation, recovering roughly 67 GiB.
- Remediated all valid Copilot findings on PR #366 with focused TDD, replied
  after pushed commits, resolved matched bot threads through fresh GraphQL
  lookups, and reached an exact-head clean review gate.
- Merged PR #366 only after explicit approval, using a merge commit.
- Fast-forwarded primary `main`, reconciled shipment `131-S`, regenerated
  unavailable ignored gate evidence without force, and archived the full
  release unit.

## Merge Proof

- Approved head: `19ac3bbf290652ae9300482db3813e222e8e3faa`
- Merge commit: `dd0ba6116a39c54f8c25ff033c72211041b2a65f`
- Merge parents: `06c1baa89c5b9a7b5222079c270d94328a3476fd`
  and `19ac3bbf290652ae9300482db3813e222e8e3faa`
- Merge tree and approved-head tree:
  `a31ecc31fd32e0d914b1b813a7741ac4cda4447d`
- PR state: `MERGED` at `2026-08-29T21:05:38Z`

## Review Remediation Commits

| Commit | Purpose |
|---|---|
| `bfb2c6de` | Preserve EOF versus response-cap diagnostics and health metadata |
| `aa73e69c` | Restore shim public API compatibility |
| `14c9ec0d` | Suppress a rejected readiness event |
| `a49ba52d` | Align final review narratives |
| `beb014b3` | Reconcile a transient readiness race |
| `19ac3bbf` | Cover persistent startup mismatch across a respawn |

The final exact-head Copilot review was `5059143701`; all 25 review threads
were resolved, requested reviewers were empty, and both required CI checks
were green before merge.

## Shipment Closure

`backlogit shipment ship 131-S` initially refused closure because ignored
local completion-gate logs were absent for `138.008-T`. The retained PR
worktree also lacked authentic logs, and a no-op `done -> done` transition did
not regenerate them.

The supported fallback was applied to `138.008-T` through `138.014-T`:
`done -> active -> done`. Every normal gate passed at merge SHA `dd0ba611...`
with report hash
`8ac746799d4f1672dc8d27729601a8a904e1db8002cd79148549cf16c5f6bbea`.
No `--force-gates` option was used.

Final state:

- `131-S`, `138-F`, and `138.001-T` through `138.014-T`: archived
- associated reviews `138.001-R` and `138.002-R`: archived
- returned items: none
- active shipments: none
- advisory lock: released
- scope-specific doctor findings: zero

## Validation

- Exact-head CI: build and Windows launcher checks passed.
- Full pre-merge runtime and quality evidence:
  `docs/closure/2026-08-27-131-s-terminal-vs-transient-health-classification-runtime-verification.md`
- Post-merge: `cargo check --locked` passed with
  `CARGO_TARGET_DIR=C:\Source\GitHub\engram\target`.
- Archive deletion guard: zero deleted archive paths.
- Backlogit shipment, feature, member, and active-shipment reads succeeded.

## Decisions and Failed Approaches

- Never discard a conflicted index to synchronize `main`; preserve exact stage
  forms and a coherent rescue snapshot first.
- Never merge or remove worktrees before explicit approval.
- Do not force shipment gates. Port authentic ignored logs when available;
  otherwise perform a real supported state transition.
- A no-op terminal-state transition does not invoke the completion broker.
- Existing compound guidance covered this failure mode and was updated with
  the no-op-transition detail rather than creating a duplicate learning.
- Engram startup diagnostics remained bounded and degraded; no daemon process
  or runtime state was manipulated.

## Compact-Context Assessment

The mandatory batch-completion assessment covered `docs/memory/`,
`docs/exec-plans/`, and `docs/closure/`. Repository-wide counts exceed the
manual size thresholds, but unrelated historical compaction is outside this
release-unit closure. The two `138-F` Stage memories are recent, distinct
phase records and remain the latest checkpoints for their decisions. The
revision-2 plan is already a resolved replacement rather than a raw appended
review transcript, and the runtime closure is less than 14 days old.

Result: **no files compacted, zero bytes moved, active/recent traceability
preserved**. A future dedicated documentation-maintenance shipment may assess
the broader historical corpus without coupling unrelated moves to this
closure PR.

## Remaining Handoff

1. Commit and push `chore/131-s-post-merge-closure`.
2. Open a protected-main closure PR; do not merge it without separate operator
   approval.
3. Leave the primary checkout clean on synchronized `main`.
4. During the first release containing `dd0ba611...`, the release operator
   owns the observation window documented in the post-merge closure artifact.
5. Preserve rescue refs, the clean PR review worktree, and shared caches unless
   separately approved for cleanup.
