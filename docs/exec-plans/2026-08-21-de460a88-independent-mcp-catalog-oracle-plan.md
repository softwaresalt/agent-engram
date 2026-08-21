---
title: Independent agent-visible MCP catalog oracle
date: 2026-08-21
type: implementation-plan
status: reviewed
source_stash_id: DE460A88
source: docs/decisions/2026-08-21-de460a88-independent-mcp-catalog-oracle-investigation.md
agent: stage
---

## Problem Frame

`tests/contract/tools_catalog_test.rs` derives its expectations from
`engram::shim::tools_catalog::all_tools()` — the artifact under test. Only tool
*names* are independently declared, and only as presence/absence rather than
exact set equality. Descriptions and input object schemas have no oracle at all.
Observation happens against in-process Rust structs, so drift introduced when
`ShimHandler::list_tools` serializes `ListToolsResult` for an MCP client is
invisible. Oracle and subject share a derivation path.

## Requirements Trace

| Requirement (stash DE460A88) | Implementation action |
|---|---|
| Independent of production `tools_catalog` | U3 fixture is human-authored; U5 mechanically asserts the oracle never reaches `all_tools()`. |
| Validates expected tool names | U4 exact set equality, not presence/absence. |
| Validates expected descriptions | U4 per-tool description equality against the fixture. |
| Validates expected object schemas | U4 per-tool schema shape: type, property names, property types, required list, `additionalProperties`. |
| Agent-visible | U2/U4 observe the serialized `tools/list` response, not Rust structs. |
| Drift detection | U6 precise per-tool diff plus documented fixture-update procedure. |
| Without duplicating the same derivation path | U5 independence guard; no build-time or test-time fixture regeneration. |

## Implementation Units

### U1 — RED: oracle harness skeleton

* Changes: contract test that loads the declarative fixture and compares it to a
  captured `tools/list` response; initially fails because neither fixture nor
  capture exists.
* Files: `tests/contract/mcp_catalog_oracle_test.rs`, `Cargo.toml` (`[[test]]`).
* Tests: 2 scenarios — exact name-set equality, per-tool description equality.
* Posture: test-first (RED).

### U2 — RED: agent-visible capture harness

* Changes: helper that obtains the serialized `tools/list` JSON as an MCP client
  receives it, driving the shim's MCP surface rather than reading Rust structs.
* Files: `tests/helpers/mcp_catalog_capture.rs`, `tests/contract/mcp_catalog_oracle_test.rs`.
* Tests: 1 scenario — the capture is well-formed JSON containing a `tools` array.
* Posture: test-first (RED).

### U3 — Fixture: declarative expected catalog

* Changes: human-authored fixture declaring, per tool, the exact name,
  description, and full input object schema, with a header stating that it MUST
  NOT be generated from `all_tools()`.
* Files: `tests/fixtures/mcp_tool_catalog.expected.json`.
* Tests: consumed by U1 and U4.
* Posture: fixture-authoring unit, width-isolated from code.

### U4 — GREEN: schema and description assertions

* Changes: implement the comparison — exact name set, description equality, and
  schema shape (object type, exact property name set, per-property declared type,
  exact required list, `additionalProperties` handling).
* Files: `tests/contract/mcp_catalog_oracle_test.rs`.
* Tests: U1 scenarios plus 2 schema scenarios turn green.
* Posture: paired GREEN for U1.

### U5 — GREEN: independence guard

* Changes: a check asserting the oracle test and its helpers do not import
  `engram::shim::tools_catalog` or reach `all_tools()` transitively, and that no
  build script, test, or CI step regenerates the fixture.
* Files: `tests/contract/mcp_catalog_oracle_test.rs`, `scripts/check-oracle-independence.ps1`,
  `scripts/check-oracle-independence.sh`.
* Tests: 2 scenarios — forbidden import detected, fixture-regeneration detected.
* Posture: paired GREEN.

### U6 — GREEN: drift reporting

* Changes: on mismatch, emit a per-tool diff classified as added / removed /
  description-changed / schema-changed, with the specific differing property.
* Files: `tests/contract/mcp_catalog_oracle_test.rs`.
* Tests: 1 scenario — an induced mismatch produces the classified diff.
* Posture: paired GREEN.

### U7 — Docs: fixture maintenance procedure

* Changes: document when and how to update the fixture, why it must never be
  generated, and how to read a drift report.
* Files: `docs/mcp-tool-reference.md`.
* Posture: docs-only.

### U8 — Runtime verification and closure

* Changes: evidence that the oracle passes against the current catalog, that an
  induced rename / description change / schema change each fail with the correct
  classification, and that the independence guard fails when the forbidden import
  is added; closure record.
* Files: `docs/closure/2026-08-21-de460a88-runtime-verification.md`.
* Posture: verification-only.

## Dependency Graph

```text
U1 ─┐
U2 ─┼─> U3 ─> U4 ─> U6 ─┐
    └────────> U5 ──────┼─> U7 ─> U8
```

