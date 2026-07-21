---
title: May 2026 Memory Compaction
type: compacted-memory
date: 2026-05-31
compacted-from:
  - docs/memory/2026-05-08/cli-full-test-cycle-memory.md
  - docs/memory/2026-05-09/029-s-indexing-resilience-ship-memory.md
  - docs/memory/2026-05-09/029-s-pr102-review-closure-memory.md
  - docs/memory/2026-05-09/031-s-ship-memory.md
  - docs/memory/2026-05-09/032-S-cli-resilience-shipping-memory.md
  - docs/memory/2026-05-09/033-s-shipped-query-graph-memory.md
  - docs/memory/2026-05-09/033-s-stage-query-graph-memory.md
  - docs/memory/2026-05-09/034-s-daemon-startup-reliability-memory.md
  - docs/memory/2026-05-09/group-a-staging-memory.md
  - docs/memory/2026-05-09/group-b-staging-memory.md
  - docs/memory/2026-05-12/daemon-release-readiness-stage-memory.md
  - docs/memory/2026-05-12/pr-130-ci-fix-memory.md
  - docs/memory/2026-05-13/documentation-refresh-session-memory.md
  - docs/memory/2026-05-14/037-s-post-merge-closure-memory.md
  - docs/memory/2026-05-14/041-s-post-merge-closure-memory.md
  - docs/memory/2026-05-14/orchestrator-pipeline-memory.md
  - docs/memory/2026-05-17/orchestrator-pipeline-memory.md
  - docs/memory/2026-05-17/ship-046-S-session-memory.md
  - docs/memory/2026-05-20/ship-047-S-post-merge-closure-memory.md
  - docs/memory/2026-05-21/ship-048-S-post-merge-closure-memory.md
  - docs/memory/2026-05-22/pbip-indexer-stage-memory.md
  - docs/memory/2026-05-22/pr-163-copilot-followup-memory.md
  - docs/memory/2026-05-23/051-s-notebook-source-support-memory.md
  - docs/memory/2026-05-28/063-f-post-merge-closure-memory.md
archived-to: docs/archive/memory/
---

# Compacted Memory - May 2026 Ship, Stage, and Closure Work

## Summary

May 2026 completed the CLI smoke-test feedback loop, several resilience and
daemon-startup shipments, the query-graph structured API, release-readiness
staging, documentation refreshes, Power BI/PBIP staging and follow-up work, and
notebook source support. The dominant themes were stricter CLI contract coverage,
queued indexing resilience, backlog/archive traceability, Copilot thread hygiene,
and avoiding out-of-scope branch or backlog drift.

## CLI smoke testing and resilience lineage

* `cli-full-test-cycle-memory.md` completed archived task `043.003-T` by running
  all 27 CLI commands against debug builds and fresh workspaces. The cycle found
  stash bugs `BC9A6B23` (`install`/`update`/`reinstall`/`uninstall` ignore
  `--workspace`), `A98E9409` (`sync/index --direct` panic on daemon-held DB),
  and `E0CF06A6` (silent first daemon spawn)
* `group-a-staging-memory.md` staged `046-F` / `031-S` from `BC9A6B23` and
  `B9E4F2A1`; the plan-review gate passed with binary-level tests called out as
  required coverage for dispatch bugs
* `031-s-ship-memory.md` shipped `046-F` in PR `#112` with merge `25cea55`.
  Files changed: `src/bin/engram.rs`, `src/installer/mod.rs`, and
  `tests/integration/installer_test.rs`. Copilot caught the missing `reinstall`
  binary regression (`S079d`), which was added before merge
* `group-b-staging-memory.md` staged `047-F` / `032-S` from `A98E9409`,
  `3AA1E6DD`, and `E0CF06A6`; duplicates `047.007-T` through `047.009-T` were
  archived and linked to canonical tasks
* `032-S-cli-resilience-shipping-memory.md` shipped `047-F` in PR `#114` with
  merge `10134ad`: DB-lock probing in `src/cli/direct.rs`, progress hints in
  `src/cli/output.rs`, hardened friendly errors in `src/cli/runner.rs`, and a
  Windows-only direct-lock integration test. Key decisions: use the exact
  `connect_db` branch sanitization, fail fast on lock-file open errors, preserve
  global `IndexInProgress` primary detection, and move IPC endpoint computation
  before daemon start so pre-spawn health can be probed

## Indexing resilience and daemon startup reliability

* `029-s-indexing-resilience-ship-memory.md` shipped `044-F` / `029-S` in PR
  `#101` with merge `8f23c3c`. It added `is_indexing()` guards to read tools,
  CLI `IndexInProgress` messaging, `pending_sync: AtomicBool`, queued sync drain,
  and configurable CLI timeout. Review fixed four P1 drain races and missing drain
  sites across `src/tools/write.rs`, `src/tools/lifecycle.rs`,
  `src/server/state.rs`, and `src/daemon/ipc_server.rs`
