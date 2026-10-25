---
title: "Harness Tuning Report — 2026-10-24"
date: "2026-10-24T19:55:00Z"
installer_version: "1.6.0"
autoharness_home: "C:\\Python\\Python315\\Lib\\site-packages\\autoharness\\data"
trigger: "operator-requested"
scope: "all"
proposals:
  - id: "TUNE-001"
    key: "upgrade-manifest-contract-1.0.0-to-1.1.0"
    priority: "P1"
    category: "degrading"
    source: "schema-contract"
    artifact: ".autoharness/harness-manifest.yaml"
    status: "resolved"
  - id: "TUNE-002"
    key: "upgrade-config-contract-1.1.0-to-1.3.0"
    priority: "P1"
    category: "degrading"
    source: "schema-contract"
    artifact: ".autoharness/config.yaml"
    status: "resolved"
  - id: "TUNE-003"
    key: "upgrade-profile-contract-1.0.0-to-1.1.0"
    priority: "P1"
    category: "degrading"
    source: "schema-contract"
    artifact: ".autoharness/workspace-profile.yaml"
    status: "resolved"
  - id: "TUNE-004"
    key: "escalation-route-resolution-stage"
    priority: "P1"
    category: "degrading"
    source: "structural-drift"
    artifact: ".autoharness/config.yaml"
    status: "resolved"
  - id: "TUNE-005"
    key: "frontmatter-conformity-fc-1-skills-and-subagents"
    priority: "P1"
    category: "degrading"
    source: "schema-contract"
    artifact: ".github/skills/*/SKILL.md, .github/agents/subagents/*.agent.md"
    status: "resolved"
  - id: "TUNE-006"
    key: "model-routing-refresh-pipeline-and-subagents"
    priority: "P1"
    category: "degrading"
    source: "schema-contract"
    artifact: ".github/agents/_*.agent.md, .github/agents/subagents/*.agent.md"
    status: "resolved"
  - id: "TUNE-007"
    key: "v1.6.0-template-refresh-pipeline-skills-instructions-policies"
    priority: "P1"
    category: "degrading"
    source: "structural-drift"
    artifact: ".github/agents/*, .github/skills/*, .github/instructions/*, .github/policies/workflow-policies.md, scripts/*_lock.*"
    status: "resolved"
  - id: "TUNE-008"
    key: "workspace-discovery-delta-cozodb-and-file-counts"
    priority: "P2"
    category: "growth"
    source: "structural-drift"
    artifact: ".autoharness/workspace-profile.yaml, .github/copilot-instructions.md, AGENTS.md, .github/instructions/constitution.instructions.md"
    status: "resolved"
  - id: "TUNE-009"
    key: "dynamic-policy-proposal-code-graph-reindex-integrity"
    priority: "P2"
    category: "growth"
    source: "learning-driven"
    artifact: ".autoharness/policy-proposals/P-DYN-001.md"
    status: "proposed"
  - id: "TUNE-010"
    key: "refresh-startup-script-ps1-and-sh-contract"
    priority: "P2"
    category: "degrading"
    source: "schema-contract"
    artifact: "start.ps1, start.sh"
    status: "deferred"
---

## Harness Tuning Report — 2026-10-24

### Drift Summary

* Breaking changes: 0
* Degrading changes: 7 (all resolved; 1 startup-script migration family intentionally deferred)
* Growth opportunities: 2 (workspace discovery stack/count refresh applied; 1 dynamic policy proposal generated)
* Cosmetic adjustments: 3 (auto-remediated during adversarial verification)

### Composition

* Installed primary stack pack: `rust-mcp-daemon`
* Current primary stack pack: `rust-mcp-daemon`
* Installed stack packs: `["rust-mcp-daemon", "rust-surrealdb-embedded", "rust-tree-sitter", "github-actions-ci"]`
* Current stack packs: `["rust-mcp-daemon", "rust-surrealdb-embedded", "rust-tree-sitter", "github-actions-ci"]`
* Installed layers: `["foundation", "instructions", "workflow", "review", "runtime", "backlog", "knowledge", "overlays"]`
* Recommended layers: `["foundation", "instructions", "workflow", "review", "runtime", "backlog", "knowledge", "overlays"]`
* Capability packs: `["agent-engram", "backlogit", "strict-safety", "release-observability", "continuous-learning", "adversarial-review"]`

### Checksum & Contract Scan

* Missing installed artifacts: 0
* Pre-tune user-modified / out-of-band artifacts: 24 (caused by operator `model_routing` updates in `.autoharness/config.yaml`, out-of-band cherry-pick `87b23274`, and LF/CRLF checkout divergence; all reconciled and re-hashed)
* Post-tune manifest-tracked artifacts: 105 (`unchanged`: 105, `user-modified`: 0, `missing`: 0)
* Schema contracts:
  * `harness-manifest`: `known-legacy` (`1.0.0`) → `current` (`1.1.0`)
  * `harness-config`: `known-legacy` (`1.1.0`) → `current` (`1.3.0`)
  * `workspace-profile`: `known-legacy` (`1.0.0`) → `current` (`1.1.0`)
