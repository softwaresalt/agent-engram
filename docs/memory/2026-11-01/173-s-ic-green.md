---
title: "173-S GREEN implementation and quality evidence"
date: 2026-11-01
shipment_id: "173-S"
feature_id: "142-F"
task_id: "142.086-T"
phase: implementation-verified
---

## Implementation

HC: `a13c424504ae784236cdb815c8bdb61ad487011b`.
IC changes only nine declaration bits in `src/tools/capabilities.rs`.
The eight default generation-DB reads and feature-gated query_changes
declare true; all other declarations remain false.
No harness, handler, schema, admission, config or shared plan is changed.

## Final observed gates

Every Cargo command used `CARGO_TARGET_DIR=target/validation`.

| Gate | Outcome |
|---|---|
| Default registry target, three runs | GREEN, 16/16 each |
| Git-graph registry target, three runs | GREEN, 17/17 each |
| All-targets check | PASS |
| Format check | PASS |
| Default all-targets pedantic clippy | PASS |
| Approved lib/bins/registry git-graph pedantic clippy | PASS |
| Coverage completeness | PASS, 281 targets, 14 modules, zero unmapped |
| Full all-targets build with two compiler jobs | PASS, 25m58s |
| Single default-parallel `cargo dev-test --no-fail-fast` | Exit 101; 2622 passed, 1 failed, 20 ignored; 283 summaries |
| Unchanged hydration timing test isolated rerun | PASS, 1/1, 4.07s; exit 0 |

The sole suite failure was
`integration_backlog_hydration::backlog_index_100_items_under_5_seconds`.
It is a known unchanged timing gate, recorded in 1346BC60 and 7E2BE2D2.
The operator's 173-S pre-approved isolated-confirmation disposition,
alongside 773841E3, satisfies Steps 4.3 and 5 without a second suite.
This is not a zero-failure suite. No test was edited, no finding was
silently fixed, and no new deferred contract expansion was introduced.

Earlier build attempts failed under Windows memory commit pressure.
The unchanged build passed with process-local `CARGO_BUILD_JOBS=2`.
No system setting, dependency, toolchain or tracked config changed.
Test execution retained default parallelism.

## Preservation and residual risk

Test SHA-256:
9896C3F2C33D357726F7DE4E516E81AF8CE3204CAB01C9354424F9B74EA8854F.
Shared 142-F SHA-256:
42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770.
Both match HC and preflight.

Accepted residuals: D74F9442, 56CCC9F0, 3749E9A0, 773841E3,
and the known hydration timing evidence 1346BC60/7E2BE2D2.
No clean audit, deployment or current-HEAD hosted CI result is claimed.
Runtime handoff is PASS_WITH_FOLLOW_UP / READY_WITH_CONDITIONS in
`docs/closure/2026-11-01-173-s-runtime-verification.md`.
Its full-suite-pending statements are historical; this note supplies the
subsequently observed outcome.

Telemetry is disabled. Observe was evaluated: the compiler-resource
workaround is a bounded one-off, captured through compound rather than
asserted as a recurring instinct.
Next: current-HEAD local review, task completion and IC association,
feature PR and CI/P-018 gates, then authorized merge and mandatory closure.
Remain on the feature branch. The slot budget remains binding.