* `029-s-pr102-review-closure-memory.md` closed late Copilot comments from PR
  `#102` via follow-up PRs `#103` and `#104`, resolving bot threads with GraphQL,
  adding a missing code-fence language, fixing markdown spacing, and archiving
  `044-F`. Learning: late review comments after merge need a new PR on the same
  branch; completed child tasks do not auto-archive parent features
* `034-s-daemon-startup-reliability-memory.md` shipped `049-F` / `034-S` in PR
  `#127` with merge `6ed2b36`. It introduced deadline-based shim polling, moved
  hydration-ready earlier, yielded during hydration upserts, gated startup
  auto-reindex behind `ENGRAM_AUTO_REINDEX=true`, and removed indexing guards from
  read tools. Critical decisions: early readiness avoids 500 ms shim timeouts;
  default-off auto-reindex prevents 14 GB startup OOM; tests must assert neither
  `INDEX_IN_PROGRESS` nor `WORKSPACE_NOT_SET`
* `daemon-release-readiness-stage-memory.md` staged direct operator request
  `034-C` / `035-S` for daemon and CLI release hardening with tasks
  `034.001-T` through `034.008-T`. It created
  `docs/decisions/2026-05-12-daemon-release-readiness-deliberation.md` and
  `docs/exec-plans/2026-05-12-daemon-release-readiness-plan.md`; no stash was
  consumed and unrelated CozoDB-blocked work stayed untouched
* `pr-130-ci-fix-memory.md` fixed PR `#130` through commits `32c9a5c`,
  `87af02e`, `e769ccc`, and `593bc54`. Changes added SQLite-busy retry helpers,
  same-path DB-open serialization, queued startup auto-sync, `.engram/` watcher
  exclusion, markdown indexing polling for full expected symbols, and shared v2
  daemon drain logic. `cargo audit` advisories remained pre-existing

## query_graph structured API

* `033-s-stage-query-graph-memory.md` resolved review comments on PRs `#110`,
  `#113`, and `#115`, selected deliberation `003-D` Option B (structured JSON API),
  and harvested `048-F` / `033-S` into five tasks. Decisions: split neighborhood
  and path execution, reuse `edge_types` for backlog edges, and defer
  `sanitize_query` handling to implementation
* `033-s-shipped-query-graph-memory.md` shipped `048-F` / `033-S` in PRs `#123`
  and `#124` with merges `5cae4a3` and `e37cb5b`. Key files:
  `src/models/graph_query.rs`, `src/db/cozo_queries.rs`, `src/tools/read.rs`,
  `src/shim/tools_catalog.rs`, `src/bin/engram.rs`, and
  `src/cli/commands/search.rs`. Copilot fixes enriched backlog-edge traversal,
  corrected incoming `concerns_edge` resolution, and filtered empty CSV edge
  values. Follow-up: integration tests for real indexed-workspace BFS traversal

## Documentation refresh and install UX

* `documentation-refresh-session-memory.md` completed `036-S` / `050-F` from
  stash `35FC7DF8`, rewriting README and focused docs pages:
  `docs/quickstart.md`, `docs/configuration.md`, `docs/mcp-tool-reference.md`,
  `docs/architecture.md`, `docs/troubleshooting.md`,
  `docs/log-observation-guide.md`, and `docs/workflows.md`. Decisions: keep README
  brochure-level, make shim-over-IPC the primary runtime story, document `sync` as
  routine refresh, and avoid over-promising generated helper templates
* `037-s-post-merge-closure-memory.md` closed `037-S` / `051-F` after PR `#136`
  merge `37bc92e`. It verified all 10 Copilot review threads resolved, used a
  dedicated closure branch, and archived the README/install UX shipment

## Orchestrated shipment runs and backlog traceability

* `orchestrator-pipeline-memory.md` on 2026-05-14 recorded shipments `037-S`,
  `035-S`, `038-S`, `039-S`, `040-S`, and `041-S` shipped and archived. Decisions:
  enforce one open Ship PR at a time, use stash-first branch handoffs for dirty
  planning state, accept local audit advisories only when CI marks audit
  `continue-on-error`, and normalize stale shipment ledger state after merges
* `041-s-post-merge-closure-memory.md` closed `041-S` / `055-F` after PR `#145`
  merge `0996156`. The shipment intentionally stopped at an investigation doc
  (`docs/decisions/2026-05-15-markdown-compaction-investigation.md`) rather than
  implementation, kept active backlog artifacts canonical-first, and rebuilt a PR
  branch from `origin/main` after local-only normalization commits polluted the
  first attempt
