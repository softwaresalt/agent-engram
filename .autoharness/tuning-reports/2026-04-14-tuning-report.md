---
type: tuning-report
generated_at: "2026-04-14T20:46:00Z"
workspace: "D:\\Source\\GitHub\\agent-engram"
autoharness_home: "C:\\Users\\derek.williams\\AppData\\Roaming\\uv\\tools\\autoharness\\Lib\\site-packages\\autoharness\\data"
manifest_installed_at: "2025-07-23"
---

# Harness Tuning Report — 2026-04-14

## Drift Summary

| Category | Count | Description |
|----------|-------|-------------|
| P1 — Degrading | 3 | Content drift in stage agent, ship agent, harvest skill |
| P2 — Growth | 1 | New stage-grouping-analysis prompt available |
| P3 — Cosmetic | 1 | Manifest template path staleness |

## Composition

| Aspect | Installed | Current |
|--------|-----------|---------|
| Preset | full | full |
| Primary stack pack | mcp-server | rust-mcp-daemon |
| Stack packs | mcp-server, cli-tool | rust-mcp-daemon, rust-surrealdb-embedded, rust-tree-sitter, github-actions-ci |
| Install layers | foundation, instructions, workflow, review, runtime, backlog, knowledge, overlays | (unchanged) |

## Checksum Scan

| Status | Count |
|--------|-------|
| Present (all artifacts) | 77 |
| Missing | 0 |
| User-modified | 0 (not checksummed — see cosmetic note on manifest staleness) |

## Healthy Checks (No Action Needed)

- ✅ All 77 manifest artifacts exist in workspace
- ✅ Plan-hardening pipeline coherent (impl-plan → stage → plan-harden → plan-review)
- ✅ Gitignore has `.*.lock` / `**/.*.lock` without blocking `Cargo.lock`
- ✅ All 6 scripts present (acquire/release lock × 2, search × 2)
- ✅ All 6 deprecated agents in `.github/agents/deprecated/`
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

## Proposed Changes (ordered by priority)

### P1 — Degrading

#### TUNE-001: Stage agent missing shipment workflow features

**Artifact**: `.github/agents/stage.agent.md`
**Issue**: Template has 551 lines; installed has 386 lines (-30%). Missing:
- Step 1 expanded → "Stash Triage and Entry Classification" with shape classification
- NEW Step 1.5: Contextual Grouping Analysis (task-shaped entries only)
- Step 2 restructured → "Deliberation" with shape-aware routing
- NEW Step 5.5: Shipment Assembly
- NEW "Shipment Context" section with interaction pattern adaptation
- Updated broadcast table with grouping and shipment events
- Updated session checkpoints for grouping and shipment milestones
- Updated behavioral constraints for grouping and shipment invariants

**Impact**: Stage agent cannot perform contextual grouping, shipment assembly, or
adapt to operator interaction patterns (to-do queue mode vs feature mode).

**Proposal**: Regenerate from template with workspace variable substitution.

#### TUNE-002: Ship agent missing shipment intake step

**Artifact**: `.github/agents/ship.agent.md`
**Issue**: Template has 390 lines; installed has 349 lines (-10.5%). Missing:
- NEW Step 0.5: Shipment Intake (backlogit with shipments only) — validates,
  claims, and scopes execution to a Stage-prepared shipment
- Fallback path for direct invocation without a Stage-prepared shipment

**Impact**: Ship agent does not know how to receive and validate shipments from Stage.

**Proposal**: Regenerate from template with workspace variable substitution.

#### TUNE-003: Harvest skill minor content drift

**Artifact**: `.github/skills/harvest/SKILL.md`
**Issue**: Template has 148 lines; installed has 134 lines (-9.5%). Missing footer.
Content differences in Phase 3 (parent-first ordering, shipment flag) and
Phase 4 (duplicate checks, backlog tool references).

**Proposal**: Regenerate from template with workspace variable substitution.

### P2 — Growth

#### TUNE-004: New stage-grouping-analysis prompt not installed

**Artifact**: `.github/prompts/stage-grouping-analysis.prompt.md` (new)
**Source template**: `templates/prompts/stage-grouping-analysis.prompt.md.tmpl`
**Issue**: New prompt available that supports Stage Step 1.5 (contextual grouping).
Stage's default session entry now references this prompt.

**Proposal**: Install from template.

### P3 — Cosmetic

#### TUNE-005: Manifest template source paths stale

**Artifact**: `.autoharness/harness-manifest.yaml`
**Issue**: 12 template source paths reference old locations after autoharness
template directory reorganization. Affected:
- `foundation/constitution.tmpl` → `foundation/constitution.instructions.md.tmpl`
- `foundation/copilot-instructions.tmpl` → `foundation/copilot-instructions.md.tmpl`
- `foundation/harness-config.yaml.tmpl` → `harness-config.yaml.tmpl`
- `foundation/start.ps1.tmpl` → `scripts/start.ps1.tmpl`
- `foundation/start.sh.tmpl` → `scripts/start.sh.tmpl`
- `scripts/file-lock/*` → `skills/file-lock/scripts/*`
- `scripts/skill-search/*` → `skills/skill-search/scripts/*`

**Impact**: Future tune/verify cycles may fail to match artifacts to templates.

**Proposal**: Update manifest source paths to current template locations.

#### TUNE-006: Brainstorm skill removed

**Artifact**: `.github/skills/brainstorm/SKILL.md`
**Issue**: Brainstorm was part of the backlog-md ecosystem. The `deliberate` skill
replaces it with a richer protocol. Template was already removed upstream.

**Resolution**: Skill deleted and manifest entry removed.

## Recommendation

Apply TUNE-001 through TUNE-004 to bring the harness up to date with
shipment workflow capabilities. These four changes form a coherent feature
set: Stage gains grouping and shipment assembly, Ship gains shipment intake,
harvest gains shipment-aware decomposition, and the new prompt supports the
grouping workflow entry point.

TUNE-005 and TUNE-006 are housekeeping that prevent future tuning confusion.
