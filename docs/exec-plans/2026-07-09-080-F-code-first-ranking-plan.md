---
title: "080-F Implementation Plan — Score-gap code-first ranking in unified_search"
type: exec-plan
date: 2026-07-09
feature: 080-F
tasks: [080.001-T]
supersedes_stash: B791DE7B
status: draft
---

# 080-F — Rank code above docs/backlog in `unified_search` (score-gap code-first)

## Decision (operator)

engram's primary purpose is semantic/graph search over **code**; documentation
and backlog are secondary. Therefore code should rank first — but via a **score
gap**, not a hard tier: a content result may still outrank code when it is more
relevant by more than a margin. (Operator also required a benchmark before
tuning; the merge ordering tests below are that regression gate.)

## Problem

`merge_unified_results` (src/services/search.rs:135) merges code-region and
content/task-region results and sorts purely by descending cosine score ("No
cross-region normalization or boosting in v0"). Because a prose query embeds
closer to prose docs, documentation/memory records routinely outrank the
implementing code for code-intent queries (observed: docs at 0.79–0.81 above
`run_daemon_status` at 0.75).

## Design — additive code boost == score gap

Apply an additive ranking boost `CODE_RANK_BOOST` to the **sort key** of
code-region results:

```text
rank_key(r) = r.score + (CODE_RANK_BOOST if r.region == Code else 0.0)
```

Sort by `rank_key` descending. Consequences:
- A content result outranks a code result **iff** `content.score > code.score +
  CODE_RANK_BOOST` — i.e. content must beat code by more than the gap. This is
  exactly "code-first within a score gap."
- Within a region, order is unchanged (every code result gets the same boost, so
  relative order is preserved; content is untouched).
- The reported `score` field stays the **true cosine** — the boost affects
  ordering only, preserving score transparency for callers.

`CODE_RANK_BOOST = 0.10` (module const, documented as the tunable knob). Rationale:
flips marginal prose-query cases (docs beating code by <0.10) to code-first while
still surfacing a genuinely-more-relevant doc (>0.10 higher). Cosine scores are
in [0,1]; 0.10 is a meaningful but not absolute preference.

### Scope
- Only the default `region:"all"` merge path. `region:"code"`, content-only, and
  single-region searches are unaffected (within-region order unchanged).
- `vector_search_symbols_native`, `hybrid_graph_vector_search`, and `hybrid_search`
  are lower-level and do not go through `merge_unified_results` — untouched.

## Regression analysis

No existing test asserts strict cross-region score-descending on
`merge_unified_results`/`unified_search`:
- `embedding_test.rs:97` → `hybrid_search` (keyword). Unaffected.
- `cozo_vector_test.rs:127` → `vector_search_symbols_native` (single-region KNN). Unaffected.
- `unified_search_knn_test.rs` → schema/error/limit only, no ordering assertion.

## Test strategy (TDD — red first, the benchmark gate)

Unit tests in `src/services/search.rs` `#[cfg(test)] mod tests` on
`merge_unified_results`, referencing `CODE_RANK_BOOST` so they track the constant:
1. **code_ranks_before_content_within_gap**: code 0.70, content `0.70 +
   CODE_RANK_BOOST/2` → code first (red today: raw sort puts content first).
2. **strongly_better_content_outranks_code**: code 0.70, content `0.70 +
   CODE_RANK_BOOST*2` → content first (the gap escape hatch).
3. **code_results_sorted_by_score_within_region**: multiple code → descending by score.
4. **content_results_sorted_by_score_within_region**: multiple content → descending.
5. **reported_score_is_unboosted_cosine**: a code result's returned `.score`
   equals its input cosine (boost is ordering-only).
6. **respects_limit_after_reordering**: truncation applies after code-first sort.

Plus a **live before/after** on the engram workspace (recorded in closure): the
same code-intent queries should surface the implementing code at/near the top.

## Constitution check
- Safety-First Rust: pure function; no `unsafe`; no `unwrap`/`expect` in prod
  (`partial_cmp(...).unwrap_or(Equal)` retained). Clippy pedantic clean.
- Test-First: red merge tests observed to fail before implementation.
- Single Responsibility: no new deps; one const + one helper.
- Context efficiency: unchanged (same result payload).

## Risk / rollback
Ordering-only change to one pure function; response shape and `score` values
unchanged. Rollback = revert the merge commit. `CODE_RANK_BOOST` is a one-line
tuning knob if the gap needs adjustment after benchmarking.

## Adversarial review refinements (applied)

Rubber-duck review — one blocker + refinements incorporated:

1. **BLOCKER — `region:"code"` is a no-op filter (pre-existing bug, tightly
   coupled).** `unified_search` (read.rs:532-623) fetches BOTH code and content
   unconditionally and never gates on `parsed.region`, so `region:"code"` still
   returns content — contradicting the tool catalog (tools_catalog.rs:199-204).
   Since my plan leaned on `region:"code"` as the "code-only" escape hatch, fix
   it here: add a pure `should_include_content(region) -> bool` helper
   (`region != "code"`), unit-test it, and in `unified_search` skip the content
   fetch when it returns false (`content_results = Vec::new()`). This makes the
   feature coherent: **code-first by default, code-only via `region:"code"`.**
2. **Float hardening** — sort with `f32::total_cmp` on the rank keys (total,
   finite-safe ordering) instead of `partial_cmp(...).unwrap_or(Equal)`.
3. **Boundary/tie test** — `content.score == code.score + CODE_RANK_BOOST` must
   rank **code first** (strict `>` semantics; stable `sort_by` keeps code, which
   is extended first, ahead on ties).
4. **Truncation test** — `limit = 1`: content within the gap → only code
   survives; content beyond the gap → only content survives. Encodes the
   accepted "code primary, no reserved content slot" behavior (a doc-intent
   escape valve remains `query_memory`, which is content-only).
5. **Provisional gap** — keep `CODE_RANK_BOOST = 0.10` but treat as provisional;
   the live before/after in closure is the relevance check (unit tests prove
   mechanics only).
6. **Stale comments** — update search.rs:130-133 and read.rs:459-461 ("sorted by
   descending cosine") to "ranked by a code-biased key; reported score is raw".

### Genuinely-red tests (TDD)
`code_ranks_before_content_within_gap`, the boundary/tie test, and the
truncation test fail against the current raw-score sort. The within-region and
unboosted-score tests are regression coverage (already pass).
