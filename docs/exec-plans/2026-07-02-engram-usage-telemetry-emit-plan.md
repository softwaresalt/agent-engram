---
title: Engram usage-telemetry EMIT (per-call usage.jsonl) — implementation plan
date: 2026-07-02
status: reviewed
author: stage
feature: 067-F
source_stash: 7D8F395B
decision: docs/decisions/decision-017 - Engram-usage-telemetry-emit-not-ingest-pivot.md
supersedes_scope:
  - 064-F Phase 2c (ExecutionEpoch CozoDB schema)
  - 064-F Phase 2d (JSONL telemetry ingestion)
risk: elevated
---

# Impl-plan — Engram usage-telemetry EMIT

## 1. Objective

Extend engram's **existing** per-call usage emitter so it produces telemetry
autoharness can consume: add a caller-supplied `correlation_id`, per-call
`latency_ms`, explicit `workspace`, coarse `params_summary`, and a pinned
`schema_version`/ISO-8601-UTC `timestamp` to each `.engram/metrics/{branch}/usage.jsonl`
record; add config path-override + size-cap rotation; and close the
choke-point coverage gap for `set_workspace`/`sync_workspace`/`index_workspace`.

This is an **extension**, not a greenfield emitter (see decision-017 grounding
correction: the emitter already exists in `src/services/metrics.rs`).

## 2. Grounded file map (real modules)

| Concern | File | Action |
|---|---|---|
| Record model + config | `src/models/metrics.rs` (`UsageEvent`, `MetricsConfig`) | Add fields (`schema_version`, `correlation_id`, `latency_ms`, `workspace`, `params_summary` + `CoarseParams`); add `usage_path_override`, `max_file_bytes`, `max_rotated_files`. |
| Choke-point hook | `src/tools/mod.rs` (`dispatch`, `should_record_metrics`) | Populate new fields at both `metrics::record` sites; extract `_meta.correlation_id`; pass `latency_ms`; extend coverage list. |
| Correlation extractor | `src/services/policy.rs` (`extract_agent_role`) | Add sibling `extract_correlation_id` (`_meta.correlation_id`). |
| Emitter + rotation | `src/services/metrics.rs` (`append_event_line`, `writer_loop`) | Honor path override; size-cap rotation via atomic rename. |
| Config surface | `src/models/config.rs` (`WorkspaceConfig.metrics`), `src/services/config.rs` (`parse_config`) | New `MetricsConfig` fields flow through automatically; add defaults + parse coverage. |
| Atomic rename reference | `src/services/dehydration.rs` (temp-then-rename, `tokio::fs::rename`) | Reuse pattern for rotation. |
| Watcher safety | `src/daemon/watcher.rs` | No change; `.engram/` already excluded (regression test only). |
| Data-dir/branch | `src/db/workspace.rs` (`resolve_data_dir`, `resolve_git_branch`) | Reuse for `workspace` field. |

## 3. Record schema (contract)

See decision-017 "Defined JSONL record schema (schema_version = 2)". Summary of
**new** fields: `schema_version:u32=2`, `correlation_id:Option<String>` (from
`_meta.correlation_id`), `latency_ms:u64`, `workspace:String`,
`params_summary:Option<{query_hash?,query_len?,limit?}>`. All new fields are
additive with serde defaults / `skip_serializing_if`; no existing field renamed.

## 4. Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust (no `unsafe`, `Result<T,EngramError>`, no `unwrap/expect`) | Plan uses `?` propagation; rotation errors map to `EngramError::Metrics(MetricsError::WriteFailed)`; no `unwrap` in new paths. |
| II. Test-First (harness before code) | Each task lists failing-first tests across the three tiers before impl. |
| III. Workspace isolation / path containment | usage.jsonl always under `.engram/`; `usage_path_override` MUST be validated to resolve within the workspace root (containment check). |
| IV. CLI workspace containment | No writes outside workspace tree. |
| V. Destructive-command approval | Rotation renames files **inside** `.engram/metrics/` only (bounded retention drop of oldest rotated file) — non-destructive to source; documented. |
| VI. Safety modes for risky work | Elevated blast radius (public persisted contract + MCP `_meta` param + concurrency + config) → **plan-harden** addendum below; freeze-scope to the metrics/dispatch/config width. |
| Three-tier tests | contract (`tests/contract`), integration (`tests/integration`, real DB/IPC), unit (`tests/unit`, proptest). |
| Naming / error style | snake_case fns, `EngramError` results, doc comments on public items. |

## 5. Task decomposition (2-hour, width-isolated, ≤3 files / ≤5 fns / ≤4 test scenarios)

Dependency order: **t1 → (t2, t3) → t4**. t2 and t3 are parallel-safe after t1.

- **t1 — Record + config schema extension** (width: `models`)
  - Extend `UsageEvent` (+ `CoarseParams`) and `MetricsConfig` in `src/models/metrics.rs`; ensure `WorkspaceConfig` default flows.
  - Tests (unit/proptest): (1) serde round-trip incl. new fields; (2) back-compat deserialize of a v1 record (missing new fields) succeeds; (3) `correlation_id`/`params_summary` omitted when `None`; (4) `MetricsConfig` defaults (opt-out on, rotation defaults).
  - Files: `src/models/metrics.rs`, `tests/unit/metrics_record_schema.rs` (+ `src/models/config.rs` default if needed). No deps.

