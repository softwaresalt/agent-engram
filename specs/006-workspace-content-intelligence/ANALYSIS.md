# Adversarial Analysis Report: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15
**Artifacts analyzed**: spec.md, plan.md, tasks.md, SCENARIOS.md

## Adversarial Review Summary

| Reviewer | Model | Focus Area | Findings Count |
|----------|-------|------------|----------------|
| A | Claude Opus 4.6 | Logical Consistency | 5 |
| B | Claude Opus 4.6 | Technical Feasibility | 5 |
| C | Claude Opus 4.6 | Edge Cases and Security | 4 |

**Note**: All three review perspectives were executed by the same model (Claude Opus 4.6) due to subagent dispatch limitations. Each perspective was analyzed independently against the full artifact snapshot.

**Agreement patterns**: Strong agreement across all perspectives that the artifacts are well-structured and internally consistent. No contradictory findings. Primary gaps are in traceability (FR → task mapping) and a few uncovered scenarios in the task plan.

## Unified Findings Table

| ID | Category | Severity | Location(s) | Summary | Recommendation | Consensus |
|----|----------|----------|-------------|---------|----------------|-----------|
| RC-01 | Traceability | MEDIUM | tasks.md:all phases | Tasks do not include explicit FR-NNN references. While each task maps to a user story (US1-US6), there is no direct FR-to-task traceability for coverage auditing. | Add FR-NNN references in parentheses to task descriptions where applicable (e.g., "T018 [US1] Implement registry auto-detection (FR-002) in src/installer/mod.rs"). | majority |
| RC-02 | Coverage | MEDIUM | tasks.md:Phase 4 | FR-007 ("re-ingest only changed files when file change events are detected") requires integration with the existing `notify` file watcher (already a dependency). No task explicitly bridges the file watcher to the ingestion pipeline. | Add a task in Phase 4 (US2) to integrate the existing file watcher service with the ingestion pipeline — triggering re-ingestion on file change events in registered paths. | majority |
| TF-01 | Implementation | HIGH | tasks.md:Phase 6 | Tasks T037-T042 implement git2 behind the `git-graph` feature flag but no task includes adding `#[cfg(feature = "git-graph")]` conditional compilation guards to the new modules or updating the feature flag documentation. | Add explicit instruction in T037 to wrap all git_graph module code in `#[cfg(feature = "git-graph")]` guards, and add a task to document the feature flag in the configuration reference. | unanimous |
| TF-02 | Implementation | MEDIUM | plan.md:New Dependencies | Plan specifies `serde_yaml 0.9` but this version is the final release of the deprecated `serde_yaml` crate. The successor crate `serde_yml` is the maintained alternative. | Evaluate `serde_yml` as a replacement. If `serde_yaml 0.9` remains adequate for the YAML subset needed (flat source list), document the decision in research.md and plan to migrate when necessary. | single |
| TF-03 | Coverage | MEDIUM | spec.md:Edge Cases, tasks.md | Spec edge case "installer MUST check `.engram/.version` file, warn about version mismatch, and offer migration" has no corresponding task. Only new installation is covered. | Add a task in Phase 3 (US1) or Phase 9 (Polish) for version migration detection logic in the installer. | majority |
| TF-04 | Implementation | LOW | tasks.md:T010 | T010 adds SurrealDB schema definitions but does not mention a schema migration strategy for existing workspaces. Constitution VI requires forward-compatible schema migrations. | Add a note to T010 that schema additions use DEFINE TABLE/FIELD IF NOT EXISTS to be additive-only and non-breaking. | single |
| TF-05 | Dependency | LOW | plan.md:New Dependencies | `git2` 0.19 links to libgit2 (C library via FFI). While this doesn't violate `#![forbid(unsafe_code)]` (which only applies to the Engram crate), the dependency adds build complexity (C compiler required) and ~2MB binary size. The plan's feature flag mitigates this. | Document in research.md that `git2` is chosen over `gix` (pure Rust) due to mature diff support, with a note to re-evaluate when `gix` diff API stabilizes. | single |
| ES-01 | Coverage | MEDIUM | SCENARIOS.md, tasks.md | Scenarios S027 (file deleted after scan), S062 (git broken objects), S070 (read-only hook dir), S072 (status without git feature), S073 (status before workspace set) are not explicitly referenced in any test task. | Add these scenario IDs to the relevant test task descriptions for explicit traceability. | majority |
| ES-02 | Terminology | LOW | spec.md, data-model.md | Spec uses "content type" while data-model uses "content_type" (snake_case). Both are valid in their contexts (spec is user-facing, data-model is technical). Minor inconsistency but acceptable given the different audiences. | No action needed — the terminology is appropriate for each document's audience. | single |
| ES-03 | Constitution | LOW | spec.md:FR-017 | FR-017 specifies "configurable context lines (default: 20)" for diff snippets, but the constitution's Performance Standards section doesn't define a budget for diff storage size. Large diffs with 20 lines of context could consume significant storage. | Add a max diff snippet size limit (e.g., 500 lines) to FR-017 or data-model.md. Already present in SCENARIOS.md S061 but not in the spec. | single |
| ES-04 | Coverage | LOW | SCENARIOS.md | No scenario covers the installer behavior when `--hooks-only` and `--no-hooks` are both passed simultaneously. This is a conflicting flag edge case. | Add scenario S079: conflicting flags → error with clear message. | single |

