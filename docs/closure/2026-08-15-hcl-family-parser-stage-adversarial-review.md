---
title: "HCL family parser Stage adversarial review"
date: "2026-08-15"
type: "stage-adversarial-review"
status: "passed"
plan: "docs/exec-plans/2026-08-15-hcl-family-parser-plan.md"
feature_id: "121-F"
review_id: "121.001-R"
shipment_id: "117-S"
models:
  - "gpt-5.4"
  - "claude-opus-4.6"
  - "gemini-3.1-pro-preview"
---

# HCL Family Parser Stage Adversarial Review

## Scope

Mandatory pre-harvest review covered every design and sequencing decision in the deliberation, compatibility spike, hardened implementation plan, standard multi-persona remediation, extraction contract, external dependency selection, final U1-U16 hierarchy, dependency graph, runtime verification, rollback, and Stage/Ship role isolation. Reviewers were independent and did not see one another's output before consensus assembly. No PR or implementation diff existed.

## Dispatch

| Reviewer | Model | Posture |
|---|---|---|
| A | `gpt-5.4` | Frontier adversarial architecture/security/decomposition |
| B | `claude-opus-4.6` | Independent Rust, graph, and plan-coherence challenge |
| C | `gemini-3.1-pro-preview` | Independent dependency, failure-mode, and operations challenge |

## Consensus Findings

| Finding | Confidence | Severity | Disposition |
|---|---|---|---|
| Dependency-agnostic RED harness claimed an ABI load before the crate existed | HIGH (3/3) | P1 | Fixed. U1-U5 are dependency-agnostic; U6 adds the exact crate; test-only U7 safely loads the grammar then remains RED on missing Engram registration before U8 production code. |
| Structural `Class` names and global reference lookup could cross-bind | MEDIUM (majority) | P1/P2 | Fixed. Symbols are `hcl.block.*`/`hcl.attribute.*`; U11 makes HCL references hint-only file self-loops and bypasses global name resolution. |
| Registry/tag mismatch weakens source provenance | MEDIUM (majority) | P1/P2 | Fixed/accepted exception. Exact crates.io checksum, manifests, binding, owners/license, dependency graph, and published node-types hash are recorded; tag/Git/path substitution is blocked. |
| HCL enablement could precede the reference-persistence guard | LOW (1/3 re-review) | P1 | Fixed despite low confidence. U12 now depends on U11 and U3; U13 remains downstream of U12. |
| `tree-sitter-hcl 1.1.0` allegedly requires tree-sitter 0.20 | LOW (1/3) | P0 | Rejected as factually contradicted by official exact-version dependencies and matching archive: normal `tree-sitter-language 0.1`, dev `tree-sitter 0.25.3`, no 0.20 runtime dependency. U6/U7 still fail closed on deviation. |
| Exact AST vocabulary was allegedly unresolved | LOW (1/3) | P1 | Resolved with safe inspection of exact archive SHA-256 and published
ode-types.json` SHA-256 `d86638c95d20335b960abb62f6758ab53f78fd0efbe4b6669473b5a20dfd1fb5`; runtime behavior remains test-gated. |
| Rollback might deserialize an unknown enum or leave HCL graph state | MEDIUM (2/3) | P2/P3 | Premise narrowed: language is persisted as a string. Added isolated rollback reconciliation rehearsal; destructive DB purge requires operator approval and is not planned. |
| Stable dedup, class serialization, retrieval delegation, repeated sync/restart | LOW (single reviewers) | P2/P3 | Incorporated into U3, U9, U15, or closure. No new framework/API/schema was added. |

## Remediation Consensus

All HIGH-confidence P0/P1 findings were remediated. Every MEDIUM/LOW finding has an explicit fix, acceptance, or evidence-based rejection. The plan was re-reviewed after each material amendment within the three-cycle circuit breaker.

## Final Gate

The final current-plan review returned `VERDICT PASS` from all three models. No P0 or P1 finding remains. Standard multi-persona review also returned PASS at the final authoritative DAG. The plan is approved for backlog harvest, not implementation.

## Safety and Role Result

- Stage created planning/backlog artifacts only; no source, test, config, build, lint, shipment claim, or Ship action occurred.
- The exact dependency may be executed only by Ship after RED and provenance gates.
- Any checksum, ABI, containment, graph, runtime, or full-gate failure blocks delivery and returns through the allowed backlog workflow.
- No planning PR is created because Stage's P-010 role boundary forbids PR creation/push/merge.
