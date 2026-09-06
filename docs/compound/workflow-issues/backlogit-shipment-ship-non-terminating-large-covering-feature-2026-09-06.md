---
title: "backlogit shipment ship does not complete within a bounded observation window against a large covering-feature roster"
description: "backlogit shipment ship (v1.10.1) did not return within any of three observation windows (up to ~5.5 minutes) for a shipment whose manifest excludes a large covering feature (59-unit roster); CPU climbs steadily with zero WAL growth, ruling out lock contention, but true indefinite non-termination is unconfirmed — only a timeout/performance symptom is established"
problem_type: "shipment closure timeout/performance symptom (root cause unconfirmed)"
category: "workflow-issues"
component: "backlogit shipment closure"
root_cause: "UNCONFIRMED. The only established fact is that backlogit shipment ship did not complete within an observed 5.5+ minute window, with steadily climbing CPU and zero WAL/database growth (ruling out lock contention and ruling out slow-but-real *persisted write* progress — the CPU activity shows computation could still be occurring even though no database write resulted from it). A plausible hypothesis is an unbounded or exponential traversal of the covering feature's full descendant tree (possibly as part of a protected-set/provenance proof) when the manifest is a small subset of a large covering feature's roster, but this has not been confirmed via profiling or a completed control run; treat as a timeout/performance symptom, not a proven algorithmic defect, until such evidence exists"
resolution_type: "workaround"
severity: "high"
message: "backlogit shipment ship 135-S never returned; CPU climbed from 0 to 211s+ over 5.5 minutes wall time with the WAL file size unchanged throughout, confirming no forward write progress"
file_path: ".backlogit/archive/135-S.md"
citations:
  - "docs/closure/2026-09-05-135-s-operational-closure.md"
  - ".backlogit/archive/135-S.md"
  - ".backlogit/reconcile/135-S-pre-20260906-110752.md"
  - ".backlogit/reconcile/135-S-post-20260906-113100.md"
tags:
  - "backlogit"
  - "shipment"
  - "closure"
  - "workflow"
  - "performance"
---

## Problem

During post-merge closure for shipment `135-S` (manifest: 4 tasks, all
already individually `done`/archived), `backlogit shipment ship 135-S --sha
... --message ... --author ...` (backlogit v1.10.1-0.20260823032255) was
invoked three times and never returned within the observation window:

* Attempt 1: killed after ~4 minutes with no log output beyond `workspace
  initialized`.
* Attempt 2 (with `--no-update-check --log-level debug`, stdin piped `y`):
  killed after ~3 minutes, same symptom.
* Attempt 3 (after terminating six long-stale orphaned `backlogit mcp`
  server processes dated 2026-09-01 through 2026-09-05, ruling out lock
  contention): killed after ~5.5 minutes. `Get-Process` samples showed CPU
  time climbing steadily and continuously (66s → 139s → 211s across three
  ~30-90s polling intervals) while `Responding: True` throughout — i.e. the
  process was actively burning CPU, not deadlocked at the OS level. Critically,
  `.backlogit/backlogit.db-wal` remained at a **fixed byte size** across the
  entire window, proving the process was making **zero forward database-write
  progress** despite the CPU consumption.

The generic fallback (`backlogit move 135-S --status shipped`) is also no
longer available: the CLI now hard-rejects it (`Error: shipment must be
shipped via ShipShipment, not a direct status update`), leaving `shipment
ship` as the only registered closure path — and that path does not
terminate for this shipment.

## Suspected Root Cause

135-S's manifest (4 tasks) is a small subset of its covering feature
`142-F`'s full roster (59 units: F00, F01–F55, F04a, F12a, F16a, per `142-F`'s
own description). This is the same shape of workspace that motivated the
Ship-agent P-015 exception logic (a shipment manifest that does NOT fully
cover its parent feature's descendant set must use safe-close, never the
cascade `shipment ship` path) — but here the tool itself, not just the
policy layer, may be attempting some form of full-tree evaluation (possibly
the "protected set" / provenance proof described in the Ship agent template's
P-015 exception commentary) that scales poorly against a large roster. This
hypothesis is unconfirmed: the only established fact is that the command
did not complete within any of three observed windows (up to ~5.5 minutes).

This is consistent with, but distinct from, the previously documented
`shipment ship` defects:

* `backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md` —
  force-releases the covering feature (a *correctness* bug)
* `ship-shipment-no-item-archive-files-2026-04-23.md` — omits per-item
  archive files (a *completeness* bug)
* `ship-shipment-overscoped-manifest-2026-04-20.md` — archives unbuilt items
  (a *correctness* bug)

This entry documents a fourth, distinct symptom: a **timeout/performance
symptom** — the command exceeding a bounded observation budget with no
forward write progress — whose root cause (possibly a *liveness* bug) is
unconfirmed pending profiling or a completed control run, specifically
observed against a large covering-feature roster disjoint from a small
manifest.

