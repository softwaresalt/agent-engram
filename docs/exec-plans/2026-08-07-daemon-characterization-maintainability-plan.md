---
title: "Daemon characterization maintainability and evidence accuracy"
type: implementation-plan
date: 2026-08-07
source: docs/decisions/2026-08-07-dark-factory-active-stash-triage-decision.md
status: reviewed
source_stash_ids: [9A4D18E9, FDE88E46]
---

# Daemon characterization maintainability and evidence accuracy

## Problem Frame

The retained 107-S runtime characterization is an oversized function that mixes setup, cleanup, request execution, and evidence parsing. Separately, 015-D durable knowledge overstates singleton non-persistence as corroborated even though the original corpus lacked a known-green control. This low-priority release unit reduces test maintenance risk and corrects authoritative evidence without expanding runtime coverage.

## Requirements Trace

- 9A4D18E9 maps to U1 and U2.
- FDE88E46 maps to U3, U4, and U5 so no task crosses the file-count cap.

## Implementation Units

### U1 — Extract characterization setup and cleanup helpers

Files: tests/integration/calls_postpass_resolution_test.rs only. Extract baseline workspace, owned-daemon identity, frozen-corpus setup, and bounded cleanup into focused helpers. Preserve the ignored status, request IDs, two-run cap, deadlines, and assertions byte-for-byte in behavior. Cap: one file, no new scenario, 100 minutes.

### U2 — Extract characterization evidence parsers

Files: tests/integration/calls_postpass_resolution_test.rs only. After U1, extract response/trace evidence assembly and parser helpers from daemon_index_runtime_boundaries_characterized. Preserve synthetic parser fixtures and live coverage exactly. Cap: one file, no new scenario, 100 minutes.

### U3 — Correct durable decision and Stage memory

Files: docs/decisions/2026-07-29-daemon-index-ipc-hang-spike-findings.md and docs/memory/2026-07-29/stage-cycle4-held-items-memory.md. Change the singleton claim to inconclusive pending known-green corpus validation, preserve the later 107-S non-defect evidence, and name corpus validation as prerequisite. Cap: two files, 60 minutes.

### U4 — Correct deliberation and backlog memory

Files: .backlogit/archive/015-D.md and .backlogit/memories.json. Apply the same evidence classification without altering unrelated history. Preserve the 107-S final classification and links. Cap: two files, 60 minutes.

### U5 — Correct archived stash provenance

Files: .backlogit/archive/stash.jsonl only. Update the historical 5765BAAB record or append an explicit correction event according to backlogit format so the archive no longer claims corroboration without the prerequisite. Validate JSONL structure and sync the index. Cap: one file, 60 minutes.

## Dependency Graph

U1 blocks U2 because both edit one test file. U3 blocks U4 and U5 so the durable decision supplies canonical wording. Test refactoring and documentation correction are otherwise independent.

## Decisions and Rationale

Refactor without widening live coverage, changing ignored status, or introducing a new daemon harness. Correct current authoritative archive locations rather than the stale queue paths named by the intake. Preserve history by correction, not deletion.

## Risks and Caveats

The characterization is timing-sensitive and owned-daemon cleanup must remain bounded. Backlog archive files are authoritative Markdown/JSONL sources; direct edits require backlogit sync and JSON validation.

## Plan Hardening Signals

- Public API, schema, or runtime contract change: absent.
- Security or permission behavior: absent.
- Migration or destructive action: absent.
- External integration: absent.
- High runtime or rollback risk: absent; the live probe remains ignored and behavior-preserving.

Requires plan hardening: no

## Runtime Verification and Closure

Ship compares the existing focused test list and ignored characterization metadata before and after refactor, runs synthetic parser fixtures, validates Markdown/JSON/JSONL, and syncs backlogit. No new live daemon run is required. Rollback is a test/docs revert. Closure records that the claim is inconclusive and that known-green corpus validation gates future persistence work.

## Plan Review

Gate: PASS. Hardening was not required because this is behavior-preserving test and documentation work with no runtime surface expansion. Constitution, Rust, scope-boundary, learnings, architecture, and agent-parity personas reviewed all five units. Findings: P0 0, P1 0, P2 0, P3 0. The file-count split and current archive paths satisfy decomposition integrity. Ready for harvest.