* Startup-script contracts:
  * `start.ps1`: `known-legacy` (preserved intentionally — contract-tested by `tests/contract/start_launcher_test.rs` per `166baadc` / `140.001-T`)
  * `start.sh`: `known-legacy` with preserved custom tail (`sha256: ca7fda0c...`, 790 bytes; preserved alongside `start.ps1`)

### Learning Signals

* Compound patterns detected: 13 (`component_hotspot`: `src/services/code_graph.rs` [8], `backlogit shipment closure` [4], `src/db/queries.rs` [3]; `cross_cutting_tags`: `clippy` [8], `code-graph` [8], `daemon` [8], `rust` [8], `016-S` [4], `copilot-review` [4], `tree-sitter` [4], `013-S` [3], `103-S` [3], `pedantic` [3])
* Promotion-ready instincts: 0 (1 observation in `.autoharness/continuous-learning/observations/2026-09-26.jsonl` below promotion threshold 3)
* Recurring closure findings: 0
* Policy-gap candidates detected: 1 (`src/services/code_graph.rs` / `code-graph` 8 compound entries)
* Learning-driven proposals generated: 1 (`.autoharness/policy-proposals/P-DYN-001.md`)

### Proposed & Applied Changes (Ordered by Priority)

#### P1 — Degrading (Applied)

1. **[TUNE-001] Upgrade `harness-manifest` contract (`1.0.0` → `1.1.0`) and `autoharness_version` (`1.5.0` → `1.6.0`)** (`source: schema-contract`)
   * **Artifact**: `.autoharness/harness-manifest.yaml`
   * **Issue**: Manifest was pinned to schema `1.0.0` and `autoharness` `1.5.0` (`Python314` path), with 21 `ROUTE_VARIABLE_STALE` warnings against `.autoharness/config.yaml` and afill-in-the-blank `.github/policies/policy-proposal.md` entry causing 11 unresolved placeholder errors.
   * **Resolution**: Upgraded `schema_version` to `"1.1.0"`, updated `autoharness_home` and `autoharness_version: "1.6.0"`, moved `.github/policies/policy-proposal.md` to `preserved_artifacts`, set `template: "skills/harness-architect/SKILL.md.tmpl"` for `autoharness harness resolve` compatibility, refreshed `variables_used` from authoritative config, and updated all 105 artifact checksums.

2. **[TUNE-002] Upgrade `harness-config` contract (`1.1.0` → `1.3.0`) and preserve operator `model_routing` edits** (`source: schema-contract`)
   * **Artifact**: `.autoharness/config.yaml`
   * **Issue**: Operator modified `.autoharness/config.yaml` to configure `context_tier: "long_context"` and updated `tier1` (`claude-haiku-5.5`), `tier2` (`claude-sonnet-5.5`), `tier3` (`gemini-4-argon`), `orchestrator` (`claude-sonnet-5.5`), `stage` (`gemini-4-argon`), `ship` (`claude-sonnet-5.5`), and `anchor_review` (`gpt-6.1-sol`), while `schema_version` remained `"1.1.0"`.
   * **Resolution**: Preserved all operator routing preferences, added empty `routes: {}` and `bindings: {}` catalog maps, and bumped `schema_version` to `"1.3.0"`.

3. **[TUNE-003] Upgrade `workspace-profile` contract (`1.0.0` → `1.1.0`) and remove retired `agent_intercom` block** (`source: schema-contract`)
   * **Artifact**: `.autoharness/workspace-profile.yaml`
   * **Issue**: Profile carried `schema_version: "1.0.0"` and a leftover `agent_intercom:` block (`removed-capability-pack` warning).
   * **Resolution**: Removed the retired `agent_intercom` block and upgraded `schema_version` to `"1.1.0"`.

4. **[TUNE-004] Fix `escalation_route_resolution` (`ESCALATION_DEGRADED` on `stage`)** (`source: structural-drift`)
   * **Artifact**: `.autoharness/config.yaml`, `.github/agents/_stage.agent.md`, `.github/instructions/escalation-protocol.instructions.md`
   * **Issue**: Because `stage` and `tier3` both resolved to `gemini-4-argon` (`google`, `high`) and `stage` had no nested `escalation:` override, `stage`'s escalation route fell back to `tier3` — a same-route no-op failing `escalation_route_resolution`.
   * **Resolution**: Added nested `stage.escalation` (`claude-sonnet-5.5` / `anthropic` / `high` / `long_context`) in `.autoharness/config.yaml` and re-rendered `_stage.agent.md` and `escalation-protocol.instructions.md`.

