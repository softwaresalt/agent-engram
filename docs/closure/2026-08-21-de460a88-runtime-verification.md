---
title: DE460A88 runtime verification and operational closure — independent agent-visible MCP catalog oracle
date: 2026-08-21
type: closure
status: verified
source_stash_id: DE460A88
shipment: 123-S
feature: 127-F
plan: docs/exec-plans/2026-08-21-de460a88-independent-mcp-catalog-oracle-plan.md
---

## Scope

Runtime verification and operational closure for Feature 127-F (plan unit U8):
an independent, agent-visible MCP `tools/list` catalog oracle validated against
a human-authored declarative fixture, with a mechanically enforced independence
guard. No production code path is modified; the change is test, fixture, script,
and documentation only. The plan records no hardening signal.

## Verified Behavior

### U4 / U6 — oracle green against the current catalog, drift classified

The oracle observes the serialized `tools/list` response an MCP client receives
(subprocess `engram shim` over stdio) and compares it to the fixture. Against
the current 21-tool catalog it reports zero drift.

```text
cargo test --test contract_mcp_catalog_oracle
running 9 tests
test oracle_sources_are_independent_of_production_catalog ... ok
test classify_diffs_reports_each_drift_class_with_the_specific_property ... ok
test compare_schema_shape_flags_malformed_required ... ok
test compare_schema_shape_flags_malformed_properties ... ok
test agent_visible_tool_names_match_fixture_exactly ... ok
test captured_tools_list_is_well_formed_json_with_tools_array ... ok
test agent_visible_tool_descriptions_match_fixture ... ok
test agent_visible_catalog_has_zero_drift ... ok
test agent_visible_tool_schemas_match_fixture_shape ... ok
test result: ok. 9 passed; 0 failed
```

The two `compare_schema_shape_flags_malformed_*` cases were added during review:
the shape comparison now surfaces a malformed observed `required` (non-array or
non-string member) or a malformed `properties` (present non-object) as a schema
difference rather than normalizing it away.

An induced mismatch against the real fixture — a tool rename, a description
edit, and a schema property-type edit applied together, then reverted with
`git checkout` — produced exactly the classified diff, each class naming the
affected tool and the specific differing facet:

```text
agent-visible catalog must exhibit zero drift; classified diffs: [
  SchemaChanged { name: "get_branch_metrics", facet: "property `branch_name` type" },
  DescriptionChanged { name: "get_daemon_status" },
  Added("set_workspace"),
  Removed("set_workspace_RENAMED")
]
```

A rename surfaces as a `Removed` (old name) plus an `Added` (new name), a
reworded description as `DescriptionChanged`, and a property-type change as
`SchemaChanged` naming the property. The catalog-level classification is also
proven at the unit level by the committed, self-contained
`classify_diffs_reports_each_drift_class_with_the_specific_property` test.

### U5 — mechanically enforced independence

The independence guard (`scripts/check-oracle-independence.ps1` and
`scripts/check-oracle-independence.sh`) and the in-test
`oracle_sources_are_independent_of_production_catalog` assertion enforce that
the oracle sources never reach the production catalog derivation path. Both
guard implementations were exercised against throwaway violating trees and the
real repository, and agree:

| Scenario | PowerShell guard | Shell guard |
|---|---|---|
| Forbidden import (`use engram::shim::tools_catalog::all_tools`) | FAIL — FORBIDDEN-IMPORT | FAIL — FORBIDDEN-IMPORT |
| Fixture regeneration (`fs::write("...mcp_tool_catalog.expected.json"...)`) | FAIL — FIXTURE-REGENERATION | FAIL — FIXTURE-REGENERATION |
| Real repository | PASS | PASS |

The human-authored JSON fixture is data, not code: its policy note names the
source contract (`src/shim/tools_catalog.rs`) deliberately, and its independence
is enforced by the fixture-regeneration scan and its header rather than by token
absence. The forbidden-token scan therefore targets the two Rust oracle sources,
matching the plan's "the oracle test and its helpers".

The invariant is enforced in CI, not only locally: the `build` job runs
`scripts/check-oracle-independence.sh` as a dedicated "oracle independence guard"
step (both the forbidden-import and fixture-regeneration scans), and the in-test
`oracle_sources_are_independent_of_production_catalog` assertion enforces the
forbidden-import scan inside `cargo test`.

### U3 — fixture fidelity

The fixture declares all 21 default-build tools (`TOOL_COUNT = 21`), matching the
served catalog with zero drift at merge time (the `agent_visible_catalog_has_zero_drift`
scenario above). The independent oracle detects any future divergence in names,
descriptions, or declared schema shape.

## Quality Gates

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | PASS |
| clippy | `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic` | PASS (0 warnings) |
| oracle test | `cargo test --test contract_mcp_catalog_oracle` | PASS (9/9) |
| independence guard | `scripts/check-oracle-independence.{ps1,sh}` (also a CI step) | PASS |

The clippy and test feature set mirrors CI (`.github/workflows/ci.yml`:
`--no-default-features --features cozo-backend,embeddings`). The `--all-features`
build has a pre-existing, unrelated OpenTelemetry compile break that CI does not
exercise and this shipment does not touch.

## Release Observability

No runtime surface, migration, or rollout is affected: the shipment adds a test,
a fixture, two guard scripts, and documentation. Consistent with the plan's
"no hardening signal", there is no monitoring plan, alert threshold, or rollback
trigger to define.

- Monitoring plan: not applicable — no production runtime signal changes.
- Pre-deploy audit: no feature flag, migration, or dependent-service coordination;
  the only new gate constrains the agent-visible MCP catalog inside the test suite.
- Rollback: the oracle, fixture, and scripts are inert if reverted; there is no
  state to unwind. If an intended catalog change is blocked, update the fixture
  by hand (see `docs/mcp-tool-reference.md`).
- Post-deploy observation window: not applicable — the gate runs in CI, not at runtime.

## Outcome

Verified. The oracle enforces exact agent-visible catalog equality (names,
descriptions, and declared schema shape) independently of the production catalog,
classifies drift precisely, and mechanically guards its own independence.