- **t2 — Choke-point hook + coverage** (width: `tools/dispatch`)
  - In `src/tools/mod.rs::dispatch`: populate `schema_version`, `latency_ms` (from `start.elapsed().as_millis()`), `workspace`, `params_summary`, `correlation_id` at both `metrics::record` sites; add `extract_correlation_id` (`src/services/policy.rs`); extend `should_record_metrics` with `set_workspace`/`sync_workspace`/`index_workspace`.
  - Tests (unit): (1) `extract_correlation_id` reads `_meta.correlation_id`; (2) absent `_meta` → None; (3) coverage predicate now includes the 3 tools; (4) `params_summary` hash/limit derivation from sample params.
  - Files: `src/tools/mod.rs`, `src/services/policy.rs`, `tests/unit/dispatch_usage_hook.rs`. Depends on t1.

- **t3 — Emitter rotation + path override** (width: `services/metrics`)
  - In `src/services/metrics.rs`: honor `usage_path_override` (with workspace-containment validation); implement size-cap rotation (`max_file_bytes`/`max_rotated_files`) using atomic `tokio::fs::rename` in `writer_loop` (single-writer, so append/rotate cannot interleave).
  - Tests (unit): (1) rotation triggers at cap and preserves lines; (2) retention drops oldest beyond `max_rotated_files`; (3) path override honored + containment-rejected when escaping workspace; (4) append still atomic per line.
  - Files: `src/services/metrics.rs`, `tests/unit/metrics_rotation.rs`, `src/services/config.rs` (parse defaults). Depends on t1.

- **t4 — Cross-platform contract + integration tests** (width: `tests`)
  - Contract test: a real tool response path yields a usage.jsonl record whose shape includes `schema_version=2`, ISO-8601-UTC `timestamp`, `correlation_id` (when `_meta` supplied), `latency_ms`, `workspace`, `branch`, `outcome`.
  - Integration test: real daemon/IPC `unified_search` (or `query_memory`) call with `_meta.correlation_id` set → record appended under branch-aware `.engram/metrics/{branch}/usage.jsonl` with matching `correlation_id`; cross-platform path assertions (Windows `\` vs POSIX `/`).
  - Files: `tests/contract/usage_telemetry_record.rs`, `tests/integration/usage_telemetry_emit.rs`. Depends on t2, t3.

## 6. plan-harden addendum (risk-triggered)

Triggers met: (a) **new persisted file-format contract** autoharness depends on
(public contract); (b) **MCP `_meta` param** surface change; (c) **concurrency**
(many tool calls + rotation); (d) **config** surface change.

Hardening requirements folded into the tasks:

1. **Contract stability** — `schema_version` pinned; additive-only fields; a
   golden contract test (t4) locks the record shape; back-compat deserialize
   test (t1) guards old files.
2. **Concurrency safety** — all writes and rotation occur on the single mpsc
   `writer_loop` task; document that append and rename never interleave; rotation
   uses atomic rename (crash-safe). No new shared mutable state beyond the
   existing channel.
3. **Path containment** — `usage_path_override` MUST be validated within the
   workspace root before use (Constitution III); escaping paths rejected with
   `EngramError` (t3 test).
4. **Bounded growth** — `max_file_bytes` + `max_rotated_files` enforce an upper
   bound on disk; retention drop is the only file deletion and is confined to
   rotated `.engram/metrics/**/usage.N.jsonl` files.
5. **Privacy** — `params_summary` persists a hash + length + limit, never raw
   query text.
6. **No latency regression** — emission remains non-blocking (`try_send`, drop on
   full channel); `latency_ms` measures dispatch, not emission.

## 7. plan-review record (self-conducted, persona-structured)

Executed by Stage in DEGRADED_MODE (no subagent-dispatch surface available; the
`plan-review` skill's persona reviewers were applied as a structured self-review).
Personas: Architecture, Testing/Contract, Safety/Concurrency, Operability.

| # | Persona | Severity | Finding | Resolution |
|---|---|---|---|---|
| F1 | Architecture | P1 (blocking) | Risk of a **duplicate** parallel emitter contradicting the existing `metrics.rs` writer. | Resolved by D1 — extend the existing emitter; plan edits `metrics.rs` in place. |
| F2 | Testing/Contract | P1 (blocking) | usage.jsonl is a public contract; without a version + golden test, autoharness parsing could silently break. | Resolved by D3 + t1 back-compat test + t4 golden contract test; `schema_version=2` pinned. |
| F3 | Safety/Concurrency | P1 (blocking) | Rotation racing concurrent appends could corrupt/lose lines. | Resolved — single-writer `writer_loop` serializes append+rotate; atomic rename (t3 tests 1,4). |
| F4 | Safety | P2 | `usage_path_override` could escape the workspace (Constitution III). | Resolved — containment validation required (t3 test 3). |
| F5 | Operability | P2 | Unbounded usage.jsonl growth. | Resolved by D5 rotation + retention (t3 tests 1,2). |
| F6 | Architecture | P2 | Addendum says "param on tool schemas"; `_meta` deviates. | Accepted with justification (D2): `_meta.correlation_id` is caller-supplied/parameterized, documented; avoids ~14-handler schema churn. Recorded as an explicit deviation for operator awareness. |
| F7 | Testing | P3 | `set_workspace` first-call may no-op emission. | Accepted/documented (D7 nuance); not gate-blocking. |

**Outcome: PASS.** All P1 (blocking) findings resolved within scope; P2 findings
have concrete task-level mitigations; P3 accepted. 1 review cycle (within the
3-cycle limit). One explicit, justified deviation (F6) flagged for the operator.

## 8. Out of scope (stashable follow-ups)

- CLI-direct/daemonless emission (`run_direct_sync`) — D9.
- Per-tool input-schema `correlation_id` param (vs `_meta`) — only if the operator
  rejects the `_meta` approach (F6).
