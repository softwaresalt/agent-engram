---
title: Pre-initialize `server/discover` compatibility for the Engram MCP shim
date: 2026-08-23
type: implementation-plan
status: reviewed
source_stash_id: null
source: docs/decisions/2026-08-23-copilot-prerelease-server-discover-mcp-compatibility-spike.md
agent: stage
blast_radius: elevated
hardened: true
---

## Problem Frame

GitHub Copilot CLI `1.0.81-8` (prerelease; latest stable is `1.0.80`) sends a
JSON-RPC request with ID `0` and method `server/discover` **before** the MCP
`initialize` request. Engram's rmcp 1.1 `ServerHandler` state machine rejects
that ordering with `expect initialized request`, the shim process terminates,
and Copilot observes a broken pipe. The shim exits with
`ShimFailureClass::TransportFailure` (exit code 13).

Evidence is fully established in the spike artifact and is **not** re-derived
here:

* `.copilot/logs/process-1787528861694-24588.log` line 105 (`server/discover`
  before `initialize`), line 106 (broken pipe, exit 13)
* Protocol-level MCP stdio against the same binary succeeds: `initialize` OK,
  `tools/list` returns 20 tools, `tools/call(get_daemon_status)` OK
* Installed Engram `0.2.0+g06813e3b-dirty` already contains the serve-first
  shim work (124-F / 870B1AFF), so serve-first is necessary but not sufficient
* The GitHub MCP server in the same Copilot run logs
  `method invalid during initialization` for `server/discover` and **survives**;
  Engram's strict rmcp path exits instead

The Tokio type names in Copilot's error describe Copilot's own child-process
pipe types. Tokio is the reporter, not the root cause.

### Explicitly out of scope

* **Daemon readiness timeout increases.** The spike ruled this out: it masks the
  symptom and does not address the handshake ordering defect.
* **Cozo cold-start cost** (135 MB database, ~7.5 min to ready, ~1.37 GB RSS,
  ~893 CPU-seconds). Tracked as an independent defect in its own spike item; it
  is deliberately excluded from this shipment to preserve width isolation.
* **Client-side mitigation** (downgrade to stable `1.0.80`, workspace-local
  `.mcp.json` Engram registration). Already applied operationally; it is a
  recovery step, not the durable fix.

## Requirements Trace

| Requirement (operator + spike recommendation) | Implementation action |
|---|---|
| Tolerate a pre-`initialize` `server/discover` probe | U2 interposes a pre-initialize filter ahead of the rmcp state machine |
| Return an appropriate JSON-RPC method-not-found response | U2 emits `error.code = -32601` echoing the request `id`, per JSON-RPC 2.0 |
| Continue waiting for standards-compliant `initialize` | U2 does not terminate; it consumes the probe and keeps reading stdin |
| Preserve normal MCP clients | U4 regression guard: a clean `initialize`-first session is byte-for-byte unaffected |
| Preserve stdout JSON-RPC purity | U4 asserts stdout contains only well-formed JSON-RPC frames, no diagnostics |
| Preserve serve-first / degraded-session behavior | U4 asserts degraded `tools/call` still returns a structured error, not an exit |
| Preserve exit-code taxonomy | U4 asserts a probe-then-`initialize` session never yields exit 13 |
| Preserve rmcp tool behavior | U3 asserts the 20-tool catalog and `tools/call` succeed after the probe |
| Test-first contract/integration coverage of the exact ordering | U1 (RED) reproduces line-105 ordering before any production change |
| Prefer no new dependency | U2 uses a local wrapper over `rmcp::transport::io::stdio()`; no new crate |
| Rollback and verification detail | U5 documents the `ENGRAM_MCP_PREINIT_COMPAT` kill-switch, revert path, and Windows verification runbook |
| Request id `0` correlates correctly (review F4) | U1/U4 assert the `-32601` frame echoes `"id": 0` as a JSON number, not null/absent |
| Interception stays minimal (review F2) | U2 allowlists `server/discover` only; all other frames forward to rmcp unchanged |

