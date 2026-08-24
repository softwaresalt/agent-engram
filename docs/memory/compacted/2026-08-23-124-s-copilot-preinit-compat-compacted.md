---
title: "124-S Copilot pre-initialize server/discover compatibility — full lifecycle"
doc_type: compacted-memory
date: 2026-08-23
type: compacted
shipment: 124-S
feature: 130-F
pr: 359
merge_commit: 8f9904a0a55516582e101d7b75b9457adaf9a0be
sources:
  - docs/archive/memory/2026-08-23/stage-copilot-server-discover-compat-session.md
  - docs/archive/memory/2026-08-23/ship-124-s-copilot-preinit-compat-session.md
  - docs/archive/memory/2026-08-23/124-s-post-merge-closure-memory.md
tags: [mcp, shim, copilot, windows, compatibility, ship, stage, closure]
---

## Outcome

Shipment `124-S` / feature `130-F` shipped: the Engram MCP shim now tolerates
Copilot's undocumented pre-`initialize` `server/discover` probe instead of
dying with TransportFailure (exit 13). PR #359 merged as two-parent commit
`8f9904a0a55516582e101d7b75b9457adaf9a0be`. All 6 members plus the plan-review
gate artifact are archived with merge evidence.

## Problem

Copilot CLI `1.0.81-8` sends `server/discover` with `"id": 0` as the **first**
stdio frame, before `initialize`. rmcp treats that as a protocol violation and
the shim exits, so Engram never appears as an MCP server in Copilot.

## Solution Shape

A narrow pre-initialize compatibility window that answers JSON-RPC `-32601`
(Method not found) to exactly `server/discover` and keeps waiting for a
standards-compliant `initialize`. Everything else forwards to rmcp unchanged.
Gated by `ENGRAM_MCP_PREINIT_COMPAT` (default on, `0` disables).

Files: `src/shim/preinit_compat.rs` (new), `src/shim/transport.rs`,
`src/shim/mod.rs`, `tests/contract/shim_pre_initialize_probe_test.rs` (new),
`tests/integration/shim_copilot_compat_test.rs` (new), plus
`docs/troubleshooting.md` and `docs/configuration.md`.

## Decisions That Survived Review

1. **Maximally narrow interception.** A frame is absorbed only when it is valid
   JSON, a JSON object, `"jsonrpc":"2.0"`, method exactly `server/discover`,
   `params` absent/object/array, and an id rmcp itself accepts (string or
   i64/u64). Narrowed twice under review (finding F2). Everything else forwards
   so rmcp's `Invalid Request` and ordering semantics stay authoritative.
2. **`-32601` rather than implementing `server/discover`.** The method is
   undocumented in the prerelease and may be prerelease-only. The GitHub MCP
   server returns a comparable refusal in the same Copilot run, and Copilot
   tolerates it.
3. **Single-writer stdout.** The first design gave the filter its own
   `tokio::io::Stdout` handle alongside rmcp's — wrong, because tokio gives no
   atomicity guarantee across handles, and rmcp really is live in the armed
   window (it answers pre-`initialize` `ping` and replies `-32700` to
   undecodable frames while continuing to wait). rmcp now binds two in-memory
   duplex pipes and a single `run_output_pump` task owns the only stdout handle.
4. **Bounded response queue.** Synthesized-response channel is depth 8 with
   awaited sends, so a probe flood against a stalled stdout backpressures stdin
   instead of buffering without limit.
5. **Named kill switch** `ENGRAM_MCP_PREINIT_COMPAT` so rollback needs no
   redeploy (finding F1).
6. **`id: 0` round-trip asserted explicitly** (finding F4) — zero is the classic
   falsy-id serialization bug and Copilot uses exactly `0`.
7. **Readiness-timeout increase rejected.** Recorded as out of scope; it would
   mask the symptom.

## Failed Approaches

* Relying on per-handle write atomicity between two `Stdout` handles — refuted
  in review.
