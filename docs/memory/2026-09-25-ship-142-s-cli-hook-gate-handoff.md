# Ship 142-S — CLI Hook-Gate Handoff

- **Date:** 2026-09-25
- **Shipment / feature / task:** `142-S` / `142-F` / `142.054-T`
- **Branch / HEAD:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs` / `c269fa79a0dbe3f2062a2e8d3f09696681773caf`
- **Outcome:** The Ship hook poll was completed via the official backlogit CLI with no events to process. The requested workspace-wide CLI/MCP policy change was captured for Stage, not implemented. 142-S was not resumed into task completion because the complete test suite is blocked.

## Safety record

- **ProposedAction:** Restore the session hook-poll operation using the official backlogit CLI, and route the separate workspace-wide tool-selection policy change through P-021 C2 for Stage deliberation. Do not change P-012, the registry, F50 source/tests, daemon state, or shipment/task status.
- **ActionRisk:** Moderate for the authorized durable capture; the global policy/registry change is broader and requires a separate staged work unit.
- **ActionResult:** `applied` for P-021 capture `DDA2506F` and its task/run references; `blocked` for policy/registry edits and further 142-S execution.

## Tool gate and hook signal

- The backlog registry is present. `backlogit --no-update-check sync` succeeded before semantic reads: `INDEX_SYNC_OK`.
- The backlogit MCP surface was not exposed. The registry had no `cli_command` for `backlogit_poll_hook_events` or `backlogit_ack_hook_events`.
- The installed official CLI documented both `backlogit hooks poll --consumer-id ...` and `backlogit hooks ack --consumer-id ... --seq ...`.
- Operator-directed read-only poll `backlogit --no-update-check hooks poll --consumer-id ship` returned `events: []` and `derived_signals: []`. No acknowledgement was issued.
- Tool report: `TOOL_DEGRADED: backlogit_poll_hook_events — MCP unavailable; official CLI used under the operator-directed gate-repair request; registry mapping/policy correction remains deferred`. The CLI poll itself was successful.
- The checkpoint recovery scan enumerated 32 summaries without status/agent filters; there were no validation/quarantine/required-field anomalies and no active Ship-owned checkpoint.

## Shipment preflight and quality results

- `142-S` re-read as `active`; its manifest remains unchanged. Its dependencies `137-S`, `140-S`, and `141-S` re-read as archived with `archived_status: done`.
- P-001 queue check returned only `142-F` as an active top-level feature. The working branch matches the shipment; `git worktree list --porcelain` showed only this worktree.
- `cargo check --all-targets` — **PASS**.
- F50 targeted harness `cargo test --test integration_preflight_gate -- --nocapture` — **PASS**, 2/2. The current harness imports `crates/engram-indexer/src/preflight.rs`; the state machine is in the committed F50 implementation `c269fa79`.
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — **PASS**.
- `cargo fmt --all -- --check` — **PASS**.
- `cargo dev-test --no-fail-fast` — **FAIL**, exit code 101. The command was run without target filters, but the oversized output was not available for exact extraction in this session, so no `EXPECTED_PENDING_RED` or `PASS` verdict is assigned. The prior run record at `docs/memory/2026-09-25-ship-142-s-checkpoint-recovery.md` documents 12 exact F51–F54 Worker-marker failures plus three unmapped failures (index timing, shutdown TTL, and archive smoke). It records existing deferred entries `41DB6AD0` and `BE626470`; neither issue was changed here.
- No task was moved to `done`; no task/shipment claim or status mutation, commit, push, PR, or merge was performed. `gh pr view` found no PR for this branch.

## P-021 capture and disposition

- The requested ability to select the backlogit CLI or MCP per operation, including poll/ack mappings, changes the workspace-wide backlog tool-selection contract. Under P-021 C1 this is a different contract surface from `142.054-T`'s typed F50 preflight state machine; it was not folded into 142-S.
- Active stash/index discovery found no positively confirmed reusable entry for this same expansion. Task-log discovery for `142.054-T`, `142-F`, and `142-S` found no prior matching capture. Relevant Ship run records were checked. No PR or review thread exists.
- The installed CLI exposes active stash listing but no archived-stash listing; the indexed query did not provide archived-stash coverage. Because archived lookup could not be completed, the capture includes `DISCOVERY-STATUS: LOOKUP-UNAVAILABLE` in field (2), per P-021.
- Created and re-read capture-only entry `DDA2506F` with all six C2 fields populated: the literal token, one-sentence expansion statement plus lookup status, C1 rationale, task/feature/shipment refs, `PR=N/A`, `review-thread=N/A`, `requires deliberation: true`, and provisional `kind: task` / `priority: high`.
- Appended a task comment to `142.054-T` citing `DDA2506F`; a structured read confirmed it persisted. This memory record is the Ship-owned run-level citation. There is no PR/closure residual-risk record yet; carry `DDA2506F` into any future closure residual-risk record.
- No edits were made to `.github/policies/workflow-policies.md`, `.github/agents/_ship.agent.md`, `.github/instructions/backlogit.instructions.md`, or `.autoharness/backlog-registry.yaml`. No versioned policy/registry template source was found. Stage must deliberate and plan a separate work unit before those workspace-wide changes are implemented.

## Next steps

1. Stage triages and deliberates on `DDA2506F`, then supplies a separately approved work unit and shipment for the P-012/registry change. Preserve fail-closed behavior when both supported surfaces are unavailable and preserve the highest-processed-concrete-event acknowledgement rule.
2. Resolve the complete-suite blockers through their authorized scope; do not modify the release-archive verifier or the backlog timing benchmark in this gate-repair scope.
3. Resume F50 only after its full-suite verdict is unambiguously allowed. Do not mark `142.054-T` done or start downstream implementation on this blocked run.
4. Do not stop PID `29360`, change daemon state, create a PR, push, or merge as part of this handoff.
