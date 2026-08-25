---
title: PR 363 exact-head five-blocker planning remediation
type: review-remediation
doc_type: closure
source: PR 363 reviews 5015373740 and 5015447062 at b1232cc4ec95015ef337c2ffa5b4055f009960f1
date: 2026-08-24
status: published-threads-resolved
---

# PR 363 exact-head five-blocker planning remediation

## Scope

Planning/backlog only on `stage/dark-factory-cycle2-20260824-1540`. No application source, tests, Cargo manifest/lockfile, config, build, source linter, shipment claim/close, merge, amend, force push, or PR #362 mutation.

## Exact review evidence

Reviews `5015373740` and `5015447062` target exact head `b1232cc4ec95015ef337c2ffa5b4055f009960f1`.

| Thread/comment | Path | Exact finding disposition |
|---|---|---|
| `PRRT_kwDORJEduc6b8xO2` / `3849908375` | `.backlogit/queue/131.011-T.md` | SDK max export timeout is not a whole cleanup-call bound. Replaced with detached native worker plus bounded daemon wait and unknown-completion reporting. |
| `PRRT_kwDORJEduc6b8_IN` / `3849992327` | OTLP plan | Same SDK semantic defect. Pinned source is cited and every timeout/rollback claim is rewritten. |
| `PRRT_kwDORJEduc6b8_Id` / `3849992351` | OTLP plan | Thirteen tasks have exactly twelve dependency edges; corrected everywhere. |
| `PRRT_kwDORJEduc6b8_Ik` / `3849992363` | final adversarial closure | `1C2A3CB3`/`5DF94427` archive events and semantic archives now use `blocked_unverified_planning` with exact blocked/replacement IDs. |
| `PRRT_kwDORJEduc6b8_I0` / `3849992378` | daemon-key plan | Cold-start create/open gap is now a named P0 blocker with a post-create/pre-open checkpoint and no ambient/reopen path. |

Review `5015373740` also preserved three suppressed findings. The stale 125-S metadata finding is already addressed by the fourteen-item roster and twelve-edge PR body/manifest. Its two suppressed archive findings are addressed by the same durable provenance correction above. No suppressed executable-harvest claim remains.

## Cleanup design and guarantees

Pinned SDK 0.26 source shows provider cleanup iterates processors synchronously; batch processor cleanup enqueues a message and blocks on an untimed oneshot; `max_export_timeout` wraps each exporter future only.

The plan now separates:

* `OTLP_EXPORT_TIMEOUT = 5s`: each exporter future/batch only; it may drop that future.
* `OTLP_CLEANUP_WAIT_TIMEOUT = 5s`: one total monotonic deadline for the daemon to await a dedicated native cleanup worker's completion channel.

The explicit provider owner moves into exactly one detached `std::thread`. It calls `force_flush` once and calls `shutdown` once only if flush returns, including error return. On daemon deadline, the receiver is dropped and no join occurs. The result/log records last phase, limit, `completion=unknown`, and `worker_detached=true`. It does not claim SDK cancellation, return, export delivery, shutdown attempt after a non-returning flush, or resource release.

Residual behavior is explicit: queued spans, exporter I/O, SDK calls, and worker resources may remain unresolved until process exit. No Tokio `spawn_blocking` or runtime-owned join exists, so the main future can return at the deadline; normal Rust binary termination does not wait for a detached native thread and the OS reclaims process resources. Reuse in an embedding that continues running is prohibited without a reaper/process-boundary redesign.

Deterministic tests use phase-entered/release barriers and paused Tokio time: pending at 4,999 ms, timeout at 5,000 ms, then test-only release/reap. Controlled child verification allows two seconds beyond the cleanup wait to prove process exit does not join the worker, not to claim cleanup completed.

Monitoring covers export failure, worker spawn/loss/panic, cleanup failure, cleanup-wait timeout, detached residual, and total exit latency. Any failure/timeout, hidden residual, missing span, controlled child over seven seconds, or feature-gate regression disables `otlp-export` and reverts owning GREEN commits.

## Corrected Cargo commands

Outer compile-neutral RED (OTLP intentionally disabled):

```text
cargo test --no-default-features --features cozo-backend --test otlp_feature_compile_contract_test -- --nocapture
```

Nested and feature-enabled commands:

```text
cargo tree --no-default-features --features cozo-backend,otlp-export
cargo check --no-default-features --features cozo-backend,otlp-export --lib
cargo test --no-default-features --features cozo-backend,otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture
cargo test --no-default-features --features cozo-backend,otlp-export otlp_daemon_red -- --nocapture
cargo test --no-default-features --features cozo-backend,otlp-export --bin engram otlp_cleanup_red -- --nocapture
```