* Echoing the request id verbatim for *any* JSON value — violates JSON-RPC for
  object/array ids and usurps rmcp's Invalid Request handling.

## Verification

* `cargo fmt`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
  clean; full `cargo test --all-targets --no-fail-fast` green (651 lib tests
  plus all contract/integration suites, 0 failures).
* MCP catalog oracle independence guard passes.
* **Live**: Copilot CLI `1.0.81-8` with only Engram enabled initialized
  successfully, enumerated 21 tools, and executed
  `tools/call(get_daemon_status)` against a live daemon. No
  `failed to initialize MCP`, no broken pipe, no `expect initialized request`.
* Three review cycles (at the circuit-breaker limit); all findings resolved,
  all threads answered and resolved.

## Merge and Closure

All six merge gates re-read at the approved HEAD `189a90e0`: Copilot review at
exact HEAD, Copilot absent from requested reviewers, 0 unresolved threads of 6,
CI green, `mergeStateStatus: CLEAN`, and merge-commit-only enforced by
repository policy (squash and rebase both disabled). Merged with
`gh pr merge --merge`; two parents confirm a true merge commit;
`merge-base --is-ancestor` confirms it in `origin/main`.
`git diff 189a90e0 8f9904a0` is empty, so the CI-verified tree is exactly what
landed.

`backlogit shipment ship 124-S --sha 8f9904a0…` archived `130.001-R`,
`130.001-T`…`130.005-T`, `130-F`, and `124-S` with `returned_ids: []`.

## Closure Obstacle Worth Remembering

`shipment ship` refused with `member 130.001-T missing passing gate evidence:
gate blocked: 130.001-T remains active` even though the artifact and the synced
index both said `done`. Cause: gate events live under the gitignored
`.backlogit/logs/`, absent from a worktree created fresh at the merge SHA. Fixed
by porting the **original** gate logs from the implementation worktree rather
than regenerating synthetic evidence — this preserves the real `head_sha`
(`3d90d5a7…`) each gate ran against. `--force-gates` was not used. The compound
learning `post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`
was refreshed to make porting the preferred path.

## Isolation Discipline

The primary checkout carried unrelated operator-staged backlog repairs and an
unresolved `UU .backlogit/archive/stash.jsonl` throughout Stage, Ship, and
closure. Nothing was ever routed through it. Implementation ran in the Stage
worktree (clean at `06813e3b`, identical to `main`); closure ran in a dedicated
`post-merge/124-s-closure` worktree created at the merge SHA. Main's HEAD,
staged set, and conflict state are byte-for-byte unchanged.

## Open Items

* **`002-SP`** (queued, high) — profile Cozo cold start: a 135 MB database took
  ~7.5 minutes to reach ready, ~1.37 GB, ~893 CPU-seconds. Deliberately excluded
  from 124-S to preserve width isolation. Raising the readiness timeout is not
  an acceptable outcome. Closure independently corroborated this: an
  `engram daemon-status --timeout 30` probe produced no output for >150 s,
  suggesting the timeout governs a post-readiness phase rather than the
  expensive open/bootstrap path. Suggested entry point
  `src/db/cozo_backend/mod.rs`.
* **Known flake** —
  `tests/integration/daemon_startup_order_test.rs::run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`
  timed out once under full-suite load (10 s TTL against a 30 s guard) and
  passed in isolation at 11.06 s. Pre-existing load sensitivity, unrelated to
  the shim transport. Candidate follow-up: raise the TTL margin.
* **Backlog hygiene** — 18 archived `029.*` artifacts plus
  `.backlogit/queue/030.005-C.md` have malformed `title:` frontmatter at
  `HEAD`; the repair is staged in the primary checkout and is outside this
  shipment's scope.
* **Memory compaction backlog** — `docs/memory/` remains above the 40-file
  threshold; 21 dated directories older than 14 days await a dedicated
  compaction pass.
