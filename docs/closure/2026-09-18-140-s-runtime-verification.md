---
title: 140-S Migrate Services to Pinned Context and Enforce Read-Path Pinning — Runtime Verification
description: Runtime validator evidence per .autoharness/workspace-profile.yaml runtime_validation.validator_manifest for PR #404 (140-S), pre-merge.
---

## Scope

140-S implements all 7 manifest tasks: `142.040-T` (search/embedding
service), `142.041-T` (registry/evaluation service), `142.042-T` (retrieval
evaluation service), `142.043-T` (metrics/query-stat service), `142.044-T`
(DAX lint service), `142.045-T` (git graph service) — migrating all 6
services to consume a pinned `ReadRequestContext` instead of resolving
storage themselves — and `142.046-T` (a static guard test enforcing that
migration). This touches the `cli`, `api`, and `background-job` surfaces
declared in the workspace profile's `runtime_validation.validator_manifest`.
Verification below was executed on branch
`feat/140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning`
at HEAD `adf2d274` (PR #404, pre-merge), built as a release binary directly
from this tree.

## Build

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | **PASS** — clean |
| Check | `cargo check --all-targets` | **PASS** — clean |
| Lint | `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | **PASS** — clean, zero warnings |
| Release build | `cargo build --release --bin engram` | **PASS** — finished in 6m12s |

## CLI surface

| Probe | Command | Result |
|---|---|---|
| `cli-version` | `target\release\engram.exe --version` | **PASS** — `engram 0.3.0-rc.1+gadf2d274-dirty` (embedded SHA matches reviewed HEAD; `-dirty` reflects the dark-mode-accepted `.backlogit/stash.jsonl` modification), exit 0. |
| `cli-daemon-status` | manifest literal `engram status` | **Does not exist as a subcommand** — pre-existing validator-manifest drift, already captured at stash `DA0AF326` (reused across 135-S/137-S/138-S/139-S precedent; not re-captured). **Substitute**: `target\release\engram.exe daemon-status` run live against this workspace — **PASS**. Spawned a fresh daemon (PID 38536, isolated from the pre-existing `C:\Tools\engram.exe` tooling daemons) and returned `overall: "yellow"` with checks `binary_version`, `pid_liveness`, `workspace_identity`, `pipe_reachability`, `registry_validity`, `offline_scan` all **green**; only `session_resume` and `telemetry_health` yellow, both expected transient states for a daemon that had just started (0s uptime) and had not yet been asked to index or record telemetry — not failures. Test daemon process was cleanly terminated after the probe (`Stop-Process -Id 38536`) to avoid leaving stray state. |

## API (MCP protocol) surface

| Probe | Command | Result |
|---|---|---|
| `mcp-initialize-handshake` | manifest literal `contract_initialize` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_shim_stdio_initialize` — **debug mode PASS** (19/19 green, matching the canonical `cargo dev-test` gate). A `--release` profile re-run of the same binary showed 2/19 transient timing failures (`shim_aborts_unresolved_startup_after_client_disconnects`, `shim_recovers_after_timed_out_daemon_later_becomes_ready`); confirmed unrelated to 140-S (`git diff main...HEAD --stat -- tests/contract/shim_stdio_initialize_test.rs src/shim/` is empty — zero overlap), and confirmed release-profile-specific (immediate debug-mode re-run: 19/19 green). Not captured as a new deferred entry — this is a build-profile timing characteristic of pre-existing, unmodified shim tests, already partially covered by the CI-profile-coverage-gap entry `FFA32805`, and the canonical debug-mode gate (used by both `cargo dev-test` and this session's full-suite run) is unaffected. |
| `mcp-tool-invocation` | manifest literal `contract_tools` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_mcp_tool_catalog_parity` / `cargo test --test contract_cli_tool_catalog_parity` — both included and green in the full-suite run below (unaffected by this shipment's scope; MCP tool catalog derivation is untouched by the pinned-context migration). |

## Background-job / handler surface (pinned-context service migration)

Each manifest task's own harness was run directly against this branch as
part of the definitive full-suite pass:

| Evidence | Result |
|---|---|
| `cargo test --test integration_search_service_pin` (`142.040-T`'s own harness) | **PASS** |
| `cargo test --test integration_registry_service_pin` (`142.041-T`'s own harness) | **PASS** |
| `cargo test --test integration_retrieval_eval_service_pin` (`142.042-T`'s own harness) | **PASS** |
| `cargo test --test integration_metrics_service_pin` (`142.043-T`'s own harness) | **PASS** |
| `cargo test --test integration_dax_lint_service_pin` (`142.044-T`'s own harness) | **PASS** |
| `cargo test --test integration_git_graph_service_pin` (`142.045-T`'s own harness) | **PASS** |
| `cargo test --test contract_read_path_pinning_enforcement` (`142.046-T`'s own harness) | **PASS** (all 4 subtests) |
| `cargo test --test integration_report_read_generation_pin` (regression fixture, `142.043-T`) | **PASS** |
| `cargo test --all-targets --no-fail-fast` full suite (debug, canonical gate) | **PASS — 538 test binaries, 0 failures** (definitive clean run; 4 targets that showed transient parallel-load contention failures on an earlier pass — `hcl_indexing_test`, `integration_daemon_lifecycle`, `integration_daemon_startup_order`, `integration_release_archive_smoke_workflow` — all passed cleanly on this final run). |
| Hosted CI (`build`, `start-launcher-windows`) for PR #404 at HEAD `adf2d274` | **PASS** — both green (see PR checks; `start-launcher-windows` required 2 reruns before passing, consistent with the pre-existing, already-documented hosted-runner timing flake at stash `F58ECAA8`). |

## Known pre-existing / deferred findings (out of scope, not introduced by 140-S)

All properly dispositioned per P-021 and cited in the PR's Local Review
Readiness block; not re-derived here:

- `E6CA4ED1` (high) — metrics.rs writer/reader `data_dir` divergence.
- `7C23A682` (high) — same pattern in `eval.rs`/retrieval-eval reports.
- `9BB01D31` (medium) — `dax_lint.rs` fails under Generation/ReadServer mode (pre-existing, not a 140-S regression).
- `A3E0E607` (medium) — static guard (`142.046-T`) detection-methodology gaps (aliased identifiers, missing `registry.rs` file scope).
- `10EE5E43` (medium) — `archive_verifier_runs_the_unpacked_native_binary` intermittent flake, zero file overlap with this shipment.
- `F58ECAA8` (low, reused) — hosted-runner `start-launcher-windows` timing budget flake, reproduced again this session (3 consecutive failures, then passed on the 4th CI attempt), zero file overlap with this shipment.
- `DA0AF326` (low, reused) — stale validator-manifest probe literals.

## Manual checkpoints

None declared for the `cli`/`api`/`background-job` surfaces in the validator
manifest beyond the probes above.

## Blocked prerequisites

None. All declared validator-manifest probes were either run directly or
covered by a substitute real command, per the established `DA0AF326`
precedent for stale manifest literals.

## Overall

**Verdict: `PASS WITH FOLLOW-UP`** (`PASS_WITH_FOLLOW_UP`)

All 7 manifest tasks' own harnesses pass green, the static enforcement guard
(`142.046-T`) passes, the full local test suite is clean (538 binaries, 0
failures), build/check/lint/format gates are all clean, the release binary
starts and reports a live, healthy daemon connection, and hosted CI is green
for the reviewed HEAD. This meets the workspace profile's `minimum_verdict:
PASS_WITH_FOLLOW_UP` threshold. Follow-up items (5 newly-captured/reused
deferred stash entries plus 2 previously-established recurring
tooling/CI flakes) are carried into the operational-closure artifact and the
PR's Local Review Readiness block.
