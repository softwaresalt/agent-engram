---
id: decision-017
title: Engram usage-telemetry EMIT (not ingest) — pivot from telemetry SINK to SOURCE
date: 2026-07-02
amended: 2026-07-03
status: Accepted (amended 2026-07-03 — see Amendment 1)
author: stage
supersedes_scope:
  - "064-F Phase 2c — ExecutionEpoch CozoDB schema (telemetry ingestion)"
  - "064-F Phase 2d — JSONL telemetry ingestion into engram"
source_stash: 7D8F395B
related:
  - 064-F
  - docs/exec-plans/2026-07-02-engram-usage-telemetry-emit-plan.md
---

# decision-017 — Engram usage-telemetry EMIT (not ingest)

## Status

Accepted (2026-07-02). Authored by the Stage agent in DEGRADED_MODE (backlogit
MCP transport closed; all backlog mutations executed via the `backlogit` CLI
fallback declared in `.autoharness/backlog-registry.yaml`).

## Context / Problem Frame

The operator has pivoted engram's telemetry direction. autoharness will own
**measuring harness effectiveness**, so engram no longer needs to **ingest**
telemetry. Instead, engram must **emit** its own per-call query/usage telemetry
to an append-only JSONL file so autoharness gains visibility into engram queries
and usage.

