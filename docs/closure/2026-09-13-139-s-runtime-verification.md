---
title: 139-S Migrate Read and Lifecycle Handlers to Pinned Generation Context — Runtime Verification
description: Runtime validator evidence per .autoharness/workspace-profile.yaml runtime_validation.validator_manifest for the post-merge closure of 139-S.
---

## Scope

139-S implements all six manifest items: `142.034-T` (core read handlers —
`map_code`, `impact_analysis`, `query_graph`, `query_changes`, `query_memory`,
`list_symbols`, `unified_search`, `get_workspace_statistics` in
`src/tools/read.rs`), `142.035-T` (report handlers), `142.036-T` (lifecycle
handlers in `src/tools/lifecycle.rs`), `142.037-T` (eval handler in
`src/tools/eval.rs`), `142.038-T` (lint handler in `src/tools/lint.rs`), and
`142.039-T` (doctor handler in `src/tools/doctor.rs`) — migrating all of them
to consume a pinned dispatch-snapshot/generation context instead of resolving
storage themselves. This touches the `cli`, `api`, and `background-job`
surfaces declared in the workspace profile's
`runtime_validation.validator_manifest`. Verification below was executed on
the `post-merge/139-s-migrate-read-and-lifecycle-handlers-to-pinned-generation-context`
branch, built directly from merge commit
`08e816394cfa1945fdf234bd77048ac867a7ea1f` (PR #393) plus this closure
branch's own non-source backlog-archival commit (`git diff --stat
08e81639..HEAD -- src/ tests/ Cargo.toml Cargo.lock` is empty — zero source
changes). The binary's embedded version string reports the local closure
branch tip (`+g7dad29c6`) rather than the bare merge-commit SHA; because no
source file differs between the merge commit and this branch, compiled and
tested behavior is equivalent to a build at
`08e816394cfa1945fdf234bd77048ac867a7ea1f` itself.

## Build

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | **PASS** — clean |
| Check | `cargo check --all-targets` | **PASS** — clean, finished in 2m28s |
| Lint | `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | **PASS** — clean, no warnings, finished in 1m38s |

## CLI surface

| Probe | Command | Result |
|---|---|---|
| `cli-version` | `target\debug\engram.exe --version` | **PASS** — `engram 0.3.0-rc.1+g7dad29c6`, exit 0. |
| `cli-daemon-status` | manifest literal `engram status` | **Does not exist as a subcommand** — pre-existing validator-manifest drift, already captured and reused from stash `DA0AF326` (not re-captured; identical finding independently reconfirmed across 135-S/137-S/138-S precedent: real subcommands are `daemon-status`, `workspace-status`, `stats`). No live daemon probe attempted against any shared-environment daemon this session. **Substitute evidence**: `integration_doctor_smoke` (3/3 green, exercises daemon-lifecycle-adjacent doctor path) and `contract_cli_tool_catalog_parity` (unchanged by this shipment, already verified green on prior closures) both cover this surface. |

## API (MCP protocol) surface

| Probe | Command | Result |
|---|---|---|
| `mcp-initialize-handshake` | manifest literal `contract_initialize` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_shim_stdio_initialize` — **PASS**, 19/19 tests green in isolation. |
| `mcp-tool-invocation` | manifest literal `contract_tools` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_mcp_tool_catalog_parity` — **PASS** (unaffected by this shipment's scope; MCP tool catalog derivation is untouched by the pinned-context migration). |

## Background-job / handler surface (pinned generation context migration)

Each manifest task's own harness was run directly against this closure
branch:

| Evidence | Result |
|---|---|
| `cargo test --test integration_core_read_generation_pin` (`142.034-T`'s own harness) | **PASS** — 2/2 tests green. |
| `cargo test --test integration_doctor_read_pin` (`142.039-T`'s own harness) | **PASS** — 3/3 tests green. |
| `cargo test --test integration_doctor_smoke` | **PASS** — 3/3 tests green. |
| `cargo test --test integration_eval_read_pin` (`142.037-T`'s own harness) | **PASS** — 2/2 tests green. |
| `cargo test --test integration_lifecycle_read_generation_pin` (`142.036-T`'s own harness) | **PASS** — 3/3 tests green. |
| `cargo test --test integration_lint_read_pin` (`142.038-T`'s own harness) | **PASS** — 2/2 tests green. |
| `cargo test --test integration_report_read_generation_pin` (`142.035-T`'s own harness) | **PASS** — 2/2 tests green. |
| **Owned-harness total** | **17/17 tests green** across all 6 manifest tasks' pin-test files plus `doctor_smoke`. |
| `cargo test --all-targets --no-fail-fast` full suite | **2467/2469 passed** on a contended, fully-parallel run — see "Known pre-existing failures" below for the two exceptions, both confirmed unrelated to 139-S. |

## Known pre-existing failures (out of scope, not introduced by 139-S)

Two tests failed during the full `cargo test --all-targets --no-fail-fast`
run; both are confirmed pre-existing, unrelated to 139-S's manifest
(`src/tools/{read,lifecycle,eval,lint,doctor}.rs`), and both were already
captured to the stash by the implementation session before this closure ran:

1. **`integration_daemon_lifecycle::t046_s050_daemon_exits_after_idle_timeout_and_restarts`**
   — failed under full-suite parallel contention; **passed cleanly in
   isolation** (`cargo test --test integration_daemon_lifecycle
   t046_s050_daemon_exits_after_idle_timeout_and_restarts`, 1/1 green,
   finished in 2.36s). Timing/contention-sensitive, not a code defect.
   Already captured at stash **`9088F47D`** (and precedent `2ED1D9BE` from
   the 138-S session) — not re-captured here.
2. **`integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`**
   — `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`; reproduces even
   in isolation on this branch. This is the same long-documented, recurring,
   pre-existing flake reproduced identically across
   `133-S`/`134-S`/`135-S`/`136-S`/`137-S`/`138-S` closures. The 139-S
   implementation session already independently confirmed this reproduces
   identically on the `origin/main` baseline **before any 139-S changes
   were applied** (clean-clone repro at commit `47eb9e1e`), and PR #393's
   hosted CI (`build`, `start-launcher-windows`) reported SUCCESS for the
   merged commit. Already captured at stash **`39049DEE`** — not
   re-captured here.

## Manual checkpoints

None declared for the `cli`/`api`/`background-job` surfaces in the validator
manifest beyond the probes above.

## Blocked prerequisites

- `engram daemon-status` (substitute for the stale manifest literal `engram
  status`) was not attempted live against any shared-environment daemon this
  session. Not a code defect introduced by 139-S; substitute evidence
  (harness test coverage) is provided above.
- The validator manifest's literal probe commands (`engram status`,
  `contract_initialize`, `contract_tools`) do not match the current CLI/test
  surface — pre-existing drift, already captured at stash `DA0AF326`, reused
  here (not re-captured).

## Overall

**Verdict: `PASS WITH FOLLOW-UP`** (`PASS_WITH_FOLLOW_UP`)

All six manifest tasks' own harnesses pass green (17/17 targeted tests across
`integration_core_read_generation_pin` (2), `integration_doctor_read_pin` (3),
`integration_doctor_smoke` (3), `integration_eval_read_pin` (2),
`integration_lifecycle_read_generation_pin` (3), `integration_lint_read_pin`
(2), `integration_report_read_generation_pin` (2)). Build, check, format, and
lint gates are all clean. The full `cargo test --all-targets --no-fail-fast`
suite passed 2467/2469 on a contended, fully-parallel run, with both
exceptions confirmed pre-existing, unrelated to 139-S, already stashed by the
implementation session, and one (`archive_verifier`) matching a
long-documented recurring flake with confirmed-green hosted CI at the merge
commit. This meets the workspace profile's `minimum_verdict:
PASS_WITH_FOLLOW_UP` threshold. Named releasability conditions are carried
into `docs/closure/2026-09-13-139-s-operational-closure.md`.
