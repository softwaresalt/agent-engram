---
date: 2026-07-04
type: plan-review
review_id: 072.001-R
target_plan: docs/exec-plans/2026-07-04-064-004-reactive-sync-verify-gate-plan.md
task: 064.004-T
feature: 064-F
shipment: 072-S
persona: skeptical staff engineer (daemon/reliability)
disposition: ACCEPTED (with conditions carried into the plan + Ship notes)
review_cycles: 1
---

# Plan-review — 072.001-R — 064.004-T reactive sync verify gate

## Verdict: **ACCEPTED**

The plan is correctly grounded in the real modules (line-cited), is honest about
the fact that reactive markdown reingest does not yet exist, keeps the code
reindex path untouched, is test-first, and enters an explicit **freeze-scope**
safety mode around the elevated-blast-radius daemon loop. Accepted for harvest.

## Findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| F1 | High | Scope is larger than "add a gate": markdown currently `Skip`s, so `ReingestContent` must first be *produced* (adapt_event) before it can be *gated*. Under-scoping would ship a no-op gate. | **Resolved in plan** §2 "Key realization" + §3 Parts A & B make both halves explicit. |
| F2 | High | Testing the gate by spinning the daemon would collide with the Windows-only `run_with_shutdown_v2` SQLite startup flake and produce a flaky suite. | **Resolved** — §3 Part B-hardening extracts a pure, injectable `verify_gated_reingest` helper; §4 targets the helper, not the loop. Condition C1 below makes this binding. |
| F3 | Medium | Two loop copies (v1 `run_with_shutdown:638`, v2 `run_with_shutdown_v2:1081`). Silently gating only one is a latent parity gap. | **Resolved** — §5.3 gates v2 (the live, `daemon/mod.rs:202`-dispatched path), comments v1, and defers v1 disposition to a separate item (Q2). Acceptable; do NOT refactor v1 here. |
| F4 | Medium | Content-source resolution for a single mutated md path is non-trivial; a wrong `content_type`/`source_path` would write mis-scoped content records. | **Accepted with condition C2** — longest-prefix match against `RegistryConfig.sources`, skip+log if unowned; add the `unowned_path_is_skipped` test (present in §4). |
| F5 | Low | `Deleted`/`Renamed` markdown still `Skip`s, so a deleted md leaves a stale content node until the next workspace sync. | **Accepted as-is** — parity with the code path's documented rationale; sweeping deletes reactively is out of scope for this task. Note it in Ship notes. |
| F6 | Low | Log-level choice for the non-conformant skip. | `warn!` with `path` + `findings` count (actionable, not noisy). Confirmed in §3 Part B. |

## Conditions carried to Ship (binding)

- **C1** — The verify→ingest decision MUST live in a pure/injectable helper with
  a `ReingestOutcome` return; **no** new test may spin the daemon or depend on
  daemon-startup timing (protects against the known SQLite flake).
- **C2** — Resolve md→source by longest path-prefix over `RegistryConfig.sources`,
  excluding `content_type` `code`/`backlog`; unowned → skip + `debug!`. Ship the
  `unowned_path_is_skipped` test.
- **C3** — `freeze-scope`: touch only `debounce.rs`, the v2 loop in
  `ipc_server.rs`, and the new `services` gate helper + tests. No startup/Ready,
  watcher-init, IPC-accept, PID/lifecycle, schema, CLI, or config edits.
- **C4** — Fail-safe: non-conformant/verify-err/ingest-err → log + continue;
  never `panic!`/`unwrap`/`expect`; never break the receive loop; TTL reset
  semantics unchanged.

## Open questions returned to operator (non-blocking)

- Q2 (v1 loop dead-or-live) and Q3 (extension set) from the plan — reasonable
  defaults recommended; flagged in the session report for an operator call.

## Test adequacy

Valid→ingested, and three distinct invalid classes (malformed frontmatter,
empty body, unresolved `{{...}}`) → skipped, plus unowned-path and
adapt_event unit coverage. Meets the acceptance criteria and the no-regression
requirement. **Sufficient.**
