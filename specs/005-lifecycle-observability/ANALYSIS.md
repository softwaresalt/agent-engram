# Adversarial Analysis Report: 005-lifecycle-observability

**Feature**: Lifecycle Observability & Advanced Workflow Enforcement
**Date**: 2026-03-09
**Artifacts analyzed**: spec.md, plan.md, tasks.md, SCENARIOS.md, data-model.md, contracts/mcp-tools.md

## Adversarial Review Summary

| Reviewer | Model | Focus Area | Findings Count |
|----------|-------|------------|----------------|
| A | Claude Opus 4.6 | Logical Consistency | 6 |
| B | GPT-5.3 Codex | Technical Feasibility | 7 |
| C | Gemini 3.1 Pro Preview | Edge Cases and Security | 5 |

All three reviewers agreed that the artifacts are comprehensive and well-structured. The primary areas of concern were: (1) a justified constitution exception needing explicit documentation, (2) query sanitizer robustness, and (3) schema versioning for event snapshots. No critical constitution violations were found — all principles are either fully compliant or have documented justifications.

## Unified Findings Table

| ID | Category | Severity | Location(s) | Summary | Recommendation | Consensus |
|----|----------|----------|-------------|---------|----------------|-----------|
| RC-01 | Constitution | HIGH | plan.md:Constitution Check (Principle VI) | Constitution Principle VI mandates "All state must be serializable to human-readable, Git-mergeable files." The event ledger is excluded from dehydration but the Constitution Check marks Principle VI as PASS without noting the justified exception. | Update the Constitution Check to note the justified exception — mark as "PASS with exception" and document that events are transient operational data per the rationale in research.md. Add to Complexity Tracking. | majority |
| TF-01 | Technical | HIGH | research.md:§Research 4, tasks.md:T073 | Query sanitizer uses a keyword blocklist which can produce false positives when keywords appear inside string literals (e.g., `SELECT * FROM task WHERE description CONTAINS 'DELETE this'`). | Specify that keyword matching must use word-boundary detection and must not match within quoted string literals. Update T073 description to include this requirement. | unanimous |
| TF-02 | Technical | MEDIUM | spec.md:FR-008, plan.md:§New Config | OTLP export over gRPC requires outbound network connection to collector. Constitution network security says "Bind to 127.0.0.1 only." This refers to inbound binding, but outbound gRPC to an external collector should be explicitly acknowledged. | Add a note in plan.md that OTLP export targets outbound-only connections and does not expose a listening port, distinguishing from the constitution's inbound binding restriction. | majority |
| ES-01 | Terminology | MEDIUM | spec.md:US5, plan.md:§Research 5 | "Collection" and "epic" used interchangeably across artifacts. Spec US5 title says "Hierarchical Workflow Groupings" but body mixes "collection", "epic", and "workflow". | Standardize on "collection" as the canonical term throughout all artifacts. Use "epic" only in parenthetical explanation on first reference: "collection (also known as epic)". | majority |
| TF-03 | Technical | MEDIUM | data-model.md:§Event, tasks.md:T055-T060 | Event snapshots store `previous_value` as serialized JSON, but no schema versioning strategy exists for these snapshots. If the schema changes (e.g., new fields on Task), rollback could fail to deserialize old snapshots. | Add a `schema_version` field to the Event entity that records the data model version at time of capture. Rollback logic should validate schema compatibility before applying. | single |
| RC-02 | Coverage | MEDIUM | spec.md:FR-013b, contracts/mcp-tools.md | FR-013b names the config parameter `event_ledger_max_events` but the plan and contracts use `event_ledger_max` and the env var uses `ENGRAM_EVENT_LEDGER_MAX`. Inconsistent naming. | Standardize on `event_ledger_max` across all artifacts (spec, plan, contracts, config). Update FR-013b to use `event_ledger_max`. | unanimous |
| TF-04 | Technical | MEDIUM | plan.md:§Source Code, tasks.md:T002 | Plan lists `src/server/observability.rs` as a new file but it is not registered in `src/server/mod.rs` in the plan's module structure listing. Task T003 says to register in `src/server/mod.rs` but T002 creates the file — ordering dependency is correct but plan structure listing should be explicit. | No fix needed — tasks handle this correctly. Note for implementation: ensure T003 includes `observability` module registration. | single |
| ES-02 | Edge Case | LOW | SCENARIOS.md | No scenario covers the case where `get_event_history` is called with an offset exceeding total events. Should return empty results gracefully. | Add scenario covering pagination beyond available events. | single |
| RC-03 | Coverage | LOW | tasks.md:Phase 9 | No explicit task for updating error code documentation in copilot-instructions.md after adding new error codes. | Add a documentation task in Phase 9 to update the MCP Tools Registry and error codes in .github/copilot-instructions.md. | single |
| ES-03 | Style | LOW | quickstart.md | Example tool calls use placeholder IDs (e.g., "task:impl-id", "task:review-id") that could confuse implementers. Should note these are illustrative. | Add a note at the top of quickstart.md examples that IDs are illustrative. | single |