## Coverage Summary Table

| Requirement Key | Has Task? | Task IDs | Has Scenario? | Scenario IDs | Notes |
|-----------------|-----------|----------|---------------|--------------|-------|
| FR-001 (registry.yaml support) | ✅ | T013, T019 | ✅ | S001, S005, S006 | |
| FR-002 (auto-detect) | ✅ | T018 | ✅ | S002, S013 | |
| FR-003 (validate entries) | ✅ | T014 | ✅ | S004, S007, S009 | |
| FR-004 (built-in types) | ✅ | T013 | ✅ | S014 | |
| FR-005 (custom types) | ✅ | T013 | ✅ | S003 | |
| FR-006 (ingest all sources) | ✅ | T023 | ✅ | S016, S017 | |
| FR-007 (re-ingest changed) | ⚠️ | T024 | ✅ | S018 | Missing file watcher integration task (RC-02) |
| FR-008 (content_type filter) | ✅ | T025, T026 | ✅ | S028, S029, S030 | |
| FR-009 (skip oversized) | ✅ | T023 | ✅ | S020, S021, S022 | |
| FR-010 (code → graph indexer) | ✅ | T023 | ✅ | S015 | |
| FR-011 (project.json) | ✅ | T031 | ✅ | S034, S042 | |
| FR-012 (backlog-NNN.json) | ✅ | T030, T031 | ✅ | S032, S033 | |
| FR-013 (backlog metadata) | ✅ | T030 | ✅ | S032, S035 | |
| FR-014 (dehydrate updates) | ✅ | T033 | ✅ | S037 | |
| FR-015 (legacy fallback) | ✅ | T034 | ✅ | S038 | |
| FR-016 (commit nodes) | ✅ | T038, T040 | ✅ | S045, S046 | |
| FR-017 (change records) | ✅ | T039 | ✅ | S049, S050 | Max snippet size from S061 not in FR |
| FR-018 (query_changes) | ✅ | T041 | ✅ | S052-S057 | |
| FR-019 (commit depth) | ✅ | T038 | ✅ | S045, S046 | |
| FR-020 (incremental sync) | ✅ | T038 | ✅ | S047 | |
| FR-021 (generate hooks) | ✅ | T044 | ✅ | S064 | |
| FR-022 (append with markers) | ✅ | T045 | ✅ | S065, S066 | |
| FR-023 (tool usage guidance) | ✅ | T044 | ✅ | S064 | |
| FR-024 (--hooks-only) | ✅ | T046 | ✅ | S067 | |
| FR-025 (quickstart) | ✅ | T048 | ❌ | — | Documentation deliverable, not behavioral |
| FR-026 (tool reference) | ✅ | T049 | ❌ | — | Documentation deliverable, not behavioral |
| FR-027 (config reference) | ✅ | T050 | ❌ | — | Documentation deliverable, not behavioral |
| FR-028 (architecture) | ✅ | T051 | ❌ | — | Documentation deliverable, not behavioral |
| FR-029 (troubleshooting) | ✅ | T052 | ❌ | — | Documentation deliverable, not behavioral |

