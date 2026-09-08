---
title: "136-S / PR #385 — operator-authorized UTF-8 ordering fix + review-cascade session"
date: "2026-09-08"
type: "session-checkpoint"
shipment_id: "136-S"
feature_id: "142-F"
pr: 385
final_head: "be58f0bf15b9b96824d5048fa59d19eda5ac82d8"
---

## Session scope

Resumed shipment 136-S on the existing branch
`feat/136-s-generation-domain-store-atomic-publication-and-database-open` and PR #385 to fix
the one operator-authorized, in-scope P-018 blocker: review thread `PRRT_kwDORJEduc6gJLCP`
(`src/db/cozo_backend/mod.rs`, `open_existing_generation_via_runtime_copy` validated
`final_path`'s UTF-8-ness only after `publish_runtime_copy` had already mutated the runtime
directory). Operator explicitly authorized fixing, not deferring.

## Items completed

1. **Root fix** (commit `26868ece`, amended once from `393e0708` for a `clippy::doc_markdown`
   nit): moved `final_path`'s UTF-8 validation to immediately after it is computed, before any
   mutation (`create_dir_all`, lock-file creation, `publish_runtime_copy`). TDD-first regression
   test `open_rejects_a_non_utf8_runtime_root_before_any_runtime_copy_side_effects` (Unix-only,
   `tests/integration/generation_db_open_test.rs`) added first, proven to fail conceptually
   pre-fix (compile-reviewed on Windows since `cfg(unix)` code is stripped there; will execute
   under Linux CI). Sibling-pattern check confirmed no other fallible conversion in the same
   function/helpers was ordered after a mutation.
2. Updated `docs/closure/2026-09-08-136-s-copilot-review-inventory-and-adversarial-review.md`
   (Part 5) and `docs/compound/best-practices/proactive-copilot-review-pattern-checklist-2026-09-08.md`
   (new recurring issue family 9: validate all fallible conversions before an irreversible
   mutation).
3. Full local gates green at `26868ece`: `cargo fmt --all -- --check`, both clippy invocations
   (default + exact CI `--no-default-features --features cozo-backend,embeddings`, both
   `-D warnings -D clippy::pedantic`), `cargo test --all-targets --no-fail-fast` (269 binaries;
   1 unrelated isolation-confirmed flake: `hcl_indexing_test::cold_start_lists_and_maps_all_three_hcl_aliases`),
   `cargo audit` (no new advisories).
4. **Review cascade (5 further rounds after the authorized fix landed)**, each triggered by a
   fresh Copilot pass on every subsequent push (including docs/stash-only pushes):
   - Round: 3 threads — 1 already-fixed (stale, pre-amend), 1 genuine out-of-scope
     error-attribution bug in `publish_runtime_copy`/`sync_runtime_copy_parent_dir`
     (stash `F0760EF2`), 1 closure-doc thread-numbering correction (fixed).
   - Round: 3 threads — 1 genuine out-of-scope `GenerationRevision::new(0)` sentinel-collision
     bug in `services::generations` (stash `959D1448`, `requires_deliberation: true`), 1 PR-body
     staleness (fixed), 1 self-referential closure-doc completeness loop — fixed at the root
     with an explicit scope/cutoff disclaimer (`inventory_cutoff_commit: 26868ece`) so the
     artifact stops chasing its own thread count.
   - Round: 1 thread — genuine out-of-scope `GenerationId` deserialization-bypass claim
     (stash `8D97190A`).
   - Round: 2 threads — one correctly identified that `8D97190A`'s premise did not match the
     current code (`GenerationId` already has a correct custom `Deserialize`); per the P-021
     single-write invariant `8D97190A` was not edited, instead captured a correction + the
     remaining actionable gap (missing malformed-manifest deserialize regression test) as
     `D1D94CF4`.
   - Round: 1 thread — genuine, unrelated lock-acquisition timeout-cleanup deadlock risk,
     captured as `259D0E38`. Recognized as the 5th consecutive round of new findings and
     concluded the review-response loop here per the explicit no-unbounded-loop directive,
     rather than continuing indefinitely.
   Every thread across all rounds was replied-to (citing the fixing commit or the P-021 stash
   ID) and resolved. Zero P-021 captures required editing an already-captured entry; each new
   or corrected finding got its own entry.
5. Final state: `autoharness gate copilot-review 385` → `SATISFIED: PASS` (0 unresolved
   Copilot threads); CI green (`build`, `start-launcher-windows`) at HEAD `be58f0bf`;
   `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`.

## Branch / commit state

Final HEAD: `be58f0bf15b9b96824d5048fa59d19eda5ac82d8` on
`feat/136-s-generation-domain-store-atomic-publication-and-database-open`. Commits this
session: `26868ece` (root fix, amended), `155e87f1`, `6f678107`, `9e0aacc1`, `924119c3`,
`be58f0bf` (all docs/stash-only P-021 captures and corrections). Working tree clean.

## Deferred follow-ups captured this session (P-021, none blocking)

- `F0760EF2` (low): `sync_runtime_copy_parent_dir` error misattributes an fsync failure to
  `final_path` instead of the actual `parent` directory (same "error attribution" family as
  the already-fixed `hash_bounded_reader` bug).
- `959D1448` (medium, `requires_deliberation: true`): `GenerationRevision::new(0)` collides
  with the internal "no current manifest" sentinel.
- `8D97190A` (medium, superseded/corrected by `D1D94CF4` — do not action directly).
- `D1D94CF4` (low): missing regression test for `GenerationManifest`'s deserialize-time
  `GenerationId` validation (the underlying implementation is already correct).
- `259D0E38` (medium): a timeout-cleanup race that can deadlock in unrelated lock-acquisition
  test/cleanup code; exact file/function not independently re-derived before capture.

## Decisions with rationale

- Chose to amend (force-push) the interim `393e0708` commit rather than stack a fix-up commit,
  since the clippy nit was in the same newly-added test and stacking would have muddied a
  single bounded logical change.
- Added an explicit scope/cutoff disclaimer to the closure-evidence doc instead of continuing
  to chase its own thread count, because editing that doc is itself a commit that re-arms
  Copilot review against the new HEAD — a self-referential loop that cannot converge by
  perpetually updating historical counts.
- Stopped the review-response loop after 5 consecutive rounds of new, unrelated findings
  surfacing after every push (including docs/stash-only pushes), per the explicit
  no-unbounded-loop directive, once the authorized finding was fully fixed and every
  surfaced thread was replied-to/resolved-or-deferred with full P-021 traceability.
- Did not merge. Did not touch 142-F's covering-feature state, other 136-S backlog items, or
  any other shipment.

## Next steps

- Awaiting explicit operator merge approval (only remaining gate).
- Stage should triage/reconcile stash entries `F0760EF2`, `959D1448`, `8D97190A` (superseded),
  `D1D94CF4`, and `259D0E38` in a future planning cycle.
- `259D0E38`'s exact file/function should be independently re-derived before scheduling a fix.
