---
title: "Spark lineage and parser correctness"
type: implementation-plan
date: 2026-08-07
source: docs/decisions/2026-08-07-dark-factory-active-stash-triage-decision.md
status: reviewed
source_stash_ids: [98CF66D5, 95885F3D, 21A4D1DE]
---

# Spark lineage and parser correctness

## Problem Frame

The Spark lineage pipeline has two correctness defects and two review nits. Python read/write reuse can suppress valid second-session lineage, and session invalidation is conflated with variable reread poison. The SQL INSERT normalizer can rewrite text still inside nested block comments or a backslash-continued line comment. resolve_table retains unreachable checks, and freshness rollout lacks a same-version skip control.

## Requirements Trace

- 95885F3D maps to U1.
- 98CF66D5 maps to U2.
- 21A4D1DE maps to U3 and U4 for code/test width isolation.

## Implementation Units

### U1 — Spark SQL comment-boundary precision

Files: src/services/parsing/sql.rs only. RED cases prove no rewrite inside a nested block comment and no rewrite after backslash-LF continuation; CRLF still terminates according to Spark behavior. Implement nested depth and line continuation without changing quoted-region handling. Cap: three scenarios, one file, 100 minutes.

### U2 — Python read/write reuse state

Files: src/services/parsing/python.rs only. Introduce a dedicated reread-invalidated set and written-since-bind state. RED cases cover read-write-read-write reuse, non-Spark invalidation followed by a valid Spark read, and the existing single-read multi-write fan-out control. Preserve fail-closed nested/unresolved behavior. Cap: three scenarios, one file, 110 minutes.

### U3 — Remove unreachable table-name guards

Files: src/models/lineage.rs only. Remove or accurately document name.contains(::) and name.starts_with(:) checks that are unreachable after component grammar validation; retain authority delimiter guards. Update focused unit expectations only. Cap: two scenarios, one file, 60 minutes.

### U4 — Freshness same-version skip control

Files: tests/integration/lineage_extractor_version_rollout_test.rs only. Between initial index and rollback, run a current-stamp unchanged control and assert ingested equals zero and the persisted graph is unchanged. No production modification. Cap: two scenarios, one file, 60 minutes.

## Dependency Graph

U1 and U2 are independent parser fixes. U1 and U2 block U3 and U4 only as shipment sequencing to land correctness before cleanup; U3 and U4 are independent.

## Decisions and Rationale

Model reread ambiguity separately from invalidated Spark session receivers. A write marks a binding reusable by a later read but does not consume the binding, preserving multi-write fan-out. Match Spark comment grammar narrowly; do not introduce a general SQL lexer. Keep code cleanup and rollout test in separate tasks.

## Risks and Caveats

Lineage is persisted, so corrected extraction affects new or reprocessed notebooks while old false edges may remain until an operator-controlled reindex. The precision floor forbids guessing after unresolved or nested events.

## Plan Hardening Signals

- Public API, schema, or wire change: absent.
- Security or permission behavior: absent.
- Migration or destructive action: absent in code, but persisted derived data may need reprocessing.
- External checkpoint: present for any released-workspace reindex.
- High runtime or rollback risk: present for lineage precision and recall.

Requires plan hardening: yes

## Runtime Verification and Closure

Use exact edge-set assertions in disposable notebooks and SQL fixtures. Verify no false-positive comment rewrite, both valid Python edges, existing fan-out, and current-stamp skip. Ship must not reindex an operator workspace; closure provides a target-specific operator handoff if exposure exists. Monitor lineage edge deltas and parser error counts for seven days. Rollback is code revert plus operator-directed reindex only if the operator chooses.

## Plan Hardening

Hardening is required because derived lineage is durable and both precision and recall are user-visible.

ProposedAction: change Python candidate state transitions for reread after write and after session invalidation.  ActionRisk: moderate.  Approval required: no additional approval.  ActionResult: planned.

ProposedAction: change Spark SQL comment skipping so nested and continued comments remain opaque.  ActionRisk: moderate.  Approval required: no additional approval.  ActionResult: planned.

Protected invariants: unresolved input emits no edge; one read may feed multiple writes; nested scopes remain excluded; quoted regions are unchanged; no automatic operator-workspace reindex. Rollback trigger: any new false edge or loss of established control recall.

## Plan Review

Gate: PASS. Hardening requirement satisfied. Constitution, Rust, scope-boundary, learnings, architecture, and agent-parity personas reviewed all units; security was not triggered. Findings: P0 0, P1 0, P2 0, P3 0. Exact controls and reindex ownership close the rollout gap. Ready for harvest.
