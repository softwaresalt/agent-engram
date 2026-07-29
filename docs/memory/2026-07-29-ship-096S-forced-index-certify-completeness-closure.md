---
title: "Ship 096-S — forced-index / revalidate certify-path completeness (103-F / 685FAA80+92EE75BB) merge closure"
date: "2026-07-29"
type: "ship-closure-memory"
feature: "103-F"
shipment: "096-S"
pr: 299
merge_commit: "300d020ac51b8f845df03809b11e334c18eef158"
status: "shipped"
---

## Outcome

Feature 103-F shipped via **PR #299** (merge commit
`300d020ac51b8f845df03809b11e334c18eef158`, merge-commit strategy per P-009 —
2 parents `f965ee0c` ∪ `7719f144`). This is the completeness/hygiene follow-up
to the freshly-merged 101-F versioned `code_graph_extraction_generation` marker
(094-S). It closes two certify-path gaps that let the marker certify a stale
graph (source stashes `685FAA80` + `92EE75BB`):

- **G1 — no orphan-edge GC (`685FAA80`).** `rm_orphan_edges` was lineage-only;
  no global sweep reconciled `calls_edge` rows whose `from`/`to` lost its
  `function_meta`. Same-file duplicate-name shadowing (100-F) plus a
  forced-index marker advance could leave orphaned/stale `direct` edges behind.
- **G2 — forced-index file-set staleness (`92EE75BB`).** The forced-index route
  (`index_workspace_impl`) advances the versioned marker on `force` alone while
  walking only *currently-discovered* files, so a previously-indexed file now
  excluded-but-still-on-disk kept its nodes/edges while the marker certified its
  generation as current.

## Git base decision

`main` == `origin/main` == `f965ee0c` (095-S absorbed; cycle-3 planning files
already on `origin/main`), so the feature branch
`103-forced-index-certify-path-completeness` was based on local `main`
`f965ee0c` and the PR base diff was clean. `start.ps1`'s unrelated uncommitted
modification was never touched or committed (explicit-path staging throughout;
never `git add -A`).

## Per-task outcomes (TDD honored, dependency order)

