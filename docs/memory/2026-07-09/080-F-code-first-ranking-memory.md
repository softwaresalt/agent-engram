---
title: "080-F Code-First Ranking — Session Memory"
type: session-memory
date: 2026-07-09
feature: 080-F
tasks: [080.001-T]
prs: [233, 234]
merge_sha: 4ff6c0e
harvested_stash: B791DE7B
status: complete
---

## Summary

Implemented the operator's decision to rank code above docs/backlog in
`unified_search` via a **score gap** (harvested from stashed Rec #3). Shipped as
080-F; a required clippy-1.97 toolchain fix was split into an atomic chore PR
(#234) per Copilot review.

## What shipped (080-F, PR #233, merge `4ff6c0e`)

- `merge_unified_results` ranks by `rank_key = score + CODE_RANK_BOOST` for code
  (`CODE_RANK_BOOST = 0.10`, tunable const), raw `score` for content, sorted with
  `f32::total_cmp` (stable → gap-boundary ties keep code). A content result
  outranks code only when more relevant by more than the gap. Reported `score` is
  unchanged (unboosted per-source value — cosine for KNN, keyword ratio for the
  content fallback).
- Fixed a pre-existing `region:"code"` no-op: `unified_search` validated `region`
  but never used it. Now `should_include_content(region)` gates the content
  fetch, so `region:"code"` returns code-only. Result: **code-first by default,
  code-only via `region:"code"`.**
- Files: `src/services/search.rs`, `src/tools/read.rs`,
  `tests/integration/unified_search_region_test.rs` (gated e2e), `Cargo.toml`.

## Chore PR #234 (merge `f9f9851`)

Rust 1.97's clippy flagged two pre-existing lints under `-D warnings` (breaking
CI on main): `question_mark` in `pbip_extract.rs` and `manual_assert_eq` in a
search test. Split into an atomic `fix:` PR (Copilot flagged the pbip refactor as
out-of-scope for 080-F). Merged first; 080-F then merged main so those changes
dropped out of its diff.

## Verification

- TDD red→green: 3 boost-dependent merge tests genuinely red before the boost.
- 10 `merge_unified_results` unit tests + 1 gated e2e (`region:"code"` excludes
  content, `region:"all"` includes both — guards the call-site wiring).
- fmt + clippy clean (1.97); full `--all-targets` suite green.
- Adversarial code review clean (total_cmp descending + stable tie→code, boundary
  bit-identical f32 tie→code, region validated before gate, truncation panic-free).
- Copilot review: 3 iterations, 10 comments total — all addressed (score-wording
  clarified, plan synced, e2e test added, pbip split to #234) and resolved.
- CI green; merged via merge commit (2 parents).

## Key learnings

- **Local clippy lagged CI**: local was 1.96, CI uses latest stable (1.97). A
  toolchain bump surfaced pre-existing lints. `rustup update stable` + running the
  exact CI clippy command locally is the reliable way to reproduce CI clippy.
- **Atomicity**: unrelated CI-unblock fixes belong in a separate PR; merge it
  first, then `git merge main` into the feature branch so they drop from its diff.
- **Flaky `t030_003`**: still fails intermittently under parallel daemon load
  (CozoDB SQLITE_BUSY, stash `100EACD8`); re-run, don't debug.

## Notes / follow-ups

- `CODE_RANK_BOOST = 0.10` is provisional. The operator agreed a relevance
  benchmark should precede further tuning — a benchmark harness remains a good
  future task (the merge unit tests only prove mechanics).
- `.github/agents/*.md` external churn continues (excluded from all commits).
