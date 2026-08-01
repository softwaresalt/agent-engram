# Stage 102-S/103-S/104-S Ship-report P1 remediation memory

Date: 2026-07-31
Branch: 107-stage-102-104-integration
Agent: Stage

## Outcome

Remediated only the valid Ship report-only P1 findings in the existing 102-S/103-S/104-S planning and backlog artifacts. No source, tests, config, agents, dependencies, shipment lifecycle, or stash state changed. No commit or push was performed.

## Decisions

- 104-S generation ownership: a producer carries the generation captured atomically with its workspace/config snapshot into publication. Under pending_sync, newer replaces, equal coalesces, and older is ignored. The paused-G then owning-G+1 RED case remains required.
- 102-S release exposure: Ship detects exposure and prepares an operator-approved handoff for a named target workspace. No automatic user/deployed workspace mutation; verification follows operator execution or explicit approval. No affected binary means record no migration/backfill.
- Shipment order: priority plus custom_fields.operator_order encode 102-S=1, 103-S=2, 104-S=3 without technical dependencies.

## Validation

- backlogit index sync succeeded.
- Targeted backlogit doctor passed for all changed backlog artifacts.
- backlogit get/list returned all three shipment priorities and operator_order values.
- git diff --check passed; implementation-plan heading lines were unchanged.

## Next Steps

Ship may claim in operator order after reviewing the remediated accepted review records.

## Second narrow 104-S/109-F concurrency remediation

A second Stage-owned pass updated only the existing 104-S/109-F plan, feature, four tasks, shipment, accepted review, and this memory. No task was created; no shipment was claimed/closed; no stash, source, tests, config, agents, commit, or remote state changed.

### Decisions

- Unit 2 uses only `src/server/state.rs` and `src/tools/write.rs` (four production functions maximum). State privately preserves the existing lifecycle `set_workspace_and_config` -> `begin_scan_generation` call sequence while making binding/generation/cancel ownership one transition; `lifecycle.rs` is not edited.
- The real queued producer in `write.rs` validates generation before/after `snapshot_graph_handler_context`, retries mismatch within a fixed private budget, and fails closed before lock/publication on exhaustion. It carries accepted G explicitly; publication never relabels by rereading current generation.
- Unit 4 captures startup generation before the initial indexing-lock attempt and passes that token to the explicit-generation publish/reacquire path. Production startup may not call `set_pending_sync` or an unqualified two-argument publisher.
- Capacity validation and every other await finish before `scan_cancel`/`pending_sync`; no synchronous guard crosses await.
- Unit 1 contains exactly two stale-order scenarios plus one same-generation sticky-coalescing control (three maximum).
- Targeted production inventory found one queued producer in `write.rs` (109.002-T), one startup producer in `ipc_server.rs` (109.004-T), and one lifecycle lost-lock re-arm. The re-arm is generation-neutral queue maintenance: it adds no binding-specific/heavy intent, preserves a nonempty owner, and routine execution re-snapshots current binding. Unqualified state helpers have no additional production callers and are deferred. Any contrary execution-time inventory blocks 104-S.

### Gate and validation

The remediated plan is a genuine **GATE PASS** for the current source inventory and stated caps, not an unconditional implementation waiver. Ship must block 104-S on any new/unclassified production publisher, Unit-2 third file/fifth function, mismatch relabel/fallback, lifecycle production edit, unqualified startup publication, synchronous guard across await, or other listed stop trigger.

- Backlogit targeted doctor passed for 104-S, 109-F, 109.001-T through 109.004-T, and accepted review 109.001-R.
- Plan/card/review structure checks passed.
- Production inventory assertion passed: `write=1`, `startup=1`, `lifecycle_rearm=1` before test modules.
- Targeted `git diff --check` passed after removing plan trailing whitespace.
- Prohibited implementation surfaces (`src`, `tests`, Cargo files, agents/instructions/workflows) remained clean.
- Shipment 104-S remained queued with covering feature 109-F and members 109-F plus 109.001-T through 109.004-T.

