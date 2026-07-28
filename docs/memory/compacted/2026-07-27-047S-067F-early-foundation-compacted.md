---
type: compacted-memory
date: 2026-07-27
period: "2026-05-20 .. 2026-07-04"
source_count: 12
archive_path: docs/archive/memory/
---

# Phase 1: Early Foundation & Verification CLI (Shipments 047-S through 067-F)

## Overview
This period consolidated three major foundation phases: Power BI project support (047-S), the `engram verify` CLI and its hardening (052-S with 064-F Phase 1a shipped; Phase 1b reactive-sync deferred), and telemetry infrastructure setup (067-F family). Significant backlog reconciliation work addressed ID-namespace collisions and coordinated parallel post-merge closures.

## Key shipments and outcomes

### 047-S: Power BI Search Foundation (May 20)
Shipped `src/models/powerbi.rs`, `src/services/powerbi_extract.rs`, and `src/services/powerbi_indexer.rs`. Architecture reused existing `content_record` table for Power BI entities; all PBIP objects (page, visual, table, measure) get synthetic IDs (SHA-256 truncated) and fine-grained indexing. Added 3 test modules (28 tests). Established precedent: no new CozoDB methods, no schema migration.

### 052-S: engram verify CLI Phase 1a (June 30 – July 1)
Delivered the local, no-daemon `engram verify <path>` CLI command for structural conformance linting. Created `src/cli/commands/verify.rs` and `src/services/verify.rs` with exit-code contract (0 pass / 1 non-conformant / 2 error), crucial for the autoharness pre-task-completion gate. Deferred Phase 1b (reactive sync gated on verify) as dependency-linked task 064.004-T under active feature 064-F. Phase 2c/2d (ExecutionEpoch telemetry) were later DROPPED (superseded by 067-F); the 064.005/006-T slots were never materialized.

### 064-F: Deterministic gates & telemetry (June 30, feature active)
Deliberately split across two shipments: Phase 1a (052-S, shipped) provides the verify CLI. Operator chose Phase 1a first as it was self-contained, unblocker for autoharness, and no schema/daemon changes required. Phase 1b (reactive-sync gate in daemon watcher, 064.004-T) remained queued; Phases 2c/2d were later DROPPED (superseded by 067-F), not queued — the 064.005/006-T slots were never created.

### 064-F → 066-F ID reconciliation (July 1)
Hard landmine discovered and resolved: backlogit permitted reuse of `064-*` IDs across archive and queue. PR #169 shipped TMDL parser as `064-F`, then PR #185 reused `064-F` for verify CLI. Fix: re-IDed terminal archived TMDL family to `066-*` (064.005–008-T became 066.005–008-T; 064-S became 066-S). Verify family kept at `064-*` to preserve merged-history references.

### 053-S: PR #187 Post-Merge Closure (July 2)
Quick daemonless docs PR (Phase 1a only; Phase 1b deferred as 065.004-T). Split-brain risk when calling `backlogit get 064-F` — cache returned archived TMDL, markdown returned verify. Resolved by not syncing (cache-union landmine) and using direct markdown edits + CLI mutations.

### PR #186 & #188: Copilot Review Fixes & Hardening (July 2)
PR #186 (chore): removed spurious verify→TMDL link from 064-F, resolved via sync-cache expertise. PR #188 (verify hardening): folded in 5 Copilot findings from PR #185 (docs frontmatter, body.empty test, Windows path containment, CRLF guard). Created compound learning on workspace-path canonicalization (resolve both sides).

