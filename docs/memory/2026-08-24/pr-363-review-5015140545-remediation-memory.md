---
title: PR 363 review 5015140545 planning remediation memory
type: session-memory
doc_type: memory
source: Copilot review 5015140545
source_commit: 2a80fd27342fbc614efc58c830b81bfa59301b7f
date: 2026-08-24
agent: stage
branch: stage/dark-factory-cycle2-20260824-1540
pull_request: 363
superseded_by: PR 363 exact-head reviews 5015373740 and 5015447062
---

# PR 363 review 5015140545 planning remediation memory

## Outcome

Stage retrieved the exact five unresolved threads and changed planning/backlog artifacts only. No application source, test, manifest, lockfile, configuration, build, test suite, source linter, shipment claim, shipment close, merge, amend, force push, or PR 362 mutation occurred.

The OTLP plan now has a compile-neutral initial RED, a separate compile-baseline GREEN, an explicit behavior-neutral seam, and three later compiling RED boundaries before provider, daemon, and cleanup behavior. Application timeout ownership, deterministic cancellation, error reporting, monitoring, and rollback are explicit. This record supersedes current-roster, task-count, RED-sequence, timeout, and proposed-PR-metadata facts in earlier PR 363 memories; their commit-specific historical evidence remains unchanged.

## Exact review threads

- `PRRT_kwDORJEduc6b8O89`: plan harness could not compile before implementation.
- `PRRT_kwDORJEduc6b8UJJ`: 131.001-T failed during production compilation rather than a behavior assertion.
- `PRRT_kwDORJEduc6b8UIv`: OTLP cleanup inherited the SDK or operator environment timeout.
- `PRRT_kwDORJEduc6b8O9M`: 021-D lacked archive source provenance.
- `PRRT_kwDORJEduc6b8O9f`: 022-D lacked archive source provenance.

## Backlog decisions

Feature `131-F` has thirteen queued child tasks in one exact chain:

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T
```

Shipment `125-S` has the parent-first fourteen-item roster and remains the sole queued, unclaimed shipment. Blocked shipments `126-S` through `129-S` and their dependencies were not changed.

Current task estimates are: 75, 45, 75, 90, 100, 105, 105, 95, 105, 105, 115, 90, and 90 minutes. Every task declares one skill domain and atomic milestone and stays within two files or evidence surfaces, four functions, and three scenarios.

## RED and timeout decisions

The exact RED commands are recorded in the plan and tasks:

```text
cargo test --no-default-features --features cozo-backend --test otlp_feature_compile_contract_test -- --nocapture
cargo test --no-default-features --features cozo-backend,otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture
cargo test --no-default-features --features cozo-backend,otlp-export otlp_daemon_red -- --nocapture
cargo test --no-default-features --features cozo-backend,otlp-export --bin engram otlp_cleanup_red -- --nocapture
```

The first test target compiles with `otlp-export` disabled and imports no OTLP production symbol. It fails runtime assertions around isolated `cargo tree` and `cargo check --lib` subprocess results. The later REDs compile against U4 interfaces that already exist.

Exact-head correction: `OTLP_EXPORT_TIMEOUT = 5s` applies to each exporter future only; it does not bound synchronous `force_flush()` or `shutdown()`. The current plan moves the explicit owner into one detached native worker and gives the daemon one five-second `OTLP_CLEANUP_WAIT_TIMEOUT` for the worker completion channel. Timeout reports completion unknown and detached residual; it does not claim cancellation. If flush returns, shutdown is attempted even on error; if flush never returns, shutdown is not falsely reported. Clean and combined failures preserve return/diagnostic precedence.

## Archive provenance

`021-D` and `022-D` now have `archived_from` and `archived_status: accepted`, with current status `archived`. Their notes preserve exact original stash identity/text and explain that targets `133-F` / `133.001-R` / `127-S` and `132-F` / `132.001-R` / `126-S` are blocked and unverified. The original stash archive records use reason `archived`, disposition `blocked_unverified_planning`, attempted target IDs, and sole replacement IDs. They do not claim successful executable harvest.

Active replacements remain exactly `8C7733CE` for original `1CB366DB` and `172AE8CE` for original `7B15B447`. Neither original exists in the active stash.

## Planning gates

Plan hardening reran for external export, provider lifetime, timeout ownership, synchronous cleanup, monitoring, and rollback. Standard local persona review reran for constitution, Rust/API, architecture, scope, tests, operations, learnings, and external-boundary security. Gate: PASS with no unresolved P0 or P1. Cross-model and intercom dispatch were unavailable and disclosed.

## Files modified

- `.backlogit/archive/021-D.md`, `022-D.md`, and `stash.jsonl`
- `.backlogit/queue/125-S.md`, `131-F.md`, and `131.001-T` through `131.013-T`
- OTLP plan and decision
- workspace identity blocker decision and adversarial closure provenance wording
- PR 363 planning-remediation closure and this memory
- structured backlogit memory

## Validation

- backlog sync: 1,126 artifacts, zero parse failures
- exact 12 dependency edges and 14-item shipment roster: pass
- shipment state: only `125-S` queued; no active shipment; `126-S` through `129-S` blocked
- 17 target doctor checks: pass after one transient concurrent target-read retry
- full doctor: no new duplicate, orphan, partial-mutation, or root-conflict finding; only 43 historical archive self-reference and 38 historical shipped-event advisories
- changed planning Markdown: YAML/frontmatter, final newline, balanced fence, unresolved-template, and 31 reference checks pass
- archive stash JSONL: 182 lines parse
- width, exact RED command, timeout ownership, roster, dependency, and stash replacement checks pass
- scope and `git diff --check`: planning/backlog only and clean

No Cargo build, check, test, or linter command from the planned RED sequence was executed because Stage is planning-only. Read-only Cargo metadata validates the corrected feature selection.

## Compact-context assessment

The mandatory assessment counted 150 memory files, 71 plans, and 113 closure files. Current PR 363 plans and memories support queued or blocked work and cannot be compacted. Broad historical compaction would violate the bounded remediation scope. Files compacted: 0. Decided plans created: 0. Active artifacts preserved.

## Publication

Substantive planning commit `7f36eed8de3f4fb7c0335497da3652636151f0b3` was pushed normally. PR 363 reflected that commit before thread closure. All five exact review threads received path and design evidence replies and are resolved; the post-resolution GraphQL check reported zero unresolved threads. The branch remained clean and the remote matched the local commit before this final continuity record.

## Next steps

Ship may apply the proposed PR title/body facts, obtain any required exact-final-head review, and later claim only after every 125-S guard passes. The current PR title/body and backlog roster describe thirteen tasks plus parent and exactly twelve task dependency edges. Exact-head cleanup semantics and provenance still require the final reviewed commit before claim.
