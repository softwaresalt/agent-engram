---
title: Canonical dev-test coverage oracle with bounded execution
date: 2026-08-21
type: implementation-plan
status: reviewed
source_stash_id: C2413934
source: docs/decisions/2026-08-21-c2413934-dev-test-coverage-oracle-investigation.md
agent: stage
---

## Problem Frame

`.cargo/config.toml` defines `dev-test` as a hardcoded allowlist of `--lib` plus
six HCL targets. `Cargo.toml` declares 208 `[[test]]` targets, so 202 non-HCL
contract and integration targets never run under the gate the constitution
mandates. `full-test` and `ci` run all 208 as separate processes, which is the
process-explosion cost the narrowing was avoiding. No mechanism detects the
omission, so the failure is silent.

## Requirements Trace

| Requirement (stash C2413934) | Implementation action |
|---|---|
| Changed non-HCL contract and integration targets cannot be omitted | U3 oracle fails when a required target is not in the selected set; U5 makes `dev-test` change-scoped. |
| Account for process-explosion constraints | U4 bounded execution with an explicit concurrent-binary cap and thread limits. |
| Do not change completed Shipment 117-S scope | Freeze-scope: no edits under `tests/**/hcl_*`, no change to HCL target definitions; asserted by U2. |
| Measurable canonical coverage oracle | U1/U3 report required, selected, and omitted counts; pass condition `omitted == 0`. |
| Not blindly broadening tests | Selection is diff-derived from a declared manifest, never "run everything". |

## Implementation Units

### U1 — RED: coverage oracle harness

* Changes: harness asserting oracle behaviour — a changed source file whose
  required target is absent from the selected set produces a failure naming the
  omitted target; a changed source file with no manifest mapping fails as an
  unmapped surface; a fully covered diff passes with `omitted == 0`.
* Files: `tests/unit/dev_test_coverage_oracle_test.rs`, `Cargo.toml`.
* Tests: 3 scenarios.
* Posture: test-first (RED).

### U2 — RED: 117-S scope-preservation guard

* Changes: harness asserting the six HCL targets remain present in the required
  set for HCL source changes and that their definitions are unmodified.
* Files: `tests/unit/dev_test_hcl_scope_guard_test.rs`, `Cargo.toml`.
* Tests: 2 scenarios.
* Posture: test-first (RED).

### U3 — GREEN: coverage manifest and oracle

* Changes: checked-in manifest mapping source surfaces to required test targets,
  plus the oracle that computes required / selected / omitted from a diff and
  exits non-zero when `omitted > 0` or an unmapped surface is touched.
* Files: `.cargo/test-coverage-manifest.toml`, `scripts/test-coverage-oracle.ps1`,
  `scripts/test-coverage-oracle.sh`.
* Tests: U1 turns green.
* Posture: paired GREEN for U1. Config-and-scripts domain.

### U4 — GREEN: bounded execution policy

* Changes: explicit concurrency bounds — cap on concurrently running test
  binaries and a `--test-threads` limit — expressed as a documented, tunable
  parameter rather than an emergent property of the target count.
* Files: `.cargo/config.toml`, `scripts/test-coverage-oracle.ps1`,
  `scripts/test-coverage-oracle.sh`.
* Tests: U1 scenario 3 runs under the bound and reports the observed peak.
* Posture: paired GREEN.

### U5 — GREEN: change-scoped dev-test alias

* Changes: redefine `dev-test` to run the oracle-derived required set for the
  current diff under the U4 bounds; retain `full-test` and `ci` unchanged as the
  exhaustive backstop.
* Files: `.cargo/config.toml`.
* Tests: U2 turns green.
* Posture: paired GREEN for U2.

### U6 — GREEN: manifest completeness check

* Changes: a check that every `[[test]]` target in `Cargo.toml` and every source
  module under `src/` appears in the manifest, so the manifest cannot silently
  drift as targets are added.
* Files: `scripts/test-coverage-oracle.ps1`, `scripts/test-coverage-oracle.sh`.
* Tests: U1 unmapped-surface scenario extended to target additions.
* Posture: paired GREEN.

### U7 — Docs: contributor and CI guidance

* Changes: document the manifest format, how to add a mapping when adding a
  target, the meaning of the required/selected/omitted report, and the
  relationship between `dev-test`, `full-test`, and `ci`.
* Files: `docs/workflows.md`.
* Posture: docs-only.

### U8 — Runtime verification and closure

* Changes: measured evidence — omitted-target count before (202 for a
  representative non-HCL diff) and after (0), plus observed peak concurrent test
  binaries and wall-clock under the bound; closure record.