* U1 and U2 are parallel RED units.
* U3 depends on U1 and U2. U4 depends on U3. U6 depends on U4.
* U5 depends on U2. U7 depends on U5 and U6. U8 depends on U7.

## Decisions and Rationale

1. **Human-authored fixture over generated snapshot.** A generated snapshot
   inherits any existing catalog defect and reproduces the shared-derivation
   problem the stash entry names.
2. **Serialized observation point.** Agents consume JSON over stdio. Validating
   Rust structs would leave the actual agent-visible contract unverified.
3. **Exact set equality.** Presence/absence lets a new or renamed tool slip
   through; exactness is the only assertion that detects both directions.
4. **Independence asserted, not assumed.** U5 makes the independence property
   mechanically enforced so a later refactor cannot quietly reconnect the paths.
5. **Existing test retained.** `tools_catalog_test.rs` answers a different
   historical question; deleting it would lose the removal regression guard.
6. **Descriptions are contract.** Agents select tools by description, so
   description drift is a behavioral change and is treated as such.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Fixture becomes a maintenance burden as tools evolve | U6 drift reports name exactly what changed; U7 documents the update procedure. Deliberate friction is the point. |
| A contributor regenerates the fixture to make a failure go away | U5 detects regeneration; U7 states the intended workflow. |
| Capturing a real `tools/list` requires MCP session setup in tests | U2 is a dedicated unit; if a full session proves impractical, the fallback is the serialized `ListToolsResult` JSON — still past the serialization boundary, still not `all_tools()`. |
| Schema comparison is over-strict and fails on benign serializer changes | U4 compares declared shape (type, properties, required, `additionalProperties`), not raw JSON bytes or key order. |
| Overlap with `tools_catalog_test.rs` causes confusion | U7 documents the division: historical removal guard vs. independent agent-visible oracle. |

## Plan Hardening Signals (REQUIRED)

* Public API, schema, or contract change: **present (test contract)** — a new
  gate constrains the agent-visible MCP catalog. No production behavior changes.
* Security, auth, permission, or compliance-sensitive behavior: **absent**.
* Migration, backfill, destructive data/config action, or irreversible step:
  **absent**.
* External integration, operator checkpoint, or external dependency: **absent** —
  the MCP client contract is exercised in-repo.
* High runtime, rollout, or rollback risk: **absent** — tests, fixture, scripts,
  and documentation only; no production code path is modified.

Requires plan hardening: **no**

## Runtime Verification and Closure

| Unit | Surface | Verification | Closure artifact |
|---|---|---|---|
| U4, U6 | Agent-visible MCP catalog | Oracle green against the current catalog; induced rename, description change, and schema change each fail with the correct classification | U8 closure record |
| U5 | Oracle independence | Adding the forbidden import fails the guard; fixture regeneration is detected | U8 closure record |
| U3 | Fixture fidelity | Fixture matches the shipped catalog with zero drift at merge time | U8 closure record |

## Plan Review

Gate: **PASS**

Personas dispatched: Test Strategy Lens (lead), Architecture Lens, Scope Lens.

Plan hardening was evaluated and correctly declined: no hardening signal is
present. The work is test, fixture, script, and documentation only, with no
production code path modified and no runtime, security, or migration surface.

### Findings

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| T1 | Test Strategy | P1 | Independence stated as an intention would decay; a future helper could reintroduce the shared derivation path invisibly. | Resolved: U5 makes independence a mechanically enforced assertion. Gate-clearing. |
| T2 | Test Strategy | P1 | Validating in-process structs would leave the actual agent-visible contract (E4) unverified, so the oracle would not meet the stash requirement. | Resolved: U2 establishes the serialized observation point, with a stated fallback that still sits past the serialization boundary. Gate-clearing. |
| T3 | Test Strategy | P2 | Presence/absence name checks let a newly added tool pass unnoticed. | Resolved: U4 requires exact set equality. |
| A1 | Architecture | P2 | Byte- or key-order-sensitive schema comparison would produce false failures on serializer upgrades. | Accepted into U4 acceptance criteria: compare declared shape, not raw bytes. |
| A2 | Architecture | P2 | Two overlapping catalog tests risk future consolidation that destroys independence. | Resolved: U7 documents the division of responsibility explicitly. |
| S1 | Scope | P2 | U3 fixture authoring could drift into editing the production catalog to make the fixture convenient. | Accepted: U3 is fixture-only; any production catalog change is out of scope for this release unit. |
| T4 | Test Strategy | P3 | The oracle could later be extended to tool annotations and output schemas. | Advisory; out of scope. |

No P0 findings. Both P1 findings were resolved before the gate decision.
Decomposition satisfies the 2-hour rule and width isolation: U1/U2/U4/U5/U6 are
test-domain, U3 is fixture-domain, U7 is docs-only, U8 is verification-only.

Review-fix cycles used: 1 of 3.
