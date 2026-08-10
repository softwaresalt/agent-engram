---
title: "RUSTSEC-2026-0041 dependency remediation feasibility spike"
type: implementation-plan
date: 2026-08-10
source: docs/decisions/2026-07-29-cozo-0_8-major-bump-feasibility-deliberation.md
status: reviewed
execution_kind: spike
source_stash_ids: [27F691AE]
source_deliberation: 017-D
---

# RUSTSEC-2026-0041 dependency remediation feasibility spike

## Problem Frame

The locked chain is `engram -> cozo 0.7.6 -> swapvec 0.3.0 -> lz4_flex 0.10.0`. `RUSTSEC-2026-0041` affects lz4_flex block decompression and is patched at 0.11.6/0.12.1. Current evidence rules out a straightforward upgrade: Cozo has no published 0.8+ release and its `main` remains 0.7.6; swapvec 0.4.2 and `main` still require lz4_flex 0.10. Cozo's `^0.3.0` requirement also rejects swapvec 0.4, and swapvec's `^0.10.0` requirement rejects lz4_flex 0.11.6.

Exposure is low but not a fix: Cozo uses default, uncompressed SwapVec temporary spill; swapvec's LZ4 path allocates a fresh output and lz4_flex 0.10 defaults to safe decoding. The advisory nevertheless remains in the distributed graph. The safe next executable unit is therefore a bounded compatibility/supply-chain spike for a 0.3-compatible swapvec patch. This plan does not authorize a production dependency override, fork, vendoring decision, Cozo replacement, or database migration.

## Requirements Trace

- `27F691AE`: U1 reproduces and bounds the advisory; U2 proves or rejects the narrow patch; U3 verifies runtime/data compatibility and records the recommendation.
- `017-D`: current release evidence closes the nonexistent Cozo-major path; U2/U3 answer the remaining patch and compatibility questions without manufacturing an implementation plan.
- Security review: all candidate source, lock, audit, rollback, and disclosure gates are explicit below.

## Implementation Units

### U1 — Establish the immutable advisory and candidate baseline

**Domain/files:** investigation evidence only; create `docs/decisions/2026-08-10-rustsec-2026-0041-remediation-spike-findings.md`. **Cap:** 90 minutes, one durable file, at most three evidence groups.

Before any prototype mutation, record the locked `cargo tree -i lz4_flex`, `cargo audit`, Cozo/swapvec release state, swapvec 0.3.0 crate checksum and MIT license, lz4_flex 0.11.6 checksum/license, and the exact affected call path. Confirm root `Cargo.toml` and `Cargo.lock` remain byte-identical. Reject unpinned branches, mutable tags, missing license/provenance, unsafe backports, and any candidate that broadens Cozo features.

### U2 — Prototype one 0.3-compatible swapvec patch in isolation

**Domain/files:** dependency/prototype only; use a disposable workspace-contained worktree under `tmp/rustsec-2026-0041/`. The root checkout must not change. **Cap:** 110 minutes, one candidate, no production/source refactor.

Harness before candidate change: capture the baseline compile result and locked graph, then make only the minimum prototype change needed for a checksum/revision-pinned swapvec 0.3-compatible package to require `lz4_flex >=0.11.6,<0.12`. Preserve swapvec API and default behavior; do not edit engram Rust source. Prove the resolved graph contains no lz4_flex 0.10.x or 0.12.0, `cargo audit` no longer reports `RUSTSEC-2026-0041`, and `cargo check --all-targets` succeeds. Stop rather than widen scope if this requires a Cozo fork, unsafe-code change, decompression backport, more than one third-party package, or a mutable dependency source. Prototype artifacts are non-shippable evidence and must not enter the release manifest.

### U3 — Verify Cozo runtime/data compatibility and decide

**Domain/files:** focused verification, one prototype-only reopen harness, and the U1 findings artifact only. **Cap:** 110 minutes, no production edits, at most three scenario groups.

First prove direct on-disk compatibility with a prototype-only reopen-in-place harness: the baseline dependency creates and populates a disposable Cozo database, all baseline handles close, and the candidate opens the same files without deletion, dehydration, hydration, migration, or copying. Verify exact graph counts and representative query results. Then run `integration_cozo_cold_restart` only as a separate dehydration/hydration regression, followed by the focused `integration_cozo_crud`, `integration_cozo_edge`, `integration_cozo_symbol_lookup`, and `integration_cozo_vector` targets as one grouped backend gate. Re-run the locked graph and audit. Record Windows results and a Linux/macOS compile disposition; if a non-Windows check is unavailable, the recommendation remains blocked pending hosted matrix proof. Conclude `proceed`, `pivot`, `defer`, or `abandon`, including the exact production patch shape, maintenance owner, dependency source pin, rollback, and whether a separate implementation plan is now justified.

## Dependency Graph

