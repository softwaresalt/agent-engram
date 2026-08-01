---
title: "Close post-105 pending-sync generation and startup handoff residuals"
description: "Decision to ship the two remaining R1/R2 producer races as one bounded daemon-lifecycle unit"
topic: "post-105 pending-sync concurrency residuals"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "018-D"
  - "docs/exec-plans/2026-07-31-post-105-pending-sync-residuals-plan.md"
source_stash:
  - FF55E51A
  - 88EB5FB1
  - 1E70A289
tags: ["daemon", "lifecycle", "pending-sync", "concurrency"]
---

## Problem Frame

Archived feature `105-F` added generation-scoped pending-sync state and the R2 producer/finisher backstop. PR #302 review found two residual producer windows:

1. `set_workspace_and_config` is value-visible before `begin_scan_generation` advances the generation. A concurrent sync can therefore observe the new binding but publish under the old generation; the old hydration can then clear that intent. `publish_pending_sync` also loads the generation before locking the queue, so a paused old-generation publish can OR-coalesce a heavy companion bit into a newer routine request.
2. `try_start_startup_sync` still uses the pre-R2 pattern (failed `try_start_indexing` then `set_pending_sync`). If it resumes after the holder's final empty peek, no finisher is guaranteed.

Both are narrow, self-healing latency/correctness defects, but they affect the same generation-tagged queue and the same R2 backstop.

## Research Findings

- Engram indexed search, symbol mapping, impact analysis, and targeted source reads confirmed the current sequence and call sites.
- `src/tools/lifecycle.rs` publishes the binding before calling `begin_scan_generation`.
- `src/server/state.rs` loads `sync_generation` outside the `pending_sync` mutex in `publish_pending_sync`.
- `src/tools/write.rs` already uses `publish_pending_sync_and_try_reacquire` and drains when it becomes the guaranteed finisher.
- `src/daemon/ipc_server.rs` leaves the startup producer on `set_pending_sync`, with no backstop or self-drain.
- The low-priority comment item `1E70A289` touches the exact `state.rs` primitive that must change. Its two corrections are part of the GREEN change, not a separate width.
- `105-F` and its archived children are predecessor context. They must not be reopened or mutated.

## Options Evaluated

| Criterion | Group FF55 + 88EB + 1E70 | FF55 only | Broaden to 015-D/other stash |
|---|---|---|---|
|Cohesion| High: same queue, generation, producer pattern| Medium| Low|
|Risk | Moderate, non-destructive| Moderate| High/unbounded|
|2-hour decomposition| Four reviewable RED/GREEN units| Two units| Not provable|
|Residual closure| Both known R1/R2 producer gaps | Startup gap remains| Unpinned IPC/data scope|

## Decision

Ship `FF55E51A` + `88EB5FB1` + `1E70A289` as one medium-priority feature with four sequenced tasks: generation RED harness, generation GREEN linearization and comment correction, startup RED harness, startup GREEN backstop. The startup pair follows the generation pair so it inherits the hardened publish primitive.

## Rejected Alternatives

- **FF55 only:** leaves a known sibling producer on the pre-R2 pattern.
- **Fold 015-D/5765BAAB:** rejected because the non-persist cause is unpinned and the IPC hang is a different architectural width.
- **Add Spark, SQL, PowerBI, deletion, or Cozo work:** rejected as unrelated to daemon queue lifecycle correctness.

## Unresolved Questions

Ship may choose the smallest internal synchronization shape. It must not hold a `std::sync::Mutex` guard across `.await`, must linearize the binding/generation transition with queue publication, and must stay within the declared scope caps. If not, Ship returns the task blocked instead of broadening it.

## Risks and Mitigations

- **Dual-lock deadlock/lock-order:** prohibit holding the synchronous queue mutex across await; document and test the order.
- **Reverse stale-capture or heavy-bit leak:** deterministic pause/step tests for both orderings.
- **Startup recursion/double-drain:** reuse the existing R2 backstop and bounded drain; do not add another queue.
- **Rollout regression:** roll back the release commit if any deterministic test flakes, a routine startup sync stalls beyond one finisher cycle, or a routine new-generation sync is dropped.
