---
title: "Stage next: advisory spike and diagnostic policy batch"
doc_type: memory
date: 2026-08-10
agent: stage
status: staged
operator_batch: dark-factory-2026-08-10
shipments: [115-S, 116-S]
---

## Session Scope

Performed Stage work only for active stash `27F691AE`, `241B503F`, and deliberation `017-D`: metadata refresh, evidence-grounded triage, deliberation resolution, implementation/spike planning, risk hardening, plan review, harvest, dependency wiring, and queued shipment assembly. No source/test/config implementation, build, test, lint, branch, PR, merge, claim, or Ship action occurred.

## Decisions

- Security/reliability is first. `27F691AE` and `017-D` are one decision stream.
- Current upstream evidence rules out the assumed Cozo upgrade: crates.io and Cozo `main` remain at 0.7.6. Latest/current swapvec remains on lz4_flex 0.10, and Cargo cannot satisfy swapvec `^0.10.0` with fixed lz4_flex 0.11.6.
- Cozo uses default SwapVec temporary collection with compression `None`; lz4_flex 0.10 defaults to safe decoding and swapvec's LZ4 path allocates a fresh vector. This lowers practical exposure but does not remove the distributed vulnerable crate or audit finding.
- Removal/replacement is disproportionate and direct override is invalid. The only safe executable next step is a bounded, hardened, security-reviewed spike for one immutable-pinned swapvec 0.3-compatible patch. No production remediation plan was manufactured.
- The circuit-breaker policy unit remains separate. Its reviewed plan requires a RED repository-content contract before the authoritative instruction edit and preserves the universal three-failure threshold.

## Intake Accounting

- `27F691AE` (critical): harvested with provenance into `119.001-T`; feature `119-F`; review `119.001-R`; shipment `115-S`.
- `241B503F` (high): harvested with provenance into `120.001-T`; feature `120-F`; review `120.001-R`; shipment `116-S`.
- `017-D`: updated from queued/deferred to `accepted`/critical, chosen direction replaced with the current spike decision, linked `informs -> 119-F`, and durable decision artifact amended.
- Active stash after harvest: empty.

## Reviewed Plans and Hierarchies

### 115-S — RUSTSEC-2026-0041 feasibility spike

Plan: `docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md`
Review: `119.001-R` PASS for spike only; hardened security/supply-chain lens.

- `119-F`
- `119.001-T` immutable graph/audit/provenance baseline (critical, <=90m)
- `119.002-T` one isolated 0.3-compatible swapvec prototype (critical, <=110m), blocked by `119.001-T`
- `119.003-T` Cozo cold-restart/runtime/platform decision (critical, <=110m), blocked by `119.002-T`

### 116-S — Circuit-breaker diagnostic policy

Plan: `docs/exec-plans/2026-08-10-circuit-breaker-diagnostic-escalation-plan.md`
Review: `120.001-R` PASS; hardened prompt/instruction-authoring lens.

- `120-F`
- `120.002-T` RED policy contract in existing `contract_verify` target (high, <=100m)
- `120.001-T` authoritative instruction edit (high, <=100m), blocked by `120.002-T`

## Ordered Batch

- `115-S`: queued, priority critical, `operator_batch=dark-factory-2026-08-10`, `operator_order=1`, `operator_predecessors=[]`.
- `116-S`: queued, priority high, `operator_batch=dark-factory-2026-08-10`, `operator_order=2`, `operator_predecessors=[115-S]`; shipment dependency `116-S` blocked by `115-S`.

Both manifests are parent-first, reviewed, and never claimed.

## Evidence and Validation

Consulted the authoritative backlog metadata/WIT/templates, stash, 017-D, Cargo manifest/lock/history, RustSec advisory, current crates.io/GitHub Cozo/swapvec manifests, Cozo/swapvec call paths, dynamic-diagnostics learning, circuit-breaker/constitution/strict-safety/release-observability instructions, 102-F advisory decision, and 111-S audit/closure evidence. Backlog doctor returned no findings. Direct shipment custom fields were required because neither MCP nor CLI exposes shipment custom-field mutation; the index was synced immediately afterward.

Engram daemon was reachable, but its status initially reported a stale prior branch while Git was on `main`. Indexed searches were supplemented with targeted local reads. A rebind was refused by the one-workspace limit and a refresh ended with `database is locked`; no retries or source mutations followed.

## Files Modified

- `docs/decisions/2026-07-29-cozo-0_8-major-bump-feasibility-deliberation.md`
- `docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md`
- `docs/exec-plans/2026-08-10-circuit-breaker-diagnostic-escalation-plan.md`
- this memory file
- backlogit-managed queue, stash/archive, link, event, memory, and shipment artifacts under `.backlogit/`

No production, test, or configuration file was modified.

## Continuity / Next Steps

1. Ship claims `115-S` only and executes the three-unit spike. Stop on any scope widening; return findings to Stage for a separate implementation plan if and only if U3 concludes proceed/pivot.
2. After `115-S` is terminal, Ship may claim `116-S`, observe the RED contract, then edit the authoritative instruction and run prompt/quality verification.
3. Compact-context assessment found 109 memory files (481.8 KiB), including 62 older than 14 days. Compaction would move/archive files and is blocked pending destructive-operation approval; no archive move was attempted.