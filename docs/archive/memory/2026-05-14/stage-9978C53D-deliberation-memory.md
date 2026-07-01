---
session: stage-agent
date: 2026-05-14
phase: stash-triage
---

# Stage Session — Stash 9978C53D Triage

## Tasks Completed

* Triaged stash entry `9978C53D` (high priority, unknown kind)
* Created deliberation artifact `008-D` at `.backlogit/queue/008-D.md`
* Updated `.backlogit/.stash.md` with harvest provenance comment

## Decision Rationale

**Classification: deliberation (not spike, not feature, not task)**

The stash text says "research whether" — feasibility and correctness
questions must be answered before implementation. At least four
implementation approaches exist with different tradeoffs (copy-and-sync,
git-diff seeding, status-quo + deletion audit, shared read replica).
This is a deliberation, not a ready-to-implement feature.

## Key Findings from Research

1. Engram already creates per-branch CozoDB DBs at
   `.engram/cozo/{branch}/engram.db` — the infrastructure exists.
2. `sync_workspace` is already incremental (content-hash-based) but the
   **first sync on a new branch is always a full index** because the DB
   starts empty with no stored hashes.
3. `.engram/cozo/` shows ~15 named branch subdirectories — branch
   proliferation is real and the cost compounds.
4. Critical open question: does `sync_workspace` correctly **delete**
   symbols for files removed on the branch? If not, Option A (copy from
   main) risks importing stale symbols. This must be answered first.

## Files Modified

* `.backlogit/queue/008-D.md` — created (deliberation artifact)
* `.backlogit/.stash.md` — updated (harvest provenance comment)

## Next Steps

1. Operator reviews `008-D` and resolves the open questions (especially
   deletion correctness in `src/services/code_graph.rs`).
2. Once a direction is chosen, Stage creates an impl-plan and harvests
   feature/task(s) for Ship.
3. Recommended first investigation: audit `sync_workspace` deletion path
   before committing to Option A.

## Artifacts

| ID | Type | Status | Title |
|---|---|---|---|
| `008-D` | deliberation | queued | Branch DB seeding — eliminate full re-index on first branch sync |
