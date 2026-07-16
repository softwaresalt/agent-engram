---
title: "Operational Closure - 088-S Option C Unit B canonical call-edge resolution"
doc_type: closure
source: "088-S shipment (feature 091-F Unit B; tasks 091.011-T..091.014-T plus 091.002-T)"
description: >-
  Post-merge closure for shipment 088-S. Records the canonical call-edge resolution feature,
  nine adversarial and Copilot review cycles, fail-closed fixes, verification evidence,
  deferred follow-ups, backlog archival state, and rollback posture after PR #255 merged.
topic: "Enable precision-gated calls_resolved_canonical edges"
depth: "closure"
decision_status: "SHIPPED - merged to main as merge commit 0d4821e via PR #255"
author: ship
date: 2026-07-16
verdict: SHIPPED
pr: 255
merge_commit: 0d4821e
target_commit: 0d4821e
branch: feat/088-canonical-identity-unit-b
scope: "Canonical singleton call-edge resolution from module-path and use-graph identities"
reviewers:
  - gpt-5.6-sol
  - gemini-3.1-pro
  - copilot
linked_artifacts:
  - "088-S"
  - "091-F"
  - "091.002-T"
  - "091.011-T"
  - "091.012-T"
  - "091.013-T"
  - "091.014-T"
  - "091.017-T"
  - "091.018-T"
  - "091.019-T"
  - "091.020-T"
  - "084-S"
  - "087-S"
  - "PR #255"
---

## Summary

Shipment 088-S enables `calls_resolved_canonical` edges: precise cross-file call edges derived from
canonical identities built from module-path and use-graph resolution. An edge is emitted only when the
canonical path resolves to exactly one non-empty `function_meta.canonical_path`; that singleton check is
the precision gate.

The feature builds on 087-S Unit A, which introduced the canonical-identity substrate, and 084-S, which
made staged-call provenance durable across JSONL dehydration and rehydration. 088-S merged as PR #255
with merge commit `0d4821e`.

## What shipped

Strict TDD governed the release unit: tests were added or extended before the implementation they gated.

| Task | Unit | Delivered |
|---|---|---|
| `091.011-T` | B1 | Stage qualified and method calls with raw provenance |
| `091.012-T` | B2 | Canonical singleton resolution in the post-pass |
| `091.013-T` | B3 | Non-vacuous adversarial precision fixtures and target-correctness checks |
| `091.014-T` | B4 | Enablement, recall and precision release gate, and reconciliation wiring |
| `091.002-T` | Reconcile | Closed the 088.005-T archived-done versus blocked resolver-acceptance state after Option C shipped |

The shipped edge class is additive. Existing call-edge surfaces remain available, while canonical edges
are derived from stored staged-call provenance and the canonical-path identity produced by Unit A.

## Adversarial and Copilot review

Nine review/fix cycles ran across the adversarial panel and Copilot. Every surfaced finding was treated
as genuine or conservatively fixed. The notable fixes all fail closed: they can drop an edge, but they do
not invent a call edge and therefore preserve the no-false-edge invariant.

* **Cycle-3 M1:** `code_graph.rs` now snapshots the `created` set before the retraction loop, preventing
  the sweep from retracting edges created in the same pass.
* **Cycle-3 M2:** generic type-parameter head shadowing now fails closed. The root cause was the
  tree-sitter-rust grammar node `type_parameter`, whose name comes from `child_by_field_name("name")`,
  not `constrained_type_parameter`. A generic parameter named like a workspace crate or local type no
  longer resolves through the workspace-crate fast path.
* **Cycle-3 M3:** nested `#[path]` recursion and multi-segment unsafe-prefix descent are handled in the
  use-graph module mapping so path-remapped modules do not leak unsafe physical identities.
* **Cycle-3 M4:** the target-correctness evaluation gate was generalized to canonical edges, not only
  the older singleton edge class.
* **Cycle-8 C8-1:** incremental `sync_workspace` now sweeps all canonical edges when any changed file
  carries a non-default module mapping. Sync intentionally skips the O(all staged calls) canonical
  post-pass for performance, so stale canonical edges are dropped and re-derived on the next full index.
* **Cycle-8 C8-4:** block-local type shadowing, such as `struct Shadowed` inside a function, fails closed
  against an outer top-level type with the same name.
* **Cycle-8 C8-5:** `extern crate ext as demo;` now re-points `demo` to a foreign crate and fails closed
  against a same-named workspace crate. Follow-up item `091.018-T` was marked done because this was fixed
  in commit `7e76fa1`.
* **Cycle-9 C9-1:** Cargo dependency renames such as `util = { package = "external-util" }` now rebind
  the crate name `util` to the external package even when a workspace member is also named `util`. Both
  the workspace-crate fast path and the `::head` `resolve_absolute` escape fail closed through
  `WorkspaceCrates::is_dependency_renamed`. Rename keys are collected from `[dependencies]`,
  `[dev-dependencies]`, `[build-dependencies]`, `[target.*.*]`, and `[workspace.dependencies]`.
  The over-approximation footprint is limited to member-name collisions because the guard is ANDed onto
  `is_workspace_crate`.

