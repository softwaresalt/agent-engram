---
policy_id: "P-DYN-001"
policy_type: "workspace-specific"
status: "proposed"
proposed_at: "2026-10-24T19:48:00Z"
evidence_count: 8
---

# P-DYN-001: Code-Graph Extraction & Single-File Reindex Integrity Gate

## Status: Proposed

This policy was auto-proposed by `auto-tune` based on recurring patterns in the compound
learning library. Review and decide: accept (copy to `.github/policies/` or append to
`workflow-policies.md`) or reject (delete this file with a note in the tuning report).

## Policy Definition

| Field | Value |
|---|---|
| **APPLIES_TO** | `ship`, `build-feature`, `rust-engineer`, `review` |
| **GATE_POINT** | Pre-review and post-implementation verification whenever `src/services/code_graph.rs` or tree-sitter extraction logic is modified |
| **PRECONDITION** | Diff modifies `src/services/code_graph.rs`, symbol/edge extraction, or incremental `sync_workspace` reindexing paths |
| **POSTCONDITION** | Single-file reindex preserves sibling symbols/edges, records file hashes, and passes multi-symbol same-file shadowing and orphan-sweep regression tests |
| **VIOLATION_ACTION** | Halt task completion and add or run targeted integration/unit tests covering same-file duplicate names, sibling preservation on reindex, and file-hash recording before proceeding to PR creation |

## Rationale

The compound learning library has accumulated 8 entries targeting `src/services/code_graph.rs` and the `code-graph` tag across `bugs`, `best-practices`, and `workflow-issues`. Repeated regressions involved single-file reindex evicting sibling records, same-file duplicate symbol shadowing, missing file-hash persistence during `sync_workspace`, and new extraction logic requiring forced reindex or schema-marker-gated revalidation.

## Evidence

Derived from 8 compound learnings sharing the pattern `component_hotspot: src/services/code_graph.rs (8 entries) / cross_cutting_tags: code-graph (8 entries)`.

Evidence references:

* `docs/compound/best-practices/certify-completeness-reconcile-fileset-and-sweep-orphans-2026-07-29.md`
* `docs/compound/best-practices/preflight-fallible-postpass-context-before-mutation-2026-08-02.md`
* `docs/compound/best-practices/versioned-schema-marker-gated-revalidation-backfill-2026-07-28.md`
* `docs/compound/bugs/same-file-duplicate-name-fail-closed-resolution-defect-was-rust-cfg-gated-2026-07-28.md`
* `docs/compound/bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md`
* `docs/compound/sealed-target-single-file-reindex-evicts-siblings-2026-09-08.md`
* `docs/compound/sync-workspace-record-file-hash-required-2026-05-08.md`
* `docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md`

## Acceptance Path

To accept this policy:

1. Review the evidence references and validate the pattern is real and recurring.
2. Copy this file to `.github/policies/` or append a new entry to `.github/policies/workflow-policies.md`.
3. Update any agents or skills that should enforce this policy gate.
4. Run `autoharness verify-workspace` to confirm no dangling cross-references remain.
5. Record acceptance in the tuning report.

To reject: delete this file and note the rejection in the tuning report with a reason.
