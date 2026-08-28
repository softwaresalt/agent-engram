---
title: "131-S / 138-F readiness closure — terminal vs transient health classification"
doc_type: closure
date: 2026-08-27
shipment_id: "131-S"
feature_id: "138-F"
pr: 366
status: ready-for-operator-merge-approval
engram_status: not-probed-this-session
---

## Scope

Pre-merge operational-closure readiness for shipment `131-S` / feature
`138-F` ("Classify terminal vs transient daemon health outcomes in the shim
late-readiness recovery path"), prepared in the dedicated worktree
`.worktrees/ship-138-f-terminal-vs-transient-health-20260827` on branch
`feat/138-f-terminal-vs-transient-health-classification`, folded into PR #366
itself per policy rather than a separate pre-merge closure PR.

## Runtime verification performed

No `engram daemon-status` probe was attempted in this session — this
shipment's runtime surface (the shim's `_health` probe classification) is
exhaustively covered by the black-box contract tests in
`tests/contract/shim_stdio_initialize_test.rs`, which spawn the real
`engram shim` binary over stdio against a real platform IPC endpoint (named
pipe / Unix socket) driven by scripted responders — this **is** the runtime
verification surface for this feature, run repeatedly (5x, zero flakes) in
this session:

| Suite | Result |
|---|---|
| `cargo test --lib shim::` | 38/38 pass, 5x repeat zero flakes |
| `cargo test --test contract_shim_stdio_initialize` | 17/17 pass, 5x repeat zero flakes, wall time 6-10s (budget 20s) |
| `cargo test --test contract_shim_lifecycle` | 13/13 pass |
| `cargo dev-test` (full workspace) | 100% clean on the final commit (`6af36553`) — 0 failures |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | pass |
| `cargo audit` | pass, 14 pre-existing allowed advisories, no new advisories |

Pinned pre-existing regression guards confirmed byte-unmodified against
`origin/main` and green throughout: `shim_recovers_after_timed_out_daemon_later_becomes_ready`
(R3), `shim_serves_initialize_and_tools_list_then_degrades_tool_calls_on_daemon_failure`
and `shim_degrades_tools_call_and_exits_with_admission_failure_code_for_invalid_workspace`
(N3 x2), `shim_aborts_unresolved_startup_after_client_disconnects` (C4
pre-existing half).

## Monitoring guidance

Operator-facing monitoring guidance is shipped in this same PR at
`docs/troubleshooting.md`'s "Transient vs terminal health classification"
section and its "Agent integration contract" subsection:

- **Terminal** (`protocol_incompatible`, wire `15005`, exit `14`): a durable
  record is appended to `<workspace>/.engram/diagnostics/shim-startup-failures.jsonl`
  the first time either publisher (the request-triggered probe or the
  independent late-readiness monitor) latches it. Operators alerting on this
  file should treat a `protocol_incompatible` line as requiring a daemon
  upgrade/replacement, not a retry.
- **Transient** (`readiness_timeout`, wire `15002`, exit `11`): unchanged
  from the pre-existing contract; no new monitoring surface.
- Agents/integrators: `recoverable` is the sole retry signal;
  `retry_after_ms` must be checked for key presence, not truthiness.

## Rollback plan

No feature flag or env-gated bypass exists for this change (rejected
explicitly by the reviewed plan's hardening doc — an env-gated bypass of a
fail-closed path is itself a fail-open risk). Rollback is a plain `git
revert` of the merge commit:

1. `git revert -m 1 <merge_commit>` on `main` (single squash-free revert;
   the change is additive — a new `ShimFailureClass::ProtocolIncompatible`
   variant, wire code `15005`, exit code `14` — so reverting removes the new
   classification path and restores the pre-138-F behavior of collapsing
   `_health` probe failures into `readiness_timeout`/transient, with no
   partial-state risk since no existing wire code, exit code, or constant
   was renumbered or repurposed).
2. No data migration, schema change, or persisted-state format change is
   introduced by this shipment (the durable startup-failure record's
   four-field schema is unchanged; only an additive `failure_class` value is
   possible in it).
3. No daemon restart or process-lifecycle change is required to roll back —
   this is shim-side classification logic only.

## Residual risk / follow-up

Stash item `BE0A8B67` (recorded during the review gate) tracks three
deferred, out-of-scope-for-this-shipment findings for future Stage triage:
sanitizing post-Ready daemon IPC/tool-call error text (`domain_to_mcp`,
pre-existing and untouched by this diff), consolidating duplicated
`send_if_modified`/`ProbeFn` seam scaffolding across `transport.rs`/`mod.rs`,
and making tokio's `test-util` feature an explicit dev-dependency.

## Verdict

**Ready for operator merge approval.** All quality gates pass, the
structured review gate found no P0 findings (P1s addressed across four
Copilot review-fix cycles closed at the documented circuit breaker), CI is
green, and the current-HEAD Copilot review (commit `6af36553`) has 0
unresolved threads. Merge is explicitly gated on operator approval and has
not been requested or performed by Ship.
