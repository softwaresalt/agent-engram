---
title: "May 9 Ship and Stage Roundup"
type: compacted-memory
date: 2026-05-09
shipments:
  - 029-S
  - 031-S
  - 032-S
  - 033-S
  - 034-S
features:
  - 044-F
  - 046-F
  - 047-F
  - 048-F
  - 049-F
sources:
  - docs/archive/memory/2026-05-09/029-s-indexing-resilience-ship-memory.md
  - docs/archive/memory/2026-05-09/029-s-pr102-review-closure-memory.md
  - docs/archive/memory/2026-05-09/031-s-ship-memory.md
  - docs/archive/memory/2026-05-09/032-S-cli-resilience-shipping-memory.md
  - docs/archive/memory/2026-05-09/033-s-shipped-query-graph-memory.md
  - docs/archive/memory/2026-05-09/033-s-stage-query-graph-memory.md
  - docs/archive/memory/2026-05-09/034-s-daemon-startup-reliability-memory.md
  - docs/archive/memory/2026-05-09/group-a-staging-memory.md
  - docs/archive/memory/2026-05-09/group-b-staging-memory.md
---

## Summary

* 029-S shipped indexing resilience: SQLITE_BUSY guards, queued sync, configurable CLI timeout, and post-merge review closure for PR #102
* 031-S shipped installer and workspace-flag fixes and established binary-level regression tests as the dispatch guard
* 032-S shipped CLI resilience and error handling improvements for direct mode, progress hints, and daemon-held DB detection
* 033-S shipped the structured `query_graph` API and Stage split the follow-up graph work into discrete tasks
* 034-S improved daemon startup reliability with early hydration-ready, auto-reindex gating, and removal of read-tool indexing guards
* Group A and B staging harvested 046-F and 047-F, normalized duplicate stash items, and moved the query_graph backlog forward

## Key Decisions

* Binary-level tests are required for CLI dispatch regressions
* `drain_pending_sync()` must run from every `finish_indexing()` path
* `ENGRAM_AUTO_REINDEX` stays opt-in to avoid startup OOMs
* `query_graph` uses a structured JSON model rather than raw text
* Copilot review threads only close after explicit thread resolution

## Verification

* Targeted integration suites, cargo fmt, cargo clippy, cargo dev-test, and green PR CI were used across the sessions
* Merge commits were preserved for shipped PRs and closure PRs

## Open Items

* 033.005-T remained blocked on upstream tree-sitter-sequel grammar
* One startup-related flaky test was pre-existing and not caused by these sessions