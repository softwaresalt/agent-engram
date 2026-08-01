---
title: "Stage memory — ordinary index fail-closed retry and empty-file eviction"
date: 2026-07-31
agent: stage
shipment: 103-S
feature: 108-F
status: complete
---

## Outcome

Staged one medium-priority cohesive release unit from stash `6487F516` and `75DAF33D`. Created queued shipment `103-S` covering feature `108-F` and two single-width tasks. Existing high-priority queued shipment `102-S` remained byte-for-byte manifest-unchanged (`updated_at` stayed `2026-08-01T03:32:35.3870601Z`) and must be handled first by Ship.

## Selection rationale

The selected bugs are both PR #301 residuals in `src/services/code_graph.rs::index_workspace_impl`, affect non-forced ordinary-index persistence, share derived-graph safety/rollback and integration verification, and fit one <=2h-per-task release unit. They do not mix daemon lifecycle, PowerBI, Spark lineage, schema, CLI, dependency, or blocked-shipment work.

`015-D` / stash `5765BAAB` remains deferred because its daemon IPC hang and singleton non-persist cause are not pinned; it still requires a Ship-owned instrumented runtime spike. `017-D` remains low-priority and unrelated.

## Artifacts

- Decision: `docs/decisions/2026-07-31-ordinary-index-fail-closed-followups-decision.md`.
- Reviewed/hardened plan: `docs/exec-plans/2026-07-31-ordinary-index-fail-closed-followups-plan.md`.
- Feature: `108-F` — queued, medium.
- Plan-review artifact: `108.001-R` — accepted / PASS after one remediation cycle.
- Task `108.002-T` — R1 topology snapshot retry state, source stash `6487F516`.
- Task `108.001-T` — R2 authoritative zero-byte eviction, source stash `75DAF33D`; blocked by `108.002-T`.
- Shipment: `103-S`, queued manifest order `108-F`, `108.002-T`, `108.001-T`.

## Review decisions

- Plan hardening was required because empty-file handling retracts persisted derived rows and snapshot certification affects runtime graph correctness.
- Use invalid UTF-8 bytes for a deterministic, cross-platform, end-to-end read failure; no public failpoint or permission/timing test.
- Restore the previous canonical snapshot, or leave absent, after any per-file error; publish current only on a clean run.
- Reuse `handle_deleted_file` only after a successful zero-byte content read.
- Do not repurpose `IndexResult.files_reconciled`; count the empty path as skipped and verify persisted state.
- No schema, public API, CLI/MCP field, response semantic, migration, or generalized non-empty teardown.

## Provenance

Consumed and archived only stash `6487F516` and `75DAF33D` after hierarchy, dependency, and shipment verification. Their full source text is preserved in task descriptions/labels and the decision/plan.

Deferred active stash IDs: `5765BAAB`, `FF55E51A`, `88EB5FB1`, `98CF66D5`, `95885F3D`, `3D4DE094`, `A36D73ED`, `A365C7D6`, `2C8F82AE`, `99AFF44B`, `05EA3D39`, `FDE88E46`, `1E70A289`, `21A4D1DE`, `7AB15FE8`, `A1BB7EB9`, `7139FB66`, `C514AE84`, `5D83D2EB`.

## Failed or degraded approaches

- Intercom capability is enabled but no tool surface was exposed; remote milestone visibility was declared degraded.
- Initial lock attempts on new planning files failed because `acquire_lock.ps1` requires the target to exist. The new files were created non-destructively; the plan was then locked for hardening/review append and the lock was released.
- Reviewer persona agent files listed in the stale install manifest were absent, and no subagent tool was exposed. Required personas were evaluated with the configured Stage model as allowed by the plan-review skill; no model override.
- One `engram symbols` CLI invocation used an unsupported positional argument; discovery continued through successful daemon/workspace status, search, map-code, and impact operations.

## Ship handoff

1. Claim and complete higher-priority `102-S` first.
2. Claim `103-S` without changing planning scope.
3. Execute `108.002-T` before `108.001-T`; both require TDD/harness-ready handling and disposable fixtures.
4. Stop and return blocked on a public test seam, flaky timing/permission test, broader non-empty teardown, control recall loss, contract widening, task >2h/>2 files/>4 scenarios, or any need to repair an operator workspace.
5. Runtime rollback triggers: current snapshot published after partial error, live/read-failed file eviction, non-zero dangling rows, wrong edge, lost control edge, or clean-path permanent reparse.

## Boundary confirmation

Stage did not modify source/tests/config, run builds/tests/linters, create or switch branches, commit/push, create PRs, claim/close shipments, or perform Ship work.