---
title: "Daemon `engram index` cross-file singleton non-persist + CLI response hang — spike FINDINGS (hands-on/live-daemon)"
type: spike-findings
date: 2026-07-29
status: findings-complete (root cause NARROWED, not fully pinned — defer to runtime-verification spike)
deliberation_id: "015-D"
stash_id: "5765BAAB"
supersedes_plan: "docs/decisions/2026-07-28-daemon-index-singleton-nonpersist-ipc-hang-spike.md"
reproduction_status:
  ipc_hang: "REPRODUCED (symptom 2) — daemon path, current post-104-F main"
  cross_file_non_persist: "INCONCLUSIVE pending known-green corpus validation — observed edge absence cannot establish persistence behavior"
confidence: "high (IPC hang is real + daemon-specific) / unclassified (non-persist claim pending known-green corpus validation)"
relates_to: ["8DD29746"]
related_to_shipped: ["104-F"]
tags:
  - code-graph
  - daemon
  - ipc
  - indexing
  - freshness
  - spike-findings
---

## Summary

A hands-on, live-daemon spike on **current main (`00665738`, post-104-F)**
reproduced the two entangled symptoms from stash `5765BAAB`:

1. **IPC / CLI response hang (symptom 2) — REPRODUCED.** Daemon-path
   `engram index` on a fresh cross-file workspace hung the CLI **> 270 s**,
   exceeding its own `--timeout 200`, while the daemon **completed the scan
   server-side within seconds**. The in-process `--direct` path returned in
   **~1 s** on the same corpus. → The hang is **daemon/IPC-path-specific**, real,
   and **not fixed by 104-F**.
2. **Cross-file singleton non-persist (symptom 1) — INCONCLUSIVE pending
   known-green corpus validation.** After the
   run, the `alpha → beta` cross-file `calls` singleton is **absent from the
   persisted resolved graph** (`workspace-status` reports `edges: 2` — both
   `defines`; `map-code beta` shows no incoming `calls` edge). That observation
   does **not** establish a persistence defect because the same minimal corpus
   did not produce the edge on the known-good `--direct` path and was never
   validated against a known-green singleton control.

The **exact root cause of symptom 1 is not pinned**, and a **repro-corpus
validity caveat** applies (below). The precise root cause of symptom 2 is
**narrowed** to the synchronous long-op IPC response model plus per-invocation
daemon-spawn/model-load happening **outside** the client request timeout — but
localizing it to a single commit/line needs **daemon-internal instrumentation**
(Ship/runtime scope). **No clean, ≤2 h, fail-safe fix is provable from the Stage
seat.** Recommendation: **produce these findings and defer to a
runtime-verification spike** with the concrete repro procedure below. Do **not**
fabricate a fix on an unproven root cause.

## What was exercised (method)

Deterministic, bounded-timeout probing via `Start-Process … WaitForExit(ms)` so
each invocation's clean-exit-vs-hang is measured, not inferred.

- **Repro corpus (fresh git workspace):**
  - `mod_a.py` → `def beta(): return 1`
  - `mod_b.py` → `from mod_a import beta` … `def alpha(): return beta()`
  - `beta` is the **unique** definition of that name in the workspace (the
    intended cross-file singleton case).
- **Paths compared:** daemon `engram index` vs in-process `engram index --direct`,
  then out-of-band graph inspection via `workspace-status` and `map-code`.
- **Live daemon:** the pre-existing repo daemon (PID 31852, model-loaded, 1.3 GB)
  was left bound to the engram repo throughout; each `--workspace <temp>`
  invocation spawned a **separate per-workspace daemon**.

## Evidence

### Symptom 2 — IPC / CLI hang (daemon path)

| Path | Return | Elapsed | Notes |
|---|---|---|---|
| daemon `engram index --workspace <tmp> --timeout 200` | **NO** | **> 270 s** (killed) | Exceeded its own `--timeout 200`. Daemon completed the scan server-side (`scan_status.running=false`, `last_completed_at` ≈ seconds after start). Per-workspace daemon (PID 35240) wrote `nodes.jsonl` + `edges.jsonl` + `engram.db`. |
| in-process `engram index --direct --workspace <tmp> --timeout 200` | **YES** | **~1.0 s** | `edges_created:2, cross_file_edges_dropped:1, embeddings_generated:2, functions_indexed:2, duration_ms:597`. No hang. |
| `engram daemon-status` (concurrent) | YES | ~0.4 s | Original daemon healthy the whole time — the hang is confined to the indexing request/response, not the daemon process. |

