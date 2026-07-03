# Session memory — 2026-07-03 — Stage: amend 067-F/067-S for CLI correlation-id + CLI-direct emission

**Agent:** Stage (planning/backlog only). **Mode:** DEGRADED (backlogit MCP transport
closed → `backlogit` CLI read-only fallback; engram daemon down → grep/view source
grounding). **Constraint honored:** did NOT run `backlogit sync`/`sync_index` (cache-union
landmine); edited `.backlogit/queue/067*.md` markdown directly. All edits uncommitted by
design (Ship carries them when it ships 067-S). No code, no branches, no PRs. 067-S stays
`queued`.

## What changed (amendment to merged plan, PR #189 @ f2835847)

Operator directive (2026-07-03): `_meta.correlation_id` OK for MCP, but engram is also
used via CLI and must fall back to CLI when MCP is lost → (1) CLI `--correlation-id` arg
threaded to the same `usage.jsonl` record; (2) CLI-direct emission now IN scope
(overturns decision-017 D9/A2).

### Key grounding finding (source-verified)
- `src/tools/mod.rs::dispatch()` is the **daemon-only** emit choke point (reached via
  `src/daemon/ipc_server.rs`). Both `metrics::record` sites live there.
- `src/cli/direct.rs::run_direct_sync` **BYPASSES `dispatch`** — calls
  `services::code_graph::{index,sync}_workspace_with_progress` directly, emits nothing.
  ⇒ Operator's "confirm shared choke point" premise is FALSE → minimal dedicated hook
  required in `run_direct_sync` (reuse `metrics::initialize/record/shutdown`).
- Direct mode holds `DaemonLock` for its whole run ⇒ daemon+direct never write
  `usage.jsonl` concurrently (mutual exclusion; resolves the two-writer concern).
- `GlobalFlags` (`src/cli/flags.rs`) hosts `--workspace`/`--timeout` as `global=true` w/
  env fallbacks → natural home for global `--correlation-id` + `ENGRAM_CORRELATION_ID`.
- IPC threading: central injection in `runner.rs::run_tool_timed` (merge
  `_meta.correlation_id` into params); daemon extracts via new `extract_correlation_id`
  mirroring `policy::extract_agent_role`. Envelope-level ⇒ NO MCP per-tool schema change.

## Artifacts edited
- `docs/decisions/decision-017 - ...pivot.md` — frontmatter `amended: 2026-07-03`;
  D9 + A2 marked SUPERSEDED; added **Amendment 1** (§A3.pre grounding, A1 CLI arg,
  A2 threading, A3 direct-emit overturn, A4 dual-source, A5 hardening).
- `docs/exec-plans/2026-07-02-engram-usage-telemetry-emit-plan.md` — objective, file map,
  record schema (dual-source), Constitution Check (+VI.a), §5 now **6 tasks**, §6.1 harden
  additions, §7.1 plan-review re-run (PASS), §8 out-of-scope (CLI-direct now IN).
- `.backlogit/queue/067-F.md` — DoD/goals/body updated (dual-source, CLI, direct).
- `.backlogit/queue/067.001-T.md`, `067.002-T.md`, `067.004-T.md` — amended bodies/deps.
- **NEW** `.backlogit/queue/067.005-T.md` — CLI `--correlation-id` arg + IPC `_meta`
  threading (flags.rs + runner.rs). deps: 067.002-T.
- **NEW** `.backlogit/queue/067.006-T.md` — CLI-direct emission (direct.rs +
  indexing.rs). deps: 067.003-T, 067.005-T.
- `.backlogit/queue/067-S.md` — manifest items now
  `067-F, 067.001-T, 067.002-T, 067.003-T, 067.005-T, 067.006-T, 067.004-T`.

## Task DAG (queued)
001 → 002 → 005 → 006 → 004 ; 001 → 003 → 006 ; 003 → 004. Acyclic. Width-isolated
(models / dispatch / services / cli-ipc / cli-direct / tests). Each ≤3 files, ≤5 fns,
≤4 test scenarios.

## Record schema (unchanged shape; `correlation_id` now dual-source)
schema_version=2; pinned ISO-8601-UTC `timestamp`; `correlation_id: Option<String>` from
MCP `_meta.correlation_id` OR CLI `--correlation-id`/`ENGRAM_CORRELATION_ID`, omitted when
neither; both sources validated (strip control/newline, cap 128).

## plan-review (self-conducted, degraded) — Amendment 1: PASS
P1 resolved: F8 (shared-choke-point correction → dedicated hook), F9 (direct-mode test
proof), F10 (id input validation). P2: F11 (DaemonLock mutual exclusion), F12 (shutdown
drain before exit). P3 accepted: F13 (env precedence), F14 (inert flag on non-emitters).

## Traceability note
Could not append backlogit comments (MCP down; registry has no CLI fallback for
`append_comment`) — traceability captured in markdown bodies + plan + decision instead.
Stash 7D8F395B (archived at harvest) NOT resurrected; DAX stash F7E89921 untouched.

## Next steps (for Ship)
Ship claims 067-S when ready, carries the uncommitted `.backlogit/queue/067*.md` +
doc edits, and executes t1→t2/t3→t5→t6→t4 per the DAG. Build gate: `cargo fmt`/`clippy
-D pedantic`/`cargo test`. Watch: id validation (t2/t5), direct-mode drain-before-exit
(t6), cross-platform path assertions (t4).