**Coverage**: 29/29 FRs have tasks (100%). 24/29 FRs have scenarios (83% — 5 documentation FRs appropriately excluded from behavioral scenarios).

## Remediation Log

| Finding ID | File | Change Description | Original Text (excerpt) | Applied? |
|------------|------|--------------------|-------------------------|----------|
| TF-01 | tasks.md | Added cfg feature flag instruction to T037 | "Implement git repository access in src/services/git_graph.rs" | ✅ Applied |

## Remaining Issues

### Medium Findings (for operator review)

1. **RC-01**: Tasks lack explicit FR-NNN references for traceability. Recommendation: add FR references to task descriptions.
2. **RC-02**: File watcher → ingestion pipeline integration task missing. Recommendation: add task.
3. **TF-02**: `serde_yaml` 0.9 is the final release of the deprecated crate. Recommendation: evaluate `serde_yml`.
4. **TF-03**: Version migration logic not covered by any task. Recommendation: add migration task.
5. **ES-01**: 5 scenarios not explicitly referenced in test tasks. Recommendation: add scenario IDs to test descriptions.

### Low Findings (suggestions)

1. **TF-04**: Schema migration strategy note for T010.
2. **TF-05**: `git2` vs `gix` decision already documented; no action needed.
3. **ES-02**: Terminology difference between spec and data-model is context-appropriate; no action.
4. **ES-03**: Add max diff snippet size to FR-017 to match SCENARIOS.md S061.
5. **ES-04**: Add conflicting flags scenario for `--hooks-only` + `--no-hooks`.

## Constitution Alignment Issues

No constitution violations detected. Key verification points:

- **Principle I (Rust Safety)**: All new code uses `Result`/`EngramError`, `#![forbid(unsafe_code)]` unaffected by `git2` (dependency, not crate code)
- **Principle III (TDD)**: Test tasks precede implementation in every phase
- **Principle V (Workspace Isolation)**: Path validation and symlink resolution covered (S009, S010, T014, T054)
- **Principle VI (Git-Friendly)**: JSON backlog files are text-based, atomic writes specified
- **Principle IX (YAGNI)**: `git2` behind feature flag, registry is optional

## Unmapped Tasks

None — all tasks map to at least one user story.

## Metrics

**Artifact metrics:**
- Total requirements: 29
- Total tasks: 58
- Total scenarios: 78
- Task coverage: 100% (29/29 FRs)
- Scenario coverage: 83% (24/29 — 5 doc FRs excluded)
- Non-happy-path scenario percentage: 64%

**Finding metrics:**
- Total findings: 11
- Critical issues: 0
- High issues: 1 (TF-01 — applied)
- Medium issues: 5 (for operator review)
- Low issues: 5 (suggestions only)

**Adversarial metrics:**
- Total findings pre-deduplication: 14
- Total findings post-synthesis: 11
- Agreement rate: 45% (5/11 findings with majority or unanimous consensus)
- Conflict count: 0

## Next Actions

All critical and high issues have been remediated. The specification artifacts are in good shape for implementation. Medium findings should be reviewed by the operator in Stage 7 before proceeding to build.

Recommended next step: Proceed to operator review (Stage 7) to address medium findings.