* Files: `docs/closure/2026-08-21-c2413934-runtime-verification.md`.
* Posture: verification-only.

## Dependency Graph

```text
U1 ─> U3 ─> U4 ─> U5 ─> U7 ─> U8
U2 ───────────────^
U3 ─> U6 ─────────^
```

* U1 and U2 are parallel RED units.
* U3 depends on U1. U4 depends on U3. U6 depends on U3.
* U5 depends on U2, U4, and U6. U7 depends on U5. U8 depends on U7.

## Decisions and Rationale

1. **Oracle before selection.** Fixing the alias without an oracle would produce
   the same stale-allowlist failure the moment a target is added. The oracle is
   the durable artifact; the alias is downstream of it.
2. **Manifest over inference.** Inferring test-to-source coverage from build
   graphs or coverage instrumentation is expensive and fragile. A reviewable
   declared manifest with a completeness check is auditable and cheap.
3. **Fail on unmapped surfaces.** A default of "unmapped means uncovered means
   pass" would recreate silent omission. Unmapped is a failure.
4. **Concurrency as an explicit parameter.** The process budget becomes a tunable
   number instead of an accident of which targets happen to be listed.
5. **117-S untouched.** HCL targets keep their exact definitions; U2 guards this
   mechanically rather than by convention.
6. **`ci` retained unchanged.** The exhaustive path stays as the backstop, so the
   oracle reduces risk without becoming a single point of failure.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Manifest becomes stale as targets are added | U6 completeness check fails on any unmapped target or module. |
| Diff-derived selection misses transitively affected targets | Manifest mappings are surface-to-target, not file-to-file; U7 documents the requirement to map a surface broadly rather than narrowly. |
| Bounded execution makes `dev-test` slow on wide diffs | U8 records wall-clock; the concurrency cap is tunable and documented. |
| Two script implementations (PowerShell and shell) drift | U1 and U6 execute whichever script matches the host, and both are covered by the same scenarios. |
| Perceived scope creep into 117-S | U2 guard plus freeze-scope: no edits under `tests/**/hcl_*`. |
| Oracle itself becomes an unreviewed gate | U7 documents the report format so the numbers are legible to reviewers. |

## Plan Hardening Signals (REQUIRED)

* Public API, schema, or contract change: **present (developer contract)** — the
  meaning of `cargo dev-test` changes for every contributor and for the
  constitutional merge gate.
* Security, auth, permission, or compliance-sensitive behavior: **absent** — no
  runtime security surface is touched.
* Migration, backfill, destructive data/config action, or irreversible step:
  **absent** — configuration and scripts only, fully revertible.
* External integration, operator checkpoint, or external dependency: **absent**.
* High runtime, rollout, or rollback risk: **present (process risk)** — a broken
  gate either blocks all merges or silently permits regressions.

Requires plan hardening: **yes**

## Runtime Verification and Closure

| Unit | Surface | Verification | Closure artifact |
|---|---|---|---|
| U3, U5 | Local merge gate | Representative non-HCL diff reports `omitted == 0` and executes the mapped contract/integration targets | U8 closure record |
| U4 | Developer machine process budget | Observed peak concurrent test binaries stays within the configured cap | U8 closure record |
| U2, U5 | Shipment 117-S scope | HCL targets still selected for HCL changes; definitions byte-identical | U8 closure record |

## Plan Hardening

Triggered by two hardening signals (developer contract change, process risk).

### Protected Invariants

1. For any diff, the set of test targets executed by `dev-test` is a superset of
   the required set derived from the manifest; `omitted == 0` is the pass
   condition.
2. A source surface with no manifest mapping fails the oracle. Silence is never
   interpreted as coverage.
3. Shipment 117-S HCL target definitions are byte-identical before and after.
4. `full-test` and `ci` semantics are unchanged.
5. Peak concurrent test binaries never exceeds the configured cap.

### Risky Actions

| ProposedAction | targets | change_kind | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|---|
| Redefine the `dev-test` alias | `.cargo/config.toml` | config change to the constitutional merge gate | moderate | restore the prior alias line; single-line revert | no | planned |
| Add coverage-oracle scripts | `scripts/` | new local tooling | low | delete scripts, revert alias | no | planned |
| Add coverage manifest | `.cargo/test-coverage-manifest.toml` | new declarative config | low | delete file | no | planned |
| Impose concurrency caps on test execution | `.cargo/config.toml`, `scripts/` | config change affecting developer machines | low | revert cap values | no | planned |

