# Ship 142-S — Option 2 Policy Reachability

- **Date**: 2026-09-24
- **Shipment / feature**: `142-S` / `142-F`
- **Branch**: `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs`
- **Outcome**: The docs-only harness blocker has no honest, supported in-workspace bypass within the authorized manifest scope. Step 2 remains blocked.

## Startup and scope

- `.autoharness/backlog-registry.yaml` is present. Backlog CLI fallback was available; `backlogit sync` succeeded before semantic reads. The MCP backlog surface was unavailable, so CLI fallback was used.
- Unfiltered `backlogit checkpoint list` returned 30 records with no validation/quarantine/required-field anomalies and no active Ship checkpoint. The Stage checkpoint `checkpoint-20260924-185101.json` was resolved. Nothing was restored.
- `142-S` is `active`, on its matching feature branch, with one attached worktree. All six manifested task artifacts are `active`; five code tasks carry `harness-ready`, and `142.059-T` carries `harness-not-applicable` with three documentation verification checks.
- Existing dirty backlog, checkpoint, plan, memory, and harness changes were preserved. No task/shipment state, manifest, claim, commit, push, or PR was changed.

## Option 2 reachability finding

- The installed Ship Step 2 requires a passing test harness and `harness-ready` for every task; it does not define a docs-only exception.
- The installed `harness-architect` skill likewise requires compilable harnesses, expected red-phase failures, and `harness-ready`; it defines no docs-only verification disposition.
- `.autoharness/config.yaml` has no active `lifecycle_hooks` block. The installed first-party validation-gates schema supports only `pre_execution` and `pre_task_completion.validation_gates`. Those checks can gate task completion, but cannot change Ship Step 2's harness/readiness requirement or recognize `harness-not-applicable`.
- Therefore adding a docs validation gate or using the three task checks would not satisfy Step 2. Changing generated policy copies/config to invent that exception here would exceed the current manifest authorization and would not establish a durable supported path. No labels or evidence were falsified.
- The Stage-authored Option 2 plan at `docs/exec-plans/2026-09-24-docs-only-verification-gated-harness-disposition-plan.md` remains **review-failed / circuit-open**. It was not rerun or represented as approved. No alternative approved upstream policy edit was available in this session.

## In-scope F54 harness correction

- Task `142.058-T`'s active acceptance forbids hard-coding the absence of a CLI surface for `get_retrieval_eval_report`. The stale negative assertion was removed from `tests/contract/read_server_cli_mcp_parity_test.rs`; declared-surface iteration and the MCP-presence assertion remain.
- `cargo check --all-targets` — **PASS**.
- `git diff --check` — **PASS**.
- `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` — compiled and failed all three tests only at the intended F54 `Worker:` stubs; **expected red phase confirmed**.
- No production code or backlog metadata was changed. The independent CLI parity follow-on remains separate and was not started.

## Stop and handoff

Step 2 remains blocked solely because `142.059-T` has no test harness and the installed Ship policy has no verification-gated docs-only path. Do not start implementation, mark it `harness-ready`, alter shipment membership, return/abandon the active shipment, or proceed to PR work.

Next required action: Stage/operator must route a **new, freshly reviewed** upstream autoharness policy change (do not retry the circuit-open review) that adds a documented docs-only disposition with explicit verification evidence and tests it end-to-end in the Ship gate. The change must cover the source templates for Ship/workflow policy and the harness/build skills, then be merged and installed/tuned before resuming `142-S`. If that upstream action cannot be authorized, the operator must provide an approved disposition for the active shipment; Ship must not invent one.

No merge or admin fallback approval exists. The feature branch and all pre-existing workspace changes remain untouched.
