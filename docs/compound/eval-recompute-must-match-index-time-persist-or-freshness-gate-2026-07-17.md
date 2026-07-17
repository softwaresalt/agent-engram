---
title: "An eval-time recompute that must match index-time state is unsafe unless every input is persisted or freshness-gated"
description: "The resolution-aware recall denominator (091.020-T) collapses two call-site spellings into one unit only when they share a resolved edge, but eval recomputed the canonical-resolution context (crates, unsafe_module_prefixes, per-file use-graph/module-path) from CURRENT disk. Any input that diverged from the index-time context under a stale index could re-open the collapse onto a coincidental edge and over-report recall. Four adversarial passes each found ONE more live-recomputed input; persisting inputs one at a time was whack-a-mole. The terminating design persists the workspace-global inputs as one snapshot and freshness-gates the per-file source-derived input, after which no live-recomputed resolution input remains."
problem_type: "eval_metric_over_report_from_index_time_context_divergence"
category: "correctness-invariant"
component: "src/services/retrieval_eval.rs resolution_recall denominator / src/services/code_graph.rs canonical snapshot persistence"
root_cause: "an evaluation metric recomputed a derived context at eval time and compared it against artifacts (resolved_edges) produced at index time; whenever the index was stale relative to disk, the two contexts could diverge and the metric could collapse a genuine miss onto a coincidental edge (over-report)"
resolution_type: "persist_workspace_global_inputs_as_one_snapshot_plus_per_file_freshness_gate"
date: "2026-07-17"
shipment: "091-F"
---

# An eval-time recompute that must match index-time state is unsafe unless every input is persisted or freshness-gated

## Problem

The canonical-resolution recall metric (`resolution_recall`) divides resolved
edges by a call-site denominator. To avoid double-counting two spellings of the
same call (e.g. `helper()` and `module::helper()`), the denominator collapses
them into one unit when both resolve to the same `(caller_id, target_id)` edge.

The load-bearing invariant is that the denominator may UNDER-report (treat two
spellings as distinct when they are the same, safe) but must NEVER OVER-report
(collapse a genuine index-time MISS onto a coincidental edge, inflating recall
toward a false 1.0).

Over-report is only reachable when eval's canonical-resolution context DIVERGES
from the index-time context that produced `resolved_edges`. Canonical resolution
has multiple inputs, and eval recomputed all of them from CURRENT disk:

* workspace crate set (from live `Cargo.toml` manifests)
* `unsafe_module_prefixes` (from a live file-discovery walk)
* per-file `UseGraph` + `ModulePath` (reparsed from live caller source)

With a stale index (code changed on disk after the last index/sync), any of
these could shrink or shift so that eval stopped bailing on a qualified call the
indexer had rejected. If that target was independently reachable via a
coincidental singleton/direct edge from the same caller, eval collapsed the
genuine miss and over-reported.

## Why one-input-at-a-time fixing failed

Five adversarial passes (GPT-5.6 Sol @ xhigh, the operator-designated key
reviewer) each found exactly ONE more live-recomputed input after the previous
one was persisted:

1. prefixes rebuilt from the indexed-file subset -> reuse production discovery
2. shared helper still recomputes from current disk -> persist prefixes snapshot
3. crates still rebuilt from live manifests -> persist crates too
4. per-file use-graph still reparsed from live source -> freshness gate

Persisting inputs individually was whack-a-mole: each fix closed one drift
vector and revealed the next, because "the canonical context" is a SET of
index-time inputs, not a single value.

## Lessons

- **If an eval/metric recomputes a derived value and compares it against an
  artifact produced earlier (at index time), treat EVERY input to that recompute
  as a potential divergence vector.** Enumerate the full input set up front
  rather than patching inputs as reviewers surface them.
- **Two closure strategies, applied by input shape:**
  - *Workspace-global inputs* (crates, prefixes): persist the WHOLE context as
    one snapshot at index time; the eval loads it and does zero live-disk reads.
    Gate the collapse on snapshot presence; absent -> degrade to the safe
    direction (syntax-only). Write the snapshot only on a successful full index
    or a no-drift incremental sync (delete-at-start + write-at-success) so a
    partial/failed run leaves it absent.
  - *Per-file source-derived inputs* (use-graph, module-path): you cannot cheaply
    persist per-file parse state, so FRESHNESS-GATE instead. Reuse the existing
    per-file content-hash staleness check (`is_index_stale(source,
    recorded_hash)`); only emit the collapse key when the file is fresh
    (`!recorded_hash.is_empty() && !is_index_stale(...)`); stale or unknown-hash
    files degrade to the safe direction.
- **"Fail-closed" must mean fail toward the SAFE direction of the invariant, not
  just "return None on error."** Here every degradation path (absent snapshot,
  empty hash, stale file, deserialize error) lands on syntax-only counting, which
  under-reports but never over-reports.
- **A same-caller coincidental edge is a real over-report vector, not a
  theoretical one.** The backstop `resolved_edges.contains(...)` blocks most
  paths, but a compound case (stale input + a coincidental singleton edge to the
  same target) slips through. Adversarial review that hunts specifically for the
  compound case is worth more than another broad pass.
- **Keep the numerator byte-unchanged.** The entire remediation touched only the
  denominator/eval path; the indexed edge set (the numerator) was never modified,
  so the blast radius stayed confined to the one eval metric.

## Fix

Persist the whole index-time `CanonicalWorkspace { crates, unsafe_prefixes }` as
one JSON snapshot in a branch-scoped Cozo relation
(`index_canonical_workspace_snapshot`), written on successful full index and
no-drift incremental sync; the eval builds its entire CanonicalWorkspace from the
snapshot (no live-disk reads) and disables collapse when it is absent. Add a
per-file freshness gate in `accumulate_call_sites` so the per-file use-graph is
only trusted when the caller file's content hash matches the index-time hash.
After both, no live-recomputed resolution input remains, so the denominator
provably never over-reports. Merged in PR #265 (merge commit ce3872a).
