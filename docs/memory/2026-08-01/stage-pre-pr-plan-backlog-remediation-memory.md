---
type: stage-memory
timestamp: 2026-08-01T00:46:00-07:00
agent: stage
branch: 107-stage-102-104-integration
scope: pre-PR Stage-owned plan/backlog/review/memory remediation
---

# Stage pre-PR planning remediation

## Outcome

Remediated the operator-listed findings only in Stage-owned plan, backlog, accepted-review, shipment, and memory artifacts. Fresh plan hardening and persona review ran under the configured `.Stage` model with no override. Final gate: PASS; open P0/P1/P2/P3: none.

No source, tests, config, agents, dependency edges, stash entries, commit, push, PR, shipment claim/close, or build/test/lint operation occurred. `025-S`, `081-S`, `015-D`, and `017-D` were untouched.

## Decisions

### 109-F / 104-S

- `109.002-T` is a two-file `state.rs` + `lifecycle.rs` GREEN. One opaque `pub(crate)` transition token includes the generation-specific cancel receiver and is consumed once by lifecycle.
- `109.006-T` adds a crate-private opaque generation token to the coherent dispatch snapshot; `write.rs` adopts it directly. Retry/exhaustion-error behavior is prohibited.
- `109.007-T` stays at three scenarios: combined stale lost-lock pending/heavy, stale acquired-lock exact-snapshot validation, and same-G re-arm/drain.
- `109.008-T` validates an atomically claimed full-mask token against the exact workspace/config/generation snapshot before acquired-lock work.
- Startup RED explicitly allows only a private cfg(test)-only pause between failed CAS and publication.
- Cross-module token APIs are `pub(crate)` with private fields; legacy unqualified publishers are not a public bypass.
- Function caps count all production touches, including visibility-only changes/removals, and every intermediate task state must build.
- `104-S.custom_fields.items` is topologically ordered after `109-F`.

### 107-F / 102-S

Ship only detects released exposure and writes a target-specific operator handoff. The operator alone runs and verifies any deployed/user-workspace full reindex. Ship never executes it after approval and never mutates or repairs that workspace.

### 108-F / 103-S

Ship uses disposable fixtures only and never reads for repair, reindexes, repairs, or mutates an operator workspace. `108.002-T` has exactly three scenarios/phases: portable invalid-UTF-8 failure, retry-state recomputation, and clean publication/hash-skip control.

## Files modified

### Plans

- `docs/exec-plans/2026-07-31-post-105-pending-sync-residuals-plan.md`
- `docs/exec-plans/2026-07-31-python-qualified-staging-caller-attribution-plan.md`
- `docs/exec-plans/2026-07-31-ordinary-index-fail-closed-followups-plan.md`

### Accepted reviews

- `.backlogit/archive/109.001-R-plan-review-post-105-pending-sync-generation-and-startup-han.md`
- `.backlogit/archive/107.001-R-plan-review-gate-duplicate-qualified-staging-caller-attribut.md`
- `.backlogit/archive/108.001-R-plan-review-gate-ordinary-index-fail-closed-retry-and-empty.md`

### Shipments/features/tasks

- `.backlogit/queue/104-S.md`, `.backlogit/queue/109-F.md`, `.backlogit/queue/109.001-T.md` through `.backlogit/queue/109.008-T.md`
- `.backlogit/queue/102-S.md`, `.backlogit/queue/107-F.md`, `.backlogit/queue/107.002-T.md`
- `.backlogit/queue/103-S.md`, `.backlogit/queue/108-F.md`, `.backlogit/queue/108.001-T.md`, `.backlogit/queue/108.002-T.md`

### Memories

- `docs/memory/2026-07-31/stage-102-104-ship-report-p1-remediation-memory.md`
- `docs/memory/2026-07-31/stage-group-next-107-f-memory.md`
- `docs/memory/2026-07-31/stage-ordinary-index-fail-closed-release-memory.md`
- `docs/memory/2026-08-01/109-f-alternate-decomposition-review.md`
- this memory
- .backlogit/memories.json (backlogit memory summary)
- .backlogit/checkpoints/checkpoint-20260801-071618.json (superseded valid checkpoint resolved)
- .backlogit/checkpoints/checkpoint-20260801-074324.json (new active schema-v1 checkpoint, validated)

## Validation

- Backlog index sync completed after out-of-band manifest edits.
- Full backlog doctor returned no findings.
- MCP reads confirmed 102-S/103-S/104-S remain queued with operator order 1/2/3 and 104-S items are topological.
- Structural assertions for 109 token/API/seam/buildability requirements and 104-S ordering passed.
- Superseded approval-based reindex and 108 four-scenario wording checks returned no unsafe occurrence.
- `git diff --check` passed after trimming planning-document whitespace.
- Working-tree scope contains only `.backlogit/`, `docs/exec-plans/`, and `docs/memory/` artifacts.
- `checkpoint-20260801-065720.json` remains present but excluded/untracked and was neither repaired nor used.

## Failed approaches

- Four initial Engram context searches used a CLI region accepted by help text but rejected by the daemon; discovery continued with supported indexed code search and narrow file reads.
- Targeted backlog doctor calls initially omitted the `.backlogit/` path prefix and returned scope errors; one full read-only doctor scan then passed with no findings.
- Two local structural assertions used overly literal text/regex patterns; corrected assertions passed without changing artifacts.

## Next step

Ship may later claim shipments only through its own workflow and in operator order. Begin each shipment at its RED task and return blocked on any plan stop condition. For 102-S and 103-S, operator-workspace action remains outside Ship scope.
