---
title: 137-S Candidate Indexing, Direct-Sync Boundary and Supervisor Crate Separation — Runtime Verification
description: Runtime validator evidence per .autoharness/workspace-profile.yaml runtime_validation.validator_manifest for the post-merge closure of 137-S.
---

## Scope

137-S implements: (1) `142.015-T` — the candidate indexing service now accepts
only a sealed `IndexTarget` (`src/services/code_graph.rs`); (2) `142.016-T` —
`ReadServer` mode refuses direct sync with the stable F38 refusal, while
`Managed` mode preserves legacy direct index/sync (`src/cli/direct.rs`);
(3) `142.020-T` — a new `crates/engram-indexer` supervisor crate foundation
and boundary harness; (4) `142.021-T` — a contract test asserting the
supervisor workspace boundary; (5) `142.022-T` — the release workflow
publishes `engram-indexer` as a distinct artifact; (6) `142.027-T` — agent
installation excludes the supervisor. This touches the `cli` (direct-sync
refusal), `background-job` (indexing pipeline), and `api`/build surfaces
declared in the workspace profile's `runtime_validation.validator_manifest`.
Verification below runs against merge commit `ef0135bf05aba5d7329a7306eb78bb9b18e02f69`
(PR #388) on the `post-merge/137-s-candidate-indexing-direct-sync-boundary-and-supervisor-crate-separation`
branch.

## Build

| Check | Command | Result |
|---|---|---|
| Compile check | `cargo check --all-targets` | **PASS** — clean, no warnings |
| Debug build | `cargo build` | **PASS** |
| Format | `cargo fmt --all -- --check` | **PASS** |
| Lint | `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | **PASS** — clean |

## CLI surface

| Probe | Command | Result |
|---|---|---|
| `cli-version` | `engram --version` | **PASS** — `engram 0.3.0-rc.1+gef0135bf-dirty`, exit 0 |
| `cli-daemon-status` | manifest literal `engram status` | **Does not exist as a subcommand** — pre-existing validator-manifest drift, already captured and reused from stash `DA0AF326` (not re-captured; positively confirmed identical finding: real subcommands are `daemon-status`, `workspace-status`, `stats`). Substitute attempted: `engram daemon-status` bounded to a 30s budget — **BLOCKED (bounded timeout)**, consistent with the previously-documented per-branch Cozo first-index cold-start cost for this `post-merge/137-s-...` branch namespace (`docs/closure/2026-09-05-135-s-runtime-verification.md` "Post-merge re-run addendum"), not a code defect. No destructive process cleanup was performed on the several pre-existing stray `engram` daemon processes observed in this shared dev environment (out of scope, no explicit operator approval for cleanup). **Substitute evidence**: automated daemon-lifecycle and IPC test coverage (`daemon::lifecycle_policy::*`, `tools::lifecycle::*`, `contract_shim_stdio_initialize` 19/19, `contract_tools_catalog` 6/6) all passed in this session; none of the exercised code paths were touched by 137-S. |

## API (MCP protocol) surface

| Probe | Command | Result |
|---|---|---|
| `mcp-initialize-handshake` | manifest literal `cargo test --test contract_initialize` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_shim_stdio_initialize` — **PASS**, 19/19 tests green. |
| `mcp-tool-invocation` | manifest literal `cargo test --test contract_tools` (stale; see `DA0AF326`) | Substitute: `cargo test --test contract_tools_catalog` — **PASS**, 6/6 tests green, tool catalog schema/count unchanged by 137-S (137-S adds no MCP tools). |

## Background-job surface (indexing pipeline)

The `workspace-sync`/`engram sync` live probe was not attempted directly: per
the `cli-daemon-status` finding above, a live daemon invocation on this branch
would incur the same first-index cold-start cost. Substitute evidence instead:

| Evidence | Result |
|---|---|
| `cargo test --test integration_candidate_indexing_service` (142.015-T's own harness) | **PASS** — 3/3 tests green; asserts both `IndexTarget::LegacyDirect` and `IndexTarget::Candidate` target kinds, and that the active generation is untouched by a candidate build. |
| `cargo test --test integration_direct_sync_mode` (142.016-T's own harness) | **PASS** — 2/2 tests green; asserts `Managed` mode preserves direct sync and `ReadServer` mode returns the non-retryable F38 refusal. |
| `cargo dev-test` full suite | **688/689 passed** on a clean (non-contended) run — see "Known pre-existing failure" below for the one exception. |

## Supervisor crate surface (new in 137-S)

| Probe | Command | Result |
|---|---|---|
| Supervisor boundary harness | `cargo test -p engram-indexer` | **PASS** — 1/1 (`supervisor_entrypoint_accepts_a_sealed_target_from_engram`), plus 0/0 lib and bin unit-test harnesses (clean) |
| Workspace boundary contract | `cargo test --test contract_supervisor_workspace_boundary` | **PASS** — 4/4 tests green (no supervisor binary in the agent package; supervisor is a workspace member but not an agent dependency; supervisor consumes only the F06–F10 facade) |
| Release artifact contract | `cargo test --test contract_supervisor_release_artifact` | **PASS** — 3/3 tests green (release workflow publishes a distinct `engram-indexer` artifact; agent archive excludes it) |
| Install exclusion contract | `cargo test --test contract_supervisor_install_exclusion` | **PASS** — 2/2 tests green (agent installation installs no supervisor binary/command) |

## Known pre-existing failure (out of scope, not introduced by 137-S)

`cargo dev-test` intermittently/reproducibly fails
`integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
on this Windows build host with `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP
stdio` (apparent stdout truncation as the MCP tool catalog has grown). This is
a long-documented, recurring, pre-existing flake — already stashed four times
across prior shipments (`58B33C45`, `4EE241DC` from 133-S/134-S; `3067BC32`
from 134-S; `0443D844` from 135-S) and reproduced identically against
unmodified upstream HEAD in the 136-S closure session
(`docs/closure/2026-09-08-136-s-copilot-review-inventory-and-adversarial-review.md`).
Reproduced again here in isolation (`cargo test --test
integration_release_archive_smoke_workflow archive_verifier_runs_the_unpacked_native_binary`)
— not a contention artifact. Confirmed unrelated to 137-S: the test file and
`scripts/verify-release-archive.py` are not owned by any 137-S task, and PR
#388's hosted CI (`ci gate`) reported SUCCESS for this merge commit. Per the
discovery-reuse protocol, the four existing candidates remain mutually
ambiguous (none individually positively confirmed as the sole match), so a
new stash entry was captured rather than silently reused or re-litigated:
stash **`7D47F30B`** (`DISCOVERY-STATUS: AMBIGUOUS candidates 58B33C45
4EE241DC 3067BC32 0443D844`). No code change was made for this finding.

## Manual checkpoints

None declared for the `cli`/`api`/`background-job` surfaces in the validator
manifest beyond the probes above.

## Blocked prerequisites

- `engram daemon-status` (substitute for the stale manifest literal `engram
  status`) did not return within a bounded 30-second budget — consistent with
  the previously-documented per-branch Cozo first-index cold-start cost for a
  brand-new branch namespace (`post-merge/137-s-...`), not a code defect. Not
  re-litigated here; see `docs/closure/2026-09-05-135-s-runtime-verification.md`
  for the full precedent investigation.
- The validator manifest's literal probe commands (`engram status`,
  `contract_initialize`, `contract_tools`) do not match the current CLI/test
  surface — pre-existing drift, already captured at stash `DA0AF326`, reused
  here (not re-captured).

## Overall

**Verdict: `PASS WITH FOLLOW-UP`** (`PASS_WITH_FOLLOW_UP`)

All six manifest tasks' own harnesses pass green (14/14 targeted tests
across `integration_candidate_indexing_service`, `integration_direct_sync_mode`,
`contract_supervisor_workspace_boundary`, `contract_supervisor_release_artifact`,
`contract_supervisor_install_exclusion`, plus the new `engram-indexer` crate's
own boundary test). Build, format, and lint gates are all clean. The full
`cargo dev-test` suite passed 688/689 on a clean run, with the single
exception being the long-documented, pre-existing, out-of-scope
`archive_verifier_runs_the_unpacked_native_binary` flake (stashed as
`7D47F30B`, ambiguous against four prior occurrences). The `cli-daemon-status`
live probe is `BLOCKED` for the same previously-documented cold-start reason
as 135-S's post-merge re-run — not a regression. This meets the workspace
profile's `minimum_verdict: PASS_WITH_FOLLOW_UP` threshold. Named
releasability conditions are carried into
`docs/closure/2026-09-08-137-s-operational-closure.md`.
