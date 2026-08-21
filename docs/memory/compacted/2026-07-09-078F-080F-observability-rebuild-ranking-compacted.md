---
title: "Completed feature memory compaction: 078-F through 080-F"
type: compacted-memory
date: 2026-08-21
status: complete
sources:
  - docs/archive/memory/2026-07-07/078-F-status-observability-fixes-memory.md
  - docs/archive/memory/2026-07-08/079-F-force-rebuild-and-graph-quality-assessment-memory.md
  - docs/archive/memory/2026-07-09/080-F-code-first-ranking-memory.md
---

# Completed feature memory compaction

## 078-F — Status observability fixes

PR #228 (merge `8ae9d57`) fixed two workspace-status defects: `memory_bytes`
reported system-wide RAM instead of process RSS (new
`services::process_memory` helper reused by `get_daemon_status`,
`get_health_report`, and the legacy `/health` handler), and `code_graph`
counts were incorrectly gated behind the `git-graph` feature flag (removed —
the code-graph indexer is always active). A first test-hardening attempt
using indexing locks was insufficient per Copilot review; the fix instead
indexed the workspace before `set_workspace` so background hydration hits the
read-only fast path. Verified live post-install:
`daemon-status.memory_bytes` = 0.62 GB (was reporting 17-22 GB).

## 079-F — Force-rebuild flag + quality assessment

A data-grounded assessment found excellent vector coverage (100%) and
complete symbol inventory, but partial call-edge recall (cross-file and
method calls dropped at extraction) and doc-biased search ranking. Of three
recommendations: Rec #2 shipped as 079-F (PR #231, merge `1be2e17`) — added
`--force` to `sync`/`index` routing on `full || force`. Rec #1 (cross-file
call resolution, architectural) and Rec #3 (search ranking) were stashed for
operator input pending a benchmark; Rec #3 was later archived directly and
re-routed as 080-F. `sync --full` was confirmed to hash-skip all files
(no-op) prior to the fix.

## 080-F — Code-first ranking

PR #233 (merge `4ff6c0e`) implemented Rec #3: `unified_search` ranks code
above docs/backlog via a score gap (`CODE_RANK_BOOST = 0.10`,
`f32::total_cmp` stable sort). Also fixed a pre-existing no-op where
`region:"code"` was validated but never enforced — `region:"code"` now
returns code-only via `should_include_content`. A required clippy-1.97
toolchain fix (two pre-existing lints breaking CI) was split into an atomic
chore PR #234 per Copilot review and merged first. Key learning: local
clippy lagged CI by a minor version, masking lints until the toolchain
bumped — reproduce CI clippy locally with `rustup update stable`.

## Preserved, not compacted

Flaky `t030_003`/`t030_001` CozoDB `SQLITE_BUSY` failures under parallel
daemon load (stash `100EACD8`) and the two stashed cross-file
call-resolution / ranking-benchmark deliberations are tracked separately in
the backlog, not in memory.