This **inverts** feature `064-F` ("Deterministic gates & telemetry — engram
Structural Authority + Telemetry **sink**"). The two deferred Phase‑2 ingestion
workstreams described in `064-F`'s body are now obsolete:

- Phase 2c — ExecutionEpoch CozoDB schema (ingestion)
- Phase 2d — JSONL telemetry ingestion into engram

The operator directed these be retired (superseded) and replaced by a
telemetry‑**output** feature. Source: stash `7D8F395B` (priority high, kind
feature), including the 2026‑07‑02 addendum on correlation‑id and pinned
timestamps.

## Grounding — what actually exists today (correction to the stash premise)

The stash grounding states *"NO existing usage.jsonl emission today … the
metrics service exists but does not persist per-call usage telemetry to JSONL."*
Direct source inspection shows this premise is **materially stale**: a per-call
usage.jsonl emitter **already exists and runs**. This reframes the work from
*"build a new emitter"* to *"extend the existing emitter, schema, config, and
choke-point coverage."*

| Concern | Real module(s) | State today |
|---|---|---|
| Single tool-dispatch choke point | `src/tools/mod.rs::dispatch()` (reached via `src/daemon/ipc_server.rs:332`) | Already measures latency (`start.elapsed()`) and calls `metrics::record(UsageEvent{…})` on the denied path (L232) and success/error path (L339). Timestamp = `chrono::Utc::now().to_rfc3339()` (ISO-8601 UTC). |
| JSONL emitter | `src/services/metrics.rs` — `record()` (non-blocking mpsc), `writer_loop`, `append_event_line` | Already appends serialized `UsageEvent` to `.engram/metrics/{branch}/usage.jsonl` via `OpenOptions::append(true)`. |
| Record model | `src/models/metrics.rs::UsageEvent` | Has `tool_name`, `timestamp`, request/response bytes, token estimates, `result_count`, `response_shape_counts`, `symbols_returned`, `results_returned`, `branch`, `connection_id?`, `agent_role?`, `outcome`, attributed tokens. **Missing:** `correlation_id`, `latency_ms`, explicit `workspace`, coarse params, `schema_version`. |
| Config toggle | `MetricsConfig` (`src/models/metrics.rs:154`), surfaced as `WorkspaceConfig.metrics` (`src/models/config.rs:24`), parsed by `src/services/config.rs::parse_config` | Has `enabled` (default **true** → opt-out) + `buffer_size` (1024). **Missing:** path override, rotation/size-cap. |
| Choke-point coverage | `should_record_metrics` (`src/tools/mod.rs:34`) | **Excludes** `set_workspace`, `sync_workspace`, `index_workspace` — but the stash lists `sync_workspace`/`set_workspace` as required emitters. Coverage gap. |
| Correlation-id extraction pattern | `src/services/policy.rs::extract_agent_role` (reads `_meta.agent_role`) | Exact pattern to mirror for `_meta.correlation_id`. |
| Atomic write (for rotation) | `src/services/dehydration.rs` — temp-file-then-rename (`serialize_nodes_jsonl` + helper L165–187, `tokio::fs::rename`) | Reuse for rotation rename; per-line append (existing metrics approach) is correct for streaming writes. |
| Data-dir / branch resolution | `src/db/workspace.rs::resolve_data_dir` + `resolve_git_branch` | Available. |
| Watcher safety | `src/daemon/watcher.rs` | `.engram/` is **always** excluded (`REQUIRED_INTERNAL_EXCLUDE_PREFIXES`); a test already pins `.engram/metrics/usage.jsonl`. |
| CLI-direct / daemonless | `src/cli/direct.rs::run_direct_sync` | Bypasses `tools::dispatch`, calls `services::code_graph` directly, does **not** init/emit metrics. |

## Decisions

### D1 — Extend the existing emitter; do NOT build a parallel one
Reuse `src/services/metrics.rs` + `UsageEvent`. Building a second emitter would
duplicate the mpsc writer, branch handling, and append path, and risk divergent
schemas. **Chosen: extend.**

### D2 — `correlation_id` transport via `_meta.correlation_id`
The addendum requires a caller-supplied correlation id "parameterized" on tool
calls. Two options: (a) a new optional top-level param on every tool's input
schema; (b) a single `_meta.correlation_id` field extracted once at the choke
point, mirroring the existing `_meta.agent_role` pattern.
**Chosen: (b) `_meta.correlation_id`** — one extraction site, zero per-tool
param-struct churn, consistent with the established `_meta` identity convention,
and still "caller-supplied and parameterized." Documented as the accepted
correlation parameter in the MCP tool docs. When absent, the field is omitted
(`skip_serializing_if`). (Rationale for not choosing (a): touching every tool
input schema multiplies blast radius across ~14 handlers for no functional gain
over `_meta`.)

### D3 — Record schema is a public contract; version it and keep it back-compatible
The `.engram/metrics/{branch}/usage.jsonl` record becomes a **public contract**
autoharness parses. Add a pinned `schema_version` and add all new fields with
serde defaults / `skip_serializing_if` so old readers and pre-existing files
remain parseable. Field names are stable and snake_case.

### D4 — Path convention: keep branch-aware, add override
Existing convention is branch-aware: `.engram/metrics/{branch}/usage.jsonl`.
The stash suggests a flat `.engram/metrics/usage.jsonl`. **Chosen: keep
branch-aware** (records already carry `branch`; branch-partitioned files match
the existing `summary.json` layout and avoid cross-branch interleaving), and add
an optional `usage_path_override` config for operators/autoharness that want a
fixed path. The watcher exclusion covers all of `.engram/` regardless.

### D5 — Rotation: size-cap + rename-based rotation with bounded retention
usage.jsonl can grow unbounded. Add `max_file_bytes` (default 10 MiB) and
`max_rotated_files` (default 5). When the active file exceeds the cap, rotate
`usage.jsonl → usage.1.jsonl → … → usage.N.jsonl` (drop oldest) using the
cross-platform atomic `tokio::fs::rename` pattern from `dehydration.rs`. Normal
per-line writes remain append-only.

### D6 — Default remains opt-out (`enabled = true`), with explicit toggle + path override
The emitter is already on by default and autoharness now depends on it. Keeping
`enabled = true` avoids a silent regression; operators can disable via
`MetricsConfig.enabled = false`. Add `usage_path_override`, `max_file_bytes`,
`max_rotated_files`.

### D7 — Choke-point coverage: add `set_workspace`, `sync_workspace`, `index_workspace`
Extend `should_record_metrics` to include these. **Nuance:** `set_workspace` is
what *initializes* the metrics writer (`lifecycle.rs:147`), so the very first
`set_workspace` call may be recorded before a sender exists and will no-op
(dropped) — acceptable and documented; subsequent calls emit normally.

### D8 — Standalone feature (067-F), linked to 064-F (not new tasks under 064-F)
`064-F` is framed as a telemetry **sink** in the **verify/CozoDB-ingestion**
width domain; its only remaining valid work (`064.004-T` Phase 1b reactive-sync)
is a daemon-watcher concern. This work is a telemetry **source** in the
**metrics-emitter / tool-dispatch** width domain. **Chosen: standalone feature**
for clean width isolation, shipment isolation, and traceability, `related_to`
064-F. This also avoids reopening the `064-*` slot-numbering confusion
(the TMDL family was re-IDed to `066-*` per
`docs/decisions/2026-07-01-064-id-namespace-collision-reconciliation.md`).

### D9 — Scope emission to daemon-served calls (via `tools::dispatch`)
> ⚠️ **SUPERSEDED by Amendment 1 (2026-07-03, §A3).** CLI-direct emission is now
> IN scope because the operator's MCP-fallback scenario is precisely "daemon
> unavailable → use the CLI (`--direct`)". Retained below for history.

CLI-direct/daemonless mode (`run_direct_sync`) bypasses the choke point and is a
narrow index/sync path, not the query surface autoharness measures. **Chosen:**
daemon-served emission only for this feature; CLI-direct coverage is an explicit
out-of-scope follow-up (stashable).

## Defined JSONL record schema (usage.jsonl, schema_version = 2)

One JSON object per line. **Existing fields retained unchanged** for
back-compat; **new fields** marked ★.

| Field | Type | Req | Notes |
|---|---|---|---|
| `schema_version` ★ | u32 | required | Pinned `2`. autoharness keys format detection off this. |
| `timestamp` | string | required | ISO-8601 **UTC** (`chrono::Utc::now().to_rfc3339()`). Pinned per addendum #2. |
| `tool_name` | string | required | e.g. `unified_search`, `query_memory`, `map_code`, `list_symbols`, `impact_analysis`, `query_graph`, `sync_workspace`, `set_workspace`. |
| `workspace` ★ | string | required | Workspace path/id resolved via `resolve_data_dir` context. |
| `branch` | string | required | Sanitized via `resolve_git_branch`. |
| `correlation_id` ★ | string | optional | Caller-supplied via `_meta.correlation_id`; **omitted when absent** (`skip_serializing_if`). Per addendum #1. |
| `latency_ms` ★ | u64 | required | `start.elapsed().as_millis()` at the dispatch choke point. |
| `outcome` | string | required | `success` \| `error` \| `denied`. |
| `params_summary` ★ | object | optional | Coarse, privacy-preserving: `{ query_hash?: string, query_len?: u32, limit?: u32 }`. No raw query text persisted. |
| `result_count` | u32 | required | Canonical result item count. |
| `symbols_returned` / `results_returned` | u32 | required | Existing tool-specific counters. |
| `request_bytes` / `response_bytes` | u64 | required | Existing payload sizes. |
| `estimated_input_tokens` / `estimated_output_tokens` / `estimated_tokens` | u64 | required | Existing token-savings figures (bytes/4). |
| `response_shape_counts` | map<string,u32> | optional | Existing; omitted when empty. |
| `connection_id` / `agent_role` | string | optional | Existing; omitted when absent. |
| `prompt_tokens_attributed` / `completion_tokens_attributed` / `cached_tokens_attributed` | u64 | optional | Existing runtime attribution. |

Back-compat guarantee: readers of `schema_version < 2` files remain valid; new
fields default/omit; no existing field is renamed or removed.

## Chosen hook point

`src/tools/mod.rs::dispatch()` — the single dispatch function for **all** MCP
tools (invoked by `src/daemon/ipc_server.rs`). Both existing `metrics::record`
sites (denied L232, success/error L339) are extended to populate the new fields;
`_meta.correlation_id` is extracted once (mirroring `extract_agent_role`);
`latency_ms` reuses the already-computed `start.elapsed()`.

## PART A — Cancellation of the superseded ingestion workstream (record)

**Finding (evidence-backed):** Tasks `064.005-T` and `064.006-T` **were never
instantiated**. Index count is 0; no files exist in `.backlogit/queue` or
`.backlogit/archive`; the `064-F` ship-note explicitly documents them as
*reserved-but-free planned slots* after the TMDL family was re-IDed to `066-*`.
There is therefore **nothing to archive**.

**Realization of the cancellation intent:** the `064-F` body/DoD is updated to
mark Phase 2c/2d ingestion **DROPPED — superseded by 067-F** (this decision),
and to record that slots `064.005-T` / `064.006-T` will **not** be created.
`064.004-T` (Phase 1b reactive-sync) remains **queued and untouched** — it is
independent daemon-watcher work, not telemetry ingestion. Supersession is also
recorded via a `relates_to` dependency edge `067-F → 064-F` (the backlogit CLI
exposes no semantic `supersedes` link; `add_link` MCP is unavailable in this
degraded session).

## Risks & Assumptions

- **R1 (contract):** usage.jsonl is now a public contract autoharness parses —
  mitigated by `schema_version` + additive/back-compat fields.
- **R2 (concurrency):** many concurrent tool calls append + occasional rotation.
  The existing mpsc single-writer (`writer_loop`) serializes writes; rotation
  happens on the same writer task, so append/rotate cannot interleave. Rotation
  uses atomic rename.
- **R3 (correlation coverage):** `_meta.correlation_id` only flows if callers
  populate `_meta`; absent otherwise (by design).
- **R4 (set_workspace first-call):** first `set_workspace` may pre-date writer
  init and no-op (D7 nuance) — accepted.
- **A1:** autoharness consumes branch-aware paths (or uses `usage_path_override`).
- **A2:** daemon-served emission is sufficient for autoharness effectiveness
  measurement; CLI-direct is out of scope (D9). — ⚠️ **SUPERSEDED by Amendment 1
  (§A3):** CLI-direct emission is now required for the MCP-fallback scenario.

---

## Amendment 1 (2026-07-03) — CLI `--correlation-id` + CLI-direct emission (MCP fallback)

**Trigger.** Operator directive (2026-07-03). `_meta.correlation_id` is accepted for
MCP mode (D2 stands), but engram is **also** invoked via the CLI, and when the harness
loses MCP access it MUST fall back to the CLI. Therefore (1) the engram CLI MUST accept
a correlation id as an argument on the emitting command(s) and thread it to the same
`usage.jsonl` `correlation_id` field; and (2) the CLI-direct (daemonless) tool-call path
MUST also emit usage telemetry.

This amendment is **additive** to the merged plan (PR #189 @ `f2835847`): it introduces
new surface but renames/removes no shipped field and no prior record semantics.
`schema_version` stays `2`; additive/back-compat discipline (D3) is preserved.

### A3.pre — Grounding correction: daemon and direct do NOT share a choke point
Direct source inspection (2026-07-03) confirms the operator's "confirm shared choke
point" premise is **false**:

- `src/tools/mod.rs::dispatch()` is reached **only** by the daemon IPC path
  (`src/daemon/ipc_server.rs` → `dispatch`). Both existing `metrics::record` sites live
  here.
- `src/cli/direct.rs::run_direct_sync` (invoked by `engram sync|index --direct` /
  `ENGRAM_DIRECT=1`) **bypasses `dispatch` entirely** and calls
  `services::code_graph::{index_workspace_with_progress, sync_workspace_with_progress}`
  directly. It never initializes or emits metrics today.

⇒ There is **no shared emit hook** for direct mode. A **minimal dedicated hook** in
`run_direct_sync` is required (see A3).

### A1 — CLI `--correlation-id` argument (new; decided: global flag + env fallback)
Add an optional `--correlation-id <string>` as a **global** CLI flag in
`GlobalFlags` (`src/cli/flags.rs`), mirroring the existing `--workspace`
(`global = true`), with env fallback **`ENGRAM_CORRELATION_ID`** (mirrors
`ENGRAM_WORKSPACE` / `ENGRAM_DIRECT` / `ENGRAM_CLI_TIMEOUT`).
**Precedence:** explicit `--correlation-id` flag > `ENGRAM_CORRELATION_ID` env > unset.

*Rationale (global + env chosen over per-subcommand arg):* zero per-subcommand churn,
uniform availability, and — decisively — the harness can export the id **once** for
every fallback CLI call in the exact MCP-loss scenario without rewriting each command
line. clap `global = true` is exactly how `--workspace` already behaves.

**Emitting subcommands** (reach a recorded tool → the arg is meaningful):
`search`(`unified_search`), `query-memory`(`query_memory`), `symbols`(`list_symbols`),
`map-code`(`map_code`), `impact`(`impact_analysis`), `query-graph`(`query_graph`),
`sync`(`sync_workspace`), `index`(`index_workspace`), `bind`(`set_workspace`, once D7
coverage lands), plus the `report`/`stats`/`health`/`branch-metrics`/`workspace-status`/
`daemon-status` `get_*` reads already in `should_record_metrics`. The flag is inert
(harmless) on non-emitting subcommands (`install`/`uninstall`/`manifest`/`verify`).

### A2 — Threading the CLI id to the emit choke point (per transport)
- **IPC / daemon path** (`src/cli/runner.rs::run_tool_timed`): when
  `flags.correlation_id` is set, merge it into the outgoing request params as
  `_meta.correlation_id` (creating the `_meta` object if absent) **before** building the
  `IpcRequest`. The daemon then extracts it at the **same single choke point**
  `src/tools/mod.rs::dispatch` via a new `extract_correlation_id` helper that mirrors
  `policy::extract_agent_role` (`_meta.agent_role`). One injection site (CLI), one
  extraction site (daemon). No per-command builder churn (`&flags` is already threaded).
- **CLI-direct path** (`src/cli/direct.rs::run_direct_sync`): the id is passed as a
  function parameter (from `flags.correlation_id` via `indexing::run_sync`/`run_index`)
  and written straight onto the `UsageEvent.correlation_id` field (see A3).

Because `_meta.correlation_id` is **envelope-level** (not a per-tool input field), there
is **no MCP per-tool input-schema change** — D2 is unaffected. The CLI `--correlation-id`
is a **CLI-surface (clap) addition only**.

### A3 — OVERTURN of D9 / assumption A2: CLI-direct emission is IN scope
In `run_direct_sync`, after the `code_graph` call returns, construct a `UsageEvent`
(`tool_name` = `sync_workspace` | `index_workspace`; pinned ISO-8601-UTC `timestamp`;
`latency_ms` from the already-measured `started_at.elapsed()`; `workspace`; `branch`;
`outcome` = success/error; `correlation_id` from the CLI arg; `schema_version = 2`) and
emit it by driving the **existing** emitter in-process:
`metrics::initialize` → `metrics::record` → `metrics::shutdown().await` (the shutdown
drains the mpsc queue before the short-lived direct process exits, so the record is not
lost). This reuses the same `append_event_line` + rotation path (t3), so **direct-mode
output is byte-compatible with daemon output**.

**Concurrency (resolves the "two writers to one file" concern):** `run_direct_sync`
holds the `DaemonLock` for the whole run, so a daemon cannot be writing the same
`.engram/metrics/{branch}/usage.jsonl` concurrently (and vice-versa). The two writers
are **mutually exclusive by construction** — there is no cross-mode append race.

This **supersedes D9** ("daemon-served only") and **assumption A2** ("CLI-direct out of
scope").

### A4 — Dual-source `correlation_id` population (record contract unchanged)
`correlation_id` remains `Option<String>`, `schema_version = 2`,
`skip_serializing_if = "Option::is_none"`. It is now populated from **either** source:
MCP `_meta.correlation_id` **OR** CLI `--correlation-id` / `ENGRAM_CORRELATION_ID`;
**omitted** when neither is supplied. The pinned ISO-8601-UTC `timestamp` requirement is
unchanged. No field renamed or removed.

### A5 — Hardening (new attacker/user-controlled surface)
A caller-supplied free-text id is now written verbatim into a JSONL line. **Both** sources
(MCP `_meta` and CLI arg/env) MUST be validated before persistence:

- **Line integrity:** strip/reject control characters and newlines (`\n`/`\r`) so a
  crafted id cannot forge or split JSONL records.
- **Bounded length:** cap at 128 chars (truncate-or-reject; decided: reject with a clear
  CLI error for the direct/CLI path, sanitize-and-truncate for the envelope path to avoid
  failing daemon calls). Documented in the exec-plan plan-harden addendum.
- **No PII amplification:** only the opaque id is persisted; no additional caller context
  is captured beyond existing fields.

See exec-plan `2026-07-02-engram-usage-telemetry-emit-plan.md` §6 (plan-harden) and §7
(plan-review) Amendment for the folded requirements and review outcome.
