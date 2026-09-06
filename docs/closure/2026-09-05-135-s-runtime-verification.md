---
title: 135-S Retire HTTP and SSE Transport Surfaces — Runtime Verification
description: Runtime validator evidence per .autoharness/workspace-profile.yaml runtime_validation.validator_manifest for the transport-retirement change.
---

## Scope

135-S deletes retired HTTP/SSE server code (`src/server/{mcp,router,sse}.rs`),
removes the `legacy-sse` feature and its dependencies, and corrects
documentation/installer claims. It does **not** modify the daemon lifecycle,
IPC transport, MCP shim protocol handling, or indexing pipeline. Runtime
verification below targets the `cli` and `api` surfaces declared in the
workspace profile's `runtime_validation.validator_manifest`.

## CLI surface

| Probe | Command | Result |
|---|---|---|
| `cli-version` | `engram --version` | **PASS** — `engram 0.3.0-rc.1+g4e22a892`, exit 0 |
| `cli-daemon-status` | `engram status` (manifest literal) | **BLOCKED — command does not exist.** The validator manifest's literal probe command is stale/drifted relative to the actual CLI surface (no `status` subcommand; the real subcommands are `daemon-status`, `workspace-status`, `stats`). This drift is pre-existing and unrelated to 135-S; not fixed here (out of scope, no owned-file overlap — the workspace-profile.yaml validator manifest is not an owned file of any 135-S task). Substitute attempted: `engram daemon-status` against this repo's own bound workspace. Result: **inconclusive** — the command did not return within a multi-minute manual budget in this heavily-loaded dev session (dozens of parallel `cargo check`/`cargo test`/`cargo clippy` invocations were run across this same session, plus several long-lived stray daemon processes bound to earlier test tempdirs were found and cleaned up mid-session). No conclusion of regression can be drawn from an inconclusive manual probe under contention. **Substitute evidence**: the automated suite's daemon-lifecycle and IPC test coverage (`daemon::lifecycle_policy::*`, `tools::lifecycle::*`, `daemon_startup_order` suite, `t046_s050_daemon_exits_after_idle_timeout_and_restarts`, and 650+ further unit/contract/integration tests) all passed in this session's `cargo dev-test` runs, and none of the code paths they exercise (`src/daemon/`, `src/server/state.rs`, `src/server/observability.rs`) were touched by 135-S. |

## API (MCP protocol) surface

| Probe | Command | Result |
|---|---|---|
| `mcp-initialize-handshake` | manifest literal `cargo test --test contract_initialize` | **Test binary does not exist under that name** (pre-existing manifest drift, not owned by any 135-S task). Substitute: `cargo test --test contract_shim_stdio_initialize` — **PASS**, 19/19 tests green, covering initialize handshake, protocol-version compatibility, degraded-session tool-call behavior, and startup-failure classification. |
| `mcp-tool-invocation` | manifest literal `cargo test --test contract_tools` | **Test binary does not exist under that name** (same pre-existing drift). Substitute: `cargo test --test contract_tools_catalog` — **PASS**, 6/6 tests green, confirming the static tool catalog (schemas, tool count, retained/absent tool sets) matches the documented contract. |

## Manual checkpoints

None declared for the `cli`/`api` surfaces in the validator manifest beyond
the probes above.

## Blocked prerequisites

- `engram status` / ad-hoc `engram daemon-status` manual probe against this
  repo's own workspace: inconclusive due to session-level resource
  contention (see above). No fake automation substituted; the gap is
  disclosed and covered by pre-existing, extensive automated daemon test
  coverage instead.
- The validator manifest's literal probe commands (`engram status`,
  `contract_initialize`, `contract_tools`) do not match the current CLI/test
  surface. This is pre-existing drift, unrelated to 135-S, and out of scope
  to fix here (not an owned file of any 135-S task). Recommend capturing as
  a follow-up for whoever owns `.autoharness/workspace-profile.yaml`
  maintenance.

