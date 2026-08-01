# Stage memory — group and stage next release unit (107-F / 102-S)

## Session outcome

Selected and staged the sole high-priority stash bug `42FB7CC5` as one narrow release unit. Created feature `107-F`, tasks `107.001-T` and `107.002-T`, accepted review `107.001-R`, and queued shipment `102-S`. No shipment was claimed and no implementation, build, test, Git commit, push, or PR operation was performed.

## Tool and state checks

- Backlog registry present; backlogit MCP v1.7.0 responded successfully.
- Backlog index synchronized before semantic reads.
- Metadata catalog, WIT types/templates, shipment state, hook events, doctor report, and all 22 active stash entries were refreshed.
- Engram daemon/workspace status was green and bound to this workspace; indexed search and memory retrieval grounded the plan.
- Hook queue was empty; backlog doctor reported no findings.
- Initial state had no active or queued shipment and only blocked `025-S` / `081-S`; both remained untouched.
- Deliberations `015-D` and `017-D` remained queued and unrelated to this release unit.

## Grouping decision

`42FB7CC5` is a high-priority PR #301 correctness gap: qualified `python_local` calls use first-match caller attribution at the full-index and incremental-sync staging sites. Duplicate top-level caller names can stage a trusted canonical target under an arbitrary duplicate and mint a wrong-origin edge. The existing ambiguity-aware helper and prior fail-closed decisions make the fix implementation-ready without another deliberation or spike.

No medium or low stash entry was folded in. The medium entries cover independent daemon sync generation/lost-wakeup races, index topology/empty-file cleanup, Spark lineage and SQL parsing, and PowerBI durability. Combining them would violate technical cohesion and widen blast radius without a dependency benefit.

## Planning and review artifacts

- Decision/source: `docs/decisions/2026-07-31-python-qualified-staging-caller-attribution-decision.md`
- Hardened implementation plan with appended PASS review: `docs/exec-plans/2026-07-31-python-qualified-staging-caller-attribution-plan.md`
- Backlog review: `107.001-R` (accepted)
- Hardening reason: persisted runtime call-graph admission and index/sync symmetry.
- Review cycle 1 fixed two issues before PASS: update stale helper/call-site comments, and extend the existing same-file shadowing harness instead of creating a duplicate test fixture.

## Harvested hierarchy

- `107-F` — Fail-closed qualified-staging caller attribution for duplicate Python callers (queued, high)
- `107.001-T` — tests-only RED regression harness (queued, high)
- `107.002-T` — code-only GREEN unique-only caller attribution at both producers (queued, high)
- Dependency: `107.002-T` is blocked by `107.001-T`.
- Informational links: `107-F` related to archived `100-F` and done `099.004-T`.
- Each task is explicitly scoped to approximately two hours, one width, one file, and at most three scenarios/two mirrored sites. Task-size metadata was unavailable in this WIT, so the tasks remain tool-reported as unsized.

## Shipment

`102-S` is queued with parent-first manifest:

1. `107-F`
2. `107.001-T`
3. `107.002-T`

Stage did not claim the shipment.

## Stash provenance

Consumed and archived only `42FB7CC5`, after recording its provenance on `107-F` and `102-S`. Twenty-one entries remain active.

Deferred medium: `5765BAAB`, `6487F516`, `75DAF33D`, `FF55E51A`, `88EB5FB1`, `98CF66D5`, `95885F3D`, `3D4DE094`, `A36D73ED`, `A365C7D6`, `2C8F82AE`.

Deferred low: `99AFF44B`, `05EA3D39`, `FDE88E46`, `1E70A289`, `21A4D1DE`, `7AB15FE8`, `A1BB7EB9`, `7139FB66`, `C514AE84`, `5D83D2EB`.

## Ship handoff conditions

- Claim `102-S`; do not bypass the harness-ready/TDD gate.
- Execute `107.001-T` RED before `107.002-T` GREEN.
- Verify exact caller and target identity on full index and incremental sync, not edge existence alone.
- Require zero wrong-origin edges, no arbitrary staged caller row, a non-zero ambiguity-drop signal, and an unchanged unique-caller control.
- Check whether an affected PR #301 binary was published. If exposed, Ship only writes a target-specific operator handoff; the operator alone runs and verifies any full reindex. Ship never executes it, even after approval, and never mutates or repairs the workspace. If not exposed, record no migration/backfill.
- Roll back on any wrong-origin edge, producer asymmetry, or unique-caller recall loss.

## Workspace notes and failed approaches

- Unrelated dirty worktree state was present before Stage writes (`.autoharness/config.yaml`, the prior 087-F archive move, and orchestrator/stash files); it was preserved.
- Backlogit rejected task size updates because this task WIT has no size field. No retry was attempted; explicit two-hour scopes remain in the plan and task bodies.
- One file-append overload call failed and was retried with an explicit string-array cast; the plan review was then verified.
- No application source, test, or configuration file was modified by Stage.

## Next step

Ship may claim queued shipment `102-S` and execute the reviewed plan. The remaining stash and queued deliberations should be triaged in later, separate release units.

## Compact-context assessment

Invoked at batch completion. `docs/memory/` contains 81 files totaling 405,530 bytes, so the file-count threshold is exceeded. Zero files in the active 107-F/102-S scope are eligible: the current memory and reviewed plan must remain available to Ship. Broad historical compaction was not mixed into this single-release-unit staging mutation; it should run as a dedicated backlog/admin operation after cross-referencing completed features and existing unrelated worktree changes.

Compact-context result for this release unit: 0 files compacted; active plan and checkpoint preserved.

Checkpoint note: the first checkpoint state dump omitted the v1 checkpoint envelope required by validation. Stage repaired the tool-created checkpoint in place to the documented schema, validated it successfully, and marked it resolved.
