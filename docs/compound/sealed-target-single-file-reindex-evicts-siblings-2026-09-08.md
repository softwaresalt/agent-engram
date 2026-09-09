---
title: "Single-file sealed reindex evicted unrelated sibling rows via unconditional deletion-authoritative sweep"
description: "A new sealed single-file indexing entry point silently deleted all other previously-indexed files in the same data_dir/branch because the deletion-reconciliation sweep treated any non-empty file-selection scan as authoritative for the whole branch"
problem_type: "data-loss-regression"
category: "runtime-errors"
component: "src/services/code_graph.rs"
root_cause: "index_workspace_with_file_selection unconditionally set deletion_authoritative = true; the deletion-authoritative sweep reused the narrowed (single-file-filtered) `files` list as the ground truth for 'everything that still exists', so any file not in that one-file list was reconciled as deleted and evicted from the database"
resolution_type: "code_fix"
severity: "high"
message: "silent data loss: previously indexed files disappear from the code graph after a single-file reindex"
file_path: "src/services/code_graph.rs"
citations:
  - "142.015-T"
  - "docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md (plan unit F10)"
tags:
  - "code-graph"
  - "deletion-reconciliation"
  - "sealed-index-target"
  - "review-caught"
---

## Problem

A new `index_sealed_target` entry point was added so the candidate-generation
indexing pipeline could accept only a sealed `IndexTarget` (never a raw path).
For `IndexTarget::LegacyDirect` (a single specific file), the implementation
narrowed the discovered `files` list down to just that one selected file via
`files.retain(|file| file == selected_file)`. Initial tests only exercised
this against a brand-new, empty `data_dir`, so `queries.list_code_files()`
returned nothing and the change looked correct — both new tests passed.

## Root Cause

`index_workspace_with_file_selection` has a downstream deletion-authoritative
reconciliation step that treats the discovered-files list as the full,
authoritative inventory of what should exist for the branch, and evicts
(`handle_deleted_file`) any previously-indexed row not present in it. The
existing code hard-coded `deletion_authoritative = true` regardless of
whether discovery had been narrowed to one file. Because the single selected
file is never hash-skipped, the guard condition
(`force || !any_hash_skipped`) was always true, so the sweep ran on nearly
every legacy-direct single-file call and deleted every other previously
indexed file in that `data_dir`/`branch`.

This is a case where a new, narrowly-scoped indexing entry point silently
inherited full-directory reconciliation semantics from a shared helper
function that was never designed with a "scan of exactly one file" case in
mind.

## Resolution

Scope the deletion-authoritative sweep to the actual discovery breadth:
`let deletion_authoritative = selected_file.is_none();` — full-directory
scans (`Candidate` targets and any other caller with `selected_file: None`)
keep the original full-reconciliation behavior; single-file `LegacyDirect`
scans no longer treat "not the one file I was asked to index" as "this file
was deleted."

A regression test with a *pre-populated* database (simulating prior indexed
siblings) was required to catch this — the original tests only ran against
an empty database and could not have detected the eviction.

## Prevention

When adding a new narrow-scope caller (single file, single directory subset,
etc.) to a shared indexing/reconciliation helper originally designed for
full-workspace scans, explicitly check every downstream use of the
"discovered files" list for reconciliation/deletion semantics — a
narrowed discovery list is not automatically a safe substitute for "the
complete set of files that currently exist." Always add at least one test
case with a non-empty pre-existing database state when testing a new
narrow-scope indexing path; empty-database tests cannot exercise
deletion-reconciliation logic at all.