## Workaround (proven, 135-S)

When `backlogit shipment ship {id}` does not return within a bounded budget
(observed: 5+ minutes with zero WAL growth) for a shipment whose covering
feature has a large roster not fully covered by the manifest:

1. Confirm via `Get-Process -Id {pid} | Select CPU` sampled twice, a minute
   apart, that CPU time is climbing (ruling out a simple hang/deadlock) —
   and confirm via `Get-Item .backlogit/backlogit.db-wal | Select Length`
   sampled the same way that the WAL size is **not** growing (ruling out
   slow-but-real write progress). Both together establish CPU-intensive
   work with no database-write progress during the observed window — they
   do not, by themselves, prove indefinite non-termination as opposed to
   bounded-but-extreme slowness (computation can proceed for a long time
   before producing a write). Treat this as a strong signal to apply the
   workaround below regardless of which explanation turns out to be true.
2. Terminate the process by PID (`Stop-Process -Id {pid} -Force`). Verify no
   partial state was written: the shipment's `status` (via `backlogit get
   {id}`) is unchanged, and `.backlogit/archive/{id}.md` does not exist.
3. Confirm every manifest item is already individually archived (pre-mode
   reconciliation, `pre-archived` classification for all items — this is
   the normal state for a shipment whose tasks completed incrementally
   during the build loop).
4. Manually author `.backlogit/archive/{shipment_id}.md` with
   `archived_from: .backlogit/queue/{shipment_id}.md`, `archived_status:
   done`, `status: archived`, and the original frontmatter (`custom_fields.items`,
   `dependencies`, `id`, `priority`, `title`, `created_at`) preserved verbatim
   from the queue file, plus a fresh `updated_at` and an `AUDIT RATIONALE`
   description block recording the manual-safe-close justification, the
   merge SHA/PR/method, and this tool defect. This mirrors the pre-existing
   134-S manual safe-close precedent (see `.backlogit/archive/134-S.md`),
   which used the identical pattern for the identical P-015 reason (shared
   large covering feature) — this defect simply makes the manual path
   mandatory rather than merely policy-preferred.
5. Delete `.backlogit/queue/{shipment_id}.md`.
6. Run `backlogit sync` to rebuild the index. Verify via `backlogit get
   {shipment_id}` that `status: archived` / `archived_status: done` now
   read correctly.
7. Verify the covering feature (`backlogit get {feature_id}`) is untouched
   (`status` unchanged from its pre-attempt value) — compare a snapshot of
   its queue file taken before the first `shipment ship` attempt against its
   current content, byte for byte.
8. Run post-mode reconciliation (archive presence for the shipment + every
   manifest item; `git status -- ".backlogit/archive/"` shows no deletions,
   P-007).

## Prevention / Escalation

* Treat `backlogit shipment ship` as **potentially non-terminating**, not
  merely potentially incorrect, whenever the covering feature's roster is
  large and disjoint from the manifest. The existing P-015 policy guard
  (prefer safe-close over cascade `shipment ship` in exactly this shape of
  workspace) already steers agents away from the cascade path for the
  *correctness* reasons above — this finding adds a *liveness* reason that
  applies even when an agent is only trying to close a **non-cascading**
  shipment record, because `shipment ship` is currently the sole registered
  CLI operation capable of transitioning a shipment to `shipped`/`archived`
  status at all (the generic `move` path is explicitly blocked for
  shipments).
* Recommend a bounded timeout (e.g., 60–90s) around any `backlogit shipment
  ship` invocation in agent tooling, with automatic fallback to the manual
  safe-close procedure above on timeout, rather than an agent needing to
  discover this ad hoc. This is offered as a pragmatic mitigation for the
  observed timeout/performance symptom, not as confirmation of the traversal
  hypothesis above; the underlying algorithmic cause remains unconfirmed
  pending profiling or a completed control run.
* Upstream issue candidate: `backlogit shipment ship` needs either (a) a
  documented, bounded-complexity provenance-proof algorithm that does not
  scale with the full covering-feature roster size, or (b) an explicit,
  fast-path, non-cascading "close shipment record only" CLI verb that
  performs exactly the `move --status shipped` → `archive` sequence the
  Ship agent template already describes at the policy layer, without
  requiring any covering-feature roster traversal at all. Stashed as a
  follow-up for Stage triage (this repository) alongside the existing
  `DA0AF326` validator-manifest-drift follow-up, since both point at
  `backlogit` tooling/config drift outside 135-S's owned-file scope.

## Result

135-S closed cleanly via the manual workaround above: shipment `135-S`
`status: archived` / `archived_status: done`; all 4 manifest items and the
shipment record itself verified present in `.backlogit/archive/`; covering
feature `142-F` verified untouched (still `active`, byte-identical to its
pre-attempt snapshot); no cascade, no force-release, no orphaned/detached
descendants.
