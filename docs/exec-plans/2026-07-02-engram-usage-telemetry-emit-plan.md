---
title: Engram usage-telemetry EMIT (per-call usage.jsonl) — implementation plan
date: 2026-07-02
amended: 2026-07-03
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

> **Amendment 1 (2026-07-03):** folds in the operator's CLI correlation-id +
> CLI-direct-emission requirement. Changes: Scope/Objective (§1), file map (§2),
> record schema note (§3), Constitution Check (§4), task decomposition (§5 — now
> **six** tasks, adds 067.005-T and 067.006-T), plan-harden (§6), plan-review (§7),
> out-of-scope (§8). See decision-017 Amendment 1 (§A1–A5) for the decision record.

## 1. Objective

Extend engram's **existing** per-call usage emitter so it produces telemetry
autoharness can consume: add a caller-supplied `correlation_id`, per-call
`latency_ms`, explicit `workspace`, coarse `params_summary`, and a pinned
`schema_version`/ISO-8601-UTC `timestamp` to each `.engram/metrics/{branch}/usage.jsonl`
record; add config path-override + size-cap rotation; and close the
choke-point coverage gap for `set_workspace`/`sync_workspace`/`index_workspace`.

**Amendment 1 (2026-07-03) additions:**

- **CLI `--correlation-id`** — accept the correlation id as a **global** CLI flag
  (`GlobalFlags`, mirrors `--workspace`) with env fallback `ENGRAM_CORRELATION_ID`
  (precedence: flag > env > unset), threaded to the same `usage.jsonl`
  `correlation_id` field. IPC path injects `_meta.correlation_id` into the request in
  `runner.rs::run_tool_timed`; direct path passes it straight onto the `UsageEvent`.
- **CLI-direct emission (scope reversal)** — the daemonless `run_direct_sync`
  (`engram sync|index --direct`) MUST now also emit a usage record (with the
  correlation id), because the operator's MCP-fallback path is exactly "daemon
  unavailable → use the CLI". Grounding shows direct mode **bypasses** `tools::dispatch`
  and shares **no** choke point with the daemon, so a **minimal dedicated hook** is
  added in `run_direct_sync` that drives the same emitter in-process
  (`metrics::initialize`→`record`→`shutdown`). `correlation_id` is **dual-source**
  (MCP `_meta` OR CLI arg); the pinned ISO-8601-UTC `timestamp` requirement is retained.

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
| **CLI arg (global flag)** ★A1 | `src/cli/flags.rs` (`GlobalFlags`) | Add `--correlation-id` (`global = true`, `env = "ENGRAM_CORRELATION_ID"`), mirroring `--workspace`. |
| **IPC id injection** ★A1 | `src/cli/runner.rs` (`run_tool_timed`) | When `flags.correlation_id` set, merge `_meta.correlation_id` into request params before building `IpcRequest`. |
| **CLI-direct emit hook** ★A1 | `src/cli/direct.rs` (`run_direct_sync`) | After the `code_graph` call, build a `UsageEvent` (id from CLI arg) and emit via `metrics::initialize`→`record`→`shutdown` (drain-before-exit). |
| **Direct id passthrough** ★A1 | `src/cli/commands/indexing.rs` (`run_sync`, `run_index`) | Thread `flags.correlation_id` into `run_direct_sync`. |

**Choke-point reality (Amendment 1):** `tools::dispatch` is the daemon-only emit hook
(reached via `ipc_server`). `run_direct_sync` **bypasses** it and calls
`services::code_graph` directly — so the CLI-direct emit is a **separate, minimal hook**,
not a `dispatch` extension. The two writers never overlap: direct mode holds the
`DaemonLock` for its whole run.

## 3. Record schema (contract)

See decision-017 "Defined JSONL record schema (schema_version = 2)". Summary of
**new** fields: `schema_version:u32=2`, `correlation_id:Option<String>`, `latency_ms:u64`,
`workspace:String`, `params_summary:Option<{query_hash?,query_len?,limit?}>`. All new
fields are additive with serde defaults / `skip_serializing_if`; no existing field renamed.

