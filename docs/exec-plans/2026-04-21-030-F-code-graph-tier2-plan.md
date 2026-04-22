---
title: "030-F Shipment 007-S — Code Graph Tier-2 Completion"
description: "Implementation plan for IPC verify, C++ inline, Markdown, SQL spike"
source_document: "docs/decisions/2026-04-21-030-F-code-graph-tier2-deliberation.md"
shipment: "007-S"
covering_feature: "030-F"
requires_plan_hardening: no
plan_review_attempts: 0
---

## Source

This plan operationalizes the deliberation at `docs/decisions/2026-04-21-030-F-code-graph-tier2-deliberation.md`, Option A.

## Primary Objective

Close the Tier-2 code graph language expansion: end-to-end verify the grammars that already landed, fill the C++ inline-member gap, add Markdown coverage, and run a spike to decide on SQL dialect support.

## Implementation Units

### Unit 1 — IPC end-to-end verification (030.001-C)

Verify swift/c/cpp file events trigger indexing and persist symbols via the daemon. Three sibling integration tests (one per language) using the existing daemon fixture pattern.

* **Touched files**: `tests/integration/swift_ipc_indexing_test.rs`, `..._c_..._test.rs`, `..._cpp_..._test.rs` (all new); existing `tests/helpers/` fixtures.
* **Test posture**: integration only — unit-level coverage exists from 005-S.

### Unit 2 — C++ inline member extraction (030.002-C)

Walk into `class_specifier` bodies for `function_definition` nodes; attribute to enclosing class.

* **Touched files**: `src/services/parsing/cpp.rs`, `tests/unit/parsing_test.rs`.

### Unit 3 — Markdown parser (030.003-C)

Add `Language::Markdown` variant; new `markdown.rs` submodule using tree-sitter-md (verify ABI before dep add); extract headings/code blocks/links.

* **Touched files**: `src/services/code_graph/language.rs`, `src/services/parsing/markdown.rs` (new), `Cargo.toml` (dep add), `tests/unit/parsing_test.rs`, `tests/integration/markdown_indexing_test.rs` (new).
* **ABI gate**: if tree-sitter-md is not at 0.23.x or 0.25-compatible, halt and re-deliberate.

### Unit 4 — SQL dialects spike (030.004-C)

Time-boxed (1 day) survey of grammar landscape; produces `docs/decisions/2026-MM-DD-sql-grammar-spike.md` with a recommendation. No code changes in this unit.

## Sequencing

1. Unit 1 first — verifies existing grammar surface is healthy before adding more.
2. Unit 2 — small, isolated.
3. Unit 3 — additive new language.
4. Unit 4 — spike; outcome may add follow-up tasks to stash.

## Rollback Plan

Each unit lives behind a single chore. Reverting any unit is a clean revert of its tasks — no schema changes, no protocol changes. Order of revert: same as land order.

## Self-Review Against Plan-Review Criteria

* Source document referenced: yes.
* Acceptance criteria traceable: yes — each task AC maps to 030-F's AC.
* 2-hour rule: each task scoped to ≤2 files, ≤2 functions, ≤6 test cases.
* Width isolation: yes — each task is single-domain.
* Out-of-scope explicit: Kotlin (030.005-C blocked-upstream); SQL grammar wire-up (deferred pending spike outcome).

## Requires plan hardening

no.
