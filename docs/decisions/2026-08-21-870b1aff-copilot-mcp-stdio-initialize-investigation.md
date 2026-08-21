---
title: 870B1AFF investigation — Copilot CLI MCP stdio initialize pipe closure on Windows
date: 2026-08-21
type: investigation
status: resolved
source_stash_id: 870B1AFF
agent: stage
confidence: high
---

## Operator Symptom

Local `copilot.exe` fails to initialize the engram MCP server:

* `failed to initialize MCP client`
* `Send message error`
* transport pipe closing with Windows `os error 232` (`The pipe is being closed`)
* transport type named in the client error includes `tokio::process::ChildStdout` and `ChildStdin`

Operator report timestamp: 2026-08-20T16:28:25-07:00.

## Investigation Method

Read-only inspection of the shim startup path plus live reproduction using the
installed `engram` CLI against two native Git worktrees. No production code was
modified and no credentials or tokens were read or recorded.

## Evidence

### E1 — Pre-initialize exit is structurally possible

`src/shim/mod.rs::run` performs three fallible steps *before* the MCP stdio
transport is ever bound:

1. `crate::db::workspace::canonicalize_workspace(&workspace)` (admission)
2. `lifecycle::ensure_daemon_running(&workspace_path)` (bounded readiness wait)
3. `crate::daemon::ipc_server::ipc_endpoint(&workspace_path)`

Only after all three succeed does `transport::run_shim` call
`rmcp::transport::io::stdio()` and `rmcp::serve_server`.

`src/bin/engram.rs::main` handles `Command::Shim` with
`engram::shim::run(...).await?`, so any `Err` propagates out of `main`,
anyhow prints `Error: {err}` on stderr, and the process exits non-zero.
Child stdout/stdin close at exit.

### E2 — The client-visible error is the expected downstream artifact

An MCP client spawns the child, then writes `initialize` to the child's stdin.
If the child has already exited, the write fails with Windows
`ERROR_BROKEN_PIPE` (232). The client surfaces its own transport types
(`tokio::process::ChildStdin` / `ChildStdout`) because that is the client's
transport implementation. Tokio is reporting the pipe state correctly.
**Tokio is not defective and is not the root cause.**

### E3 — Live reproduction of a pre-initialize failure

Installed binary reports `engram 0.2.0+g6268c1ac-dirty`.

| Probe | Worktree | Result |
|---|---|---|
| `engram workspace-status` | `.worktrees/stage-dark-factory-20260820-batch4` | `Error: cannot compute IPC endpoint: Path '...' is not a Git repository root` (exit 2) |
| `engram daemon-status` | `.worktrees/ship-119-s-compact-context` | `Error: daemon unavailable: Daemon failed to reach Ready state within 30000ms` (exit non-zero) |

Both are exactly the failure classes that terminate `shim::run` before
`serve_server`. Under the CLI these surface as readable stderr text; under an
MCP stdio client the same terminations present only as a closed pipe.

### E4 — Version skew is live in the operator environment

Repo `main` is at `6f50f0e4`; Feature 122-F (worktree-safe admission, bounded
startup) landed at `08676d34`. The installed shim reports build `g6268c1ac`,
which predates that fix. The operator's Copilot CLI therefore launches a shim
that still rejects native worktrees at admission — a pre-initialize `Err`.

### E5 — Latent stdout-contamination hazard

`src/lib.rs::init_tracing` builds `fmt::layer()` with no explicit writer.
`tracing_subscriber`'s `fmt` layer defaults to **stdout**. The shim path does
not call `init_tracing` today (only `Command::Daemon` does), so this is not the
current root cause, but any future or environment-triggered initialization on
the shim path would emit non-JSON-RPC bytes onto the MCP framing channel and
break `initialize` in a way that looks identical to a transport bug.

### E6 — Diagnostic dead-end

There is no durable record of a shim startup failure. When the child dies
pre-initialize, the operator has only the client's transport text. Exit codes
are undifferentiated (anyhow default `1`), and stderr may be discarded or
buffered by the launching client.

## Root Cause

**Pre-initialize child exit**, not a transport defect.

`engram shim` treats workspace admission, daemon readiness, and endpoint
derivation as hard preconditions evaluated *before* the MCP stdio contract is
established. Any failure in that window terminates the process while the client
is mid-`initialize`, producing `os error 232`. The immediate trigger in the
operator's environment is a stale installed binary whose admission logic rejects
native Git worktrees (E4), but the defect class is structural and will recur for
any pre-initialize failure (daemon spawn failure, readiness timeout, endpoint
derivation error, permission error).

## Windows Stdio Initialization Contract (as it must hold)

1. Once a client spawns the shim, the shim MUST reach a state where it can read
   `initialize` from stdin and write an `InitializeResult` to stdout, or it MUST
   fail in a way the client can attribute.
2. stdout is reserved exclusively for JSON-RPC framing. No logging, progress,
   banner, or diagnostic output may be written to stdout on the shim path.
3. All human-readable diagnostics go to stderr.
4. Process exit before the `initialize` response is a contract violation unless
   the failure is genuinely unrecoverable *and* is accompanied by a durable,
   attributable diagnostic record.

## Chosen Direction

**Serve-first, degrade-in-session.** Bind the stdio transport and answer
`initialize` before evaluating daemon-dependent preconditions. Carry startup
failure state into the session so `tools/list` still returns the static catalog
and `tools/call` returns a structured JSON-RPC error naming the real cause.
Reinforce with a stderr-pinned diagnostic writer, a differentiated exit-code
taxonomy, a durable startup-failure record, and Windows regression coverage that
asserts the client-visible contract.

## Rejected Alternatives

| Alternative | Why rejected |
|---|---|
| Treat as a Tokio/rmcp transport bug | E1–E3 show the pipe close is a downstream artifact of child exit. No evidence of transport defect. |
| Only bump the installed binary | Fixes E4 for one operator; leaves the structural pre-initialize exit class (E1, E6) intact. |
| Increase the readiness timeout | Trades a fast opaque failure for a slow opaque failure. Does not restore attributability. |
| Print diagnostics to stdout so the client shows them | Violates the stdio framing contract (E5) and would corrupt `initialize`. |

## Open Questions

None blocking. Exit-code numbering and the on-disk location of the startup
failure record are implementation choices constrained by the plan.

## Traceability

Source stash `870B1AFF`. Related: Feature 122-F, Shipment 118-S.