5. **[TUNE-005] Bring 18 managed skills and 3 workspace-authored subagents into `fc-1` frontmatter conformity** (`source: schema-contract`)
   * **Artifact**: `.github/skills/*/SKILL.md`, `.github/agents/subagents/{mcp-protocol-reviewer,rust-mcp-expert,rust-safety-reviewer}.agent.md`
   * **Issue**: `frontmatter_conformity` failed on 18 `SKILL.md` files missing required `name:` frontmatter and emitted warnings on 3 workspace-authored subagents missing `max_subagent_tier`, `subagent_depth`, and route keys.
   * **Resolution**: Re-rendered all managed skills from v1.6.0 templates (adding `name:`) and updated the 3 workspace-authored subagents to `fc-1` tier-routed frontmatter.

6. **[TUNE-006] Refresh route frontmatter across all 3 pipeline agents and 17 managed subagents** (`source: schema-contract`)
   * **Artifact**: `.github/agents/_*.agent.md`, `.github/agents/subagents/*.agent.md`
   * **Issue**: All 17 managed subagents and 3 pipeline agents carried stale v1.5.0 model routes lacking `context_tier: "long_context"` and carrying legacy `model_routing` / `model_tier` keys.
   * **Resolution**: Re-rendered all pipeline agents and managed subagents against the operator's `.autoharness/config.yaml` routes.

7. **[TUNE-007] Synchronize pipeline agents, skills, instructions, policies, and lock scripts to v1.6.0 templates** (`source: structural-drift`)
   * **Artifact**: `.github/agents/_*.agent.md`, `.github/skills/*/SKILL.md`, `.github/instructions/*.instructions.md`, `.github/policies/workflow-policies.md`, `.autoharness/backlog-registry.yaml`, `scripts/{acquire_lock,release_lock}.{ps1,sh}`
   * **Issue**: Partial manual backport `87b23274` left `_ship.agent.md`, `harness-architect/SKILL.md`, `build-feature/SKILL.md`, and `workflow-policies.md` out of sync with v1.6.0 (causing the 144-S active-code-task route block and failing `ship_release_closure_sequence`, `dark_factory_visibility_contract`, and `orchestrator_invocation_routing_directive`).
   * **Resolution**: Re-rendered from autoharness v1.6.0 templates (including P-002.7 unstarted claim-activated task preparation, P-002.8 claim-marker admission, 192-F `cascade-close`, P-015 INV-12 linked-deliberation disposition, `Dark Factory Visibility Protocol`, `agent-engram` structural-routing assertions, and H2/H4/O2 file-lock token containment) while preserving workspace customizations (`commit_id == HEAD` and `--paginate` in `github-pr-automation.instructions.md`, P-018 engagement rules, security-reviewer globs in `review/SKILL.md`).
   * **Base-branch activation note (`copilot-code-review.instructions.md`)**: GitHub Copilot code review reads instruction files from the pull request's **base branch**, so a refreshed `copilot-code-review.instructions.md` takes effect for code review only after the tuning pull request is merged to the base branch (`main`). The tuning pull request itself is validated structurally only, and the Settings → Copilot → Code review custom-instructions toggle is an advisory prerequisite, not a hard tuning gate.

#### P2 — Growth & Deferred Proposals

8. **[TUNE-008] Update workspace profile and foundation docs for CozoDB and current file counts** (`source: structural-drift`, **Applied**)
   * **Artifact**: `.autoharness/workspace-profile.yaml`, `.github/copilot-instructions.md`, `AGENTS.md`, `.github/instructions/constitution.instructions.md`
   * **Resolution**: Replaced retired `surrealdb`/`axum` references with `cozo` (`0.7`), updated source/test counts (`149 .rs files`, `270 test files: contract/64, integration/140, unit/62, helpers/3`), removed uninstalled `browser-verification` sections from `AGENTS.md` and `copilot-instructions.md`, and bumped Constitution version to `1.1.0`.

9. **[TUNE-009] Dynamic policy proposal `P-DYN-001` for `src/services/code_graph.rs` reindex integrity** (`source: learning-driven`, **Proposed for Operator Review**)
   * **Artifact**: `.autoharness/policy-proposals/P-DYN-001.md`
   * **Evidence**: 8 compound entries in `docs/compound/` sharing component `src/services/code_graph.rs` and tag `code-graph`. Written to `.autoharness/policy-proposals/P-DYN-001.md` for operator review.

