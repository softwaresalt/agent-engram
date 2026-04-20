---
name: shipment-reconcile
description: "GI/GR reconciliation gate for shipment manifests — verifies every manifest item exists in queue (pre-mode) or archive (post-mode) with the expected status before or after backlogit_ship_shipment runs."
---

# Shipment Reconcile

Provides a double-entry (GI/GR) integrity check for shipment manifests. Run
`mode: pre` before calling `backlogit_ship_shipment` and `mode: post` after
the archive + restore steps complete.

## When to Use

* **Ship Step 6** (mandatory): pre-mode immediately before `backlogit_ship_shipment`;
  post-mode immediately after the `git restore .backlogit/archive/` step.
* **Ship Step 0.5** (sanity check): pre-mode at intake with `expected_status: queued`
  to catch Stage-side over-inclusion before any build work begins.
* **Ad-hoc audit**: any time an operator suspects manifest drift.

## Inputs

| Parameter | Required | Values | Notes |
|---|---|---|---|
| `mode` | yes | `pre` \| `post` | Controls which check phase runs |
| `shipment_id` | yes | e.g. `004-S` | The shipment to reconcile |
| `expected_status` | pre-mode only | `queued` \| `done` | `queued` for intake checks; `done` for pre-ship checks |
| `merge_commit_sha` | post-mode only | git SHA | The merge commit that closed the PR |

## Output

A structured **reconciliation report** (see
`docs/exec-plans/2026-04-20-shipment-reconcile-schema.md`) stored at
`.backlogit/reconcile/{shipment_id}-{mode}-{timestamp}.md`.

Every item in the manifest is classified as one of:

| Classification | Meaning |
|---|---|
| `matched` | Queue/archive file present AND status matches `expected_status` |
| `missing` | No queue or archive file found for this manifest item |
| `status-mismatch` | File present but declared status does not match `expected_status` |
| `orphan` | Queue file declares this `shipment_id` in its frontmatter but is NOT in the manifest |

The report ends with a `recommendation`:

* `PROCEED` — all items matched; no action needed
* `HALT — operator reconcile required` — one or more missing, status-mismatch, or orphan items

## Behavioral Constraints

* **Report-and-halt only.** This skill NEVER modifies the shipment manifest, queue
  files, or archive contents. Operator must manually reconcile via existing
  `backlogit_*` tools and re-invoke Ship Step 6.
* **No prune mode in v1.** Auto-mutation of the manifest is reserved for a future
  version after upstream `backlogit` validation surface is confirmed.
* **Single-writer lock.** When invoked from Ship Step 6, this skill holds the
  `.backlogit/queue/{shipment_id}.md` file lock (via the `file-lock` skill) for
  the duration of pre-mode → post-mode. See lock protocol in the Required Protocol
  section below.
* **Halt on RECONCILE_FAIL.** Do not proceed to `backlogit_ship_shipment` unless
  pre-mode returns `PROCEED`. Surface the report path to the operator.

## Required Protocol

### Pre-Mode

1. **Acquire single-writer lock** (Ship Step 6 invocations only, not intake):
   Invoke the `file-lock` skill to acquire `.backlogit/queue/{shipment_id}.md`.
   If lock acquisition fails, count as a session stall (circuit-breaker protocol)
   and prompt the operator.

2. **Load manifest** via `backlogit_get_shipment(shipment_id)`.
   Extract the `items` list.

3. **Check each manifest item**:
   * Attempt to locate the file at `.backlogit/queue/{id}.*`
   * Read its frontmatter and compare `status` to `expected_status`
   * Classify as `matched`, `missing`, or `status-mismatch`

4. **Orphan scan**:
   Scan `.backlogit/queue/` for any files whose YAML frontmatter declares
   `shipment_id: {shipment_id}` but whose ID is NOT present in the manifest `items` list.
   Classify each such file as `orphan`.

5. **Produce report** per the schema in
   `docs/exec-plans/2026-04-20-shipment-reconcile-schema.md`.

6. **Gate decision**:
   * If all items are `matched` and no orphans exist → `recommendation: PROCEED`
   * If any `missing`, `status-mismatch`, or `orphan` items exist →
     `recommendation: HALT — operator reconcile required`
   * On `HALT`: emit the report path, release the lock, and halt with
     `RECONCILE_FAIL`. Do NOT call `backlogit_ship_shipment`.
   * On `PROCEED` from Ship Step 6: retain the lock until post-mode completes.

### Post-Mode

1. **Verify archive presence**:
   List `.backlogit/archive/` and confirm a file exists for the shipment itself
   (`{shipment_id}.*`).

2. **Per-item archive check**:
   For every item in the manifest, verify a corresponding archive file exists.
   If any are absent, flag them in the report.

3. **Deleted-file guard** (known `backlogit_ship_shipment` quirk):
   Run `git status -- ".backlogit/archive/"` and inspect for deletions.
   If any archive files are reported as deleted, recommend
   `git restore .backlogit/archive/` before the commit step.

4. **Produce post-mode report** per the schema.

5. **Gate decision**:
   * If all archive files present and no deletions detected → `recommendation: PROCEED`
   * If missing archive files or unrestored deletions detected →
     `recommendation: HALT — restore archives`
   * On `HALT`: release the lock and report. Ship must restore archives before committing.

6. **Release lock** (acquired in step 1 of pre-mode):
   Invoke `file-lock` release for `.backlogit/queue/{shipment_id}.S.md`.
   If release fails, log a warning — stale locks are operator-recoverable.

### Lock-Conflict Scenario

If pre-mode cannot acquire the lock because another process holds it:

1. Retry once after 30 seconds.
2. If retry also fails, count as a session stall and prompt the operator:
   `Shipment lock conflict on {shipment_id}.S.md. Another process holds the lock.`
3. Do NOT proceed without the lock. Do NOT call `backlogit_ship_shipment`.

Example lock-conflict transcript:

```
[RECONCILE] Acquiring lock: .backlogit/queue/004-S.S.md
[RECONCILE] Lock acquisition failed — another process holds the lock.
[RECONCILE] Waiting 30 seconds and retrying...
[RECONCILE] Retry failed. Session stall recorded. Prompting operator.
RECONCILE_FAIL: lock conflict on 004-S.S.md — another agent or terminal session
may be running Ship for this shipment. Resolve manually and re-invoke Ship Step 6.
```

## Quality Criteria

* `mode: pre` runs before every `backlogit_ship_shipment` call in Ship Step 6
* `mode: pre` with `expected_status: queued` runs at Ship Step 0.5 intake
* `mode: post` runs after every archive + restore sequence in Ship Step 6
* All four item classifications are represented in the schema
* Lock is acquired before pre-mode and released after post-mode (or on any halt)
* Report-and-halt is the only mutation path; no auto-prune

## Related Artifacts

* `docs/exec-plans/2026-04-20-shipment-integrity-plan.md` — implementation plan
* `docs/exec-plans/2026-04-20-shipment-reconcile-schema.md` — report schema
* `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` — incident root cause
* `.github/skills/file-lock/SKILL.md` — lock acquisition/release primitives
* `.github/agents/ship.agent.md` — integration points (Step 0.5, Step 6)
* `.github/agents/stage.agent.md` — scope guard (Step 5.5/3)

Generated by autoharness | Template: skill/SKILL.md.tmpl