→ The hang is **specific to the daemon `engram index` request/response path**;
the identical index runs in-process in ~1 s. The client `--timeout` did **not**
bound the observed wait (>270 s ≫ 200 s), consistent with the timeout not
covering daemon-spawn/model-load/health-wait that `run_tool_dispatch` performs
(`ensure_daemon_running`) **before** the timed `send_request`.

### Symptom 1 — cross-file singleton observation (daemon path; inconclusive)

Out-of-band inspection of the persisted graph after the daemon run:

- `workspace-status`: `code_files:2, functions:2, edges:2, scan_status.running:false,
  last_completed_at ≈ 10:34:56, last_flush ≈ 10:34:57`. **Only 2 edges — both
  `defines`.** No `calls` edge.
- `map-code beta`: `beta`'s only neighbor is its defining `code_file` via
  `defines`. **No incoming `alpha → beta` `calls` edge.**
- `edges.jsonl` mirror: two `defines` rows, **no `calls` row**.

→ The cross-file singleton was **absent from the observed persisted resolved
graph**, but this is **inconclusive pending known-green corpus validation**.
Because the unvalidated corpus also failed to produce the edge on the known-good
`--direct` path, the observation cannot distinguish daemon non-persistence from
a corpus that never exercises the intended singleton-resolution case.

## Hypothesis resolution

| ID | Hypothesis | Verdict |
|---|---|---|
| **H1** commit boundary (post-pass singleton not flushed before response) | **NOT TESTED conclusively.** The observed graph lacked the singleton, but the corpus was not validated against a known-green control. Corpus validation is prerequisite to any commit-boundary inference. |
| **H2** IPC framing / synchronous long-op | **CONFIRMED as the hang mechanism (narrowed).** CLI awaits a single response for the entire index; return exceeded `--timeout`. Refined: daemon-spawn + model-load happen **outside** the client request timeout (`run_tool_dispatch` calls `ensure_daemon_running` before the timed `send_request`), so the client-side deadline does not bound the wait. |
| **H3** routing divergence (daemon takes a debounce/sync path skipping the post-pass) | **PARTIALLY SUPPORTED.** The daemon path behaves differently from `--direct` (which returns in ~1 s and does not hang), but I could **not** confirm the daemon skips the post-pass vs runs-it-without-committing. Needs internal tracing. |
| **H4** staged-unresolved (post-pass never invoked) | **OPEN.** Indistinguishable from H1 without querying the `staged_call` relation directly (per-workspace-daemon auto-reindex-on-query confound blocked a clean read). |

**Refuted last cycle and still refuted:** the stash's original attribution to
`src/cli/direct.rs:162` (`use_index = full || force`). Both `engram index` paths
route to `index_workspace`; the defect is on the daemon request/response +
persist path, not that flag.

## Confounds and honesty caveats (important)

1. **Per-workspace-daemon + auto-reindex-on-query.** Every `--workspace`
   invocation (including `workspace-status`/`map-code`) spawns or drives a
   **separate per-workspace daemon** that (re)indexes on bind. A `workspace-status`
   query issued moments after `--direct` reported `edges_created:2` returned a
   **different, partial** state (`code_files:1, edges:1, functions:1,
   last_flush:null`) because the query itself triggered a fresh daemon
   re-index mid-flight. This nondeterminism is **itself a signal** but it
   **prevents a clean, deterministic proof** of the non-persist mechanism from
   the Stage seat.
2. **Repro-corpus validity caveat.** Even the `--direct` path reported
   `cross_file_edges_dropped:1` and did **not** surface a resolved `alpha → beta`
   edge in the queried graph. `cross_file_edges_dropped` most likely counts the
   `from mod_a import beta` **Imports** edge, not the call — but I could **not**
   confirm the minimal corpus produces the same resolved singleton the GREEN
   18/18 recall suite exercises. The minimal `from N import name; name()` shape
   may be modeled as an import-resolved call rather than a staged cross-file
   singleton. **Any runtime-verification follow-up must first validate the corpus
   against a known-GREEN singleton case** before drawing persist/non-persist
   conclusions.
