---
type: session-memory
date: 2026-07-05
agent: orchestrator
session: autonomous AFK run — stash drain (075-S, 076-S)
shipments: [075-S, 076-S]
prs: [213, 214, 215, 216]
---

## Objective

Autonomous AFK pipeline run: drain the stash/queue to completion, adversarial
review before every PR, patient Copilot iteration, admin-merge, per-shipment
closure. Operator granted merge authority.

## Work completed

### Shipment 075-S — engram usage MEASUREMENT (feature 073-F)
- **PRs:** #213 (code, merge `93eea65`), #214 (closure, merge `b76cd5a`).
- Closed the EMISSION→MEASUREMENT gap (builds on 067-S). Source stash `A7F72BC0`.
- **Model** (`src/models/metrics.rs`): added `unique_tools_exercised` +
  `distinct_correlation_ids` scalars to `MetricsSummary`; new
  `correlation_metrics(&events) -> BTreeMap<String, CorrelationMetrics>` free
  function (call_count, unique_tools, time_range per correlation_id). The heavy
  per-correlation map is **deliberately kept OFF `MetricsSummary`** and surfaced
  only by `get_token_savings_report` (token-efficiency — it must not bloat
  `get_health_report`/`get_branch_metrics`/`summary.json`).
- **Report** (`src/tools/read.rs`): `get_token_savings_report` gained an additive
  `metrics` object; loads events once, derives summary + correlation breakdown.
- **Coverage** (`src/tools/mod.rs`): `index_git_history` now emits (git-graph
  cfg); `flush_state` stays excluded (internal lifecycle, always-0 anyway).
- **Docs:** decision-018; `docs/design-docs/engram-usage-telemetry-consumption-contract.md`.
- **Carry-along:** `.gitignore` + `.github/workflows/detect-direct-push.yml` (the
  prior working-tree drift). NOTE: the `.gitignore` `!.engram/*` negations are
  **INERT** — git ignores `.engram/` at the directory level, so nothing is
  un-ignored/committed. Flagged in PR; a follow-up must use `.engram/*` + decide
  on machine-specific `.workspace-id` before those files can actually be tracked.

### Shipment 076-S — F1 NotReady `--direct` hint fix (feature 074-F)
- **PRs:** #215 (code, merge `5ee7375`), #216 (closure, merge `409bd53`).
- Source stash `E0659C5C`. `DaemonError::NotReady` (`src/errors/mod.rs`) reworded
  to be accurate in ALL sub-cases: daemon **exited** → `--direct`; **still
  starting** → wait/retry; **stuck** → stop the process (it holds the lock).
- Chose **option (c) reword** over (a) lock-probing: (a) is risky — `poll_until_ready`
  lacks the workspace path (3 call sites), endpoint isn't cross-platform
  reversible to a path, and `DaemonLock::acquire` is NOT a benign probe (its
  stale-PID path DELETES `engram.lock`/`engram.pid`, `src/daemon/lockfile.rs:186`).
- Wire contract unchanged: code `8006`/`DaemonNotReady`; no consumer parses the text.

### Housekeeping
- Closed stale `064-F` (`active` → `done`). All scope complete: Phase 1a (052-S),
  1b (072-S/064.004-T), 2c/2d superseded by decision-017. Was dangling since
  before this session.

## Reviews
- **075-S:** full multi-model adversarial review (gemini-3.5-flash /
  claude-sonnet-4.6 / gpt-5.4). No gate-blocking defects. 2 HIGH-confidence P1
  consensus findings fixed at the root pre-merge: dead `session_count` excluded
  from the adoption surface; `by_correlation_id` confined to the report tool.
  Closure: `docs/closure/2026-07-05-075-usage-measurement-adversarial-review.md`.
- **076-S:** cross-model rubber-duck (gpt-5.4). No blockers; validated (a) is
  risky. Incorporated its refinement (slow-start/would-recover case).
  Closure: `docs/closure/2026-07-05-076-notready-hint-fix-review.md`.

## Copilot review iterations
- #213: 1 comment (broken `§Branch Protection` doc anchor in detect-direct-push.yml)
  → fixed to reference constitution Principle XI; replied + resolved.
- #215: 1 comment (074.001-T archived without `archived_from`/`archived_status`
  provenance because created via `--status done`) → normalized frontmatter;
  replied + thread auto-resolved (diff outdated).

## Key facts / gotchas confirmed this session
- **CI feature set:** `--no-default-features --features cozo-backend,embeddings`.
  NEVER `--all-features` (breaks on otlp-export/observability.rs).
- **main is ruleset-protected** → all merges via `gh pr merge --merge --admin`
  (P-009 merge-commit only). Direct `git push origin main` is rejected — closure
  commits must go through a branch + PR.
- **Known Windows-only flake:** `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`
  ("database is locked") — fails identically on `main` in this env; two stale
  engram daemons (PIDs 17972/28384, started 5:51 PM pre-session) hold the CozoDB
  lock. Passes on Ubuntu CI.
- **CI flake on Ubuntu:** `integration_markdown_indexing::t030_003` flaked once
  on #215 (backlog-only commit); `gh run rerun --failed` → green. Documented flake.
- **backlogit `--status done` on task creation** archives it WITHOUT the
  `archived_from`/`archived_status` provenance; `shipment ship` later normalizes
  manifest items (adds provenance + merge SHA). Prefer creating tasks
  active/queued and letting ship archive them, OR expect a Copilot flag.
- **shipment ship flow:** create (queued) → `shipment claim` (active) →
  `shipment ship --sha` (shipped + archives scope with merge SHA). Skipping claim
  → "shipment status conflict".
- **union landmine avoided:** no reflexive `backlogit sync`; hand-edited archived
  markdown (074.001-T) without syncing; verified no markdown resurrection.

## Remaining work (all correctly deferred)
- **F7E89921** (stash, DAX tree-sitter): PARKED. Deliberation
  `docs/decisions/2026-06-13-dax-tree-sitter-spike.md` = defer — no in-repo
  consumer for symbolic DAX; reopen when a concrete consumer appears. Requires
  product/operator input (NOT auto-shippable).
- **025-S / 041-F / 041.001-T** (blocked): upstream cozo ≥ 0.8.
- **033.005-T** (blocked): tree-sitter-sequel CREATE PROCEDURE release.

## End state
0 open PRs, 0 active/queued non-blocked items. main @ `409bd53` (+ this memory PR).