Read-only locked/offline Cargo metadata selected `cozo`, `cozo-backend`, `otlp-export`, and all four OpenTelemetry optional dependencies. No Cargo build/check/test/linter command was run by Stage.

## Provenance correction

| Original | Deliberation | Disposition | Blocked feature/review/shipment | Sole active replacement |
|---|---|---|---|---|
| `1C2A3CB3` | `024-D` | `blocked_unverified_planning` | `134-F` / `134.001-R` / `128-S` | `721A42F0` |
| `5DF94427` | `025-D` | `blocked_unverified_planning` | `135-F` / `135.001-R` / `129-S` | `BD5DD62A` |

Archive JSONL uses reason `archived`, `attempted_harvest_artifact_id`, exact blocked IDs, and one replacement ID. `harvested_artifact_id` is absent. Deliberation, feature, review, closure, blocker decision, and session-memory wording all state that no successful executable harvest occurred.

## Daemon-key blocked plan

The corrected plan carries one private `EngramAuthority` state through existing or vacant child selection. Existing-child paths open once before any probe. Cold start may transition from retained vacancy to created authority only through a pinned safe primitive/protocol that returns or proof-preservingly retains the exact created object. Create-then-open, ambient access, named helper reopen, retry-until-stable, and post-hoc checking of a potentially substituted object are forbidden.

`132.002-T` owns a deterministic checkpoint immediately after creation and before any first named open/publication. Replacement state must never be read or written; code retains the exact created object or fails closed. No safe portable primitive is yet proven, so standard review is `FAIL / BLOCKED`; adversarial review is still failed/unverified; `132-F`, all tasks, review `132.001-R`, and shipment `126-S` remain blocked.

## Task graph and shipment

No extra 131 task is required. Cleanup isolation remains one file/domain task at 115 minutes. All thirteen estimates are 45-115 minutes and all tasks remain at most two files/evidence surfaces, four functions, three scenario groups, one domain, and one atomic milestone.

`125-S` roster is `131-F` plus `131.001-T` through `131.013-T` (fourteen items). The exact linear task chain has twelve dependency edges. `125-S` is sole queued and unclaimed; no shipment is active; `126-S` through `129-S` remain blocked.

## Hardening and standard review

OTLP exact-head hardening: required and satisfied. Local constitution, Rust/API, architecture, scope, test, operations, learnings, and security lenses return PASS with no unresolved P0/P1 after the worker-isolation redesign.

Daemon-key exact-head hardening: required and blocked. Standard review returns FAIL on the unproven exact-create-and-retain primitive, preserving blocked status. Intercom/cross-model dispatch was unavailable and is disclosed.

## Validation before publication

* Exact worktree started at requested `b1232cc4ec95015ef337c2ffa5b4055f009960f1`; remote matched.
* Target backlog sync indexed 1,126 artifacts with zero parse failures.
* Twenty-four modified backlog Markdown artifacts pass target doctor.
* Full doctor exits zero; only 43 historical `archived_from_self_ref` and 38 historical `missing_shipped_event` advisories remain.
* Shipment query: queued `[125-S]`, active `[]`, blocked `[126-S,127-S,128-S,129-S]`.
* Custom planning validator: 13 tasks, 12 edges, 14 roster items, exact parents/statuses, estimates <=120, provenance pairs, references, final newlines, fences, templates, JSON/JSONL, and planning-only scope pass.
* Global docline lint still reports 771 pre-existing repository-wide findings (beginning with frontmatter-less `AGENTS.md`); changed planning docs pass targeted frontmatter/reference/structure checks.
* `git diff --check` passes. No source/test/Cargo/config file is changed.

## Publication and thread state

Substantive commit `7068ecb43b3b8cb28a0b36fffd1c13fe7b84ea2c` was pushed normally. Evidence replies were posted to all five exact bot comments and threads `PRRT_kwDORJEduc6b8xO2`, `PRRT_kwDORJEduc6b8_IN`, `PRRT_kwDORJEduc6b8_Id`, `PRRT_kwDORJEduc6b8_Ik`, and `PRRT_kwDORJEduc6b8_I0` were resolved. GraphQL re-query at that head returned zero unresolved threads. Reply review IDs are `5015614628`, `5015614634`, `5015614639`, `5015614643`, and `5015614674`. No PR #362 operation occurred.
