# Session memory — Stage — 067-F Engram usage-telemetry EMIT + 064-F Phase-2 cancellation

- **Date:** 2026-07-02
- **Agent:** Stage
- **Source stash:** 7D8F395B (high, feature) — harvested & archived
- **Mode:** DEGRADED_MODE — backlogit MCP transport closed; all backlog ops via CLI `C:\Tools\backlogit.exe` (registry-declared fallback, `tool_type: both`). engram MCP not exposed / `engram` CLI `daemon-status` blocked → grounding via filesystem (grep/glob/view). No DB lock files; no stale backlogit PIDs — MCP server unreachable at host level, not a lock.

## Outcomes

### PART A — cancellation of superseded ingestion workstream
- **Key finding:** tasks `064.005-T` and `064.006-T` **never existed** (index count 0; not in `.backlogit/queue` or `.backlogit/archive`). The `064-F` ship-note documents them as *reserved-but-free planned slots* after the TMDL family was re-IDed to `066-*`. **Nothing to archive.**
- Realized the cancellation intent by updating `064-F` **DoD** to mark Phase 2c (ExecutionEpoch CozoDB schema) + Phase 2d (JSONL ingestion) **DROPPED — superseded by 067-F**, and that slots `064.005-T`/`064.006-T` will not be created.
- `064.004-T` (Phase 1b reactive-sync) left **queued & untouched** (still valid).
- Supersession also recorded in `decision-017` and via a `relates_to` dep `067-F → 064-F` (backlogit CLI has no semantic `supersedes` link; `add_link` MCP unavailable).
- **doctor:** 43 findings, ALL pre-existing `archived_from_self_ref` warnings on historical archived items (prior-shipment hygiene debt, out of scope). Zero findings on 064-F/067-F/067.*; zero orphan/duplicate; zero dangling 064.005/064.006 refs.

### PART B — telemetry-EMIT feature
- **Grounding correction:** a per-call `usage.jsonl` emitter **already exists** (`src/services/metrics.rs` → `.engram/metrics/{branch}/usage.jsonl` via `record()/writer_loop/append_event_line`). Stash premise "NO existing emission" is **stale**. Work reframed as **extension**, not greenfield.
- **Choke point:** `src/tools/mod.rs::dispatch()` (single dispatch for all MCP tools; via `src/daemon/ipc_server.rs:332`) — already measures `start.elapsed()` latency and records `UsageEvent` (denied L232, success/error L339), timestamp `chrono::Utc::now().to_rfc3339()` (ISO-8601 UTC).
- **Gaps addressed by plan:** add `correlation_id` (via `_meta.correlation_id`, mirror `policy::extract_agent_role`), `latency_ms`, explicit `workspace`, coarse `params_summary` (hash/len/limit), pinned `schema_version=2`; config `usage_path_override` + rotation (`max_file_bytes`/`max_rotated_files`); coverage for `set_workspace`/`sync_workspace`/`index_workspace` (excluded today in `should_record_metrics` @ `src/tools/mod.rs:34`).
- **Scope:** daemon-served emission only; CLI-direct (`src/cli/direct.rs::run_direct_sync`) bypasses dispatch → out of scope.

## Artifacts created
- Decision: `docs/decisions/decision-017 - Engram-usage-telemetry-emit-not-ingest-pivot.md`
- Exec-plan: `docs/exec-plans/2026-07-02-engram-usage-telemetry-emit-plan.md` (Constitution Check + plan-harden addendum + plan-review record; outcome PASS)
- Deliberation: `012-D` (linked to stash 7D8F395B)

## Backlog structure
- Feature: **067-F** (queued) — linked to stash 7D8F395B, deliberation 012-D, `relates_to` 064-F
- Tasks (dep order **t1 → (t2,t3) → t4**):
  - `067.001-T` t1 — usage-record + metrics-config schema extension (width: models)
  - `067.002-T` t2 — dispatch choke-point hook (correlation_id/latency/params) + coverage (width: tools/dispatch) [blocks: 067.001-T]
  - `067.003-T` t3 — emitter size-cap rotation + config path-override (width: services/metrics) [blocks: 067.001-T]
  - `067.004-T` t4 — cross-platform contract + integration tests (width: tests) [blocks: 067.002-T, 067.003-T]
- Shipment: **067-S** (status **queued**) — items: 067-F, 067.001-T, 067.002-T, 067.003-T, 067.004-T

## JSONL record schema (usage.jsonl, schema_version=2)
Retained existing UsageEvent fields; NEW additive/back-compat fields: `schema_version:u32=2`, `correlation_id:Option<String>` (`_meta.correlation_id`, skip when None), `latency_ms:u64`, `workspace:String`, `params_summary:Option<{query_hash?,query_len?,limit?}>`. Timestamp pinned ISO-8601 UTC. See decision-017 for the full field table.

## Open items / next steps (for Ship)
- Ship to claim **067-S** when ready (do NOT claim from Stage).
- Operator confirmation on F6 deviation: `_meta.correlation_id` vs per-tool-schema param (plan recommends `_meta`).
- Uncommitted backlog-hygiene debt (~14 files) + these new artifacts remain uncommitted (Stage does no git); future chore PR.
- Sibling stash F7E89921 (DAX tree-sitter) untouched (out of scope).