## Coverage Summary Table

| Requirement Key | Has Task? | Task IDs | Has Scenario? | Scenario IDs | Notes |
|-----------------|-----------|----------|---------------|--------------|-------|
| FR-001 | ✅ | T021, T024 | ✅ | S001-S003 | |
| FR-002 | ✅ | T022, T025 | ✅ | S006-S008 | |
| FR-003 | ✅ | T023, T026 | ✅ | S004 | |
| FR-004 | ✅ | T021 | ✅ | S003 | |
| FR-005 | ✅ | T031 | ✅ | S057 | |
| FR-006 | ✅ | T033 | ✅ | S059 | |
| FR-007 | ✅ | T032 | ✅ | S058 | |
| FR-008 | ✅ | T036 | ✅ | S056 | |
| FR-008a | ✅ | T036 | ⚠️ | — | No explicit scenario for runtime toggle |
| FR-009 | ✅ | T034 | ✅ | S056, S060 | |
| FR-010 | ✅ | T055, T057 | ✅ | S013-S015 | |
| FR-011 | ✅ | T059, T060 | ✅ | S023-S024 | |
| FR-011a | ✅ | T060 | ✅ | S025-S026 | |
| FR-012 | ✅ | T059 | ✅ | S028 | |
| FR-013 | ✅ | T055 | ✅ | S020 | |
| FR-013a | ✅ | T055 | ✅ | S016 | |
| FR-013b | ✅ | T006 | ✅ | S022 | |
| FR-014 | ✅ | T074 | ✅ | S031-S032 | |
| FR-015 | ✅ | T073 | ✅ | S033-S035, S041-S042 | |
| FR-016 | ✅ | T074 | ✅ | S036-S037 | |
| FR-017 | ✅ | T074 | ✅ | S043 | |
| FR-018 | ✅ | T084 | ✅ | S044 | |
| FR-019 | ✅ | T089 | ✅ | S048 | |
| FR-020 | ✅ | T087 | ✅ | S050 | |
| FR-021 | ✅ | T089 | ✅ | S049 | |
| FR-022 | ✅ | T041 | ✅ | S061-S062 | |
| FR-023 | ✅ | T043 | ✅ | S066 | |
| FR-024 | ✅ | T043 | ✅ | S064 | |
| FR-025 | ✅ | T044 | ⚠️ | — | No scenario for template validation |

## Remediation Log

| Finding ID | File | Change Description | Original Text (excerpt) | Applied? |
|------------|------|--------------------|-------------------------|----------|
| RC-01 | plan.md | Updated Constitution Check Principle VI to note justified exception; added Complexity Tracking entry | "✅ PASS \| Event ledger stored in SurrealDB..." | ✅ Applied |
| TF-01 | tasks.md | Updated T073 description to require word-boundary matching and string literal exclusion | "Implement query sanitizer in src/services/gate.rs..." | ✅ Applied |
| RC-02 | spec.md | Updated FR-013b to use `event_ledger_max` instead of `event_ledger_max_events` | "FR-013b: System MUST expose the retention window size as a configuration parameter (`event_ledger_max_events`)" | ✅ Applied |

## Remaining Issues

**Medium (deferred to operator review):**
- TF-02: OTLP outbound connection acknowledgment
- ES-01: Terminology standardization (collection vs epic)
- TF-03: Schema versioning for event snapshots

**Low (suggestions):**
- ES-02: Add pagination edge-case scenario
- RC-03: Add documentation task for copilot-instructions.md
- ES-03: Add illustrative note to quickstart examples

## Constitution Alignment Issues

| Principle | Finding | Resolution |
|-----------|---------|------------|
| VI. Git-Friendly Persistence | Event ledger excluded from dehydration | Justified exception documented — events are transient operational data, not user-editable state. Added to Complexity Tracking. |

## Unmapped Tasks

None — all tasks map to at least one requirement.

## Metrics

**Artifact metrics:**
- Total requirements: 29 (FR-001 through FR-025, plus FR-008a, FR-011a, FR-013a, FR-013b)
- Total tasks: 98
- Total scenarios: 62
- Task coverage: 100% (29/29 requirements have tasks)
- Scenario coverage: 93% (27/29 requirements have scenarios; FR-008a and FR-025 lack explicit scenarios)
- Non-happy-path percentage: 68%

**Finding metrics:**
- Ambiguity count: 2 (ES-01 terminology, TF-02 network scope)
- Duplication count: 1 (RC-02 naming inconsistency)
- Critical issues found: 0
- Critical issues remediated: 0
- High issues found: 2
- High issues remediated: 2

**Adversarial metrics:**
- Total findings pre-deduplication: 18
- Total findings post-synthesis: 10
- Agreement rate: 50% (5/10 findings with majority or unanimous consensus)
- Conflict count: 0

## Next Actions

All critical and high issues have been remediated. The artifacts are ready for operator review of medium-severity findings before proceeding to implementation.

Recommended next step: **Stage 7: Operator Review** — present medium findings via agent-intercom for operator approval.
