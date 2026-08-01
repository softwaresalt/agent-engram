# Deliberation — 091.019-T + 091.021-T canonical-resolver tail (defer)

- **Date:** 2026-07-31
- **Cycle:** Stage drain-closeout (branch `101-drain-closeout`)
- **Source tasks:** 091.019-T (task/low), 091.021-T (task/low); parent 091-F done
- **Subsystem:** `src/services/code_graph.rs` canonical call-resolution path
  (shared by both items)
- **Disposition:** DEFER both as future features. Both tasks parked to `blocked`;
  NOT scheduled into a shipment this cycle.

## Problem frame

Both tasks sit on the canonical call-resolution path in
`src/services/code_graph.rs`, where the fail-closed invariant (resolution may only
DROP edges, never mint a false one) governs the resolution stage. 091.019 stays
within that invariant — a missing facade edge, the safe direction. 091.021 does
NOT: its stale-`unsafe_prefixes` window is a narrow FALSE-edge TOCTOU at the
index-build stage that the resolution-stage invariant does not cover, bounded
instead by requiring a concurrent mid-index remap and self-healing on reindex.
Because they share the subsystem, we deliberate them together while keeping a
distinct recommendation for each.

### 091.019-T — apply the A4 `ReexportMap` before the canonical match

The A4 `ReexportMap` (`src/services/parsing/canonical/reexport.rs`) rewrites a
path through the longest matching `pub use` re-export prefix so a facade call
resolves to the definition's canonical identity. Verified on main: the type, its
`canonicalize` method, and its unit tests all exist, but the canonical module
(`mod.rs`) declares `pub mod reexport` WITHOUT re-exporting it, and NO production
consumer constructs or applies it. It is production-dead today. As a result Unit
B never rewrites a facade path (`crate::facade::helper`) to its defining path
(`crate::a::helper`) before matching against `function_meta.canonical_path`
(`function_ids_by_canonical_path` @ ~1035), so a `pub use` facade call produces
NO edge. This is a MISSING edge, never a false edge, and it self-heals on the
next full index once the map is wired in. It is single-reviewer, P2,
adversarial-only, and was deliberately left out of the 088-S cycle-8 false-edge
fix set.

### 091.021-T — single-snapshot source reuse and pre-pass/main-pass TOCTOU

091.016-T removed the blocking pre-pass reads and the duplicate canonical PARSE,
but the second file READ remains. The global unsafe-module pre-pass
(`unsafe_module_prepass` @ ~88, verified) reads every Rust file once, and the
main index/sync passes read the same paths again; the full-index post-pass
`reresolve_calls_edges_with_canonical_context` (@ ~970) re-parses each staged
file via `rust_ctx_for_staged_file` (@ ~720), a third parse of the same source.
Reusing a single per-file byte snapshot removes the second read and closes the
pre-existing pre-pass/main-pass TOCTOU on the global `unsafe_prefixes` set: a file
that gains a `#[path]` or `#[cfg]` mod remap between the two reads can otherwise
leave the prefix set stale. The trade-off is peak memory scaling with total
source, so the cache must be drained per file and bounded against the
max-file-size guard, and the sync path (changed-files-only) must scope the
snapshot to avoid holding all source. This is NOT purely performance: a stale
`unsafe_prefixes` set can fail to drop a now-unsafe canonical definition/target
(`code_graph.rs` gates on `is_under_unsafe_module_prefix` @ ~222 and ~709), so it
would emit a canonical edge that a fresh snapshot would suppress — a narrow
FALSE-edge window. It is bounded: it requires a file to gain a `#[path]`/`#[cfg]`
remap DURING an in-progress index pass (a concurrent-modification TOCTOU, not
steady state), and it self-heals on the next clean full index with no measured
occurrence — so the deferral is robustness hardening against a rare race, not an
active correctness bug. A `force_prepass_cache_miss` seam already exists
(@ ~1278, verified).

## Shared trade-offs

- The two items differ in risk direction. 091.019 is a MISSING-edge only (a
  `pub use` facade call produces no edge) — the safe direction, bounded by the
  fail-closed invariant. 091.021 is different: its stale-`unsafe_prefixes` TOCTOU
  is a narrow FALSE-edge window (a not-yet-dropped unsafe target under a mid-index
  remap race) that the resolution-stage fail-closed invariant does NOT cover; it
  is bounded instead by requiring a concurrent mid-pass modification and by
  self-healing on the next clean full index.
