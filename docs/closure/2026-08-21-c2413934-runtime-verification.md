---
title: C2413934 runtime verification and operational closure — canonical dev-test coverage oracle
date: 2026-08-21
type: closure
status: verified
source_stash_id: C2413934
shipment: 122-S
feature: 126-F
plan: docs/exec-plans/2026-08-21-c2413934-dev-test-coverage-oracle-plan.md
---

## Scope

Runtime verification and operational closure for Feature 126-F (plan unit U8):
the canonical `cargo dev-test` coverage oracle with bounded execution. Verifies
the protected invariants from plan hardening with measured evidence rather than
qualitative claims.

## Measured Evidence

### Omitted required targets (measurable canonical coverage)

Representative non-HCL diff: `src/db/workspace.rs`. Total declared `[[test]]`
targets: 214 (208 non-HCL + 6 Shipment 117-S HCL). Required target set for the
diff: 208.

| Gate | Selected | Required | Omitted | Status |
|---|---|---|---|---|
| Before — legacy `dev-test` (six HCL targets) | 6 | 208 | **208** | FAIL |
| After — change-scoped `dev-test` (selected = required) | 208 | 208 | **0** | PASS |

The legacy gate silently omitted every one of the 208 non-HCL contract,
integration, and unit targets for this diff. The change-scoped gate omits none.
Reproduce standalone without running the suite:

```bash
scripts/test-coverage-oracle.sh --mode report --changed src/db/workspace.rs \
  --selected hcl_parser_contract_test,hcl_grammar_abi_test,hcl_parsing_test,hcl_routing_test,hcl_security_test,hcl_indexing_test
scripts/test-coverage-oracle.sh --mode report --changed src/db/workspace.rs
```

### Bounded execution (process-explosion constraint)

Configured cap: `max_concurrent_test_binaries = 8`, `test_threads = 4`
(`.cargo/test-coverage-manifest.toml [settings]`, overridable via
`ENGRAM_DEVTEST_MAX_BINARIES` / `ENGRAM_DEVTEST_TEST_THREADS` in
`.cargo/config.toml [env]`).

| Diff | Required | Planned peak (dry-run) | Observed peak (real run) | Failed | Wall-clock |
|---|---|---|---|---|---|
| `src/db/workspace.rs` | 208 | 8 (= cap) | — | — | — |
| `scripts/test-coverage-oracle.ps1` | 2 | 2 | **2** (≤ cap 8) | 0 | 114.4 s |

The runner never launches more than `cap` test binaries concurrently: the peak
is `min(required, cap)` by construction of the job pool. For the wide 208-target
diff the planned peak equals the cap (8); for a narrow diff the change-scoped
selection collapses the run to the two affected targets. The exhaustive
`cargo ci` remains the transitive backstop.

### Shipment 117-S scope preservation

The six HCL `[[test]]` definitions are byte-identical before and after
(asserted mechanically by `tests/unit/dev_test_hcl_scope_guard_test.rs`, not by
name match). An HCL source change (`src/services/parsing/hcl.rs`) keeps all six
HCL targets in the required set. `full-test` and `ci` aliases are unchanged.

### Manifest completeness

`--mode completeness`: 214 targets, 13 top-level `src/` modules, 0 unmapped
targets, 0 unmapped modules. The manifest cannot drift silently as targets are
added.

## Protected Invariant Verification

| Invariant | Result |
|---|---|
| `dev-test` executes a superset of the required set; `omitted == 0` | PASS (0 omitted for the representative diff) |
| Unmapped source surface fails the oracle | PASS (`src/zzz_unmapped_surface/thing.rs` → STATUS=FAIL) |
| 117-S HCL target definitions byte-identical | PASS (guard green) |
| `full-test` and `ci` semantics unchanged | PASS (aliases untouched) |
| Peak concurrent test binaries never exceeds cap | PASS (observed 2 ≤ 8; planned 8 = cap) |

## Monitoring Plan

| SLI | Where observed | Baseline | Alert threshold | Owner |
|---|---|---|---|---|
| Omitted required targets per `dev-test` run | Oracle `report` output | 208 (legacy) → 0 | any value > 0 | Ship agent, then contributors |
| Unmapped surfaces / targets | `--mode completeness` | 0 | any value > 0 | CI |
| Peak concurrent test binaries | `run` output `OBSERVED_PEAK_CONCURRENT` | ≤ 8 | > configured cap | Ship agent |
| `dev-test` wall-clock on a representative diff | `run` timing | narrow diffs seconds; wide diffs bounded | > 3x baseline | Ship agent |
| `cargo ci` result | CI | green | any regression attributable to this change | CI |

## Pre-Deploy Audit

- Rollback procedure: restore the prior `dev-test` alias line in
  `.cargo/config.toml`; the manifest and scripts become inert. No state to
  unwind, no migration, no schema, no data.
- Backward compatibility: `full-test` and `ci` unchanged, so CI keeps the
  exhaustive backstop while the local gate changes.
- Dependent surface: contributor workflow. `docs/workflows.md` guidance landed
  with the alias change (U7), not after.
- Feature flags / rollout gates: not applicable (local developer tooling).

## Post-Deploy Observation Window

Duration: the next three merged release units. Owner: Ship agent for the first,
then operator. At window close, record the outcome as healthy, degraded, or
rolled back.

## Rollback Triggers

1. The oracle blocks a diff whose required targets are genuinely all selected
   (false positive) — revert the alias, keep the scripts, fix forward.
2. `dev-test` wall-clock exceeds 3x the six-target baseline on a representative
   diff — revert the alias and retune the cap.
3. A regression reaches `main` that the oracle claimed was covered — treat as a
   manifest defect and halt use of the gate until corrected.

## Operational Notes

`cargo dev-test` delegates to the `cargo-devtest` external subcommand, which
requires `scripts/` on `PATH`. Contributors without that setup invoke the
runner directly (`bash scripts/test-coverage-oracle.sh --mode run` /
`pwsh scripts/test-coverage-oracle.ps1 --mode run`); `cargo ci` and
`cargo full-test` remain PATH-free backstops. This one-time setup is documented
in `docs/workflows.md`.
