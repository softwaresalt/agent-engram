---
title: "Python qualified-staging caller attribution decision"
doc_type: decision
source: "stash 42FB7CC5; PR #301 review follow-up to 099.004-T"
date: 2026-07-31
status: selected-for-planning
---

## Provenance

- Date: 2026-07-31
- Source stash: `42FB7CC5` (high-priority bug)
- Origin: PR #301 review follow-up to `099.004-T`
- Prior shipped context: `096-F`, `100-F`, and decisions `013-D`, `014-D`, `016-D`

## Problem

Function-local Python imports are promoted to qualified `python_local` calls before staging. The full-index and incremental-sync staging paths then map the extracted caller name to a function ID with first-match `find_function_id`. If one file contains duplicate top-level caller names, the extractor cannot identify which duplicate owns the call. First-match attribution can therefore stage the trusted canonical target under an arbitrary duplicate caller and later mint a wrong-origin call edge.

The bare-call path already uses `find_unique_function_id` and fails closed when the caller is ambiguous. Qualified staging does not preserve that guard.

## Triage decision

Select this bug as the next release unit by itself. It is the only high-priority stash entry, has a pinned root cause, reuses an existing fail-closed mechanism, and is small enough for a test-first two-task shipment. The medium-priority stash entries concern independent sync topology, daemon lifecycle, Spark lineage, SQL parsing, and PowerBI durability domains; combining any of them would widen blast radius without a dependency benefit.

## Chosen direction

Reuse `find_unique_function_id` at both qualified/provenance staging caller-attribution sites in `src/services/code_graph.rs`:

- exactly one caller match: preserve current staging behavior
- ambiguous caller: do not stage; increment the existing same-file ambiguity counter
- no caller match: preserve the current no-op behavior

Apply the same fail-closed attribution rule symmetrically to full index and incremental sync. Do not implement Python last-wins inference and do not change `find_function_id` globally. This follows `013-D` target-correctness and the `100-F`/`016-D` decision to prefer zero false edges over rare duplicate-name recall.

## Scope

In scope:

- a deterministic target-identity regression harness for duplicate top-level Python callers
- full-index and incremental-sync staging paths
- a unique-caller recall control
- existing ambiguity observability

Out of scope:

- source-order or last-wins caller inference
- schema, staged-call key, extraction-version, CLI, daemon, or persistence-format changes
- unrelated PR #301/#302 follow-ups
- `015-D` and `017-D`

## Planning disposition

No new deliberation or spike is required. The root cause, policy invariant, implementation seam, and rollback are already known. Proceed through implementation planning, plan hardening for runtime call-graph correctness, plan review, and harvest.

## Success criteria

- Duplicate same-name callers cannot produce a staged row or resolved edge attributed to an arbitrary duplicate.
- Unique callers still stage and resolve the same qualified target.
- Full-index and incremental-sync behavior remain symmetric.
- Any ambiguous qualified caller increments the existing ambiguity counter.
- No schema or persisted-format change is introduced.