### Files in this second remediation

- `docs/exec-plans/2026-07-31-post-105-pending-sync-residuals-plan.md`
- `.backlogit/archive/109.001-R-plan-review-post-105-pending-sync-generation-and-startup-han.md`
- `.backlogit/queue/104-S.md`
- `.backlogit/queue/109-F.md`
- `.backlogit/queue/109.001-T.md`
- `.backlogit/queue/109.002-T.md`
- `.backlogit/queue/109.003-T.md`
- `.backlogit/queue/109.004-T.md`
- `docs/memory/2026-07-31/stage-102-104-ship-report-p1-remediation-memory.md`

## Final 104-S/109-F review-cycle disposition

**This section supersedes all earlier PASS and claim language in this memory.** The final Concurrency Reviewer invalidated the prior lifecycle-neutrality proof. `src/tools/lifecycle.rs::drain_pending_sync` lost-lock re-arm can, after G -> G+1, preserve old heavy companions or relabel old intent through unqualified `set_pending_sync` as the current generation.

### Blocker and operator decision

- A complete generation correction requires `src/server/state.rs` + `src/tools/write.rs` + `src/tools/lifecycle.rs`.
- That width exceeds 109.002-T / Unit 2's hard two-production-file generation GREEN cap.
- Moving `lifecycle.rs` into 109.004-T / Unit 4 would violate its `src/daemon/ipc_server.rs`-only startup GREEN cap.
- Shipment stop conditions therefore apply. Stage did not widen, split, or invent scope.
- Before this work can be re-queued, a future operator-directed replan must explicitly choose whether to authorize a three-production-file generation GREEN cap or approve different task/shipment decomposition; the revised plan must then pass Stage review.

### Exact state transitions

- `104-S`: `queued` -> `blocked`.
- `109-F`: `queued` -> `blocked`.
- `109.001-T`: `queued` -> `blocked` (unstarted RED is non-executable under the blocked plan).
- `109.002-T`: `queued` -> `blocked` (direct cap blocker).
- `109.003-T`: `queued` -> `blocked` (depends on blocked 109.002-T).
- `109.004-T`: `queued` -> `blocked` (depends on blocked chain and may not absorb `lifecycle.rs`).
- `109.001-R`: remains `accepted`; review label changed `pass` -> `block`, and gate changed `PASS` -> `BLOCK`.
- `102-S` and `103-S`: remain `queued` with existing `operator_order` 1 and 2; no content/state mutation in this final disposition.

### Files changed in this final disposition

- `docs/exec-plans/2026-07-31-post-105-pending-sync-residuals-plan.md`
- `.backlogit/archive/109.001-R-plan-review-post-105-pending-sync-generation-and-startup-han.md`
- `.backlogit/queue/104-S.md`
- `.backlogit/queue/109-F.md`
- `.backlogit/queue/109.001-T.md`
- `.backlogit/queue/109.002-T.md`
- `.backlogit/queue/109.003-T.md`
- `.backlogit/queue/109.004-T.md`
- `docs/memory/2026-07-31/stage-102-104-ship-report-p1-remediation-memory.md`

No source, tests, config, agents, stash, task/shipment creation, shipment claim/close, commit, or push occurred.

### Validation

- Backlogit targeted doctor passed for 104-S, 109-F, 109.001-T through 109.004-T, and 109.001-R.
- Engram structural verification passed for the plan and accepted review artifact.
- Targeted `git diff --check` passed.
- Backlogit SQL/MCP state checks passed: 104-S, 109-F, and 109.001-T through 109.004-T are blocked; 109.001-R remains accepted with the block label; 102-S/103-S remain queued with operator_order 1/2 and unchanged update timestamps from session intake.
- Full backlogit doctor reported no findings; the working-tree path set gained no source, tests, config, agents, or stash changes. End-of-session backlog index sync is the final action after these checks.
