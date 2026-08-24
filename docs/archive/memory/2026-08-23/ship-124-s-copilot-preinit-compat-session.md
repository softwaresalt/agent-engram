---
title: Ship session — 124-S Copilot pre-initialize server/discover compatibility
date: 2026-08-23
type: session-memory
agent: ship
shipment: 124-S
feature: 130-F
branch: feat/124-s-copilot-preinit-server-discover
pr: 359
status: awaiting-operator-merge-approval
---

## Outcome

Shipment `124-S` / feature `130-F` implemented end-to-end through quality
gates, review, and CI. PR #359 is green and merge-gate clean. **Not merged** —
operator approval is required and was not given in this session.

## Tasks completed

| Task | Unit | Result |
|---|---|---|
| `130.001-T` | U1 RED contract harness | done |
| `130.002-T` | U2 GREEN compatibility filter | done |
| `130.003-T` | U3 integration probe-then-handshake | done |
| `130.004-T` | U4 regression guards | done |
| `130.005-T` | U5 rollback + Windows runbook | done |

## Isolation handling

The main worktree carried unrelated staged backlog repairs and a stranded
`UU` conflict in `.backlogit/archive/stash.jsonl`. Nothing was routed through
main. The implementation branch was created **inside** the Stage worktree
(`.worktrees/stage-130-copilot-server-discover-20260823`), whose tracked tree
was clean at exactly `06813e3b` — identical to both `main` and `origin/main` —
with the Stage artifacts present only as untracked additions. That made
`git checkout -b` a safe, policy-clean intake that preserved the reviewed
planning artifacts and committed them as the first commit of the PR.

Verified at session end: main's HEAD, staged set, and conflict state are
unchanged from the dispatch snapshot.

## Files changed

* `src/shim/preinit_compat.rs` (new) — the compatibility window
* `src/shim/transport.rs` — binds the interposed transport in `run_shim`
* `src/shim/mod.rs` — module registration
* `tests/contract/shim_pre_initialize_probe_test.rs` (new) — U1 + U4
* `tests/integration/shim_copilot_compat_test.rs` (new) — U3
* `docs/troubleshooting.md` — contract, kill switch, monitoring plan, runbook
* `docs/configuration.md` — `ENGRAM_MCP_PREINIT_COMPAT` row
* `docs/decisions/…-server-discover-mcp-compatibility-spike.md` — resolution

## Key decisions

* **Interception is maximally narrow.** A frame is absorbed only when it is
  valid JSON, a JSON object, `"jsonrpc":"2.0"`, method `server/discover`,
  `params` absent/object/array, and an id rmcp itself accepts (string or
  i64/u64). Everything else forwards so rmcp's `Invalid Request` and ordering
  semantics stay authoritative. This narrowed twice under review.
* **Single-writer stdout.** The first design gave the filter its own
  `tokio::io::Stdout` handle alongside rmcp's. That was wrong: tokio gives no
  atomicity guarantee across handles. rmcp now binds two in-memory duplex
  pipes and one `run_output_pump` task owns the only stdout handle.
* **Bounded response queue.** The synthesized-response channel is depth 8 with
  awaited sends, so a probe flood against a stalled stdout backpressures stdin
  instead of buffering without limit.
* **`-32601` over implementing `server/discover`.** The method is undocumented
  in the `1.0.81-8` prerelease and may be prerelease-only. The GitHub MCP
  server returns a comparable refusal in the same Copilot run and Copilot
  tolerates it.

## Failed approaches

* Relying on per-handle write atomicity between two `Stdout` handles — refuted
  by review; rmcp answers pre-`initialize` `ping` and replies `-32700` to
  undecodable frames while **continuing** to wait for `initialize`, so both
  writers really are live in the armed window.
* Echoing the request id verbatim for *any* JSON value — violates JSON-RPC for
  object/array ids and usurps rmcp's Invalid Request handling.

## Verification

* `cargo fmt`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
  clean; full `cargo test --all-targets --no-fail-fast` green (651 lib tests
  plus all contract/integration suites, 0 failures).
* MCP catalog oracle independence guard passes.
* CI green on final HEAD `1da35ed5`.
* **Live**: Copilot CLI `1.0.81-8` launched with only Engram enabled
  initialized successfully, enumerated 21 tools, and executed
  `tools/call(get_daemon_status)` against a live daemon. Run log shows no
  `failed to initialize MCP`, no broken pipe, no `expect initialized request`.

## Known flake

`tests/integration/daemon_startup_order_test.rs::run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`
timed out once under full-suite load (10 s TTL against a 30 s guard) and passed
in isolation at 11.06 s and on the final full run. Pre-existing load
sensitivity, unrelated to the shim transport. Candidate follow-up: raise the
TTL margin.

## Review cycles

Three cycles, at the circuit-breaker limit. All findings resolved and all
threads answered and resolved. Findings addressed: id/envelope/params
validation narrowing, single-writer stdout, bounded backpressure, the
release-observability monitoring plan, and a stray-escape doc fix.

## Next steps

1. Operator decides on merge for PR #359 (merge commit only, per Principle XI).
2. After merge: close shipment `124-S`, graduate the compatibility-window
   rationale, and run `backlogit_sync_index`.
3. Backlog note: the backlogit index cannot build in any worktree until the 19
   malformed `029.*` / `030.005-C` artifacts are repaired — that repair is
   currently staged in main and outside this shipment's scope.