No `high` or `destructive` action. No operator approval gate required.

### Reinforced Verification

* U8 MUST report the omitted-target count for a representative non-HCL diff both
  before the change (expected 202) and after (required 0). A qualitative claim of
  improvement is not acceptable evidence.
* U8 MUST record observed peak concurrent test binaries and total wall-clock, so
  the process-explosion constraint is demonstrated rather than asserted.
* U2 MUST compare HCL target definitions byte-for-byte, not by name.
* U6 MUST fail if a `[[test]]` target exists in `Cargo.toml` with no manifest
  entry, proving the manifest cannot drift silently.
* The oracle MUST be runnable standalone so a reviewer can reproduce the numbers
  without running the suite.

### Monitoring Plan

| SLI | Where observed | Baseline | Alert threshold | Owner |
|---|---|---|---|---|
| Omitted required targets per `dev-test` run | Oracle report | 202 today | any value > 0 | Ship agent, then contributors |
| Unmapped surfaces | U6 completeness check | 0 after U3 | any value > 0 | CI |
| Peak concurrent test binaries | U8 measurement | unbounded today | > configured cap | Ship agent |
| `dev-test` wall-clock on a representative diff | U8 measurement | current six-target time | > 3x baseline | Ship agent |
| `cargo ci` result | CI | green | any regression attributable to this change | CI |

### Pre-Deploy Audit

* Rollback procedure: revert the `.cargo/config.toml` alias line; the manifest and
  scripts become inert. No state to unwind.
* No migration, no schema, no data.
* Backward compatibility: `full-test` and `ci` unchanged, so CI keeps the
  exhaustive backstop while the local gate changes.
* Dependent surface: contributor workflow. U7 must land with U5, not after.

### Post-Deploy Observation Window

Duration: the next three merged release units. Owner: Ship agent for the first,
then operator. Outcome recorded as healthy, degraded, or rolled back.

### Rollback Triggers

1. The oracle blocks a diff whose required targets are genuinely all selected
   (false positive) — revert the alias, keep the scripts, fix forward.
2. `dev-test` wall-clock exceeds 3x the current baseline on a representative diff
   — revert the alias and retune the cap.
3. A regression reaches `main` that the oracle claimed was covered — treat as a
   manifest defect and halt use of the gate until corrected.

## Plan Review

Gate: **PASS**

Personas dispatched: Test Strategy Lens (lead), Architecture Lens, Operational
Readiness Lens, Scope Lens.

### Findings

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| T1 | Test Strategy | P1 | The original framing risked being satisfied by simply adding targets to the alias, which would regress the moment target 209 is added. | Resolved: decision 1 and U6 make the oracle and its completeness check the durable artifact, not the alias contents. Gate-clearing. |
| T2 | Test Strategy | P1 | If unmapped surfaces defaulted to "pass", the oracle would reproduce the exact silent-omission defect it exists to fix. | Resolved: invariant 2 and U1 scenario 2 make unmapped a hard failure. Gate-clearing. |
| A1 | Architecture | P2 | Diff-derived selection cannot see transitive effects; a change to a shared module could require targets not mapped to that file. | Accepted: manifest maps *surfaces* rather than files, and U7 instructs mapping broadly. Residual risk is accepted and backstopped by `cargo ci`. |
| A2 | Architecture | P2 | Two script implementations invite drift. | Accepted into U3/U6 acceptance criteria: both are exercised by the same scenarios on their respective hosts. |
| S1 | Scope | P1 | The stash entry forbids changing Shipment 117-S scope; a plan that only promised this in prose would not be enforceable. | Resolved: U2 is a mechanical byte-identical guard and freeze-scope forbids edits under `tests/**/hcl_*`. Gate-clearing. |
| O1 | Operational Readiness | P2 | A merge gate that misfires blocks all work; no rollback trigger originally covered false positives. | Resolved: rollback trigger 1. |
| O2 | Operational Readiness | P2 | "Measurable" was asserted but the plan did not require before/after numbers. | Resolved in hardening: U8 must report 202 → 0 and peak concurrency. |
| T3 | Test Strategy | P3 | The 208-target structure is itself a design smell worth revisiting. | Advisory; explicitly a non-goal for this release unit. |

No P0 findings. All three P1 findings were resolved during hardening before the
gate decision. Decomposition satisfies the 2-hour rule and width isolation:
U1/U2 test-only, U3–U6 config-and-scripts, U7 docs-only, U8 verification-only.

Review-fix cycles used: 1 of 3.
