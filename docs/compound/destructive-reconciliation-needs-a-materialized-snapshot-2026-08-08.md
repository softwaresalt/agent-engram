---
title: "Destructive reconciliation needs a materialized snapshot, not only a complete directory walk"
description: "A complete traversal can still be unsafe deletion authority when a collected file later fails stat, read, decoding, or parsing; materialize once before destructive work and carry that exact snapshot into reconciliation."
doc_type: learning
source: docs/exec-plans/2026-08-07-fail-closed-source-reconciliation-plan.md
problem_type: "fail-closed reconciliation and TOCTOU"
category: "data-safety"
component: "source traversal and content indexers"
root_cause: "Traversal completeness described directory enumeration only, while deletion depended on whether collected files were successfully materialized and indexed."
resolution_type: "single materialized snapshot"
date: 2026-08-08
shipment: "110-S"
pr: 327
severity: high
---

## Problem

A directory walk may be authoritative about names while still being
non-authoritative about indexable content. A selected alias winner can fail
metadata inspection, size policy, reading, UTF-8 decoding, or parsing. If the
pass remains `complete`, a sweep may remove the previous live alias even though
no replacement was materialized.

Power BI had a second form of the same race: prepasses read TMDL files, deleted
dirty scopes, and then reopened files for the build. A failure or replacement
between those reads could leave the last-known-good scope deleted.

## Resolution

Use one operation-scoped snapshot with two layers:

1. checked traversal establishes bounded file-set authority;
2. materialization establishes that every selected file can participate in the
   index operation.

Any metadata, size, read, decoding, or required parse failure downgrades the
snapshot to non-authoritative. The indexer may retain or update safe rows, but
that snapshot cannot authorize alias-stale deletion. Power BI materializes and
parses all inputs before marker/content/graph deletion, then reuses those
immutable values for schema construction, dirty-scope calculation, and writes.

The physical-state oracle must also distinguish `Present`, `Absent`, and
`Unknown`. An otherwise complete walk does not turn a transient stat or
canonicalization error into evidence of absence.

## Verification Pattern

For every destructive source reconciler, keep three database-backed controls:

- proven physical absence removes exactly one expected path;
- complete alias supersession removes exactly one stale alias; and
- unavailable or incomplete materialization removes zero and retains
  last-known-good content, graph nodes, and completion markers.

Also carry the indexing snapshot into the sweep and mutate the filesystem after
collection. The just-indexed path must survive until the next operation.

## Operational Signal

Monitor removed-count deltas alongside fail-closed warning rate. A warning with
zero removals is the expected degraded behavior. Any removals during an
incomplete pass are a rollback trigger.