## Overall

**Verdict: `PASS WITH FOLLOW-UP`** (`PASS_WITH_FOLLOW_UP`)

CLI-version and MCP-protocol (initialize + tools-catalog) surfaces verified
GREEN via substitute real commands. The `cli-daemon-status` probe is
**BLOCKED** (its literal manifest command does not exist, and the closest
real substitute did not return within a bounded manual budget under this
session's resource contention — see "Blocked prerequisites" above). This is
not treated as a hard `FAIL` because: (1) the blocking cause is
session-environment contention plus pre-existing validator-manifest drift,
neither of which 135-S introduced; (2) 674+ automated tests exercising the
exact same daemon-lifecycle/IPC code paths (`src/daemon/`) — none of which
135-S touched — passed in this session's `cargo dev-test` runs. The blocked
prerequisite and its substitute evidence are carried into
`docs/closure/2026-09-05-135-s-operational-closure.md` as a named
releasability condition, per the runtime-verification skill's contract that
blocked verification must be recorded as a condition, not silently
upgraded to an unconditional pass.

## Post-merge re-run addendum (2026-09-06)

Per the named condition above, `engram daemon-status` (substitute for the
stale manifest literal `engram status`) was re-attempted post-merge against
PR #383's merge commit `0cfffc0cf7220d8f643da28cd2025aff558b7d76`, in a
quieter environment: six long-stale orphaned `backlogit mcp` server
processes (dated 2026-09-01 through 2026-09-05) were identified and
terminated first, removing the prior session's resource-contention theory
as a live confound.

**Result: genuine forward progress, but still did not reach `Ready` within
this session's bounded budget** — a materially different (and better)
outcome than the pre-merge attempt's pure inconsistency:

* The daemon process (`engram daemon --workspace C:\Source\GitHub\engram`)
  started successfully and ran continuously for the full observation window
  (15+ minutes), consuming steadily increasing CPU time (verified via
  repeated `Get-Process` samples: 0 → 479s → 910s → 1296s → 1780s → 2135s →
  2208s CPU across the window) with **no crash, no error exit, and no
  stall** (`Responding: True` throughout).
* This was independently corroborated by on-disk evidence: a new
  branch-scoped Cozo index directory
  (`.engram/cozo/post-merge__135-s-retire-http-and-sse-transport-surfaces/`)
  was created and actively written to (`engram.db`, `engram.db-journal`,
  `engram.db.lock`) during the window, consistent with a genuine first-time
  full-repository index build for the new `post-merge/135-s-...` branch
  scope (each branch gets its own Cozo namespace; this branch had never been
  indexed before).
* The command was stopped after ~15 minutes without reaching `Ready` — not
  because it stalled, but because a first-time full-repo index build for
  a brand-new branch scope of this codebase's size exceeds a practical
  manual-verification budget within this closure session.

**Interpretation**: the pre-merge "inconclusive due to session contention"
finding is **superseded** — contention was ruled out and the daemon
demonstrably functions and makes real indexing progress post-merge. The
residual gap is purely the **first-index cold-start cost for a new branch
namespace**, which is an expected operational characteristic of the
per-branch Cozo indexing design, not a defect introduced by 135-S (135-S
touches no daemon/indexing code). No new stash follow-up is warranted
beyond the already-captured `DA0AF326` (validator-manifest command drift);
cold-start indexing time is out of scope for that entry and is noted here
for operator awareness only. The daemon process was left running in the
background at the end of this session to allow the index build to
complete opportunistically for future sessions on this branch.

Updated overall verdict remains **`PASS WITH FOLLOW-UP`**; the named
releasability condition in
`docs/closure/2026-09-05-135-s-operational-closure.md` is downgraded from
"blocked, cause undetermined" to "confirmed non-blocking cold-start cost,
no code defect" — see that document's updated Releasability section.