## Design Decision

`run_shim` currently binds the transport directly:

```rust
let transport = rmcp::transport::io::stdio();
let running = rmcp::serve_server(handler, transport).await?;
```

The compatibility layer is a **narrow interposing transport wrapper** placed
between `stdio()` and `serve_server`. It is active only while the session is in
the pre-initialize state. For each inbound frame in that window:

* method == `initialize` → forward unchanged, disarm the filter permanently
* method == `server/discover` **with an `id`** → answer `-32601`
  (`Method not found`) directly on stdout, echoing the request `id` **verbatim
  and type-preserving** (id `0` must serialize as the JSON number `0`), do
  **not** forward, stay armed
* method == `server/discover` **without an `id`** (notification) → drop
  silently per JSON-RPC; emit no response frame, stay armed
* **everything else** → forward to rmcp unchanged (review finding F2)

The allowlist is deliberately **exactly one method**. Non-`initialize`,
non-`server/discover` frames are *not* intercepted, so rmcp's existing error
and ordering semantics are preserved verbatim for every case except the single
reproduced defect.

Rejected alternatives (recorded so review does not relitigate):

* **Implement `server/discover` semantically.** Rejected: the method is
  undocumented in the `1.0.81-8` prerelease notes and may be prerelease-only.
  Guessing a response shape creates a worse compatibility trap than
  `-32601`, which the GitHub MCP server already proves Copilot tolerates.
* **Patch/fork rmcp's state machine.** Rejected: heavy blast radius, upstream
  drift, and it would change behavior for all clients rather than a probe window.
* **Raise the daemon readiness timeout.** Rejected by the spike.
* **Add a JSON-RPC framing dependency.** Rejected: `serde_json` is already
  present; the filter needs only `id` and `method` extraction.

The filter disarms permanently on the first `initialize`, so there is no
steady-state cost and no post-handshake behavior change.

## Implementation Units

### U1 — RED: contract test reproducing the exact Copilot ordering

* Changes: a failing contract test that drives the shim over stdio with
  `server/discover` (id `0`) as the **first** frame, then a standard
  `initialize`. Asserts the process does not exit and that a `-32601` response
  carrying `"id": 0` **as a JSON number** is returned (review F4). Assertions
  must be bounded-timeout and event-driven, never a bare sleep (review F3).
* Files: `tests/contract/shim_pre_initialize_probe_test.rs`, `Cargo.toml`
  (`[[test]]` entry).
* Posture: test-first (RED). Must fail against current `main` with the observed
  `expect initialized request` / broken-pipe signature.
* Size: ≤ 2h.

### U2 — GREEN: pre-initialize compatibility filter

* Changes: introduce the interposing wrapper in the shim transport module and
  bind it in `run_shim` in place of the bare `rmcp::transport::io::stdio()`.
  Arms at session start, disarms on `initialize`. Intercepts **only**
  `server/discover`; all other frames forward to rmcp unchanged (review F2).
  Gated by `ENGRAM_MCP_PREINIT_COMPAT` (default enabled; `0` disables and
  restores strict rmcp ordering) (review F1). No new dependency.
* Files: `src/shim/transport.rs` (plus a sibling module if the wrapper warrants
  its own file).
* Posture: minimum change to turn U1 green.
* Size: ≤ 2h.

### U3 — Integration: probe-then-handshake end-to-end

* Changes: integration coverage proving that **after** the probe, the same
  session completes `initialize`, returns the full `tools/list` catalog, and
  completes `tools/call(get_daemon_status)`. Catalog integrity delegates to the
  existing independent catalog oracle (123-S / 129-F) where practical; the
  20-tool count is a smoke assertion only, not the oracle (review F5).
* Files: `tests/integration/shim_copilot_compat_test.rs`.
* Posture: proves the spike's success criteria end-to-end in one session.
* Size: ≤ 2h.

