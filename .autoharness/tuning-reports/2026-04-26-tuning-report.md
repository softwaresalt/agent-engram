---
type: tuning-report
generated_at: "2026-04-26T12:44:00-07:00"
workspace: "<WORKSPACE_ROOT>"
autoharness_home: "<AUTOHARNESS_HOME>"
autoharness_version: "1.3.2"
manifest_installed_at: "2025-07-23"
previous_tune: "2026-04-25T12:34:00-07:00"
trigger: "operator-requested tune-up"
---

# Harness Tuning Report — 2026-04-26

## Drift Summary

| Category | Count | Description |
|----------|-------|-------------|
| P1 — Stage Agent Drift | 3 sections | Step Sequence Contract, Shipment Assembly NON-NEGOTIABLE upgrade, Pre-Summary Verification Gate |
| P1 — Ship Agent Drift | 3 sections | Post-Merge Branch Protocol, Branch Management Rules, Source artifact cleanup rewrite |
| P2 — PR-Lifecycle Skill | 2 hunks | Branch retention rule + post-merge branch guidance |
| P2 — Shipment-Reconcile Skill | +269 bytes | Content expansion in existing sections |
| P2 — Architecture-Doc Instructions | +202 bytes | Template content improvements |
| P2 — Continuous-Learning Instructions | +108 bytes | Template content improvements |
| P2 — Misplaced Global Agents | 2 files | auto-mergeinstall/auto-tune duplicated in .github/agents/ (belong in .github/local-agents/ only) |
| P3 — Sparse Continuous Learning | 1 obs | Near-zero observation capture despite active development |
| P3 — verify-workspace CLI Bug | 1 | `AttributeError: 'str' object has no attribute 'get'` on current manifest format |

## Composition

| Aspect | Previous (v1.3.0) | Current (v1.3.2) |
|--------|-------------------|-------------------|
| autoharness version | 1.3.0 | 1.3.2 |
| Preset | full | full (no change) |
| Primary stack pack | rust-mcp-daemon | rust-mcp-daemon (no change) |
| Stack packs | (unchanged) | rust-mcp-daemon, rust-surrealdb-embedded, rust-tree-sitter, github-actions-ci |
| Install layers | (unchanged) | foundation, instructions, workflow, review, runtime, backlog, knowledge, overlays |
| Capability packs | (unchanged) | agent-intercom, agent-engram, backlogit, strict-safety, release-observability, continuous-learning, adversarial-review |
| Managed artifacts | 75 | 75 (no new artifacts; patches only) |
| New skill templates | 0 | None since v1.3.0 |

## Structural Profile Verification

All workspace counts match the installed profile — no structural drift detected.

