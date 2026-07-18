---
title: 091.016-T closure — async pre-pass + parse-dedup (partial delivery)
type: closure-memory
date: 2026-07-18
task: 091.016-T
parent: 091-F
pr: 267
merge_commit: 36a944a5cd7b9bd0ad81535f23165b417b8b0af6
status: done
follow_up: 091.021-T
---

## Outcome

091.016-T shipped Options 1+2 of the `unsafe_module_prefixes` canonical pre-pass
optimization and merged via PR #267 (merge commit `36a944a`). This is a
**partial delivery** of the task's originally-stated acceptance: the concrete
Option 1 and Option 2 deliverables landed; the second-read elimination and the
large-workspace benchmark were spun off to follow-up **091.021-T**.

## What shipped

- **Option 1 (async reads):** the global pre-pass now reads via `tokio::fs`
  instead of blocking `std::fs`, so it no longer blocks the async index runtime.
- **Option 2 (parse-dedup):** the parsed Rust canonical contexts
  (`ModulePath` / `UseGraph`) computed in the pre-pass are hash-checked and
  reused in the main pass, eliminating the duplicate parse. On a content-hash
  mismatch the code recomputes that file's context — reproducing exactly the
  pre-cache behavior.
- **Load-bearing invariant:** canonical edge output is byte-identical to the
  pre-091.016 baseline. Pure performance; no canonical edge added, dropped, or
  reordered.

## Files modified

- `src/services/code_graph.rs` — async pre-pass, `CachedRustCanonicalContext`,
  `rust_ctx_from_prepass_cache` (hash-gated reuse with recompute fallback),
  `force_prepass_cache_miss` test seam, and inline `#[cfg(test)]`
  fallback-equivalence tests.
- `tests/integration/canonical_call_resolution_test.rs` — equivalence coverage.
- `.backlogit/queue/091.021-T.md` — new follow-up (added in the PR).

## Decisions and rationale

- **Adversarial-before-Copilot (operator directive):** GPT-5.6 Sol (xhigh)
  reviewed the base optimization and the test-hardening delta. Verdict: no
  P0/P1, output-equivalent. The forced-cache-miss seam is inert in production —
  the parameter is compiled in all builds but the sole production entry point
  `index_workspace` hardcodes it to `false`, and the only `true` caller is in a
  `#[cfg(test)]` module.
- **Partial delivery over scope creep:** Copilot correctly flagged that Option 2
  eliminates the duplicate *parse*, not the second *read*, and that the
  pre-pass/main-pass TOCTOU on global `unsafe_prefixes` is real (but
  pre-existing — baseline built prefixes from pre-pass bytes too, so this PR is
  output-equivalent and does not regress it). Eliminating the read requires
  caching all source bytes (a single workspace snapshot), which trades
  workspace-scaling memory for I/O. Under operator-AFK conditions this
  higher-blast-radius change was judged out of scope for a low-priority perf
  task and deferred to 091.021-T rather than rushed in.

## Copilot review rounds (PR #267)

1. Round 1 (`e5c7c01`): 2 findings — "not fail-closed" (TOCTOU) and "double read
   not eliminated". Both accuracy/scope, not correctness regressions.
2. Round 2 (`4d02472`): documented the fallback contract on
   `rust_ctx_from_prepass_cache`, narrowed the task acceptance, corrected the PR
   body. New findings: no follow-up item existed; the PR overstated the seam as
   `#[cfg(test)]`-gated.
3. Round 3 (`7c2a514`): created follow-up 091.021-T, corrected the seam text to
   the accurate hardcoded-false guarantee, reframed the PR as partial delivery.
   Copilot COMMENTED with 0 new threads. 4-point gate clean, merged.

## Follow-up

- **091.021-T** (queued): single-snapshot source reuse — remove the second
  canonical pre-pass read and close the global-prefix TOCTOU; includes the
  large-workspace benchmark. Low priority, correctness-neutral today.

## Next steps

- Continue the queue: 091.019-T, 091.017-T, 090.005-T, 092.003-T (assess scope
  and safety per item before building).
- 091.015-T remains BLOCKED (operator input needed on backfill trigger design).