- Both are substantial. 091.019 flips on re-export canonicalization, a
  precision-sensitive change that must be adversarially reviewed on its own with
  an exactly-one-target precision gate plus regression fixtures. 091.021 threads
  a bounded, per-file-drained source cache across the pre-pass, main, sync, and
  post-pass reresolve stages and needs benchmarking on a large workspace.
- Both cards self-describe as needing their own adversarial review or
  benchmarking. Neither fits a single sub-2h, width-isolated task.

## Options (apply to each item)

- **Option A — build now as low tasks.** Rejected. Each is a substantial
  single-subsystem feature (091.019 is a recall-recovery precision feature;
  091.021 is a performance/robustness rework of the read/parse pipeline).
  Treating either as a low pick-up-ready task under the done 091-F misrepresents
  feature-sized work as ready cleanup and risks the 2-hour rule.
- **Option B — defer as future features.** Promote each to its own feature when a
  trigger fires, each with its own plan-review (091.019 adversarial; 091.021
  plan-harden for the index/sync/post-pass blast radius plus the memory bound)
  and regression/benchmark acceptance.
- **Option C — hold at the status quo.** Zero work. The missing facade edge
  (091.019, safe direction) and the second read plus the bounded stale-prefix
  false-edge TOCTOU (091.021) persist.

## Chosen direction

**091.019-T -> DEFER as a future feature (recall recovery).** Build-later is
gated on a MEASURED re-export recall gap (facade `pub use` calls appearing in
eval misses), not on speculation. When picked up it needs: (1) wire `ReexportMap`
into the canonical match step; (2) preserve the exactly-one-target precision gate
so a fan-out re-export still fails closed; (3) regression fixtures asserting an
edge IS produced for a `pub use` facade call and NO edge for an ambiguous
fan-out. Adversarial plan-review is required. Safe to defer indefinitely because
it is a missing edge that self-heals on reindex.

**091.021-T -> DEFER as a future feature (performance and robustness).**
Build-later is gated on a MEASURED read/parse cost signal on a large workspace;
the acceptance already demands a benchmarked reduction. When picked up it needs
plan-harden for the cross-pass blast radius (pre-pass, main, sync, post-pass) and
a bounded-peak-memory design (per-file drain plus the max-file-size guard), with
the global-prefix TOCTOU regression fixture and byte-identical canonical-edge
output versus the pre-091.016 baseline as acceptance. Urgency is low but NOT
because it is correctness-neutral: the stale-prefix TOCTOU is a real-but-narrow
false-edge window (concurrent mid-index remap only, self-healing on reindex, no
measured occurrence), so it is deferred as robustness hardening rather than an
active-bug fix.

Why defer-as-future-feature and NOT build-later-as-a-task: both cards are
feature-sized (new precision behavior; a cross-pass pipeline rework) and each
explicitly requires its own adversarial review or benchmarking. Keeping them as
`queued` low tasks under the done 091-F is exactly the misrepresentation this
drain-closeout removes.

## Open questions (for the future plans)

1. 091.019 — where is the `ReexportMap` meant to be CONSTRUCTED in the canonical
   context? No production construction site exists today, so building the map is
   part of the feature, not only applying it.
2. 091.019 — does the precision gate treat a re-export that resolves to exactly
   one def via a multi-hop chain (within `MAX_REEXPORT_DEPTH` = 64) as unique,
   and how does that interact with the singleton post-pass?
3. 091.021 — what is the measured peak-memory ceiling on the largest supported
   workspace, and does the per-file drain interact correctly with the sync path's
   changed-files-only scope?
4. 091.021 — can the post-pass reresolve consume the same snapshot without
   holding all source for the duration of a full index?

## Notes

- Grounding verified 2026-07-31 on main `f5c955a2`:
  `src/services/parsing/canonical/reexport.rs` (`ReexportMap` plus
  `canonicalize`, unit-tested, no production consumer; canonical `mod.rs`
  declares `pub mod reexport` but does not re-export or apply it);
  `src/services/code_graph.rs` (`unsafe_module_prepass` @ ~88,
  `rust_ctx_for_staged_file` @ ~720,
  `reresolve_calls_edges_with_canonical_context` @ ~970,
  `function_ids_by_canonical_path` @ ~1035, `force_prepass_cache_miss` @ ~1278).
- Governed by the fail-closed invariant (013-D / 082-F target-correctness) and
  the 2-hour rule. This deliberation is a future-cycle input; it does NOT
  authorize implementation now.
- Parent 091-F is done; tasks 091.019-T and 091.021-T are parked to `blocked`
  pending this decision and reference this document.
