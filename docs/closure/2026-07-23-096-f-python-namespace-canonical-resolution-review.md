---
title: "Multi-persona review — Python namespace-qualified call resolution (096-F, shipment 091-S)"
type: closure
date: 2026-07-23
slug: 096-f-python-namespace-canonical-resolution-review
subject_commit: 82488eae61d152f8c740cd8c7580309aba29b9b5
subject_branch: feat/py-namespace-canonical-resolution
scope: >-
  src/services/code_graph.rs, src/services/parsing/python_canonical/*,
  src/db/cozo_queries.rs, src/db/cozo_backend/schema.rs,
  src/cli/commands/indexing.rs, src/cli/direct.rs, docs/architecture.md,
  tests/integration/*, tests/unit/python_canonical_test.rs
reviewers: 4
review_models:
  - rust-reviewer: gpt-5.6-sol
  - security-reviewer: gemini-3.1-pro-preview
  - architecture-strategist: gpt-5.6-terra
  - constitution-reviewer: claude-sonnet-4.6
verdict: APPROVE-WITH-FIXES
gate_blocking: true
---

# Multi-persona Review — Python namespace-qualified call resolution (096-F)

## Verdict: **APPROVE-WITH-FIXES**

Feature 096-F extends Engram's canonical call-resolution to Python module
namespaces, resolving cross-module same-name calls to the exact target and
failing closed on any ambiguity (013-D zero-false-edge invariant). The change
is **correct on its central axis** — the adversarial acceptance corpus
(import-after-def, def-after-import, later-star, receiver rebinds, duplicate
same-name import, function-local poison) passes and directly exercises the
fail-closed paths.

Review surfaced **one confirmed false-edge gap (P0)** and **one confirmed CLI
footgun (P1)**, both fixed before merge. The remaining high-severity claims from
the Rust persona were verified as **documented v1 recall-safe fallbacks,
design decisions with a fail-closed backstop, or pre-existing cross-cutting
behavior** — rejected with evidence or deferred as backlog follow-ups.

- **Gate-blocking findings fixed:** 2 (1×P0 false edge, 1×P1 CLI footgun).
- **Rejected as false positives (with evidence):** 8.
- **Deferred to backlog (P2/P3):** 7.
- **Quality gates:** fmt clean, clippy `-D warnings -D clippy::pedantic` clean,
  `cargo dev-test` 459 lib tests + all integration/contract binaries green.

---

## Applied fixes

### F1 — Fail closed on a dynamically rebound module receiver (P0, false edge)

**Commit `c7432e7`.** Source: rust-reviewer P0 (`code_graph.rs:860`).

The module-qualifier resolution guard checked `binding.kind`,
`python_name_is_function_local`, and `python_receiver_rebound_after`, but **not**
`is_dynamically_rebound`. `PythonShadowIndex::build` stops descending at function
bodies, so a `global m; m = factory()` write inside a *sibling* function is
captured only by the `dynamic_rebinds` signal — never by `module_rebinds`, which
`python_receiver_rebound_after` consults. The bare-call path already guards this
at `code_graph.rs:760`; the module-qualifier path did not. Result: `m.f()` with
a globally rebound `m` minted a false `a.f` canonical edge.

**Fix:** add `|| shadow.imports.is_dynamically_rebound(&call.raw_qualifier)` to
the module-qualifier guard, routing the case to `CompetingBindings` (which does
**not** allow name-only fallback — verified at `cozo_queries.rs:238`), so the
call fails fully closed. Strictly more conservative: it can only drop an edge,
never add one. Added RED→GREEN acceptance test
`python_module_receiver_dynamic_global_rebind_fails_closed`
(`calls_recall_acceptance_test.rs`), verified failing before and passing after.
The positive `python_module_and_from_import_resolve_to_exact_target` still passes.

### F2 — Force re-extraction on `sync --full --backfill-python-canonical` (P1, footgun)

**Commit `82488eae`.** Source: architecture-strategist P1#3.

`run_sync`'s `full || force` branch dispatched `index_workspace` with
`force_params(force)`, silently dropping `--backfill-python-canonical`. Combined
with the marker gate (`force || !python_hash_skipped`, `code_graph.rs:1758`), a
non-forced `sync --full --backfill` hash-skipped every unchanged `.py` and never
advanced the extraction-version marker — the migration silently did nothing. The
same pattern existed on the `--direct` path (`direct.rs`).

**Fix:** fold `--backfill` into `--force` on the full-scan path only —
`let force = force || (full && backfill_python_canonical);` — in both `run_sync`
(`indexing.rs`) and `run_direct_sync` (`direct.rs`), mirroring `run_index`'s
existing fold. The bare incremental `sync --backfill` path is preserved
(`full` is false, so the fold is a no-op). Also corrected `docs/architecture.md`:
a plain `engram index` and `engram sync --full` default to `force=false` and
hash-skip unchanged files — they are **not** a forced reparse.

---

## Rejected findings (false positives, with evidence)

| ID | Finding | Evidence for rejection |
|---|---|---|
| RJ1 | rust P0 `:969` — canonical target lookup not language-scoped | Python canonical paths use `.` separators; Rust uses `::` (`Type::method`). The namespaces are disjoint — a Python dotted target can never match a Rust path. The fallback path passes `"python"` explicitly (`code_graph.rs:990`). No cross-language edge is possible. |
| RJ2 | rust P0 `:814` — duplicate/conditional imports reach fallback | Duplicate same-name imports map to `UnsupportedImportForm` by design, then fail closed via `function_ids_by_name` non-uniqueness (`ids.len() != 1 → continue`). Covered by the duplicate-canonical-path acceptance case. |
| RJ3 | rust P0 `:379` (bindings) — conditional wildcard loses fail-closed | A star import yields the recall-safe `UnsupportedImportForm`; the fallback is unique-or-nothing. `later-star` is tested and green; a conditional star is no more resolvable than an unconditional one. |
| RJ4 | rust P0 `:832` — bindings after caller misclassified | A forward-referenced def resolves correctly via the unique name-only singleton (same target), not a false edge. Order-uncertainty with a winner is handled by the `all_positions > winner` shadow check (`:823`). |
| RJ5 | rust P0 `:868` — NoModuleContext bypasses shadow checks | `NoModuleContext` preserves the 094-F name-only recall-safe fallback (Anchor B). Not a regression; documented v1 behavior. Same-name shadowing on the fallback path is the FF7DE872-family non-goal. |
| RJ6 | rust P0 `:966` — missing caller metadata → unconstrained scope | `usize::MAX` for non-top-level callers is intentional (nested/method callers are a v1 non-goal — documented at `python_caller_position`). The orphaned-row case requires DB inconsistency. |
| RJ7 | rust P0 `:988` / `:1283` topology & singleton staleness | Pre-existing incremental-index behavior, not introduced by 096-F. The index-path marker gate refuses to advance when any `.py` is hash-skipped, so it fails closed on currency; C6-1 was DoD-scoped to `sync`. |
| RJ8 | arch P1#1/P1#2 — Rust byte-identical / snapshot bypass repeats | `cozo_queries.rs` diff is additions-only; `function_ids_by_name` is language-scoped. Snapshot is republished at `code_graph.rs:1748` & `:2656`; topology invalidation independence from the version gate is the intended C6-1 design. |

---

## Deferred backlog follow-ups (P2/P3)

1. **rust P0 `:2022` — emptied `.py` files skipped as `files_unchanged`.**
   Pre-existing, cross-cutting sync guard ("handles TOCTOU race") affecting all
   languages; leaves stale symbols/edges when a file is truncated to 0 bytes.
   Out of scope for 096-F.
2. **rust P0 `:1283` — extend C6-1 package-topology invalidation to the index
   path** for parity with `sync` (currently backstopped by the marker gate).
3. **rust P1 `:686` — harden post-pass error propagation.**
   `python_ctx_for_staged_file` uses `.ok()?`, so a transient read/DB failure in
   the reresolve post-pass is swallowed and could advance the backfill marker
   despite an incomplete pass (C7-3 edge). The primary extraction-loop failure
   path is covered and tested.
4. **rust P1 `:760` — resolve provable function-local imports** via
   `ImportBindings::resolve_call` instead of the boolean poison (v1 recall
   non-goal; the bindings module is richer than the production path uses).
5. **rust P2 `schema.rs:430` — `std::thread::sleep` in the SQLITE_BUSY retry**
   blocks a Tokio worker (pre-existing pattern shared with the schema-meta flag
   setter; contention-only).
6. **arch P2/P3 — Python resolution leaks into `code_graph` orchestration;
   `module_path` placement.** Cohesion follow-up.
7. **constitution P2 — file-level `#![allow(clippy::...)]` in the new test
   file.** Prefer per-item allows.

Notes: the constitution-reviewer's CHANGELOG P3 is **moot** — the repo has no
`docs/CHANGELOG.md` and no `cliff.toml`. The rust P2 `bin/engram.rs`
`#![forbid(unsafe_code)]` suggestion is a pre-existing, out-of-scope crate-root
change unrelated to this feature.

---

## Security & constitution posture

- **security-reviewer:** clean (≤P3). Bound query params (no injection),
  `module_path` rejects `..` via identifier checks, `follow_links(false)`, file
  sizes bounded (DoS-safe).
- **constitution-reviewer:** no P0/P1. Principle VI (zero new dependencies) and
  Principle I (no `unsafe`/`unwrap`/`expect` in production) upheld.

## Runtime verification

Recommended mode: **manual (CLI)**. Exercise the migration path on a real
workspace — `engram index --backfill-python-canonical`, then confirm the marker
advances and cross-module canonical edges materialize; separately confirm
`engram sync --full --backfill-python-canonical` now forces re-extraction
(the F2 fix). No API/browser surface is affected.

## Strict-safety / rollout

No destructive actions in this change set. The migration is version-gated and
fail-closed toward retry (marker persists only on a fully-successful `.py`
pass). The one residual rollout risk — a swallowed post-pass failure advancing
the marker (deferred item 3) — is narrow and backlogged.
