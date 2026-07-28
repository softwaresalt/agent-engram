---
title: "Daemon `engram index` cross-file singleton non-persist + CLI response hang — spike investigation plan"
type: spike
date: 2026-07-28
status: investigate-first (deferred — hands-on repro pending)
deliberation_id: "015-D"
stash_id: "5765BAAB"
conclusion: "pending (root cause unproven; stash attribution to direct.rs:162 refuted by static analysis)"
confidence: "n/a"
relates_to: ["8DD29746"]
tags:
  - code-graph
  - daemon
  - ipc
  - indexing
  - freshness
  - investigate-first
---

## Goal

Pin the true root cause of two entangled symptoms observed with the **daemon**
`engram index` on a **fresh** git workspace:

1. cross-file singleton `calls` edges (e.g. `alpha → beta`) are left
   staged/unresolved (not persisted), and
2. the `engram index` CLI **hangs on response** while the daemon completes
   server-side.

## Static analysis this cycle (refutes the stash's suspected cause)

The stash suspected `src/cli/direct.rs:162` (`use_index = full || force`) routing
"plain index" to the sync path. **This is not the cause for `engram index`:**

* `engram index` (daemon) → `cli/commands/indexing.rs::run_index` →
  `run_tool_timed("index_workspace", …)` — the **full-scan index tool**.
* `engram index --direct` → `run_direct_sync(full = true, …)` →
  `use_index = full || force = true` → `index_workspace`.
* The index path runs the **unconditional** cross-file post-pass
  (`code_graph.rs:1815`, `reresolve_calls_edges_with_canonical_context`).

So both `engram index` paths already take the post-pass path. The
**content-hash skip** freshness landmine
(`docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md`)
explains the *already-indexed* no-op case, but **not** the *fresh-workspace*
non-persist. The real defect is on the **daemon path** (routing/commit) and/or
**IPC response completion** — unproven, needs hands-on reproduction.

## Hypotheses to test

| ID | Hypothesis | Where |
|---|---|---|
| **H1** commit boundary | daemon `index_workspace` commits symbols but the post-pass singleton resolution is not flushed/committed before the IPC response (runs outside the persisted txn) | `tools/write.rs:113`, `code_graph.rs:1815` |
| **H2** IPC framing | daemon finishes the index but the response frame is never sent / the CLI read blocks → CLI hang; edges may actually be persisted but completion is unobservable | `daemon/ipc_server.rs` (response completion) |
| **H3** routing divergence | daemon routes `engram index` through an event-debounce/sync path, skipping the post-pass on the daemon path (divergence from `--direct`) | `daemon/debounce.rs`, `ipc_server.rs:515/693/928/1150` |
| **H4** staging unresolved | cross-file call is staged correctly but the post-pass is never invoked on the daemon path, leaving it staged | `code_graph.rs` post-pass invocation |

## Investigation steps (hands-on, deferred)

1. On a fresh git workspace, run `engram index` via the daemon; capture whether
   the IPC response returns and whether the hang exceeds `INDEXING_TIMEOUT_SECS`
   or is indefinite.
2. Inspect the DB after the run for the `alpha → beta`
   `calls_resolved_singleton` edge and residual `staged_call` rows.
3. Compare three paths: daemon `engram index` vs `engram index --direct` vs
   in-process `index_workspace` (the GREEN 18/18 recall path) — isolate where
   persistence diverges.
4. Bisect the chain: `run_index` → `run_tool_timed` → IPC dispatch
   (`tools/mod.rs:352`) → `tools::write::index_workspace` → post-pass → IPC
   response.

## Exit criteria

Root cause pinned to a specific layer (routing vs commit boundary vs IPC
framing) with a **proven minimal repro**. Only then author a fix plan. **Do not**
write a fix plan on the misattributed `direct.rs:162` cause.

## Sequencing / relationship

* **Deferred** — hands-on daemon reproduction is required and was not executed
  this Stage cycle (runtime execution outside the cycle's role scope).
* **Relates to 8DD29746** (versioned revalidation/backfill): that item mitigates
  the *sync-path* freshness gap but does **not** address the daemon
  fresh-workspace non-persist or the IPC hang — separate width, separate fix.
* Highest operator-assessed value; recommend scheduling the hands-on spike next.