### U4 — Regression guard: invariants preserved

* Changes: assertions that (a) an `initialize`-first session is unchanged,
  (b) stdout carries only JSON-RPC frames, (c) degraded-session `tools/call`
  still returns a structured error rather than exiting, (d) a
  probe-then-`initialize` session never produces `TransportFailure` / exit 13,
  (e) notifications received pre-initialize produce no response frame,
  (f) `"id": 0` round-trips as a JSON number (review F4), and (g) with
  `ENGRAM_MCP_PREINIT_COMPAT=0` the strict pre-change behavior returns
  (review F1).
* Files: `tests/contract/shim_pre_initialize_probe_test.rs` (extension).
* Posture: locks the four preservation requirements.
* Size: ≤ 2h.

### U5 — Rollback, verification runbook, and docs

* Changes: document the compatibility window, the `-32601` contract, the
  `ENGRAM_MCP_PREINIT_COMPAT` kill-switch (default **on**; `0` disables the
  filter and restores strict rmcp ordering), the single-commit revert path, the
  Copilot `1.0.81-8` prerelease provenance so the layer can be retired
  deliberately rather than forgotten, and a Windows Copilot verification
  runbook covering both prerelease `1.0.81-8` and stable `1.0.80`.
* Files: `docs/decisions/` cross-link, shim module docs, operator runbook.
* Size: ≤ 2h.

## Blast Radius and Hardening

Elevated. This changes the **Windows MCP transport handshake path** — the single
entry point every MCP client traverses. A defect here is a total loss of MCP
availability, not a degraded feature.

Hardening controls carried into the tasks:

| Risk | Control |
|---|---|
| Filter swallows a legitimate `initialize` | U4 asserts `initialize`-first sessions are byte-identical; filter disarms on first `initialize` |
| Filter leaks non-JSON-RPC bytes to stdout | U4 stdout-purity assertion |
| Wrapper changes framing/partial-read semantics | Wrapper delegates framing to the underlying stdio transport; it only inspects decoded frames |
| Responding to a notification (JSON-RPC violation) | U2 explicitly drops id-less frames; U4 asserts no response |
| Regression escapes to non-Windows clients | U3 runs the standard handshake unchanged; filter is platform-neutral |
| Change proves wrong once Copilot stabilizes | U5 kill-switch + single-commit revert |
| Silent masking of real ordering bugs | Interception allowlist is exactly `server/discover`; every other pre-initialize frame still hits rmcp's strict path unchanged (review F2) |
| Client cannot correlate the refusal | `id` is echoed type-preserving; `id: 0` asserted as JSON number `0` (review F4) |

## Rollback

1. **Runtime**: set `ENGRAM_MCP_PREINIT_COMPAT=0` to disable the filter; the
   shim reverts to strict rmcp ordering with no redeploy.
2. **Source**: the change is confined to `src/shim/transport.rs` plus new test
   files; a single `git revert` of the implementation commit restores prior
   behavior. Test files are additive and safe to leave.
3. **Blast-radius floor**: because the filter is armed only pre-`initialize`,
   any rollback affects only the handshake window.

## Verification

* `cargo test` contract + integration suites (U1, U3, U4) green
* Manual Windows verification against Copilot CLI `1.0.81-8`: Engram appears in
  the MCP server list, initializes, and `tools/list` shows 20 tools
* Manual Windows verification against stable Copilot CLI `1.0.80`: unchanged,
  still initializes
* Confirm the shim exits `0` on clean client disconnect, not 13
* Confirm daemon readiness timeout is **unchanged** from `main`

## Follow-ups (not in this shipment)

* Cozo cold-start profiling (135 MB DB, ~7.5 min, ~1.37 GB, ~893 CPU-s) —
  tracked independently per operator instruction and spike recommendation.
* Confirm whether stable Copilot `1.0.80` omits `server/discover` entirely
  (spike remaining unknown; verification only, no code impact).