U1 blocks U2; U2 blocks U3. No implementation shipment may depend directly on this plan before U3 records a proceed/pivot recommendation. The policy shipment is ordered after this spike for operator priority, but is technically independent.

## Decisions and Rationale

- Do not plan a Cozo upgrade: no 0.8+ release exists.
- Do not accept a direct lz4_flex override: it violates swapvec's semver requirement.
- Do not remove/replace Cozo: that is disproportionate and introduces schema/query migration risk.
- Do not call runtime containment remediation: current default-uncompressed/safe-decode evidence lowers exposure but leaves the vulnerable package and audit finding.
- Investigate one minimal swapvec-compatible patch because it isolates the dependency change and can be falsified quickly.

## Constitution Check

- Safety-first Rust/unsafe prohibition: no unsafe implementation or backport is permitted.
- Test first: U1 captures the baseline and U2 records the pre-change harness before candidate mutation; U3 validates behavior before any implementation plan.
- Workspace containment: all prototypes stay under `tmp/rustsec-2026-0041/`; no external filesystem writes.
- Single responsibility: one advisory, one transitive package boundary, one candidate.
- Two-hour/width limits: every unit is <=110 minutes and isolated to evidence, dependency prototype, or verification.
- Destructive approval: no deletion, force operation, live-data repair, or history rewrite is authorized.

## Risks and Caveats

A fork or vendored package creates supply-chain ownership; a git source can become unavailable; LZ4 API compatibility does not alone prove on-disk safety; and Cargo may retain multiple lz4 versions. Cozo's swapvec storage is temporary, but existing Cozo SQLite databases must still cold-restart unchanged. The spike must fail closed on provenance, audit, compile, runtime, data, or platform uncertainty.

## Plan Hardening Signals

- Public API, schema, or contract change: absent in the spike; possible in a later implementation.
- Security-sensitive behavior: present; high-severity memory-exposure advisory and third-party source selection.
- Migration/destructive action: absent; live databases and root manifests are read-only.
- External dependency/operator checkpoint: present; candidate provenance and any future fork/vendoring choice need operator approval.
- High runtime/rollback risk: present; the Cozo backend is the durable graph store.

Requires plan hardening: yes

## Runtime Verification and Closure

The spike runs only in disposable workspaces and databases. Healthy evidence is: one pinned candidate, no affected lz4 version in the resolved graph, advisory absent, direct reopen-in-place of the untouched baseline database with exact graph/query results, the separate dehydration/hydration regression green, all focused Cozo targets green, and no root tracked diff. Failure/rollback trigger: advisory remains, duplicate lz4 versions, compile/API break, direct-reopen or hydration mismatch, graph result drift, unpinned source, missing license, or any required scope widening. Rollback is to discard the isolated worktree; no live data or root manifest is changed. Closure is the findings artifact plus backlog comments, with operator ownership of any later fork and a seven-day post-implementation audit/runtime observation window if a fix is eventually planned.

## Plan Hardening

Hardening is required because the apparent small dependency change crosses a security and supply-chain boundary while sitting below the durable Cozo store.

- **Protected invariants:** `#![forbid(unsafe_code)]`; exact Cozo feature set; no production source change; no operator database access; byte-identical cold restart; one lz4 version; immutable source pin and license; native audit exit preserved.
- **Reinforcing evidence:** RustSec advisory text; 102-F advisory triage; 111-S audit closure; Cozo/swapvec crates.io metadata and current manifests; Cozo default SwapVec use; `cozo_cold_restart_test`; strict-safety and release-observability instructions.
- **ProposedAction:** execute a third-party dependency prototype in a disposable workspace-contained worktree. **ActionRisk:** high. **approval_required:** yes; the operator explicitly requested a bounded security spike. **rollback:** discard or retain the disposable worktree under operator safeguards; root checkout remains untouched. **ActionResult:** planned.
- **ProposedAction:** select a forked or vendored dependency for production. **ActionRisk:** high. **approval_required:** yes, after spike findings. **rollback:** reviewed manifest/lock revert. **ActionResult:** blocked pending U3; not authorized by this shipment.
- **Monitoring/rollback:** record graph and audit deltas, candidate checksum/revision, platform matrix, cold-restart outcome, and source availability. Any failed invariant blocks promotion; no waiver converts this spike into implementation.

## Plan Review

**Gate: PASS for spike harvest only.** Hardening requirement is satisfied. Constitution, Rust, scope-boundary, learnings, architecture, supply-chain, and security-lens personas reviewed all units. Cross-model dispatch was unavailable; independent persona passes were consolidated locally.

- **P0:** 0.
- **P1:** 0 remaining. The prior ungrounded Cozo-major assumption was removed; the plan now forbids implementation until compatibility/provenance evidence exists.
- **P2:** 0.
- **P3:** 0.

The security lens confirms fail-closed source pinning, audit/lock proof, disposable data, cross-platform disposition, and explicit operator ownership. This PASS does not approve a dependency fix; it approves only the three-unit time-boxed investigation for harvest.