- **103.001-T (U1) orphan `calls_edge` sweep** — RED `d4df0fdb` (3 scenarios:
  revalidation-path sweep, forced-index sweep, direct primitive exact +
  idempotent) confirmed failing at `count_dangling_calls_edges() == 0` (orphans
  survive). GREEN `848c2250`: added
  `CodeGraphQueries::retract_dangling_calls_edges()` in `cozo_queries.rs` — a
  count-first-then-retract primitive expressing the OR-predicate ("`from` has no
  def **or** `to` has no def") via an intermediate `orphan[from,to]` relation
  with two rules (auto-deduped by Datalog set semantics), retracting via
  `:rm calls_edge { from, to }`. Wired into BOTH certify blocks
  (`index_workspace_impl` + the gated-revalidation sync) before the generation
  marker advances. Added `dangling_edges_swept` to `IndexResult` + `SyncResult`
  (A6 observability).
- **103.002-T (U2, depends on U1) forced-index file-set reconciliation** — RED
  `41286318` (3 scenarios: H5+ excluded-evicted, H5− discovered-kept negative
  control, H4 idempotence) confirmed failing. GREEN `ac0ce9ab`: in the
  `index_workspace_impl` certify block, gated by `force || !any_hash_skipped`,
  compute `discovered_rel` from the walked file set and evict every
  `indexed − discovered` file via the proven `handle_deleted_file` eviction
  primitive **before** the U1 orphan sweep (defensive single-pass-clean order;
  `handle_deleted_file` already retracts the evicted file's resolved edges in
  both directions + its same-file direct edges, so the sweep is a legacy
  final-state GC, not a cleanup of eviction-produced orphans). Added
  `files_reconciled` to `IndexResult`. The H5−
  negative branch is structurally guaranteed: a discovered/hash-skipped file
  stays in `discovered_rel` (kept) and `any_hash_skipped` also skips the whole
  certify block.

## Copilot review remediation (cycle 1 of 3; cap respected)

The review surfaced 3 threads on the initial HEAD `1845d284`; all bot-authored,
all replied-to (referencing the fixing SHA `7719f144` or a grounded decline +
stash ID) and resolved via `resolveReviewThread` (3/3 resolved at merge). The
fresh Copilot re-review at the fix HEAD raised **no** new threads.

- **Fixed (2, valid A6-observability test gaps), commit `7719f144`:**
  - `forced_index_fileset_reconciliation_test.rs` idempotence scenario — capture
    both `IndexResult`s and assert `files_reconciled == 1` (first forced index)
    then `== 0` (second), proving the observability field, not just DB state.
  - `orphan_calls_edge_sweep_test.rs` — capture the `SyncResult` (revalidation)
    and `IndexResult` (forced index) and assert `dangling_edges_swept == 2` and
    `== 1` respectively for the injected orphan rows.
- **Declined + stashed (1, out-of-scope non-regression):** `code_graph.rs:1936`
  — Copilot argued the U2 eviction should run **before** the cross-file
  singleton post-pass (`reresolve_calls_edges_with_canonical_context`) so a
  duplicate-callee name in an about-to-be-evicted file does not withhold a
  recoverable singleton. **Not a regression:** pre-103-F the excluded file was
  never evicted, so the post-pass saw the identical ambiguity and withheld the
  same singleton — 103-F changes no cross-file behavior, only adds
  retraction-only hygiene (fail-closed: a missing edge, never a false one). The
  recall suite stays 18/18 green (DoD met). Reordering interacts with the
  082-F/094-F/096-F/101-F post-pass invariants and needs its own unit + a
  dedicated duplicate-callee-then-excluded recall-**recovery** test → stash
  `7A317008` (task/medium).

## Gates + review + runtime

- `cargo fmt --all -- --check` PASS; `cargo clippy --all-targets
  --no-default-features --features cozo-backend,embeddings -- -D warnings
  -D clippy::pedantic` PASS; `cargo test --lib` (CI feature set) **466/466**;
  both new integration suites **3/3 + 3/3**; recall 18/18 + revalidation +
  shadowing suites green. `cargo audit` = 1 pre-existing accepted advisory
  (`lz4_flex` RUSTSEC-2026-0041, deferred cozo-bump) + unmaintained warnings —
  **no new dependencies** (Cargo.lock unchanged).
- CI `build` PASSED on final HEAD `7719f144` (SUCCESS).
- Copilot merge gate FULLY GREEN, re-verified immediately before merge: latest
  review `commit_id == 7719f144 == HEAD == local HEAD`, Copilot off
  `requested_reviewers`, 0 unresolved threads (3/3), `mergeStateStatus == CLEAN`.
  No fresh review landed past HEAD (circuit-breaker guard clear).
- Runtime verification: both new certify paths are exercised by the integration
  suites (real forced index + gated revalidation sync against temp workspaces);
  the eviction + sweep are retraction-only and reuse the existing
  `handle_deleted_file` primitive, so no traversable wrong edge is created and
  no recall is lost on the existing corpus.

## Closure actions

- Shipment 096-S → **shipped**; archived scope: 103.001-T, 103.002-T, 103-F,
  096-S. Merge SHA recorded; archive metadata preserved `references:` (no links
  dropped).
- 1 follow-up stashed: `7A317008` (forced-index reconciliation ordering vs the
  cross-file singleton post-pass; recall-recovery enhancement, task/medium).
- Durable knowledge: updated the 094-S versioned-marker compound entry to record
  that the forced-index reconciliation gap `92EE75BB` and the orphan-edge GC gap
  are now **closed by 103-F/096-S**, and graduated a focused best-practices entry
  (`certify-completeness-reconcile-fileset-and-sweep-orphans-2026-07-29`).

## Key finding (hard-won) — a completeness marker must reconcile the full input set

A versioned/generation marker that certifies "this graph is materialized under
logic version N" is only sound if the pass that advances it reconciles the
**complete** persisted input set, not just the files it happened to walk. The
101-F marker advanced on `force` alone while the forced-index route discovers
only currently-present files, so two silent staleness leaks survived: (1)
legacy `calls_edge` rows already orphaned when their keying `function_meta` was
retired by a pre-101-F ordinary sync (no global GC existed), and (2) whole files
that dropped out of discovery (excluded-still-on-disk) kept their nodes/edges —
because, unlike the incremental sync path (which reconciles `indexed − discovered`
in its Phase 1 deletion sweep), the forced-index route had no such comparison.
The fix is two retraction-only reconciliations in the certify block, ordered
**file-set eviction → orphan-edge sweep → marker advance**. Eviction reuses the
shared `handle_deleted_file` primitive, which retracts a file's resolved edges in
**both** directions (`from` and `to`) plus its same-file `direct` edges, so
eviction is self-cleaning and does not itself produce dangling rows; the orphan
sweep is a global *final-state / legacy* GC for pre-existing orphans (e.g.
same-file shadowing re-mints), and running it after eviction just certifies a
single clean exit. Both are retraction-only and reuse proven same-file/lineage
primitives, so no cross-file edge and no recall is lost. The residual subtlety —
that eviction after the cross-file singleton post-pass forgoes a recall
*recovery* opportunity — is fail-closed and correctly deferred (`7A317008`)
rather than reordered under review pressure.

## Process learnings for next ship

- **"Advance the completeness marker" ⇒ audit every input the marker claims
  to certify.** When a marker advances on `force`/singleton conditions, grep the
  route's discovery source: if it walks only currently-present files, indexed
  artifacts that dropped out of discovery are silently certified stale. The
  incremental sync path already reconciled this via its Phase 1
  `indexed − discovered` deletion sweep; the gap was that the **forced-index**
  route had no equivalent — so reconcile `indexed − discovered` (and sweep
  pre-existing orphans) before advancing.
- **Order the reconciliations eviction → sweep for a single clean exit — but
  know why.** `handle_deleted_file` retracts an evicted file's resolved edges in
  both directions and its same-file direct edges, so eviction does *not* itself
  produce dangling rows; the orphan sweep is a legacy/final-state GC. Ordering
  eviction first is a defensive single-pass-clean choice (the plan notes
  idempotence holds either way), not a hard orphan-production dependency.
- **New observability fields need test assertions, not just DB assertions.**
  Both valid Copilot findings were "you assert the DB state but discard the
  result struct" — capture `IndexResult`/`SyncResult` and assert the new
  `files_reconciled` / `dangling_edges_swept` counts so the API contract is
  covered.
- **Batch deferred-finding stash bookkeeping into the fix commit** (repeat of the
  095-S lesson) — folding `stash.jsonl` into `7719f144` avoided a second
  review-clock reset; the single fix HEAD got one fresh Copilot pass and merged.
- **A recall finding on a recall-anchored feature can still be a correct
  decline** when it is a non-regression fail-closed *enhancement* out of the
  plan's scope — preserve it via a stash with a grounded rationale rather than
  reorder a high-invariant pipeline under review-time pressure.
