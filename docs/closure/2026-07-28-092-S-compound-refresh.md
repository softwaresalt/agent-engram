---
title: "Compound Refresh — 100-F same-file duplicate-name fail-closed direct-edge target (FF7DE872)"
date: "2026-07-28"
scope: "recent"
mode: "apply"
context: "Post-merge closure of shipment 092-S / feature 100-F (PR #291, merge 8a6c6e32)"
feature: "100-F"
shipment: "092-S"
pr: 291
merge_commit: "8a6c6e32507434ff80e7453b92ecf27d21992bc4"
---

# Compound Refresh — 100-F (same-file duplicate-name fail-closed)

Post-merge capture of durable learnings from feature 100-F, which fixes the
`find_function_id` first-match wrong-target direct edge by failing closed on
`>1` same-file same-name candidate (Decision 014-D, Option A — fail-closed,
language-agnostic). Evidence gathered from the branch merged as PR #291
(merge commit `8a6c6e32`).

## New Entries Created (mode=apply)

| File | Category | Learning |
|---|---|---|
| `bugs/same-file-duplicate-name-fail-closed-resolution-defect-was-rust-cfg-gated-2026-07-28.md` | bugs | Resolves FF7DE872 fail-closed via additive `find_unique_function_id` at the two direct-edge minting sites. Captures the TDD discrepancy: at ship HEAD the Python two-def case was ALREADY fail-closed (096-F `is_contested`/`module_binding_counts` is Python-scoped), so the live wrong edge was Rust-only via `#[cfg]`-gated duplicate defs (tree-sitter extracts both branches); the plan's inline-`mod` repro is unreachable (extractor has no `mod_item` descent). |

## Entries Reviewed for Overlap (classification: keep + status-update)

| Existing Entry | Classification | Rationale |
|---|---|---|
| `bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md` | keep (status: RESOLVED by 100-F) | The 094-F known-issue that introduced stash FF7DE872. The new 100-F entry is its resolution and `supersedes_status_of` it; the original is retained as the origin/diagnosis record. |
| `workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md` | keep | The hash-skip staleness characteristic that grounds the deferred versioned-backfill follow-up (stash `8DD29746`). Cross-referenced. |
| `workspace-status-code-graph-cfg-gate-false-premise-2026-07-07.md` | keep | Adjacent "cfg encodes a false premise" theme but about `#[cfg(feature=...)]` gating a data read; the new entry is about tree-sitter extracting all `#[cfg]` source branches and a language-scoped runtime guard. Non-duplicative. |

No existing entries were consolidated, replaced, or deleted.

## Evidence Used

- `src/services/code_graph.rs` — `UniqueFunctionId` + `find_unique_function_id`
  (added after the byte-identical `find_function_id`); guarded minting at the
  full-index and incremental-sync sites; `same_file_ambiguous_dropped` counter on
  `IndexResult`/`SyncResult`; `ShadowIndex::is_contested` over
  `module_binding_counts`, fed only by `increment_python_binding` (~L384/401/419),
  consulted at ~L1698/2616.
- `src/services/parsing/rust.rs` — top-level dispatch matches `function_item` /
  `struct_item` / … with **no `mod_item`** case (inline-mod repro unreachable).
- `tests/integration/same_file_shadowing_acceptance_test.rs` — 4 tests
  (target-identity across resolution classes, unique-name recall, cross-file
  singleton unchanged, sync-path fail-closed regression).
- `docs/decisions/2026-07-27-ff7de872-...-deliberation.md` +
  `docs/exec-plans/2026-07-27-ff7de872-...-plan.md` (both carry an "Execution
  correction" note); `docs/architecture.md` FF7DE872 subsection.

## Quality Gates + Runtime (evidence)

- fmt PASS; clippy `--all-targets -D warnings -D clippy::pedantic` PASS;
  `cargo dev-test` 461 lib tests PASS + affected integration suites PASS
  (incl. `calls_recall_acceptance` 18 — no recall loss); `cargo audit` = 10
  pre-existing transitive advisories (Cargo.lock byte-identical to base; 0 deps
  added).
- Copilot merge gate FULLY GREEN across 2 review cycles (7 threads resolved);
  merge-commit only (P-009): merge `8a6c6e32` has 2 parents (`a3b26705` ∪
  `5d2ed0a8`).
- Runtime (real binary + git workspace + live cozo/JSONL): same-file
  `describe -> plat` fail-closed (no edge); same-file `caller_unique -> helper`
  direct edge (recall).

## Follow-Up Items (stashed, not part of this refresh)

| Stash | Kind / Priority | Item |
|---|---|---|
| `8DD29746` | task / medium | Versioned code-graph revalidation / stale-direct-edge backfill (Copilot thread PRRT_kwDORJEduc6UUYBN; mirror 096-F opt-in backfill). |
| `B94772CB` | feature / low | Deferred Python-only last-wins recall recovery (v1 non-goal). |
| `F97D51DF` | task / low | Remediate pre-existing `cargo audit` transitive-dep advisories. |
| `5765BAAB` | bug / medium | Daemon `engram index` cross-file singleton not persisted + CLI response hang (`cli/direct.rs:162` index-vs-sync routing). |

Item #5 (Stage reconciliation of planning-artifact wording) was assessed **moot**:
the execution-correction notes landed on `main` in the authoritative deliberation,
plan, and `architecture.md`; the only residue is an archived transient backlog
task file.