* `orchestrator-pipeline-memory.md` on 2026-05-17 staged and shipped `046-S` /
  `060-F` from stash `9978C53D`, creating deliberation `008-D` and an audit task
  before branch-DB seeding. Follow-up questions Q2-Q5 in `008-D` remained open,
  and blocked `025-S` stayed untouched
* `ship-046-S-session-memory.md` merged PRs `#154` and `#155` for the branch DB
  deletion-correctness audit. `tests/integration/sync_workspace_deletion_test.rs`
  and `Cargo.toml` were updated; audit found `sync_workspace` deletion handling in
  `src/services/code_graph.rs` is correct and removes symbols, edges, file nodes,
  and hashes. No follow-up backlog item was needed

## Power BI, PBIP, and notebook support

* `ship-047-S-post-merge-closure-memory.md` closed `047-S` against merge
  `e84fe92`, archived tasks `061.002-T` through `061.004-T`, restored `061-F` to
  active for remaining shipments, and updated architecture plus closure docs
* `ship-048-S-post-merge-closure-memory.md` closed `048-S` against merge
  `fecd69b`, archived `061.001-T` and `061.005-T`, repaired `061-F` so `049-S`
  still had a valid active parent, and stopped before further implementation work
* `pbip-indexer-stage-memory.md` staged `062-F` / `050-S` from stash `48A5986F`.
  It created `docs/exec-plans/2026-05-22-pbip-project-definition-indexer-plan.md`
  plus tasks `062.001-T` through `062.007-T`. Decisions: introduce a dedicated
  `pbip` project-definition source boundary, keep legacy `powerbi` JSON/BIM
  separate, and split linkage, page/visual extraction, and semantic-model
  extraction into separate tasks
* `pr-163-copilot-followup-memory.md` fixed late PR `#163` Copilot comments in PR
  `#165`: Windows-style `definition` paths gained regression coverage, semantic
  model graph node IDs were seeded from `model.id`, and closure rollback guidance
  standardized on `git revert --no-edit -m 1 <merge_commit>`
* `051-s-notebook-source-support-memory.md` implemented `051-S` / `063-F` notebook
  support at feature head `3acd337`: new notebook models and indexer, notebook
  extraction service, fixtures, integration/unit tests, docs, and content-only
  records. Decisions: use a dedicated `notebook` content source, emit one summary
  plus per-cell records, keep code-cell language in content payload, and defer
  outputs, execution state, graph edges, and symbol extraction
* `063-f-post-merge-closure-memory.md` converted the notebook closure record after
  PR `#167` merge `bc85c89`, retargeting `051-S`, `063-F`, and tasks
  `063.001-T` through `063.005-T` to the merge commit. A broad search timeout was
  resolved by narrowing the search, and one bulk patch had to be repaired because
  `063.002-T.md` had drifted `updated_at`

## Consolidated originals

These verbose originals were consolidated here and moved to `docs/archive/memory/`:

* `docs/memory/2026-05-08/cli-full-test-cycle-memory.md`
* `docs/memory/2026-05-09/029-s-indexing-resilience-ship-memory.md`
* `docs/memory/2026-05-09/029-s-pr102-review-closure-memory.md`
* `docs/memory/2026-05-09/031-s-ship-memory.md`
* `docs/memory/2026-05-09/032-S-cli-resilience-shipping-memory.md`
* `docs/memory/2026-05-09/033-s-shipped-query-graph-memory.md`
* `docs/memory/2026-05-09/033-s-stage-query-graph-memory.md`
* `docs/memory/2026-05-09/034-s-daemon-startup-reliability-memory.md`
* `docs/memory/2026-05-09/group-a-staging-memory.md`
* `docs/memory/2026-05-09/group-b-staging-memory.md`
* `docs/memory/2026-05-12/daemon-release-readiness-stage-memory.md`
* `docs/memory/2026-05-12/pr-130-ci-fix-memory.md`
* `docs/memory/2026-05-13/documentation-refresh-session-memory.md`
* `docs/memory/2026-05-14/037-s-post-merge-closure-memory.md`
* `docs/memory/2026-05-14/041-s-post-merge-closure-memory.md`
* `docs/memory/2026-05-14/orchestrator-pipeline-memory.md`
* `docs/memory/2026-05-17/orchestrator-pipeline-memory.md`
* `docs/memory/2026-05-17/ship-046-S-session-memory.md`
* `docs/memory/2026-05-20/ship-047-S-post-merge-closure-memory.md`
* `docs/memory/2026-05-21/ship-048-S-post-merge-closure-memory.md`
* `docs/memory/2026-05-22/pbip-indexer-stage-memory.md`
* `docs/memory/2026-05-22/pr-163-copilot-followup-memory.md`
* `docs/memory/2026-05-23/051-s-notebook-source-support-memory.md`
* `docs/memory/2026-05-28/063-f-post-merge-closure-memory.md`