3. **Interrupted run.** The daemon-path CLI was killed at ~270 s; the daemon may
   or may not have executed the full finalize/post-pass by the time the graph was
   inspected (though `scan_status.running:false` + `last_flush` set indicates the
   scan phase had completed).

## Relationship to 104-F (shipped, `related_to` 015-D)

104-F hardened the **pending-sync drain** state machine (single-shot →
`drain_pending_sync_to_completion` bounded loop at `write.rs:170`; `clear_all_pending_sync`
on cancel/DB-fail at `lifecycle.rs:255/280`). That addresses a **drain-stall /
companion-bit leak** — a different layer. This spike confirms 104-F **does not
fix** the daemon-path IPC hang. The singleton observation cannot support a
persist/non-persist conclusion until the corpus passes a known-green control.

## Root-cause conclusion (honest posture)

- **Symptom 2 (IPC hang):** REAL and daemon-specific. Root cause **narrowed** to
  the synchronous long-op response model + daemon-spawn/model-load occurring
  outside the client request timeout. **Not** pinned to a single line/commit
  without runtime instrumentation.
- **Symptom 1 (non-persist):** **INCONCLUSIVE pending known-green corpus
  validation.** The observed graph lacked the edge, but the unvalidated corpus
  also failed on the known-good `--direct` path. H1/H4 inference is therefore
  gated by corpus validation, in addition to the per-workspace-daemon and
  auto-reindex confounds.

Later controlled 107-S characterization classified the current daemon behavior
as **no current defect**. This historical correction preserves that later
evidence; it only retracts the unsupported corroboration claim from this
earlier, unvalidated corpus.

**No fix is authored.** A fix plan on an unproven exact root cause would be low
quality and risk trading one defect for another (per 013-D discipline).

## Recommendation — DEFER to a runtime-verification spike

Advance 015-D to a **runtime-verification spike** (Ship-owned or a dedicated
instrumented investigation) with the concrete procedure below. Do not schedule a
fix shipment until the root cause is pinned on a controlled, single-daemon,
pre-warmed repro.

### Concrete runtime-verification repro procedure

1. **Remove confounds:** use ONE daemon bound to ONE workspace for the entire
   run. Pre-warm it (bind + first index) so the model is already loaded, then
   **re-index the same workspace** — this removes daemon-spawn + cold model-load
   from the timed path (isolating H2's response-framing from spawn/load latency).
2. **Validate the corpus first:** confirm the chosen corpus produces a resolved
   cross-file `calls` singleton via the **known-GREEN in-process `index_workspace`
   path** (the 18/18 recall path) before testing the daemon path. If
   `from N import name; name()` does not, add a bare-unqualified-call corpus that
   does.
3. **Instrument the daemon path** (Ship scope): trace whether, on the daemon
   index, (a) the cross-file post-pass `reresolve_calls_edges_with_canonical_context`
   (`code_graph.rs:1815`) is invoked, (b) it resolves the singleton, (c) the
   resolution is committed/flushed before `finalize_indexing_request` returns,
   and (d) the IPC response frame is written (`ipc_server.rs`). This distinguishes
   H1 vs H3 vs H4.
4. **Bound the hang:** confirm whether the client wait exceeds `--timeout` because
   `ensure_daemon_running` (spawn + health-wait + model-load) runs **before** the
   timed `send_request` (`runner.rs`), and whether an "indexing started" ack /
   streamed progress would bound the observable wait.

### Candidate fix directions (for the follow-up plan, NOT to build now)

- **Async / streaming index response:** have the daemon ack "indexing started"
  and stream progress/terminal status, so the CLI never blocks on a single
  synchronous response for a multi-minute op. (Architectural — likely > 2 h.)
- **Bound daemon-spawn/model-load under the client deadline** (or surface it as a
  distinct, user-visible "warming daemon" phase) so `--timeout` is meaningful.
- **Persist-boundary fix (if H1/H4 confirmed):** ensure the daemon index path
  runs and **commits** the same unconditional post-pass the in-process path does,
  before responding — with a regression test proving the cross-file singleton is
  persisted via the daemon route.

## Sequencing

- Stay `related_to 8DD29746` (versioned revalidation/backfill mitigates the
  **sync-path** freshness gap — it does **not** address this daemon fresh-workspace
  non-persist or the IPC hang; separate width).
- Highest operator-assessed value, but the responsible next step is a
  **runtime-verification spike**, not a speculative fix shipment.