No `091.021-T` was created: the dependency-rename finding was fixed in commit `6267751`, not deferred.

## Verification

Quality gates were green for the merged feature: formatting, clippy with `-D warnings -D clippy::pedantic`,
and tests. The canonical integration suite finished 16/16. The recall-acceptance,
target-correctness, post-pass, and edge-resolution-storage suites were green.

The GitHub Actions `build` check went green after re-running the known unrelated flake
`t030_003_markdown_heading_and_code_block_indexed_via_ipc`. That failure is IPC timing under CI load and
is orthogonal to the canonical call-edge change.

The merge gate was satisfied before PR #255 merged:

* Copilot review `commit_id == HEAD` for `6267751`
* Copilot removed from `requested_reviewers`
* 0 unresolved review threads
* `mergeable_state == clean`

## Deferred follow-ups

These follow-ups remain queued and low priority. None blocks the enabled feature because each represents a
missing-edge or observability-hardening path, not a known false-edge path.

| Item | Status | Follow-up |
|---|---|---|
| `091.017-T` | queued | Physical-target-file dual identity for path-remapped modules |
| `091.019-T` | queued | Apply the A4 re-export map before matching canonical paths |
| `091.020-T` | queued | Make the canonical recall denominator resolution-aware |

`091.018-T` is not deferred after closure. It was fixed in commit `7e76fa1` and archived as done.

## Operational notes and rollback

### Invariants to preserve

* Canonical edges remain precision-gated: emit an edge only for exactly one non-empty canonical target.
* Ambiguity, external rebinding, glob imports, macro opacity, local shadowing, malformed module mappings,
  and unknown receivers fail closed to no edge.
* The feature is additive to the code graph. It does not require consumers to abandon older edge classes.

### Pre-deploy audit

* Feature flags: none.
* Data migration: none.
* Schema note: `SCHEMA_VERSION` was bumped by 084-S from 5.0.0 to 5.1.0; 088-S rides on that
  generation-gated staged-call sidecar.
* Review gate: nine adversarial and Copilot cycles resolved, with Copilot review bound to HEAD `6267751`.
* Backlog gate: 088-S, 091-F, 091.002-T, and 091.011-T..091.014-T are closed and archived; 091.017-T,
  091.019-T, and 091.020-T remain queued follow-ups.

### Deployment or rollout path

The rollout path is merge-only for the repository: PR #255 landed on `main` through a merge commit
(`0d4821e`). Runtime absorption occurs when users run a binary containing that merge and perform a full
index, which re-derives canonical edges from the current workspace.

### Post-deploy checks

* Run a full index on representative Rust workspaces and compare canonical edge counts against CI baseline
  expectations for the canonical integration suite.
* Spot-check logs for canonical post-pass failures or unexpected edge retractions.
* Track CI for the canonical integration, recall-acceptance, target-correctness, post-pass, and
  edge-resolution-storage suites.

### Healthy signals

* Canonical integration remains 16/16 in CI and locally.
* `calls_resolved_canonical` edge counts are stable after a full re-index of unchanged inputs.
* Target-correctness remains 1.0 for the seeded adversarial fixtures.
* Incremental sync with non-default module mappings drops stale canonical edges rather than preserving
  them across changed identity contexts.

### Failure signals

* Any observed false or mis-resolved `calls_resolved_canonical` edge.
* Target-correctness below 1.0 in the canonical gate.
* Canonical edges persisting after a changed file invalidates a non-default module mapping.
* New canonical post-pass panics or repeated errors during full indexing.

### Monitoring plan

Manual and CI observation are the monitoring surfaces for this local-first daemon. The primary SLIs are
canonical integration pass rate, target-correctness, canonical edge-count deltas after full index, and
unexpected canonical-edge retractions during sync. Baseline is the green PR #255 suite: canonical
integration 16/16 with target-correctness 1.0. Threshold to investigate is any false-edge report,
target-correctness below 1.0, or a deterministic failure in the canonical suites.

### Rollback trigger and procedure

* Trigger: a reproducible false `calls_resolved_canonical` edge, target-correctness below 1.0, or stale
  canonical edges surviving an invalidating module-mapping change.
* Procedure: revert PR #255 with a merge-commit revert of `0d4821e`, rebuild, and ship the reverted
  binary. No data migration or manual database rewrite is required because canonical edges are additive
  and re-derived on full index.

### Risky action record

* **ProposedAction:** merge the edge-emitting Unit B feature after adversarial and Copilot review.
* **ActionRisk:** moderate. The change affects shared code-graph behavior but uses additive edges and a
  fail-closed precision gate.
* **ActionResult:** applied. PR #255 merged as `0d4821e` after the four-point Copilot merge gate passed.
* **Approval path:** operator assigned the Ship closure task; the orchestrator retains the final closure
  PR merge gate.

### Validation window and owner

* Owner: ship and repository maintainer.
* Window: next 7 days or next 3 CI runs touching canonical call resolution, whichever gives earlier
  confidence, plus any first full-index run on a representative workspace after the merge.
* Closure status: **READY WITH FOLLOW-UP**. The feature is shipped and validated; deferred follow-ups are
  low-priority missing-edge or observability improvements.