10. **[TUNE-010] Startup-script contract migration (`refresh-startup-script-ps1-contract`, `refresh-startup-script-sh-contract`)** (`source: schema-contract`, **Deferred — Intentional Local Customization**)
    * **Artifact**: `start.ps1`, `start.sh`
    * **Rationale**: `start.ps1` and `start.sh` are hardened workspace launchers contract-tested by `tests/contract/start_launcher_test.rs` (bounded cleanup wait and shared pre-warm budget; see commit `166baadc` and task `140.001-T`). Both scripts are intentionally preserved unchanged.

---

## Adversarial Verification

### Reviewer Dispatch

* **Anchor Reviewer (`model_routing.anchor_review`)**: `openai` / `gpt-6.1-sol` (`high`, `TOOL_OK: anchor-review-model`) — 3 findings
* **Reviewer A (Template Fidelity)**: `google` / `gemini-4-argon` (`high`, `long_context`) — 0 findings
* **Reviewer B (Overlay Coherence)**: `anthropic` / `claude-sonnet-5.5` (`high`, `long_context`) — 3 findings (1 auto-fixed, 1 false positive dismissed, 1 upstream template note)
* **Reviewer C (Cross-Reference Integrity)**: `anthropic` / `claude-haiku-5.5` (`xhigh`, `long_context`) — 12 findings (6 auto-fixed, 6 advisory / upstream template notes)

### Consensus Findings (Ordered by Confidence × Severity)

#### HIGH Confidence

| # | Severity | Domain | File | Issue | Status |
|---|---|---|---|---|---|
| 1 | MAJOR | `cross-reference` | `.github/skills/pr-lifecycle/SKILL.md` | Step 5c last-mile check omitted unconditional P-018 `copilot-review` re-run and Step 5d omitted `COPILOT_REVIEW_BLOCK` + `ADMIN_FALLBACK_ATTEMPTED` / `DARK_MODE_HALTED` events | **AUTO-FIXED** |
| 2 | MAJOR | `cross-reference` | `.github/agents/_ship.agent.md` | Step 5 items 5, 7a, 7b needed explicit unengaged-vs-engaged P-018 wording and `Shadow review` / `Review-body findings` readiness fields | **AUTO-FIXED** |
| 3 | MINOR | `overlay-coherence` | `.github/instructions/constitution.instructions.md` | Principle VIII (`Explicit Safety Modes`) needed explicit cross-reference to `strict-safety.instructions.md` (`ProposedAction`, `ActionRisk`, `ActionResult`) | **AUTO-FIXED** |
| 4 | MINOR | `overlay-coherence` | `.github/copilot-instructions.md` | `agent-engram`, `backlogit`, and `continuous-learning` sections lacked explicit `follow .github/instructions/<pack>.instructions.md` bullets; `Commands` table was inside bash code fence; uninstalled `browser-verification` section was present | **AUTO-FIXED** |
| 5 | MINOR | `cross-reference` | `AGENTS.md` | Uninstalled `Capability Overlay — browser-verification` section was still present | **AUTO-FIXED** |
| 6 | MINOR | `overlay-coherence` | `.autoharness/workspace-profile.yaml` | Header comment still referenced `v1.5.0` and schema `1.0.0` instead of `v1.6.0` and `1.1.0` | **AUTO-FIXED** |

#### MEDIUM Confidence (Advisory / Upstream Template Notes)

* **`<!-- engram:start -->` block in `.github/copilot-instructions.md`**: Managed by `engram install --hooks-only` (`src/installer/templates.rs`); left untouched so installer hook parity tests continue to pass.
* **Upstream template prose items (`AGENTS.md.tmpl` principle numbering, `workflow-policies.md.tmpl` P-017/P-018 step citations, `adversarial-review.agent.md.tmpl` default model prose)**: Originate in upstream `autoharness` 1.6.0 templates; kept aligned with upstream templates to avoid introducing local template drift.

### Auto-Remediation Summary

* Findings auto-fixed: 6
* Findings requiring manual review: 2 (`.autoharness/policy-proposals/P-DYN-001.md` and deferred `start.ps1`/`start.sh` contract reconciliation under `140.001-T`)
* False positives dismissed: 2 (`model_routing.alt_review` key would violate `harness-config` 1.3.0 `additionalProperties: false`; `skills/harness-architect/SKILL.md.tmpl` manifest template path is required by `autoharness harness resolve`)

### Verification Result: PASS

* `autoharness verify-workspace --workspace C:\Source\GitHub\engram`: **EXIT 0** (0 strict schema blockers, 0 blockers, 0 warnings, 0 unresolved placeholders, 69/69 targeted checks PASS).

### Git Branch & PR Handoff Recommendation

* All backups are stored under `.autoharness/backups/2026-10-24/`.
* Per branch safety policy (Phase 0c / Step 1a), **never commit or push tune output directly to `main`**. Review the diff on a dedicated feature/chore branch (e.g., `chore/autoharness-tune-2026-10-24`) and merge via pull request.
