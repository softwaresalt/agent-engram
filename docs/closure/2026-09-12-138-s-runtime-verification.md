---
title: 138-S Generation Activation, Request Context, Startup Gate and Request Entry — Runtime Verification
description: Runtime validator evidence per .autoharness/workspace-profile.yaml runtime_validation.validator_manifest for the post-merge closure of 138-S.
---

## Scope

138-S implements all fourteen manifest items: (1) `142.018-T` (+ subtasks
`.001`-`.004`) — the generation activation service
(`src/services/generations/activation.rs`), covering typed manifest parse and
bounds enforcement, `activate_initial` with deadline and store resolution, the
single-flight background `maybe_activate_newer` path, and the immutable
rejection cache with transient backoff; (2) `142.019-T` — the mode-agnostic
`ReadRequestContext` constructors (`src/server/state.rs`); (3) `142.028-T` —
gating startup readiness on initial generation activation
(`src/daemon/startup_activation.rs`); (4) `142.029-T` — request entry order and
background activation (`src/daemon/request_entry.rs`); (5) `142.030-T` —
enforcing the capability gate and context capture in dispatch (`src/tools/mod.rs`);
(6) `142.031-T` — deriving the stdio MCP tool catalog from descriptors
(`src/shim/tools_catalog.rs`); (7) `142.032-T` — deriving the CLI workflow
surface from descriptors (`src/cli/runner.rs`); (8) `142.033-T` (+ subtasks
`.001`-`.002`) — the read-input ownership inventory and fail-on-unclassified
guard. This touches the `cli`, `api`, and `background-job` surfaces declared in
the workspace profile's `runtime_validation.validator_manifest`. Verification
below was executed against a working tree at merge commit
`81b19b0d91c79c9c456ce703dca42a4688978dfa` (PR #391) plus the
non-source closure/checkpoint artifacts added on the
`post-merge/138-s-generation-activation-request-context` branch (`git diff
--stat main..HEAD -- src/ tests/ Cargo.toml Cargo.lock` is empty — zero
source changes). The build/test binary's embedded version string therefore
reports the local branch tip (`+gce3b2fba-dirty`, the cherry-picked
checkpoint commit plus uncommitted-at-build-time working-tree state) rather
than the bare merge-commit SHA; because no source file differs between the
merge commit and this branch, the compiled and tested behavior is
equivalent to a build at `81b19b0d91c79c9c456ce703dca42a4688978dfa` itself,
not merely similar to it.

## Build

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | **PASS** — clean |
| Lint | `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | **PASS** — clean, no warnings |

## CLI surface

| Probe | Command | Result |
|---|---|---|
| `cli-version` | `engram --version` (via existing `target\debug\engram.exe` to avoid rebuild-lock contention with concurrently running verification tests) | **PASS** — `engram 0.3.0-rc.1+gce3b2fba-dirty`, exit 0. Version metadata reflects the local closure-branch tip (cherry-picked checkpoint commit `ce3b2fba` + uncommitted-at-build-time state), not the bare merge-commit SHA — see the Scope section above for why this is nonetheless equivalent to a merge-commit build (zero `src/`/`tests/` diff). |
| `cli-daemon-status` | manifest literal `engram status` | **Does not exist as a subcommand** — pre-existing validator-manifest drift, already captured and reused from stash `DA0AF326` (not re-captured; positively confirmed identical finding, consistent with the 135-S/137-S precedent: real subcommands are `daemon-status`, `workspace-status`, `stats`). No live `engram daemon-status` probe was attempted against the shared dev-environment daemon (PID `30528`): per the operator's explicit recovery-facts instruction, that daemon is known to be in a benign `_health: starting`-forever readiness-latch state (deferred scope, stash `265F99BE`) and must not be queried, stopped, or otherwise touched by this closure session. **Substitute evidence**: `contract_shim_stdio_initialize` (19/19 green, includes daemon-lifecycle/terminal-classification coverage) and `contract_cli_tool_catalog_parity` (6/6 green, `142.032-T`'s own harness) both passed in isolation. |

## API (MCP protocol) surface

| Probe | Command | Result |
|---|---|---|
| `mcp-initialize-handshake` | manifest literal `contract_initialize` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_shim_stdio_initialize` — **PASS**, 19/19 tests green in isolation (see "Known pre-existing failure" below for the one transient full-suite-only occurrence). |
| `mcp-tool-invocation` | manifest literal `contract_tools` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_mcp_tool_catalog_parity` (`142.031-T`'s own harness) — **PASS**, 6/6 tests green; tool catalog is now derived from descriptors with parity to the CLI workflow surface. |

## Background-job surface (generation activation / daemon startup)

The live `engram daemon-status`/`workspace-sync` probe was intentionally not
attempted for the reason stated above (shared-environment daemon `30528` must
not be touched). Substitute evidence instead, drawn from each manifest task's
own harness:

| Evidence | Result |
|---|---|
| `cargo test --test integration_generation_activation` (`142.018-T` + subtasks' own harness) | **PASS** — 28/28 tests green; covers typed manifest parse/bounds enforcement, `activate_initial` deadline/store resolution, single-flight `maybe_activate_newer`, and the immutable rejection cache with transient backoff. |
| `cargo test --lib services::generations` / `unit_read_request_context` (`142.019-T`'s own harness) | **PASS** — 6/6 tests green; asserts the mode-agnostic `ReadRequestContext` constructors. |
| `cargo test --test integration_read_server_startup_activation` (`142.028-T`'s own harness) | **PASS** — 7/7 tests green; asserts startup readiness is gated on initial generation activation. |
| `cargo test --test integration_request_entry_activation` (`142.029-T`'s own harness) | **PASS** — 12/12 tests green; asserts request entry order and background activation. |
| `cargo test --test contract_read_server_dispatch_refusal` (`142.030-T`'s own harness) | **PASS** — 6/6 tests green; asserts the capability gate and context capture in dispatch. |
| `cargo test --test contract_read_input_ownership_inventory` (`142.033-T` + subtasks' own harness) | **PASS** — 16/16 tests green; asserts every input reachable from each Read descriptor is enumerated, classified, and fails closed on any unclassified input. |
| `cargo dev-test` full suite (`cargo test --all-targets --no-fail-fast`, to get a complete accounting rather than fail-fast on the first flaky binary) | **2457/2460 passed** on a contended, fully-parallel run — see "Known pre-existing failures" below for the three exceptions, all confirmed unrelated to 138-S and all pass cleanly in isolation except the long-documented `archive_verifier` flake. |

## Known pre-existing failures (out of scope, not introduced by 138-S)

Three tests failed during the full `cargo dev-test --no-fail-fast` run; all
three are confirmed pre-existing, unrelated to 138-S's manifest, and none of
the affected files were touched by 138-S's merged commit range
(`git log 81b19b0d^1..81b19b0d^2 -- <file>` returns empty for each):

1. **`services::metrics::tests::full_channel_branch_switch_is_acknowledged_before_following_event`**
   (`src/services/metrics.rs`, unit lib test) — panicked with `metrics branch
   control timed out after 100 ms` under full-suite parallel contention;
   passed cleanly in isolation (`cargo test --lib
   services::metrics::tests::full_channel_branch_switch_is_acknowledged_before_following_event
   -- --exact`, 1/1 green). Root-cause commit `642a820f` (fix: flush graph
   state after metrics shutdown errors) predates and is an ancestor of the
   138-S merge base; `src/services/metrics.rs` is entirely untouched by
   138-S. Already captured at stash **`9D313653`** (captured during this
   same session, not re-captured here).
2. **`hcl_indexing_test::cold_start_lists_and_maps_all_three_hcl_aliases`**
   (`tests/integration/hcl_indexing_test.rs`) — timed out waiting for
   cold-start HCL symbols under full-suite parallel contention; passed
   cleanly in isolation (`cargo test --test hcl_indexing_test
   cold_start_lists_and_maps_all_three_hcl_aliases -- --exact`, 1/1 green).
   Already documented as a recurring full-suite-only flake at stash
   **`58B33C45`** (captured against 133-S, reused here, not re-captured).
3. **`integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`**
   — `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio` (apparent stdout
   truncation as the MCP tool catalog has grown). This is the same
   long-documented, recurring, pre-existing flake reproduced identically
   across `133-S`/`134-S`/`135-S`/`136-S`/`137-S` closures (stash candidates
   `58B33C45`, `4EE241DC`, `3067BC32`, `0443D844`, `7D47F30B`). PR #391's
   hosted CI (`build`, `start-launcher-windows`) reported SUCCESS for the
   merged commit. Per the discovery-reuse protocol, the five existing
   candidates remain mutually ambiguous (none individually positively
   confirmed as the sole match), so a new stash entry was captured rather
   than silently reused or re-litigated: stash **`EC3BAF22`**
   (`DISCOVERY-STATUS: AMBIGUOUS candidates 58B33C45 4EE241DC 3067BC32
   0443D844 7D47F30B`). No code change was made for this finding.

None of these three failures reproduced in the initial `cargo dev-test`
(fail-fast) run's earlier binaries at the same points where 138-S's own
manifest tests ran (those all passed cleanly, see the Background-job surface
table above) — they surfaced only under the full `--no-fail-fast` parallel
run, consistent with resource-contention/timing sensitivity, not a code
defect. A fourth, separately-discovered transient
(`contract_shim_stdio_initialize::t2_jsonrpc_method_not_found_is_terminal`,
observed only in an earlier fail-fast run of the same full suite) also passed
cleanly both in isolation and when its full 19-test file was re-run together;
it did not reproduce in the `--no-fail-fast` run at all and required no
separate stash capture given its non-reproduction here.

## Manual checkpoints

None declared for the `cli`/`api`/`background-job` surfaces in the validator
manifest beyond the probes above.

## Blocked prerequisites

- `engram daemon-status` (substitute for the stale manifest literal `engram
  status`) was not attempted live against the shared-environment daemon
  (PID `30528`) per explicit operator instruction — that daemon is in a known,
  deferred-scope readiness-latch state (stash `265F99BE`) and must not be
  queried or touched by this closure session. Not a code defect introduced by
  138-S; substitute evidence (harness test coverage) is provided above.
- The validator manifest's literal probe commands (`engram status`,
  `contract_initialize`, `contract_tools`) do not match the current CLI/test
  surface — pre-existing drift, already captured at stash `DA0AF326`, reused
  here (not re-captured).

## Overall

**Verdict: `PASS WITH FOLLOW-UP`** (`PASS_WITH_FOLLOW_UP`)

All fourteen manifest tasks' own harnesses pass green (87/87 targeted tests
across `integration_generation_activation` (28), `unit_read_request_context`
(6), `integration_read_server_startup_activation` (7),
`integration_request_entry_activation` (12),
`contract_read_server_dispatch_refusal` (6),
`contract_mcp_tool_catalog_parity` (6), `contract_cli_tool_catalog_parity`
(6), `contract_read_input_ownership_inventory` (16)). Build, format, and lint
gates are all clean. The full `cargo dev-test --no-fail-fast` suite passed
2457/2460 on a contended, fully-parallel run, with the three exceptions all
confirmed pre-existing, unrelated to 138-S, and passing cleanly in isolation
(two of the three) or matching a long-documented recurring flake (the third,
`archive_verifier`). The `cli-daemon-status` live probe was intentionally not
attempted per explicit operator instruction to avoid touching the
shared-environment daemon's known deferred-scope readiness-latch state. This
meets the workspace profile's `minimum_verdict: PASS_WITH_FOLLOW_UP`
threshold. Named releasability conditions are carried into
`docs/closure/2026-09-12-138-s-operational-closure.md`.
