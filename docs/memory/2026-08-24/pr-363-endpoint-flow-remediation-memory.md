---
title: "PR 363 endpoint-flow remediation memory"
type: session-memory
doc_type: memory
source: "Copilot reviews 5014929105 and 5014953024"
date: 2026-08-24
agent: stage
branch: stage/dark-factory-cycle2-20260824-1540
pull_request: 363
starting_head: 9dcd33f5e49583f8138f4896b70c89c00251e25f
---

# PR 363 endpoint-flow remediation memory

## Outcome

Stage remediated the executable-width blocker without source, test, manifest, lockfile, config, shipment claim/close, merge, amend, force-push, or blocked-security mutation. Feature `131-F` now has eight queued tasks, and `125-S` remains the sole queued, unclaimed shipment. This record supersedes the current-roster and current-index-count assertions in earlier PR #363 memories; their commit-specific historical evidence remains unchanged.

## Source and configuration flow discovered

At the requested starting HEAD, `src/bin/engram.rs::Cli` parses `GlobalFlags`; `GlobalFlags` has no OTLP endpoint. `Command::Daemon` is a unit variant that calls `engram::init_tracing(daemon_log_format())` before extracting the workspace and awaiting `engram::daemon::run`. `src/config/mod.rs::Config::otlp_endpoint` is dead because `Config::parse` has no production caller. `PluginConfig::load` runs inside `daemon::run`, after tracing starts, and has no endpoint. `src/shim/lifecycle.rs` starts `engram daemon --workspace <path>` and inherits process environment. `src/lib.rs::init_tracing` accepts only `LogFormat`; `src/server/observability.rs::build_otlp_layer` has no caller and drops its local provider.

The reviewed design makes `Command::Daemon` the canonical boundary: daemon-only `--otlp-endpoint` with `ENGRAM_OTLP_ENDPOINT` fallback passes one typed `Option<&str>` into a daemon-only initializer. Explicit flag precedence is supplied by clap; the normal shim path uses inherited environment and needs no lifecycle edit. Provider construction accepts that supplied argument and cannot reread hidden config.

## Backlog and plan changes

- Updated `131-F` and `131.001-T` through `131.007-T`.
- Created `131.008-T` under parent `131-F`.
- Exact chain: `131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T -> 131.006-T -> 131.007-T -> 131.008-T`.
- Exact `125-S` roster: `131-F`, then all eight tasks in dependency order.
- `131.003-T` owns endpoint propagation; `131.004-T` provider construction; `131.005-T` attachment/retention; `131.006-T` cleanup; `131.007-T` runtime proof; `131.008-T` closure.
- The decision and plan now record the actual source/config/CLI/daemon flow, rerun hardening, and standard review PASS.

## Widths

| Task | Domain | Files/surfaces | Functions | Scenarios/groups | Estimate |
|---|---|---:|---:|---:|---:|
| 131.001-T | RED test harness | 2 | 4 | 3 | 105m |
| 131.002-T | Cargo graph | 2 | 0 | 1 | 45m |
| 131.003-T | daemon config propagation | 2 | 4 | 3 | 95m |
| 131.004-T | provider construction | 1 | 4 | 3 | 90m |
| 131.005-T | attachment/retention | 2 | 4 | 3 | 105m |
| 131.006-T | cleanup/error propagation | 1 | 4 | 3 | 105m |
| 131.007-T | runtime verification | 2 | 0 | 3 | 80m |
| 131.008-T | closure evidence | 2 | 0 | 3 | 90m |

## Continuity and validation

- Backlog sync: 1,121 artifacts, zero parse failures.
- Immutable compact baseline: `git ls-tree -r -l 9dcd33f5e49583f8138f4896b70c89c00251e25f` yields memory 149 files / 443,643 bytes; plans 71 / 1,148,248; closure 112 / 826,421.
- Target doctor passes `125-S`, `131-F`, and all eight tasks.
- Full doctor has only 43 historical `archived_from_self_ref` and 38 historical `missing_shipped_event` advisories.
- Hierarchy, exact seven dependency edges, nine-item parent-first roster, sole queued shipment, no active shipment, and archived stash provenance pass.
- Plan, decision, and compact-context authoring-frontmatter lint pass.
- Source paths and named current symbols/call sites pass targeted existence checks.
- No application source, test, manifest, lockfile, configuration, or blocked shipment changed; no build/test suite/linter ran.

## PR and review handoff

The historical eight-task title/body proposal from this checkpoint is withdrawn and must not be applied. Current copy-ready metadata is owned by `docs/closure/2026-08-25-pr-363-review-5015710467-remediation.md`: title `chore(stage): queue 13-task OTLP repair; keep four identity plans blocked`; tasks `131.001-T` through `131.013-T`; `131-F` plus thirteen tasks (fourteen roster items); exactly twelve task edges; 1,126 indexed artifacts; sole queued/unclaimed `125-S`; blocked `126-S` through `129-S`. Stage does not mutate live PR title/body. The review-5015710467 closure also records safe child-process environment tests and actual SDK provider retention semantics.

## Next steps

Ship must not claim `125-S` from this planning branch or from review alone. Claim requires the exact final reviewed PR #363 head integrated into `origin/main`, zero unresolved/pending/adverse exact-head review state, no active competing shipment, and the exact roster/dependency chain intact.
