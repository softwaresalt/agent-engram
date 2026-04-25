---
type: tuning-report
generated_at: "2026-04-25T02:50:00Z"
workspace: "D:\\Source\\GitHub\\agent-engram"
autoharness_home: "C:\\Users\\derek.williams\\AppData\\Roaming\\uv\\tools\\autoharness\\Lib\\site-packages\\autoharness\\data"
manifest_installed_at: "2025-07-23"
previous_tune: "2026-04-14"
---

# Harness Tuning Report — 2026-04-24

## Drift Summary

| Category | Count | Description |
|----------|-------|-------------|
| P1 — Degrading | 6 | Content drift in fix-ci, workflow-policies, constitution, git-merge, stage, ship |
| P2 — Growth | 4 | Manifest tracking gaps, stack-pack alignment, profile staleness |
| P3 — Cosmetic | 1 | Stale deprecated agent references in manifest |

## Composition

| Aspect | Previous | Current |
|--------|----------|---------|
| Preset | full | full |
| Primary stack pack | mcp-server | rust-mcp-daemon |
| Stack packs | mcp-server, cli-tool | rust-mcp-daemon, rust-surrealdb-embedded, rust-tree-sitter, github-actions-ci |
| Install layers | (unchanged) | foundation, instructions, workflow, review, runtime, backlog, knowledge, overlays |

## Profile Changes Since Last Tune

