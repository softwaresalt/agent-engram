---
title: "079-F Force-Rebuild Flag + Graph/Search Quality Assessment — Session Memory"
type: session-memory
date: 2026-07-08
feature: 079-F
tasks: [079.001-T]
prs: [231]
merge_sha: 1be2e17
stashes: [3AF329FF, B791DE7B]
status: complete
---

## Summary

Two-part session: (1) a data-grounded assessment of engram's codebase and
search/graph quality, then (2) routing the resulting recommendations through the
pipeline — one implemented (079-F), two stashed for operator input.

## Assessment findings (2026-07-08)

- **Vector data**: excellent — 100% embedding coverage (2333/2333 symbols).
- **Graph data**: complete symbol inventory (272 files, 2134 fns, 198 classes,
  3703 edges = 2333 defines + 1368 calls + 2 inherits) but **partial call-edge
  recall** — cross-file calls are dropped (file-local resolution) and method
  calls are dropped at extraction. `map_code`/`impact_analysis` unreliable for
  functions that call cross-file or via receivers.
- **Semantic search**: good retrieval (found `process_memory.rs` at 0.84 for a
  behavior query) but doc-biased ranking for prose queries.
- **Operational**: hit a duplicate-daemon / cold-start "not indexed" state from
  prior daemon juggling; resolved by stopping the stale daemon. `health`'s
  `embedding_status` is always 0/0 by design (`embedding::status(None)`); real
  coverage is via `stats`.
- **`sync --full` no-op**: confirmed it hash-skips all files (never rebuilds).

## Work routed

- **Rec #2 → 079-F (SHIPPED, PR #231, merge `1be2e17`)**: added `--force` flag to
  `sync`/`index` → sends `index_workspace` `force=true` (IPC + direct paths),
  routing on `full || force`. Additive, no regression. Files: `src/bin/engram.rs`,
  `src/cli/commands/indexing.rs`, `src/cli/direct.rs`,
  `tests/integration/cli_direct_test.rs` (2 subprocess TDD tests).
- **Rec #1 → STASHED (`3AF329FF`, high)**: call-edge cross-file + method
  resolution. Architectural, org-wide blast radius, precision/recall product
  tradeoff. Full design in
  `docs/decisions/2026-07-08-callgraph-cross-file-resolution-deliberation.md`
  (recommends Option B: deferred post-pass + unambiguous-name guard, reusing the
  existing `reresolve_references_edges` pattern).
- **Rec #3 → STASHED (`B791DE7B`, medium)**: search ranking balance. Product/
  relevance decision; needs a benchmark; `region=code` already mitigates.

## Verification

- 079-F: fmt + clippy clean; TDD red→green; adversarial code review clean (all 12
  sync/index/force/direct combinations traced correct); Copilot review clean
  (0 comments); CI green; merged via merge commit (2 parents).
- Full `--all-targets` run: 2 failures in `lang_ipc_indexing_test.rs`
  (`t030_001_swift/cpp`) — **pre-existing** CozoDB 0.7.6 SQLITE_BUSY flakiness
  under parallel daemon load; **pass in isolation** (`--test-threads=1`);
  unrelated to the CLI change (tracked stash `100EACD8`).

## Notes for next session

- `.github/agents/*.md` shows recurring uncommitted churn (an external process
  renames agent defs to dot/underscore prefixes and deletes originals). Not from
  this work; excluded from all commits. Worth investigating separately.
- The two stashed deliberations (Rec #1, Rec #3) await operator answers to the
  design questions before harvesting into features.
- To make 079-F live: rebuild + reinstall the binary, then `engram index --force`
  performs a true rebuild.
