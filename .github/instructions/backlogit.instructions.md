---
description: "Backlogit workflow rules for query-driven lookup, explicit dependency wiring, checkpoints, and execution traceability"
applyTo: '**'
---

# Backlogit Instructions

Use these rules when the workspace enabled the `backlogit` capability pack. This pack deepens the
generic backlog integration with backlogit-native query, queue, dependency, continuity, and
traceability workflows.

## Required Tool Surface

The workspace should expose a backlogit-style tool surface for these behaviors when the registry
advertises them:

* **query / SQL lookup** — retrieve targeted backlog state without scanning many markdown files
* **queue view** — list ready or grouped work in execution order
* **dependency operations** — create, remove, and inspect explicit work dependencies
* **memory / checkpoint** — persist concise agent continuity state between sessions or phases
* **comments** — append operator- or agent-visible execution notes to a task
* **commit tracking** — associate commits with task IDs for traceability
* **sync / rehydrate** — refresh the query index after out-of-band edits

Use the workspace's registered backlogit operation names or aliases. Do not invent a parallel task
tracking system when backlogit is available.

## Query-First Protocol

When inspecting backlog state:

1. Prefer targeted query operations over reading many `.backlogit/` markdown files directly.
2. Use direct item retrieval for current-state lookups.
3. Fall back to file reads only when the query surface cannot answer the question.

The goal is token-efficient lookup, not ritual compliance.

## Queue and Dependency Protocol

When selecting work or establishing execution order:

1. Prefer queue-aware operations for ready-work selection when supported.
2. Use explicit dependency operations to encode task ordering that truly matters.
3. Do not hide critical sequencing only in prose when the dependency graph can represent it.
4. Re-check unfinished dependencies before claiming a task that appears ready.

## Intercom Coherence Rule

When the `backlogit` and `agent-intercom` capability packs are both enabled and
an agent is presenting queue, stash, or triage choices remotely:

1. Include item IDs, priority, kind or type, and a one-line summary in the
   broadcast.
2. Include the recommended ordering and the exact choice being requested.
3. Prefer self-contained broadcasts over "see chat above" summaries.

## Continuity Protocol

At meaningful boundaries such as task completion, review handoff, or session end:

1. Write the normal markdown memory artifact required by the harness.
2. When memory or checkpoint operations are supported, also persist a concise structured summary through backlogit.
3. Summaries should capture outcome, changed files or surfaces, decisions, blockers, and next steps.
4. Do not dump raw transcript logs into backlogit memory fields.

## Traceability Protocol

When work changes backlog state materially:

1. Append concise comments for notable outcomes, blocked conditions, or handoff notes when supported.
2. Associate commits with task IDs when commit-tracking is supported.
3. Keep comments focused on operationally relevant facts rather than verbose narration.

## Index Freshness Rule

If `.backlogit/` content was edited outside the usual backlogit mutation flow, refresh the index
before relying on query or queue output. Treat stale index results as suspect until rehydration completes.

## Shipment Reconciliation

`backlogit_ship_shipment` MUST NOT be called without first invoking the
`shipment-reconcile` skill with `mode: pre`. This is a mandatory gate, not
an optional enhancement. See `.github/skills/shipment-reconcile/SKILL.md`.

### Required invocation pattern in Ship Step 6

```text
1. Invoke shipment-reconcile mode: pre, expected_status: done
   → If RECONCILE_FAIL: halt, prompt operator, do NOT proceed to step 2
   → If PROCEED: continue
2. Call backlogit_ship_shipment(shipment_id, merge_sha)
3. Run git restore .backlogit/archive/ (always, known deletion quirk)
4. Invoke shipment-reconcile mode: post
   → If HALT: restore archives before committing
5. Commit
```

### Required invocation pattern in Ship Step 0.5 (intake)

```text
Invoke shipment-reconcile mode: pre, expected_status: queued
→ Catches Stage-side over-inclusion before build work begins
→ RECONCILE_FAIL at intake means Stage swept in items outside the
  current harvest scope — reconcile the manifest before claiming work
```

### Why this is required

`backlogit_ship_shipment` does not validate per-item completion state before
archiving. It will archive items that are still `queued`, `active`, or even
`missing` from disk. The `shipment-reconcile` skill provides the compensating
GI/GR double-entry check the tool itself lacks (upstream issue documented at
`docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md`).

## Data Ownership Rule

Treat backlogit's markdown files as the current-state source of truth, its query index as a
disposable cache, and its event or telemetry streams as append-only tool-managed history. Do not
edit generated cache artifacts directly.

Generated by autoharness | Template: backlogit.instructions.md.tmpl