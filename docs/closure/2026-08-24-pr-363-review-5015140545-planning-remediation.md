---
title: PR 363 review 5015140545 planning remediation
type: review-remediation
doc_type: closure
source: Copilot review 5015140545 on PR 363
source_commit: 2a80fd27342fbc614efc58c830b81bfa59301b7f
date: 2026-08-24
status: planned-remediation-complete
---

# PR 363 review 5015140545 planning remediation

## Scope

This is planning and backlog closure only. No application source, test, manifest, lockfile, configuration, build, linter, shipment claim, shipment close, merge, amend, force push, or PR 362 mutation is part of this remediation.

## Findings and dispositions

| Thread | Finding | Planning disposition |
|---|---|---|
| `PRRT_kwDORJEduc6b8O89` | Original sequence could not reach a compiling RED before implementation. | Replaced by compile-neutral U1, graph U2, compile-baseline U3, explicit seam U4, and compiling provider RED U5. |
| `PRRT_kwDORJEduc6b8UJJ` | 131.001-T could only fail during production compilation. | 131.001-T now compiles with the feature disabled and fails runtime assertions over isolated `cargo tree` and `cargo check --lib` subprocesses. |
| `PRRT_kwDORJEduc6b8UIv` | Flush and shutdown inherited an operator-controlled SDK timeout. | U6 owns a five-second application constant wired after batch defaults; U5 proves deterministic cancellation; U10 and U11 prove exactly-once cleanup and failure reporting. |
| `PRRT_kwDORJEduc6b8O9M` | 021-D lacked semantic archive provenance. | Added original queue path and archived status fields plus exact original stash, blocked feature/review/shipment, active replacement, and failed-harvest explanation. |
| `PRRT_kwDORJEduc6b8O9f` | 022-D lacked semantic archive provenance. | Added the parallel provenance record for original stash 7B15B447 and active replacement 172AE8CE. |

## Reviewed execution shape

Feature `131-F` now has thirteen linear tasks. All estimates are 45 to 105 minutes. `131.001-T` remains one test file and 75 minutes. No task exceeds two files or evidence surfaces, four functions, three scenarios, one skill domain, or one atomic milestone.

Shipment `125-S` contains the parent plus all thirteen tasks and remains queued and unclaimed. Blocked shipments `126-S` through `129-S` and their dependencies remain unchanged.

## Timeout and operational closure

Production timeout ownership belongs to `src/server/observability.rs` through `OTLP_EXPORT_TIMEOUT = Duration::from_secs(5)` and `BatchConfigBuilder::with_max_export_timeout`. The environment cannot override the final value. Flush and shutdown each receive that phase cap, yielding a declared ten-second maximum for two sequential phases. The deterministic test uses paused Tokio time and a 25 ms injected test value to prove exporter-future cancellation without wall-clock sleep.

Ship owns the post-deploy observation window: 30 minutes or three controlled daemon exits. Healthy state is zero OTLP export, timeout, or cleanup failure records and exits below ten seconds. Any exit at ten seconds, hidden cleanup error, missing focused span, or default/all-features regression triggers disablement of `otlp-export` and revert of the owning GREEN commit or commits.

## Provenance closure

Deliberations `021-D` and `022-D` are semantic archives of accepted deliberation, not successful executable harvests. Their attempted planning targets `133-F` / `127-S` and `132-F` / `126-S` are blocked and their review artifacts are invalidated. The original stash archive records now use reason `archived`, disposition `blocked_unverified_planning`, attempted target IDs, and sole replacement IDs. Active replacements are exactly `8C7733CE` for original `1CB366DB` and `172AE8CE` for original `7B15B447`; neither original is active.

## Review gate

Plan hardening and standard review were rerun after the redesign. Gate: **PASS** with no unresolved P0 or P1 planning finding. Cross-model and intercom dispatch were unavailable and are disclosed in the plan. External review threads remain unresolved until a commit with validated evidence exists.

## PR metadata proposal for Ship

Proposed title:

```text
chore(stage): queue thirteen-task OTLP repair and fail-close identity plans
```

Proposed body facts: feature `131-F`; tasks `131.001-T` through `131.013-T`; fourteen-item parent-first `125-S` roster; exact linear chain; five-second per-phase and ten-second two-phase timeout; `125-S` sole queued/unclaimed; blocked shipments unchanged; archive replacements `8C7733CE` and `172AE8CE`. Stage does not mutate PR metadata.
