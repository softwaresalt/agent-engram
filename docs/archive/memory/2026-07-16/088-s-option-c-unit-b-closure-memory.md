---
type: session-memory
date: 2026-07-16
agent: ship
session: "2c95481b - 088-S Option C Unit B closure"
topic: "088-S canonical call-edge resolution merged; post-merge closure"
---

# Session memory - 088-S Option C Unit B closure

## Outcome

Shipment **088-S** (Option C Unit B canonical call-edge resolution) merged to `main` as merge commit
**0d4821e** via **PR #255**. The feature enables precision-gated `calls_resolved_canonical` edges from
canonical module-path and use-graph identities. Closure artifacts and backlog reconciliation were produced
on branch `chore/088-closure` from a separate closure worktree.

## Tasks completed or reconciled

* `091.011-T` - B1: staged qualified and method calls with raw provenance; shipped in PR #255.
* `091.012-T` - B2: canonical singleton resolution in the post-pass; shipped in PR #255.
* `091.013-T` - B3: adversarial precision fixtures and target-correctness gate; shipped in PR #255.
* `091.014-T` - B4: enablement, release gate, and reconcile wiring; shipped in PR #255.
* `091.002-T` - reconciled the 088.005-T archived-done versus blocked resolver-acceptance state after
  Option C shipped; marked done and archived.
* `091.018-T` - extern-crate rename alias shadowing (`C8-5`); fixed in commit `7e76fa1`, marked done,
  and archived.

## Files modified in the shipped feature

Feature implementation touched these primary surfaces:

* `src/services/parsing/canonical/module_path.rs`
* `src/services/parsing/canonical/resolver.rs`
* `src/services/parsing/canonical/use_graph.rs`
* `src/services/code_graph.rs`
* `src/services/retrieval_eval.rs`
* `tests/integration/canonical_call_resolution_test.rs`
* sibling integration tests for `calls_*` and `staged_call_*` behavior

Closure work touched only documentation and backlog state:

* `docs/closure/2026-07-16-088-s-option-c-unit-b-canonical-call-edges-closure.md`
* `docs/memory/2026-07-16/088-s-option-c-unit-b-closure-memory.md`
* `.backlogit/archive/088-S.md`
* `.backlogit/archive/091-F.md`
* `.backlogit/archive/091.002-T.md`
* `.backlogit/archive/091.018-T.md`
* `.backlogit/queue/091.017-T.md`
* `.backlogit/queue/091.019-T.md`
* `.backlogit/queue/091.020-T.md`

## Key decisions

* **No-false-edge invariant holds.** Canonical edges are emitted only when resolution yields exactly one
  non-empty `function_meta.canonical_path`. Ambiguity or shadowing drops the edge.
* **Fail-closed over-approximation is acceptable.** The dependency-rename guard is intentionally
  conservative but ANDed onto `is_workspace_crate`, so its footprint is limited to workspace-member name
  collisions.
* **Sync preserves precision over recall.** Incremental `sync_workspace` skips the expensive full
  canonical post-pass, so it sweeps canonical edges when non-default module mappings change and lets the
  next full index re-derive them.
* **091.018-T is closed, not deferred.** The extern-crate alias finding was fixed by commit `7e76fa1`.
* **No 091.021-T.** The Cargo dependency-rename finding (`C9-1`) was fixed by commit `6267751`.

## Adversarial and Copilot cycle summary

Nine cycles ran, and all meaningful findings were resolved. Notable fixes included:

* M1: snapshot the `created` set before canonical-edge retraction.
* M2: fail closed when a generic type parameter shadows a workspace crate or local type; root cause was
  the tree-sitter-rust `type_parameter` node.
* M3: handle nested `#[path]` recursion and unsafe-prefix descent in module mapping.
* M4: generalize target-correctness evaluation to canonical edges.
* C8-1: sweep canonical edges during incremental sync when non-default module mappings change.
* C8-4: fail closed on block-local type shadowing.
* C8-5: fail closed on `extern crate ext as demo;` alias rebinding.
* C9-1: fail closed on Cargo `package = "..."` dependency renames through
  `WorkspaceCrates::is_dependency_renamed`.

Merge-gate evidence before PR #255 merged: Copilot review bound to HEAD `6267751`, Copilot de-requested,
0 unresolved review threads, and `mergeable_state == clean`.

## Verification state

* Formatting, clippy with `-D warnings -D clippy::pedantic`, and tests were green for the merged feature.
* Canonical integration suite: 16/16.
* Recall-acceptance, target-correctness, post-pass, and edge-resolution-storage suites were green.
* CI `build` went green after re-running known unrelated flake
  `t030_003_markdown_heading_and_code_block_indexed_via_ipc` (IPC timing, orthogonal to canonical edges).

## Deferred follow-ups

* `091.017-T` - physical-target-file dual identity for path-remapped modules; queued, low priority.
* `091.019-T` - apply the A4 re-export map before matching canonical paths; queued, low priority.
* `091.020-T` - make canonical recall denominator resolution-aware; queued, low priority.

These do not block the enabled feature. They are missing-edge or observability improvements, not known
false-edge fixes.

## Known external flakes and open operator items

* The HF embeddings CI flake is environment-not-code: model download/cache timing can trip the fixed
  deadline and is unrelated to canonical call resolution.
* PR #248 remains open for the operator. Option C superseded the blocked name/spelling approach, but the
  operator owns final disposition.

## Next steps

1. Push this closure branch and open the closure PR.
2. Request Copilot review, then stop for the orchestrator merge-gate.
3. Queue order after closure: **083-S -> 085-S -> 086-S**, one active shipment at a time per P-001.