| Metric | Profile | Actual | Status |
|--------|---------|--------|--------|
| src/*.rs files | 87 | 87 | ✅ |
| tests/contract/ | 26 | 26 | ✅ |
| tests/integration/ | 50 | 50 | ✅ |
| tests/unit/ | 24 | 24 | ✅ |
| tests/helpers/ | 2 | 2 | ✅ |
| docs/adrs/ | 17 | 17 | ✅ |
| Source modules | 10 | 10 | ✅ |

## Proposed Changes

### TUNE-007: Stage Agent — Step Sequence Contract (P1)

**Artifact**: `.github/agents/stage.agent.md`
**Issue**: Template v1.3.2 added `## Step Sequence Contract (NON-NEGOTIABLE)` — a 26-line section that enforces step ordering as a P-005 policy. The installed agent does not enforce step ordering; agents can skip steps or present summaries prematurely.
**Change**: Insert the Step Sequence Contract section after the `## Inputs` section and before `## Required Steps`.
**Impact**: Prevents agents from skipping mandatory steps or presenting summaries without completing shipment assembly.

### TUNE-008: Stage Agent — Shipment Assembly NON-NEGOTIABLE Upgrade (P1)

**Artifact**: `.github/agents/stage.agent.md`
**Issue**: Step 5.5 header is `### Step 5.5: Shipment Assembly` — missing the `(NON-NEGOTIABLE when shipments are supported)` qualifier. The section body also lacks strengthened enforcement language and a transition directive after Step 5.3.
**Change**: Update header and body language to match template. Add "NEXT STEP" transition text after Step 5.3.
**Impact**: Strengthens shipment assembly as a mandatory gate rather than an advisory step.

### TUNE-009: Stage Agent — Pre-Summary Verification Gate (P1)

**Artifact**: `.github/agents/stage.agent.md`
**Issue**: Template v1.3.2 added `#### Pre-Summary Verification Gate (NON-NEGOTIABLE)` — a 12-line subsection under Step 6 that verifies all prior steps completed before presenting the summary. Without this, agents may present summaries with missing shipment IDs.
**Change**: Insert the verification gate after Step 5.6 and before the summary output.
**Impact**: Blocks premature summary delivery; catches the specific failure mode of "summary without shipment ID."

### TUNE-010: Ship Agent — Post-Merge Branch Protocol (P1)

**Artifact**: `.github/agents/ship.agent.md`
**Issue**: Template v1.3.2 added `#### Step 6.0: Post-Merge Branch Protocol (NON-NEGOTIABLE)` — a 28-line section requiring post-merge closure work to happen on a dedicated `post-merge/{feature_slug}` branch with its own PR. The installed agent commits closure work directly to the feature branch or main.
**Change**: Insert Step 6.0 before the existing Step 6.1 content.
**Impact**: Ensures closure work (backlog archival, knowledge graduation, doc updates) goes through code review instead of landing directly on main.

### TUNE-011: Ship Agent — Branch Management Rules (P1)

**Artifact**: `.github/agents/ship.agent.md`
**Issue**: Template v1.3.2 added `## Branch Management Rules (NON-NEGOTIABLE)` — a 7-line top-level section summarizing branch discipline. Also added a branch retention rule (point 11) in Step 5 preventing premature branch switching during merge approval.
**Change**: Insert the Branch Management Rules section and the branch retention rule.
**Impact**: Prevents branch-switching bugs during the Ship pipeline.

### TUNE-012: Ship Agent — Source Artifact Cleanup Rewrite (P1)

**Artifact**: `.github/agents/ship.agent.md`
**Issue**: Template v1.3.2 rewrote Step 6 point 7 from "Archive stale deliberation and stash artifacts" (broad heuristic search) to "Source artifact cleanup". The installed backlog registry does not expose dedicated stash-removal or deliberation-archive operations, so the cleanup text must stay aligned with supported operations and verifiable `references`/`custom_fields` links.
**Change**: Replace the existing Step 7 content with a targeted cleanup approach that records source stash IDs and deliberation references through supported backlog comments and closure artifacts.
**Impact**: More reliable post-merge cleanup guidance without referencing unavailable backlog operations.

### TUNE-013: PR-Lifecycle Skill — Branch Retention + Post-Merge Guidance (P2)

**Artifact**: `.github/skills/pr-lifecycle/SKILL.md`
**Issue**: Template v1.3.2 added branch retention rules (+8 lines): (1) NON-NEGOTIABLE rule to stay on feature branch while awaiting merge, (2) guidance not to checkout main after merge — post-merge closure belongs on a dedicated branch.
**Change**: Regenerate from template with variable substitution.
**Impact**: Aligns pr-lifecycle with Ship's new post-merge branch protocol.

### TUNE-014: Shipment-Reconcile Skill Refresh (P2)

**Artifact**: `.github/skills/shipment-reconcile/SKILL.md`
**Issue**: +269 bytes of template content improvements.
**Change**: Regenerate from template with variable substitution.

### TUNE-015: Architecture-Doc Instructions Refresh (P2)

**Artifact**: `.github/instructions/architecture-doc.instructions.md`
**Issue**: +202 bytes of template content improvements.
**Change**: Regenerate from template with variable substitution.

### TUNE-016: Continuous-Learning Instructions Refresh (P2)

**Artifact**: `.github/instructions/continuous-learning.instructions.md`
**Issue**: +108 bytes of template content improvements.
**Change**: Regenerate from template with variable substitution.

### TUNE-017: Remove Misplaced Global Agent Copies (P2)

**Artifact**: `.github/agents/auto-mergeinstall.agent.md`, `.github/agents/auto-tune.agent.md`
**Issue**: These global autoharness agents exist in both `.github/agents/` (untracked by git) AND `.github/local-agents/` (where `start.ps1` correctly injects them). The copies in `.github/agents/` are duplicates. Since `agent-engram.code-workspace` sets `chat.agentFilesLocations` to only `.github/agents`, the `.github/local-agents/` copies are not discoverable in VS Code.
**Options**:
  - **Option A (recommended)**: Remove copies from `.github/agents/`, update `agent-engram.code-workspace` to include both locations in `chat.agentFilesLocations`
  - **Option B**: Remove copies from `.github/agents/`, accept that global agents are not discoverable in the VS Code agent picker
  - **Option C**: Keep copies in `.github/agents/` and add them to `.gitignore` with `!.github/agents/auto-*.agent.md` pattern
**Change**: Operator choice required.

## Preserved Artifacts (Not Regenerated)

These artifacts have intentional local additions (installed > template). They are NOT drifted — they are enhanced.

| File | Local Delta | Reason |
|------|------------|--------|
| backlog-integration.instructions.md | +1,268 bytes | Local shipment reconciliation rules |
| backlogit.instructions.md | +1,055 bytes | Local additions |
| copilot-instructions.md | +762 bytes | Workspace-specific search/memory |
| commit-message.instructions.md | +658 bytes | Local commit scopes |
| AGENTS.md | +611 bytes | Workspace-specific quality gates |
| constitution.instructions.md | +417 bytes | Workspace-specific additions |
| rust-reviewer.agent.md | +327 bytes | Local Rust expertise |
| backlogit-yaml-header-tooling.instructions.md | +285 bytes | Local additions |
| technology-rust.instructions.md | +234 bytes | Local conventions |
| mcp-server.instructions.md | +164 bytes | Local MCP conventions |

## Template Variable Drift (No Action Needed)

Template v1.3.2 migrated many hardcoded tool names (e.g., `backlogit_list_checkpoints`) to template variables (e.g., `{{OP_LIST_CHECKPOINTS_MCP}}`). The installed versions have correct resolved values. No functional difference — these will be picked up automatically on next full regeneration.

## Not Applicable (Skipped)

| Template | Reason |
|----------|--------|
| browser-verification.instructions.md | Capability pack not enabled |
| technology-go/python/typescript instructions | Not applicable to Rust workspace |
| technology.instructions.md (generic) | Renamed to technology-rust.instructions.md |

## Learning-Driven Findings

### Continuous Learning Sparsity (P3 — Process Health)

| Metric | Count | Expected |
|--------|-------|----------|
| Observations | 1 | 10+ (given 9 closures and active development) |
| Instincts | 0 | 2-3 (clusters from observations) |
| Learned artifacts | 0 | 0 (expected — promotion requires 3+ observations) |
| Compound library | 16 entries | Healthy |
| Recent closures | 9 | Active |

The observation capture pipeline is nearly dormant despite active development. The compound library is healthy (16 entries, recently refreshed), but the observe → learn → evolve pipeline has no material to work with.

**Recommendation**: Invoke the `observe` skill after the next 2-3 Ship sessions to seed the observation pipeline with recent session patterns. Current compound learnings may be a better immediate source for instinct formation.

### verify-workspace CLI Bug (P3 — Upstream)

The `autoharness verify-workspace` command crashes with:
```
AttributeError: 'str' object has no attribute 'get'
```
at `verify_workspace.py:946`. The manifest artifact entries use dict objects with `path`/`source` keys, but some code path iterates them as strings.

**Recommendation**: Document in `docs/upstream/` and report to autoharness maintainers.

## Verification Checklist

| Check | Status |
|-------|--------|
| Template variable sweep | Deferred (verify-workspace CLI broken) |
| Profile structural alignment | PASS — all counts match |
| Artifact inventory | PASS — 75 managed, 5 preserved, 2 misplaced |
| Backups required | Yes — create before patching |
| Breaking drift | None — all drift is additive |

## Recommended Next Steps

1. Create feature branch: `chore/autoharness-tune-2026-04-26`
2. Apply TUNE-007 through TUNE-016 (operator approval per item)
3. Resolve TUNE-017 (operator choice for global agent placement)
4. Update manifest `tuned_at` and `verification.last_tune_applied`
5. Open PR for review before merging to main

## Next Tuning Triggers

- After CozoDB migration completes (backend change → profile + stack-pack drift)
- After next 3+ Ship sessions (continuous learning should have material by then)
- On next autoharness CLI upgrade past v1.3.2
- Monthly cadence: target 2026-05-26

Generated by autoharness Auto-Tune agent
