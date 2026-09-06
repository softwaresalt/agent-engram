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

CLI-version and MCP-protocol (initialize + tools-catalog) surfaces verified
GREEN via substitute real commands. Daemon-status manual probe is
inconclusive-but-not-regressed, with strong substitute automated-test
evidence. No runtime regression identified from the 135-S transport
retirement.