### 067-F: Usage-telemetry EMIT (July 2 – July 4)
Defined telemetry extension to existing `metrics.rs` (usage.jsonl emitter). Scope: daemon-served MCP tool dispatch only (CLI-direct `src/cli/direct.rs` initially out of scope, later amended in). Added `correlation_id`, `latency_ms`, `workspace`, `params_summary` to JSONL schema. Shipped as 067-S (merged July 4 as PR #190). Key fix: `tokio::fs::File::write_all` + `flush().await?` before rotation to prevent dropped lines.

### 067-F Amendment: CLI correlation-id + direct emission (July 3)
Operator directive reversed scope: CLI-direct IS in scope. Amendment added `--correlation-id` CLI arg + `ENGRAM_CORRELATION_ID` env, IPC `_meta.correlation_id` threading (runner.rs), and dedicated direct-mode emit hook. Expanded tasks from 067.001–004-T to 067.001–006-T, dependency-ordered.

## Traceability & decisions

- **Backlogit cache-union landmine**: a blind `sync` can re-union stale SQLite cache back into markdown. Workaround: markdown is authoritative — avoid blind `sync` mid-conflict; routine edits use CLI mutations + direct markdown edits. Durable repair: stop stale MCP holders, delete the disposable `backlogit.db{,-wal,-shm}`, then `backlogit sync` to rebuild the index from authoritative markdown.
- **Path containment security (064.003-T)**: canonicalize both workspace root AND resolved path before containment check (resolves symlinks + `..`).
- **Merge-policy precedent**: merge-commit only, no squash/rebase (P-009/P-011).
- **Verification contract**: exit-code pinned in contracts (not requirements docs); test gate ensures immutability.

## Cross-domain impact

- 064-F (verify) is a dependency of the autoharness `pre_task_completion` gate; blocked all downstream work until Phase 1a shipped.
- 067-F telemetry created infrastructure for durable correlation-id threading (set by agent runner, propagated through daemon/CLI).
- Backlog ID collision fix (064→066) established a "re-ID terminal archived, preserve merged-history" rule for future ID reuse.

## Archived originals (traceability)

| File | Summary |
|---|---|
| 2026-05-20-ship-047-S-session.md | Power BI entity extraction (PBIP reports, pages, visuals, tables, measures) indexed as content_records; synthetic IDs. |
| 2026-06-30-stage-0E042A84-session.md | Stash triage: CLI-only index already shipped in 030-S (045-F), not duplicate; marked duplicate. |
| 2026-06-30-stage-B87680AB-session.md | Stash triage: engram-evals-gates deliberation; scoped Phase 1a verify CLI into 052-S; deferred 1b/2c/2d. |
| 2026-07-01-ship-052-S-session.md | Post-merge closure PR #185: verify CLI shipped; backlog ID collision found; all closure artifact stored. |
| 2026-07-01-stage-064-namespace-collision-reconcile-session.md | ID collision fix executed: 064 TMDL → 066; verify 064 retained; manual markdown surgery + doctor verification. |
| 2026-07-02-ship-053-S-pr187-postmerge-closure-session.md | Post-merge closure PR #187: daemonless docs; 053-S archived; 065-F reopened; backlogit-sync landmine documented. |
| 2026-07-02-ship-pr186-064F-copilot-fix-session.md | Copilot finding: spurious 064-F → 062.003-T link removed; cache-union landmine required PID cleanup + DB rebuild. |
| 2026-07-02-ship-pr186-postmerge-pr187-refresh-session.md | PR #186 merged; #187 reopened + merged; local main synced; strict one-PR-at-a-time honored; 0 open PRs final. |
| 2026-07-02-ship-pr188-064.004-T-postmerge-closure-session.md | Post-merge closure PR #188: verify hardening 5 findings fixed; 064.004-T kept queued (Phase 1b); multi-PR feature tracking. |
| 2026-07-02-ship-pr188-verify-hardening-session.md | PR #188 hardens PR #185: docs, body.empty coverage, Windows path containment (both sides), CRLF guard; 5 Copilot cycles. |
| 2026-07-02-stage-067F-usage-telemetry-emit-064F-cancel-session.md | Stash triage: telemetry-EMIT feature 067-F harvested; 064.005-T/006-T cancelled (superseded); shipment 067-S queued. |
| 2026-07-03-ship-067S-usage-telemetry-emit-resume-flush-fix-session.md | 067-S resumed after restart: rotation flush bug fixed (tokio fs write_all + flush); PR #190 merged; 067-S archived. |
