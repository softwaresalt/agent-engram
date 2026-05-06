---
type: operational-closure
mode: pre-merge
date: 2026-05-05
feature: 002-F — Backlog Markdown Hydration
shipment: 024-S
branch: feat/002-F-backlog-hydration
pr: https://github.com/softwaresalt/agent-engram/pull/82
readiness: READY
---

# Operational Closure — 002-F Backlog Markdown Hydration (Pre-Merge)

## Change Summary

Adds a `backlog` content source type to the engram daemon ingestion pipeline. `.backlogit/` markdown files are parsed for YAML frontmatter, stored as `BacklogNode`/`BacklogEdge`/`BacklogContentRecord` records in CozoDB, and become searchable via `query_memory` and `unified_search`. Incremental sync uses SHA-256 content hashing. A deletion sweep removes stale nodes when files are removed.

**Scope**: Internal library only. No MCP tool signatures changed, no CLI surface changed, no IPC protocol changed, no HTTP routes changed.

## CI and Review Status

| Gate | Status |
|---|---|
| `cargo fmt --all -- --check` | ✅ clean |
| `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | ✅ clean (CI) |
| `cargo test` (non-cozo-backend, Linux CI) | ✅ passing |
| GitHub CI `CI/build (pull_request)` | ✅ green |
| Automated review comments | None received |
| Unresolved review findings | None |

Pre-existing local failure: `contract_shim_lifecycle` (6 tests, SQLite BUSY on Windows IPC) — confirmed pre-existing, not introduced by this branch.

## Runtime Verification

**Report**: `docs/closure/2026-05-05-002-F-backlog-hydration-runtime-verification.md`  
**Verdict**: PASS WITH FOLLOW-UP

| Scenario | Result |
|---|---|
| Schema bootstrap (`backlog_node`, `backlog_edge`, `backlog_content_record`) | ✅ PASS |
| End-to-end ingestion dispatch (`ingest_all_sources` → `index_backlog_source`) | ✅ PASS |
| DB node persistence and query | ✅ PASS |
| `query_memory` returns backlog content | ✅ PASS |
| Deletion sweep removes stale nodes | ✅ PASS |
| 100-item ingestion under 5 seconds | ✅ PASS (2.22s) |
| Binary build (`cargo build --features cozo-backend`) | ✅ PASS |

**Follow-up**: `query_graph` is a pre-existing stub — backlog relationship graph traversal (parent-child, depends-on) is not yet queryable via MCP. Not introduced by this feature.

## Invariants to Preserve

1. `ingest_all_sources` must not break existing `code`, `git`, or generic content sources — backlog dispatch is additive and gated on `content_type == "backlog"`.
2. Existing CozoDB relations (`code_file`, `function_node`, `content_record`, etc.) must be unaffected — new relations are additive, schema bootstrap is idempotent.
3. `query_memory` results for non-backlog content must be unchanged — `backlog_content_record` is a separate relation; join queries are unchanged.
4. Unknown source status must not silently drop backlog sources — status skip logic now only skips `Missing` / `Error`.

## Pre-Deploy Checks

- [x] Feature is library-internal — no deployment, config change, or feature flag required
- [x] Schema is additive — no migration required; existing DB instances gain the 3 new relations on next `connect_db` bootstrap call
- [x] No external dependencies added
- [x] Backlog source activation requires a user to add `content_type: backlog` entry to `.engram/registry.yaml` — opt-in, not automatic

## Deployment / Rollout Path

**Merge-only**. The daemon is a local binary distributed per-workspace. No cloud deployment or staged rollout applies. Backlog indexing activates only when a user explicitly configures a `backlog` source in `.engram/registry.yaml`.

## Post-Deploy / Post-Merge Checks

1. Verify `cargo test --test integration_backlog_hydration --features cozo-backend` still passes on the default branch after merge.
2. Verify `git log main --oneline -3` shows the merge commit.
3. Optional: add a `backlog` source to a workspace `.engram/registry.yaml` and call `sync_workspace` — confirm `query_memory` returns backlog artifact content.

## Healthy Signals

- `sync_workspace` completes without error when a `backlog` source is configured
- `query_memory` returns `BacklogContentRecord`-backed results for backlog artifact IDs and titles
- Schema bootstrap logs no errors at daemon startup

## Failure Signals

- `ingest_all_sources` returns an `IngestionError::Failed` for a backlog source
- `select_backlog_nodes` returns empty results after ingestion (schema bootstrap did not run)
- DB file cannot be opened after merge (would indicate schema incompatibility — extremely unlikely given additive-only changes)

## Monitoring Plan

| Signal | Method | Owner |
|---|---|---|
| Ingestion errors | `tracing::warn!` / `tracing::error!` logs in `index_backlog_source` | Operator |
| Schema bootstrap retries | `MUTABLE_RETRY_COUNT` counter (existing telemetry) | Operator |
| Test suite regression | GitHub CI on `main` | CI |

No new dashboard or alert configuration required — this feature is library-internal with no production service exposure.

## Rollback Trigger

- CI fails on `main` after merge for `integration_backlog_hydration` tests
- Daemon crashes at startup on a workspace with an existing CozoDB DB that cannot bootstrap the new relations (extremely unlikely — additive schema, idempotent bootstrap)

## Rollback Procedure

```bash
git revert --no-edit -m 1 <merge_sha>
```

Additive schema relations are safe to leave in existing databases after rollback (no destructive migration). The reverted `ingest_all_sources` will simply not dispatch to `index_backlog_source`.

## Validation Window

**24 hours** post-merge. Owner: repository maintainer (softwaresalt).

The change is low-risk (additive, opt-in, fully tested). No extended monitoring window is warranted beyond confirming CI passes on `main` after merge.

## Readiness Status

**READY**

All quality gates pass. Runtime verification returns PASS WITH FOLLOW-UP (follow-up is pre-existing `query_graph` stub limitation, not introduced by this feature). Rollback is simple and non-destructive.

## Source Artifact Traceability (002-F)

- `custom_fields.backlog_md_source_path`: `.backlog/drafts/draft-002 - Need-to-be-able-to-hydrate-requirements-backlog-from-markdown.md` — source draft document, not a backlogit stash entry
- `custom_fields.backlog_md_id`: `DRAFT-002` — legacy draft ID
- `custom_fields.source_stash_id`: not present (originated as a backlog draft, not a stash entry)
- `custom_fields.source_deliberation_id`: not present
- `references`:
  - `docs/exec-plans/2026-05-05-backlog-markdown-hydration-plan.md`
  - `docs/decisions/2026-05-05-backlog-markdown-hydration-deliberation.md`

No stash retirement or deliberation archival required.

## Follow-Up Items (Stash)

| Item | Source | Priority |
|---|---|---|
| Expose backlog relationship traversal via `query_graph` when stub is implemented | Runtime verification follow-up | low |
| Add `backlog` source to default `engram install` registry scaffold | Feature enhancement | medium |

These follow-ups will be stashed for Stage after merge.