**`correlation_id` is dual-source (Amendment 1 §A4):** populated from **either** the MCP
`_meta.correlation_id` envelope field **OR** the CLI `--correlation-id` /
`ENGRAM_CORRELATION_ID` arg; **omitted** (`skip_serializing_if`) when neither is supplied.
The record shape is identical across daemon and CLI-direct emission. The pinned
ISO-8601-UTC `timestamp` is required on every record from both paths. Both id sources are
validated (control-char/newline strip, 128-char cap) before persistence (see §6 harden).

## 4. Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust (no `unsafe`, `Result<T,EngramError>`, no `unwrap/expect`) | Plan uses `?` propagation; rotation errors map to `EngramError::Metrics(MetricsError::WriteFailed)`; no `unwrap` in new paths. |
| II. Test-First (harness before code) | Each task lists failing-first tests across the three tiers before impl. |
| III. Workspace isolation / path containment | usage.jsonl always under `.engram/`; `usage_path_override` MUST be validated to resolve within the workspace root (containment check). Direct-mode emit writes to the same branch-aware `.engram/metrics/{branch}/usage.jsonl`. |
| IV. CLI workspace containment | No writes outside workspace tree. CLI `--correlation-id` is inert data (an opaque id), never a path. |
| V. Destructive-command approval | Rotation renames files **inside** `.engram/metrics/` only (bounded retention drop of oldest rotated file) — non-destructive to source; documented. |
| VI. Safety modes for risky work | Elevated blast radius (public persisted contract + MCP `_meta` param + **new public CLI arg** + **new CLI-direct emit path** + concurrency + config) → **plan-harden** addendum below; freeze-scope to the metrics/dispatch/config/**cli** width. |
| **VI.a CLI/direct surface (Amendment 1)** | New global `--correlation-id` flag + `ENGRAM_CORRELATION_ID` env are a public CLI contract; the id is validated (control-char/newline strip, 128-char cap) before it enters a JSONL line. CLI-direct emit reuses the vetted `metrics::*` path; no new file-format branch. |
| Three-tier tests | contract (`tests/contract`), integration (`tests/integration`, real DB/IPC), unit (`tests/unit`, proptest). |
| Naming / error style | snake_case fns, `EngramError` results, doc comments on public items. |

## 5. Task decomposition (2-hour, width-isolated, ≤3 files / ≤5 fns / ≤4 test scenarios)

**Amendment 1** grows this from 4 to **6 tasks** (adds t5 CLI-arg/IPC wiring and t6
CLI-direct emission). Dependency order:

```
t1 ─┬─► t2 ──► t5 ─┐
    └─► t3 ────────┴─► t6 ──► t4
```

`t1 → (t2, t3)`; `t5` after `t2`; `t6` after `t3` and `t5`; `t4` (full contract +
integration) after all. Each task stays width-isolated (models / dispatch / services /
cli-ipc / cli-direct / tests).

- **t1 — Record + config schema extension** (`067.001-T`, width: `models`) — *CHANGED*
  - Extend `UsageEvent` (+ `CoarseParams`, `schema_version`, `correlation_id`,
    `latency_ms`, `workspace`, `params_summary`) and `MetricsConfig`
    (`usage_path_override`, `max_file_bytes`, `max_rotated_files`) in
    `src/models/metrics.rs`; ensure `WorkspaceConfig` default flows.
  - **Note (Amendment 1):** `correlation_id: Option<String>` is **dual-source** — filled
    from MCP `_meta.correlation_id` (t2) **or** the CLI arg (t5/t6). Field stays additive,
    `skip_serializing_if`.
  - Tests (unit/proptest): (1) serde round-trip incl. new fields; (2) back-compat
    deserialize of a v1 record (missing new fields) succeeds; (3)
    `correlation_id`/`params_summary` omitted when `None`; (4) `MetricsConfig` defaults.
  - Files: `src/models/metrics.rs`, `tests/unit/metrics_record_schema.rs` (+
    `src/models/config.rs` default if needed). No deps.

- **t2 — Daemon dispatch choke-point hook + coverage** (`067.002-T`, width:
  `tools/dispatch`) — *CHANGED*
  - In `src/tools/mod.rs::dispatch`: populate `schema_version`, `latency_ms` (from
    `start.elapsed()`), `workspace`, `params_summary`, `correlation_id` at both
    `metrics::record` sites; add `extract_correlation_id` (`src/services/policy.rs`,
    mirrors `extract_agent_role`, **with control-char/newline strip + 128-char cap**);
    extend `should_record_metrics` with `set_workspace`/`sync_workspace`/`index_workspace`.
  - **Scope note (Amendment 1):** this covers the **daemon/IPC** path only. The CLI-direct
    path bypasses `dispatch` and is handled by **t6** — do not attempt to route direct
    mode through `dispatch`.
  - Tests (unit): (1) `extract_correlation_id` reads `_meta.correlation_id`; (2) absent
    `_meta` → None; (3) coverage predicate now includes the 3 tools; (4) id sanitization
    (newline stripped, over-cap rejected/truncated).
  - Files: `src/tools/mod.rs`, `src/services/policy.rs`, `tests/unit/dispatch_usage_hook.rs`.
    Depends on t1.

- **t3 — Emitter rotation + path override** (`067.003-T`, width: `services/metrics`) —
  *UNCHANGED (annotated)*
  - In `src/services/metrics.rs`: honor `usage_path_override` (with workspace-containment
    validation); implement size-cap rotation (`max_file_bytes`/`max_rotated_files`) using
    atomic `tokio::fs::rename` in `writer_loop` (single-writer, so append/rotate cannot
    interleave).
  - **Note (Amendment 1):** the same `append_event_line`/rotation path is reused by the
    CLI-direct emit (t6) via `metrics::initialize`/`record`/`shutdown` — keep it
    mode-agnostic (no daemon-only assumptions). Do **not** add CLI wiring here (width
    isolation).
  - Tests (unit): (1) rotation triggers at cap and preserves lines; (2) retention drops
    oldest beyond `max_rotated_files`; (3) path override honored + containment-rejected
    when escaping workspace; (4) append still atomic per line.
  - Files: `src/services/metrics.rs`, `tests/unit/metrics_rotation.rs`,
    `src/services/config.rs` (parse defaults). Depends on t1.

- **t5 — CLI `--correlation-id` arg + IPC threading** (`067.005-T`, width: `cli/ipc`) —
  *NEW*
  - Add `correlation_id: Option<String>` to `GlobalFlags` (`src/cli/flags.rs`) as
    `#[arg(long, global = true, env = "ENGRAM_CORRELATION_ID")]` (precedence flag > env),
    mirroring `--workspace`.
  - In `src/cli/runner.rs::run_tool_timed`: when set, merge sanitized
    `_meta.correlation_id` into the request `params` (create `_meta` if absent) before
    building `IpcRequest`. One central injection site (no per-command builder churn).
  - Tests (unit): (1) flag parses; env fallback resolves; flag overrides env;
    (2) injection merges `_meta.correlation_id` into params with no `_meta`;
    (3) injection preserves an existing `_meta.agent_role`; (4) no injection when unset
    (params unchanged).
  - Files: `src/cli/flags.rs`, `src/cli/runner.rs`, `tests/unit/cli_correlation_id.rs`.
    Depends on t2 (the daemon must extract what the CLI injects for a coherent increment).

- **t6 — CLI-direct emission** (`067.006-T`, width: `cli/direct`) — *NEW*
  - In `src/cli/direct.rs::run_direct_sync`: accept `correlation_id: Option<String>`;
    after the `code_graph` call, build a `UsageEvent`
    (`tool_name`=`sync_workspace`|`index_workspace`, pinned ISO-8601-UTC `timestamp`,
    `latency_ms` from `started_at.elapsed()`, `workspace`, `branch`, `outcome`,
    `correlation_id`, `schema_version=2`) and emit via
    `metrics::initialize`→`metrics::record`→`metrics::shutdown().await`
    (shutdown drains before the short-lived process exits). In
    `src/cli/commands/indexing.rs` (`run_sync`, `run_index`), pass `flags.correlation_id`
    through to `run_direct_sync`.
  - **Concurrency:** direct mode holds `DaemonLock` → no concurrent daemon writer to the
    same `usage.jsonl` (mutual exclusion). Emission respects `MetricsConfig.enabled`
    (opt-out) and skips cleanly when disabled.
  - Tests (integration): (1) `engram sync --direct --correlation-id X` appends a record
    with `correlation_id=X`, ISO-8601-UTC `timestamp`, and matching `tool_name`;
    (2) `--direct` without the arg omits `correlation_id`; (3) `ENGRAM_CORRELATION_ID`
    env path populates the record; (4) metrics disabled → no record written.
  - Files: `src/cli/direct.rs`, `src/cli/commands/indexing.rs`,
    `tests/integration/cli_direct_usage_emit.rs`. Depends on t3 and t5.

- **t4 — Cross-platform contract + integration tests** (`067.004-T`, width: `tests`) —
  *CHANGED*
  - Contract test: a real tool response path yields a usage.jsonl record whose shape
    includes `schema_version=2`, ISO-8601-UTC `timestamp`, `correlation_id` (when
    supplied), `latency_ms`, `workspace`, `branch`, `outcome`.
  - Integration tests (the four required dual-source proofs): (a) daemon/IPC call with
    **MCP `_meta.correlation_id`** → record carries it; (b) daemon/IPC call driven via the
    **CLI `--correlation-id`** (id arrives as `_meta` and is recorded); (c) **CLI-direct**
    `--direct` invocation emits a record carrying the correlation id (cross-references t6);
    (d) every record has a present, ISO-8601-UTC `timestamp`; cross-platform path
    assertions (Windows `\` vs POSIX `/`) on the branch-aware
    `.engram/metrics/{branch}/usage.jsonl`.
  - Files: `tests/contract/usage_telemetry_record.rs`,
    `tests/integration/usage_telemetry_emit.rs`. Depends on t2, t3, t5, t6.

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

### 6.1 plan-harden — Amendment 1 additions (2026-07-03)

New triggers: a **public CLI arg** (`--correlation-id`) + a **new CLI-direct emit path**
= broader public contract and a new caller-controlled value entering a JSONL line.

7. **Correlation-id input validation (both sources)** — the id from MCP
   `_meta.correlation_id` **and** from CLI `--correlation-id`/`ENGRAM_CORRELATION_ID` is
   caller-controlled free text. Before it is persisted: strip/reject control characters
   and `\n`/`\r` (JSONL line-integrity — prevents record forgery/splitting), and cap
   length at 128 chars. Decided split: **CLI/direct path rejects** an invalid/over-cap id
   with a clear exit-2 CLI error (fail fast for a human-driven surface); **envelope path
   sanitizes-and-truncates** to avoid failing an otherwise-valid daemon tool call.
   (t2 `extract_correlation_id`, t5 flag/inject tests.)
8. **No new file-format branch for direct mode** — CLI-direct emission reuses the exact
   `metrics::{initialize,record,shutdown}` → `append_event_line`/rotation path (t3), so
   direct-mode records are byte-compatible with daemon records; the golden contract test
   (t4) covers both. No parallel writer.
9. **Cross-mode concurrency** — `run_direct_sync` holds the `DaemonLock` for its entire
   run, so daemon and direct writers to `.engram/metrics/{branch}/usage.jsonl` are
   **mutually exclusive by construction**; there is no concurrent-append race between
   modes. Within direct mode, the single mpsc `writer_loop` still serializes and drains on
   `shutdown` before process exit (no lost record).
10. **Env-var precedence + leakage** — explicit `--correlation-id` overrides
    `ENGRAM_CORRELATION_ID`; when only the env is set it applies to all CLI invocations in
    that shell (documented, matching `ENGRAM_WORKSPACE` semantics). This is intentional
    for the harness "export once" fallback; operators unset it to avoid stale ids.
11. **Opt-out honored in direct mode** — direct emission checks
    `MetricsConfig.enabled` and skips cleanly when disabled, matching daemon behavior
    (no silent always-on write on the CLI surface).

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

### 7.1 plan-review — Amendment 1 re-run (2026-07-03, self-conducted)

Re-run of the persona-structured gate over the Amendment 1 delta (CLI arg + CLI-direct
emission). DEGRADED_MODE: no subagent-dispatch surface (backlogit MCP transport closed,
engram daemon down) → applied as structured self-review. Personas: Architecture,
Testing/Contract, Safety/Concurrency, Safety/CLI-surface, Operability.

| # | Persona | Severity | Finding | Resolution |
|---|---|---|---|---|
| F8 | Architecture | **P1 (blocking)** | Operator premise "daemon and direct share `tools::dispatch`" is **false** — `run_direct_sync` bypasses `dispatch` and emits nothing today. Routing direct mode through `dispatch` would be a large, wrong refactor. | Resolved (A3, t6) — add a **minimal dedicated** emit hook in `run_direct_sync` reusing `metrics::{initialize,record,shutdown}`; do not reroute through `dispatch`. Grounding recorded. |
| F9 | Testing/Contract | **P1 (blocking)** | Without a `--direct` integration test, the entire MCP-fallback rationale is unverified. | Resolved (t6 tests 1–4 + t4(c)) — `engram sync --direct --correlation-id X` asserts a record carrying `X`. |
| F10 | Safety/CLI-surface | **P1 (blocking)** | Caller-controlled `--correlation-id`/`_meta` free text written verbatim into JSONL → line-forgery / injection / unbounded growth. | Resolved (§6.1-7, t2/t5 tests) — strip control chars + newlines, 128-char cap; CLI rejects, envelope sanitizes-and-truncates. |
| F11 | Safety/Concurrency | P2 | Direct-mode append vs daemon append to the same `usage.jsonl`. | Resolved (§6.1-9) — `DaemonLock` makes the two modes mutually exclusive; no concurrent writer. Documented, no new lock needed. |
| F12 | Architecture | P2 | Direct mode drives process-global `OnceLock` metrics singletons; a one-shot process could exit before the mpsc drains and lose the record. | Resolved (A3, §6.1-9) — `run_direct_sync` calls `metrics::shutdown().await` (drains) before returning; t6 asserts the record is present. |
| F13 | Operability | P3 | `ENGRAM_CORRELATION_ID` exported broadly could attach a stale id to unrelated CLI calls. | Accepted/documented (§6.1-10) — flag overrides env; precedence documented; matches `ENGRAM_WORKSPACE` semantics. Not gate-blocking. |
| F14 | Testing | P3 | Global flag is inert on non-emitting subcommands (`install`/`manifest`/…). | Accepted — harmless; documented in decision-017 §A1. |

**Amendment outcome: PASS.** All three P1 (blocking) findings (F8 shared-choke-point
correction, F9 direct-mode test proof, F10 id validation) resolved within scope; P2s
(F11, F12) carry concrete task-level mitigations; P3s (F13, F14) accepted. 1 review cycle
(within the 3-cycle limit). Deviation F6 (`_meta` vs per-tool param) remains the only
open, previously-flagged deviation and is unaffected by this amendment.

## 8. Out of scope (stashable follow-ups)

- ~~CLI-direct/daemonless emission (`run_direct_sync`) — D9.~~ **NOW IN SCOPE** per
  Amendment 1 (§A3, task t6/`067.006-T`).
- Per-tool input-schema `correlation_id` param (vs `_meta`) — only if the operator
  rejects the `_meta` approach (F6).
- CLI-direct emission for the **query** tool surface (search/query-memory/map-code/…):
  today only `sync`/`index` support `--direct`; the query commands are IPC-only, so their
  correlation-id already flows via the t5 IPC-injection path. Extending `--direct` to the
  query surface is a separate feature, not required by the MCP-fallback scenario.