| Metric | Previous | Current | Delta |
|--------|----------|---------|-------|
| Source files (src/*.rs) | 68 | 87 | +28% |
| Test files | 82 | 102 | +24% |
| tree-sitter version | 0.24 | 0.25 | ABI upgrade |
| New dependency | — | cozo 0.7 | CozoDB backend (migration target) |
| New feature flags | — | surreal-backend, cozo-backend | Database backend selection |
| Language parsers | 1 (Rust) | 11 (Rust, Python, JS, TS, Go, C#, C, C++, Swift, Kotlin, Markdown) | +10 languages |
| New docs directory | — | docs/upstream/ | Upstream tool issue docs |

## Checksum Scan

| Status | Count |
|--------|-------|
| Present (all artifacts) | 78 |
| Missing | 0 |
| User-modified | 6 (regenerated in this tune) |
| Ignored | 0 |

## Healthy Checks (No Action Needed)

- ✅ All manifest artifacts exist in workspace
- ✅ Plan-hardening pipeline coherent (impl-plan → stage → plan-harden → plan-review)
- ✅ Gitignore has `.*.lock` / `**/.*.lock` without blocking `Cargo.lock`
- ✅ All 6 scripts present (acquire/release lock × 2, search × 2)
- ✅ Continuous-learning directory structure present (observations/, instincts/, learned/)
- ✅ Circuit-breaker and concurrency instructions installed and referenced
- ✅ Agent-intercom weaving coherent across agents, instructions, and constitution
- ✅ Agent-engram weaving coherent across agents, instructions, and constitution
- ✅ Backlogit weaving coherent across agents, instructions, and constitution
- ✅ Strict-safety weaving coherent across plan-harden, review, and closure
- ✅ Release-observability weaving coherent across operational-closure and runtime-verification
- ✅ Browser-verification not needed (browser_tooling: false in profile)
- ✅ Agent-native-parity-reviewer installed
- ✅ Backlog tool unchanged (backlogit) — no migration needed
- ✅ Config suffix_map has all 9 entries — no backfill needed

## Approved Local Overlays (Do Not Regenerate)

These files are intentionally larger than their templates due to workspace-specific content:

| File | Delta | Reason |
|------|-------|--------|
| `.github/copilot-instructions.md` | +18.8% | Workspace-specific search strategy and session memory requirements |
| `.github/instructions/backlogit.instructions.md` | +43.5% | Shipment reconciliation rules and backlogit-specific workflows |
| `.github/instructions/backlog-integration.instructions.md` | +13.2% | Extended operations for backlogit shipments |
| `.github/instructions/commit-message.instructions.md` | +17.5% | Workspace-specific commit scopes |
| `AGENTS.md` | +1.3% | Workspace-specific quality gates (3 vs template's 4) |

## Proposed Changes (ordered by priority)

### P1 — Degrading (Applied)

#### TUNE-001: fix-ci/SKILL.md — Major content drift (-61%)

**Artifact**: `.github/skills/fix-ci/SKILL.md`
**Issue**: Template has 377 lines; installed had 147 lines. Missing:
- Prerequisites section with tool availability checks
- Quick Start section
- Parameters table (replaced old `## Inputs` section)
- Step 2.5: Copilot Review Comment Detection (NEW)
- Step 6.5: Reply Gate (NON-NEGOTIABLE) — agents were not required to reply before resolving
- Step 8.5: Defect Logging — circuit breaker halt path lacked defect logging
- Common Fix Patterns (Format, Lint, Test, Build) — practical fix guidance
- Terminal Output Management — log suppression guidance
- Intercom Events table — intercom broadcast integration

**Impact**: Agents using fix-ci lacked Copilot review detection and the non-negotiable reply gate.
**Resolution**: Regenerated from template with full variable substitution. Backup at `.autoharness/backups/2026-04-24/fix-ci-SKILL.md`.

#### TUNE-002: workflow-policies.md — Missing policies (-23.5%)

**Artifact**: `.github/policies/workflow-policies.md`
**Issue**: Template has 217 lines; installed had 166 lines. Missing:
- P-008: Markdown Conformance policy
- P-009: Merge-Commit-Only (No Squash or Rebase Merge) policy

**Impact**: Agents could squash-merge or rebase-merge without policy violation detection.
**Resolution**: Regenerated from template. Backup at `.autoharness/backups/2026-04-24/workflow-policies.md`.

#### TUNE-003: constitution.instructions.md — Missing principle (-4.9%)

**Artifact**: `.github/instructions/constitution.instructions.md`
**Issue**: Template has 306 lines; installed had 291 lines. Missing:
- Principle XI: Merge Commit History Preservation (NON-NEGOTIABLE)

**Impact**: Constitution lacked explicit merge-commit preservation enforcement.
**Resolution**: Regenerated from template. Backup at `.autoharness/backups/2026-04-24/constitution.instructions.md`.

#### TUNE-004: git-merge.instructions.md — Missing section (-25.7%)

**Artifact**: `.github/instructions/git-merge.instructions.md`
**Issue**: Template has 74 lines; installed had 55 lines. Missing:
- `## Merge Strategy Policy (NON-NEGOTIABLE)` section

**Impact**: Git merge instructions lacked the non-negotiable merge-commit-only policy.
**Resolution**: Regenerated from template. Backup at `.autoharness/backups/2026-04-24/git-merge.instructions.md`.

#### TUNE-005: stage.agent.md — Missing workflow steps (-8.2%)

**Artifact**: `.github/agents/stage.agent.md`
**Issue**: Template has 607 lines; installed had 557 lines. Missing:
- Validation Boundary section after Step 0
- Step 1.8: Learnings Retrieval (compound library and continuous-learning mining)
- Step 5.6: Archive Consumed Stash Entries

**Impact**: Stage agent skipped learnings retrieval and stash archival after harvest.
**Resolution**: Regenerated from template. Backup at `.autoharness/backups/2026-04-24/stage.agent.md`.

#### TUNE-006: ship.agent.md — Missing validation boundary (-5.6%)

**Artifact**: `.github/agents/ship.agent.md`
**Issue**: Template has 447 lines; installed had 422 lines. Missing:
- Validation Boundary section after Step 0.5

**Impact**: Ship agent lacked the explicit pre-flight validation gate.
**Resolution**: Regenerated from template. Backup at `.autoharness/backups/2026-04-24/ship.agent.md`.

### P2 — Growth (Applied)

#### TUNE-007: shipment-reconcile skill untracked in manifest

**Issue**: `.github/skills/shipment-reconcile/SKILL.md` exists and matches a template but was not listed in the manifest skills section.
**Resolution**: Added to manifest skills list.

#### TUNE-008: cp-review-fix.prompt.md untracked

**Issue**: `.github/prompts/cp-review-fix.prompt.md` is a workspace-specific prompt not from a template.
**Resolution**: Added to manifest as preserved artifact (not template-managed).

#### TUNE-009: Stack-pack composition updated

**Issue**: Config and manifest recorded `mcp-server` + `cli-tool`; profile already reflected `rust-mcp-daemon` + `rust-surrealdb-embedded` + `rust-tree-sitter` + `github-actions-ci`.
**Resolution**: Updated config and manifest to match profile stack-pack composition.

#### TUNE-010: Workspace profile updated

**Issue**: Profile file counts, dependency versions, and module descriptions were stale.
**Resolution**: Updated source count (68→87), test counts (82→102), tree-sitter (0.24→0.25), added CozoDB dependency, expanded feature flags, updated module descriptions, added docs/upstream/ directory.

### P3 — Cosmetic (Applied)

#### TUNE-011: Stale deprecated agent references removed

**Issue**: Manifest listed 6 deprecated agents in `.github/agents/deprecated/` but that directory does not exist. These agents were previously cleaned up.
**Resolution**: Removed stale `deprecated_agents` section from manifest `preserved_artifacts`.

## Learning Signals

| Source | Finding |
|--------|---------|
| Compound library | Empty (0 entries) — no patterns to mine |
| Continuous-learning observations | Empty — no observations captured yet |
| Continuous-learning instincts | Empty — no instincts formed yet |
| Closure artifacts | 6 closure reports, 10 compacted memory files — insufficient volume for recurring pattern detection |
| Prior tuning reports | 1 report (2026-04-14): all 6 proposals applied successfully; no recurring drift detected |

## Verification

| Check | Result |
|-------|--------|
| Template variable sweep | PASS — 0 unresolved `{{UPPERCASE}}` variables |
| fix-ci/SKILL.md | 376L (template 377L, -1 for QUALITY_GATE_4 removal) |
| workflow-policies.md | 217L (template 217L) |
| constitution.instructions.md | 305L (template 306L, -1 for QUALITY_GATE_4 removal) |
| git-merge.instructions.md | 74L (template 74L) |
| stage.agent.md | 607L (template 607L) |
| ship.agent.md | 447L (template 447L) |
| Backups created | 6 files in `.autoharness/backups/2026-04-24/` |

## Recommendation

The harness is now aligned with the current template set. Key improvements:

1. **fix-ci skill** gained Copilot review detection, the non-negotiable reply gate, defect logging, common fix patterns, and intercom events — the largest single improvement.
2. **Merge-commit-only policy** is now enforced across constitution, workflow policies, and git-merge instructions.
3. **Stage agent** now includes learnings retrieval and stash archival steps.
4. **Ship agent** now has the validation boundary gate.

### Next Tuning Triggers

- After the CozoDB migration completes (new backend will change the stack significantly)
- When the compound library accumulates 3+ entries (enables pattern detection)
- When continuous-learning observations begin capturing data
- Monthly cadence: target 2026-05-24

Generated by autoharness tune-harness skill
