---
title: "Harness Tuning Report — 2026-11-20"
date: "2026-11-20T02:15:15Z"
installer_version: "1.6.0"
gsd_home: ".gsd/gsd-assets-v1.6.0.zip"
trigger: "operator-requested"
scope: "all"
proposals:
  - id: "TUNE-001"
    key: "folded-core-pack-cleanup"
    priority: "P1"
    category: "degrading"
    source: "folded-core-pack"
    artifact: ".gsd/harness-manifest.yaml, .gsd/workspace-profile.yaml"
    status: "resolved"
  - id: "TUNE-002"
    key: "core-protocol-weaving-and-retired-intercom-cleanup"
    priority: "P1"
    category: "degrading"
    source: "structural-drift"
    artifact: "AGENTS.md, .github/copilot-instructions.md, .github/instructions/constitution.instructions.md"
    status: "resolved"
  - id: "TUNE-003"
    key: "guarded-checkpoint-registry-and-v1.6.0-scripts-sync"
    priority: "P0"
    category: "breaking"
    source: "structural-drift"
    artifact: ".gsd/backlog-registry.yaml, .github/instructions/backlog-integration.instructions.md, .markdownlint.json, scripts/*, .github/skills/harness-doctor/SKILL.md"
    status: "resolved"
  - id: "TUNE-004"
    key: "verify-harness-consensus-remediation"
    priority: "P1"
    category: "degrading"
    source: "structural-drift"
    artifact: ".github/agents/{_stage,_ship}.agent.md, .github/agents/subagents/{adversarial-review,security-sentinel}.agent.md, .github/prompts/*.prompt.md, .github/skills/{review,harvest,plan-harden,gsd-dark-mode,gsd-show-dag-queue,harness-architect,compact-context,deliberate,file-lock,impl-plan,spike}/SKILL.md"
    status: "resolved"
  - id: "TUNE-005"
    key: "workspace-discovery-crates-counts-and-kbdb-detection"
    priority: "P2"
    category: "growth"
    source: "structural-drift"
    artifact: ".gsd/workspace-profile.yaml"
    status: "resolved"
  - id: "TUNE-006"
    key: "review-startup-script-ps1-contract"
    priority: "P1"
    category: "degrading"
    source: "schema-contract"
    artifact: "start.ps1, start.sh"
    manual_review: true
    status: "resolved"
  - id: "TUNE-007"
    key: "legacy-backlogit-storage-root-followup"
    priority: "P1"
    category: "degrading"
    source: "structural-drift"
    artifact: ".backlogit/"
    manual_review: true
    status: "proposed"
  - id: "TUNE-008"
    key: "enable-kbdb-capability-pack"
    priority: "P2"
    category: "growth"
    source: "structural-drift"
    artifact: ".gsd/config.yaml, .github/instructions/kbdb.instructions.md"
    status: "proposed"
  - id: "TUNE-009"
    key: "workspace-authored-subagents-route-and-intercom-review"
    priority: "P2"
    category: "cosmetic"
    source: "route-drift"
    artifact: ".github/agents/subagents/{rust-mcp-expert,mcp-protocol-reviewer,rust-safety-reviewer}.agent.md"
    manual_review: true
    status: "proposed"
  - id: "TUNE-010"
    key: "dynamic-policy-proposal-code-graph-reindex-integrity"
    priority: "P2"
    category: "growth"
    source: "learning-driven"
    artifact: ".gsd/policy-proposals/P-DYN-001.md"
    status: "proposed"
  - id: "TUNE-011"
    key: "community-templates-opt-in"
    priority: "P2"
    category: "growth"
    source: "community-registry"
    artifact: "templates/community/skills/mcp-builder/SKILL.md.tmpl"
    status: "proposed"
---

## Harness Tuning Report — 2026-11-20

### Drift Summary

* Breaking changes: 1 (auto-remediated — unguarded `create_checkpoint` registry + broken `gsd.backlog_root` Python import in `_ship.agent.md`)
* Degrading changes: 5 (4 resolved, including PA-2 `start.ps1`/`start.sh` preservation in post-tune hygiene; 1 legacy `.backlogit/` root migration follow-up surfaced as detect-and-report)
* Growth opportunities: 4 (1 applied — workspace profile inventory & `kbdb` detection; 3 proposed for operator opt-in: `kbdb` capability-pack activation, `P-DYN-001` policy proposal, and `mcp-builder` community skill)
* Cosmetic adjustments: 1 (advisory — 3 workspace-authored `preserved_artifacts` subagents retained untouched per 037-DL)

### Composition

* Installed primary stack pack: `rust-mcp-daemon`
* Current primary stack pack: `rust-mcp-daemon`
* Installed stack packs: `["rust-mcp-daemon", "rust-surrealdb-embedded", "rust-tree-sitter", "github-actions-ci"]`
* Current stack packs: `["rust-mcp-daemon", "rust-surrealdb-embedded", "rust-tree-sitter", "github-actions-ci"]`
* Installed layers: `["foundation", "instructions", "workflow", "review", "runtime", "backlog", "knowledge", "overlays"]`
* Recommended layers: `["foundation", "instructions", "workflow", "review", "runtime", "backlog", "knowledge", "overlays"]`
* Enabled capability packs (`.gsd/config.yaml`): `["agent-engram", "backlogit"]`
* Detected candidate capability packs (`.gsd/workspace-profile.yaml`): `["agent-engram", "backlogit", "kbdb"]`
* Folded core packs normalized (`FOLDED_CORE_PACK` logged): `strict-safety`, `release-observability`, `continuous-learning`, `adversarial-review`

### Checksum & Contract Scan

* Manifest-tracked artifacts: 113 (after moving `start.ps1` and `start.sh` into `preserved_artifacts`, 10 preserved total)
* Missing installed artifacts: 0
* Unchanged managed artifacts: 113
* User-modified artifacts: 0 (`start.ps1` and `start.sh` are tracked under `preserved_artifacts` for the 142-F PA-2 launcher contracts)
* Ignored artifacts: 0
* Schema contracts:
  * `harness-config`: `current` (`1.3.0`, `.gsd/config.yaml`)
  * `harness-manifest`: `current` (`1.1.0`, `.gsd/harness-manifest.yaml`)
  * `workspace-profile`: `current` (`1.1.0`, `.gsd/workspace-profile.yaml`)
  * `carry-forward`: `valid` (`1.0.0`, `.gsd/carry-forward.yaml`, normalized to `files: [.gsd/config.yaml]`)
* Startup-script contracts:
  * `start.ps1`: `preserved` (`preserved_artifacts` — PA-2 `Invoke-EngramPreflight` launcher restored from `main` with `.bin`/`.kbdb\bin`/`.gsd` `PATH` wiring; 12/12 PowerShell contract tests PASS)
  * `start.sh`: `preserved` (`preserved_artifacts` — PA-2 `run_engram_preflight` launcher restored from `main`; 10/10 Unix contract tests PASS)

### Learning Signals

* Compound library (`docs/compound/`, 105 entries):
  * Component hotspots: `src/services/code_graph.rs` (8 entries), `backlogit shipment closure` (4 entries), `src/db/queries.rs` (3 entries)
  * Category distribution: `workflow-issues` (28), `best-practices` (19), `build-errors` (10), `test-failures` (6), `concurrency-issues` (5), `bugs` (4)
* Continuous learning (`.gsd/continuous-learning/`):
  * Observations: 5 JSONL logs (`2026-04-25` through `2026-11-01`)
  * Promotion-ready instincts: 0 (`174-s-windows-build-resource-cap.md` has `corroborating_observation_count: 1` < `promotion_threshold: 3`; stale `.autoharness/` citation path updated to `.gsd/`)
* Closure artifacts (`docs/closure/`, 196 records): 0 unresolved recurring closure-gate failures after GSD v1.6.0 `shipment-documentation-audit` and `evidence-consistency-reviewer` installation
* Policy-gap candidates detected: 1 (`.gsd/policy-proposals/P-DYN-001.md` refreshed for `gsd verify-workspace` CLI naming; remains `status: proposed` for operator review)

### Proposed & Applied Changes (Ordered by Priority)

#### P0 — Breaking (Applied)

1. **[TUNE-003] Sync guarded `create_checkpoint` registry, v1.6.0 `scripts/`, `.markdownlint.json`, and stdlib backlog-root resolver** (`source: structural-drift`)
   * **Artifacts**: `.gsd/backlog-registry.yaml`, `.github/instructions/backlog-integration.instructions.md`, `.github/agents/_ship.agent.md`, `.markdownlint.json`, `scripts/*`, `.github/skills/harness-doctor/SKILL.md`
   * **Issue**: `.gsd/backlog-registry.yaml` still exposed raw `backlogit_create_checkpoint` instead of the guarded `gsd checkpoint create --state-dump-file {{state_dump_file}}` adapter; `_ship.agent.md` Step 4.1 docs-route baseline called `from gsd.backlog_root import resolve_backlog_root` (unavailable in the standalone Go `gsd.exe` runtime); `scripts/pre-push-quality-gates.{ps1,sh}` lacked `AGENT_TEMP_DIR` containment and P-019 required `markdownlint` gating; `scripts/search.ps1` used 3-arg `Join-Path` incompatible with Windows PowerShell 5.1; `harness-doctor/SKILL.md` queried `GSD_version` instead of lowercase `gsd_version`.
   * **Resolution**: Synced `.gsd/backlog-registry.yaml`, `backlog-integration.instructions.md`, `.markdownlint.json`, `harness-doctor/SKILL.md`, and all 13 `scripts/*` from the `gsd.exe` v1.6.0 staging render, and replaced the broken Python import in `_ship.agent.md` with a zero-dependency Python stdlib resolver honoring `BACKLOGIT_WORKSPACE_DIR` and fail-closed `.backlog`/`.backlogit` ambiguity detection.

#### P1 — Degrading (Applied)

2. **[TUNE-001] Remove folded core pack residue (`FOLDED_CORE_PACK`)** (`source: folded-core-pack`)
   * **Artifacts**: `.gsd/harness-manifest.yaml`, `.gsd/workspace-profile.yaml`
   * **Issue**: `variables_used.CAPABILITY_PACKS` / `CAPABILITY_PACKS_YAML` and profile `recommendation_reasons` still listed `strict-safety`, `release-observability`, `continuous-learning`, and `adversarial-review` as capability packs.
   * **Resolution**: Logged `FOLDED_CORE_PACK` for each, normalized `CAPABILITY_PACKS` to `"agent-engram,backlogit"`, and kept all 5 core instructions, `adversarial-review.agent.md`, and core skills active.

3. **[TUNE-002] Synchronize Core Workflow Protocols and remove stale `agent-intercom` prose** (`source: structural-drift`)
   * **Artifacts**: `AGENTS.md`, `.github/copilot-instructions.md`, `.github/instructions/constitution.instructions.md`
   * **Issue**: Foundation documents still framed `continuous-learning`, `strict-safety`, `release-observability`, and `adversarial-review` as optional capability packs, omitted `browser-verification` and `grill-me`/`brainstorm` pipeline steps, and retained a stale `### agent-intercom` section in `.github/copilot-instructions.md`.
   * **Resolution**: Wove all 5 Core Workflow Protocols across `AGENTS.md`, `.github/copilot-instructions.md`, and `constitution.instructions.md`, removed the retired `agent-intercom` section, added the `Workspace-Local .gsd/ Runtime` note, and preserved `docs/operator-glossary.md` plus `<!-- engram:start -->` hooks.
   * **Base-branch activation note**: GitHub Copilot code review reads `.github/instructions/copilot-code-review.instructions.md` from the pull request's base branch (`main`), so updated review instructions govern subsequent PRs once merged.

4. **[TUNE-004] Apply multi-model `verify-harness` consensus remediations** (`source: structural-drift`)
   * **Artifacts**: `.github/agents/{_stage,_ship}.agent.md`, `.github/agents/subagents/{adversarial-review,security-sentinel}.agent.md`, `.github/prompts/*.prompt.md`, `.github/skills/{review,harvest,plan-harden,gsd-dark-mode,gsd-show-dag-queue,harness-architect,compact-context,deliberate,file-lock,impl-plan,spike}/SKILL.md`
   * **Resolution**: Added `'engram/*'` to `_stage.agent.md` and `_ship.agent.md` tool allowlists; fixed prompt `agent:` targets (`_Orchestrator`, `_Stage`); set `max_subagent_tier: 3` and added `## Model Routing` to `adversarial-review.agent.md`; cleared unconfigured `ALT_*`/`ANCHOR_REVIEW_*` literal substitutions; normalized `SECURITY_*_PATTERNS` to single-line backtick-free strings so inline code spans render cleanly; fixed `.backlogit/config.yaml` overlay target paths; added `## Quality Criteria` and `## Model Routing` to `gsd-dark-mode` and `gsd-show-dag-queue` skills; promoted H2 opening headings to H1 (`MD041`); and aligned `review`, `harvest`, and `plan-harden` protocol references.

#### P1 — Degrading (Deferred / Operator Review Required)

5. **[TUNE-006] `review-startup-script-ps1-contract`** (`source: schema-contract`, `status: resolved`, `manual_review: true`)
   * **Artifacts**: `start.ps1`, `start.sh`, `.gsd/harness-manifest.yaml`
   * **Resolution**: Restored `main`'s 142-F PA-2 fail-closed `engram preflight` launchers (`start.ps1` and `start.sh`), added safe conditional `.gsd`/`.kbdb\bin`/`.bin` `PATH` setup to `start.ps1`, moved both scripts into `preserved_artifacts` (`10` total), and verified all `22/22` launcher contract tests (`contract_start_launcher`, `contract_start_launcher_failure`, `contract_start_sh_launcher`).

6. **[TUNE-007] Legacy `.backlogit/` storage root follow-up** (`source: structural-drift`, detect-and-report only)
   * **Artifact**: `.backlogit/`
   * **Issue**: Workspace uses legacy `.backlogit/` storage root (`.backlog/` absent).
   * **Action**: When ready to migrate outside queued shipment `176-S`, review `backlogit migrate --workspace-dir --dry-run`. Tune performs no directory rename or backlog mutation.

#### P2 — Growth & Advisory Proposals (Operator Opt-In)

7. **[TUNE-005] Workspace discovery refresh & `kbdb` profile recording** (`source: structural-drift`, **Applied to `.gsd/workspace-profile.yaml`**)
   * Updated source/crate/test counts (`152` `src/` `.rs` files + `12` `crates/` `.rs` files across `engram-indexer` and `powerbi-tmdl-parser`; `284` test files: `contract/66`, `integration/152`, `unit/62`, `helpers/4`) and recorded `kbdb` detection markers (`.mcp.json`, `.kbdb/config/sources.yaml`, `.bin/kbdb.exe`, `C:\Tools\.kbdb\models\all-MiniLM-L6-v2`).

8. **[TUNE-008] Enable `kbdb` capability pack in `.gsd/config.yaml`** (`source: structural-drift`, **Proposed**)
   * **Rationale**: `.kbdb/` (4 SQLite GraphRAG DBs, 7 sources, 68,714 chunks) and `.mcp.json` (`kbdb` server via `.copilot\kbdb-mcp-shim.cjs`) are active, while `.gsd/config.yaml` currently specifies `capability_packs: ["agent-engram", "backlogit"]`.
   * **To adopt**: Add `"kbdb"` to `capability_packs` and `sidecars` in `.gsd/config.yaml`, then re-run `/tune-harness` to install `.github/instructions/kbdb.instructions.md` and weave `kbdb` into `capability-pack-enforcement.instructions.md`, `AGENTS.md`, `copilot-instructions.md`, and `constitution.instructions.md`.

9. **[TUNE-009] Advisory review of workspace-authored subagents (`preserved_artifacts`)** (`source: route-drift`, **Proposed / Preserved per 037-DL**)
   * `rust-mcp-expert.agent.md` still declares `claude-sonnet-5.5` / `anthropic` (prior Tier 2 before `mai-code-2` / `microsoft`).
   * `mcp-protocol-reviewer.agent.md` and `rust-safety-reviewer.agent.md` still declare `reasoning_effort: "xhigh"` and contain legacy `## Agent-Intercom Communication (NON-NEGOTIABLE)` sections.

10. **[TUNE-010] Dynamic policy proposal `P-DYN-001`** (`source: learning-driven`, **Proposed**)
    * `.gsd/policy-proposals/P-DYN-001.md` (`Code-Graph Extraction & Single-File Reindex Integrity Gate`, backed by 8 `src/services/code_graph.rs` compound entries) has been updated to reference `gsd verify-workspace` and awaits operator accept/reject disposition.

11. **[TUNE-011] Applicable community templates** (`source: community-registry`, **Opt-In Only**)
    * Top match for `mcp-server` profile: `mcp-builder` (`templates/community/skills/mcp-builder/SKILL.md.tmpl`, deepens Primitives 5 & 9), plus `adr-generator`, `debugging`, and `verification-before-completion`.

---

## Adversarial Verification

### Reviewer Dispatch

* **Anchor Reviewer (`model_routing.anchor_review` default)**: `openai` / `gpt-5.6-sol` (`high`, `TOOL_OK: anchor-review-model`) — 2 findings (1 auto-fixed, 1 staging-scratch false positive dismissed)
* **Reviewer A (Template Fidelity)**: `google` / `gemini-4-carbon` (`high`, `long_context`) — 19 findings (16 auto-fixed, 3 upstream template advisory notes)
* **Reviewer B (Overlay Coherence)**: `microsoft` / `mai-code-2` (`high`, `long_context`) — 8 findings (8 auto-fixed)
* **Reviewer C (Cross-Reference Integrity)**: `anthropic` / `claude-haiku-5.5` (`high`, `long_context`) — 6 findings (1 upstream template prose note, 5 upstream `gsd_home` template-path citation notes)

### Consensus Findings (Ordered by Confidence × Severity)

#### HIGH Confidence

| # | Severity | Domain | File | Issue | Status |
|---|---|---|---|---|---|
| 1 | CRITICAL | `template-fidelity` | `.gsd/backlog-registry.yaml`, `.github/instructions/backlog-integration.instructions.md` | Stale unguarded `backlogit_create_checkpoint` surface instead of v1.6.0 `gsd checkpoint create --state-dump-file` adapter | **AUTO-FIXED** |
| 2 | CRITICAL | `template-fidelity` | `.github/agents/_ship.agent.md` | Docs-only baseline invoked non-existent `from gsd.backlog_root import resolve_backlog_root` Python module in standalone Go `gsd.exe` workspace | **AUTO-FIXED** |
| 3 | MAJOR | `template-fidelity` | `scripts/*`, `.markdownlint.json`, `.github/skills/harness-doctor/SKILL.md` | Pre-push hooks lacked `AGENT_TEMP_DIR` containment and P-019 required `markdownlint` gate; `search.ps1` had PS 5.1 3-arg `Join-Path` bug; `harness-doctor` queried `GSD_version` | **AUTO-FIXED** |
| 4 | MAJOR | `overlay-coherence` | `.github/agents/{_stage,_ship}.agent.md` | `tools:` frontmatter lacked `'engram/*'` while requiring `agent-engram` code-graph workflow | **AUTO-FIXED** |
| 5 | MAJOR | `template-fidelity` | `.github/prompts/*.prompt.md` | Prompt frontmatter referenced `Orchestrator` / `Stage` instead of canonical `_Orchestrator` / `_Stage` | **AUTO-FIXED** |
| 6 | MAJOR | `template-fidelity` | `.github/skills/review/SKILL.md`, `.github/agents/subagents/security-sentinel.agent.md`, `.github/skills/security-audit/SKILL.md` | Multi-line / backtick-bearing `SECURITY_*` variables inside inline backticks corrupted Markdown code spans and lists | **AUTO-FIXED** |
| 7 | MAJOR | `template-fidelity` | `.github/agents/subagents/adversarial-review.agent.md`, `.github/skills/doc-review/SKILL.md` | Missing `## Model Routing` section, `max_subagent_tier: 1` (dispatches Tier 2/3), and unconfigured `ALT_*`/`ANCHOR_REVIEW_*` fallback literals | **AUTO-FIXED** |
| 8 | MAJOR | `overlay-coherence` | `.gsd/harness-manifest.yaml`, `.gsd/workspace-profile.yaml` | `backlogit` overlay target referenced non-existent `.backlogit/config.yml` instead of `.backlogit/config.yaml` and omitted 6 woven targets | **AUTO-FIXED** |
| 9 | MAJOR | `overlay-coherence` | `.github/skills/{review,harvest,plan-harden}/SKILL.md`, `.github/instructions/constitution.instructions.md` | Persona search directive omitted `agent-engram` priority; `harvest` omitted live catalog check; `plan-harden` omitted `release-observability` link; `constitution` omitted 4 core protocols | **AUTO-FIXED** |
| 10 | MINOR | `template-fidelity` | `.github/skills/{gsd-dark-mode,gsd-show-dag-queue,harness-architect,compact-context,deliberate,file-lock,impl-plan,spike}/SKILL.md` | Missing `## Quality Criteria` / `## Model Routing` in 2 new skills, `tests//` double slashes and `{SUFFIX_*}` single braces in `harness-architect`, `MD041` H2 opening headings in 6 skills | **AUTO-FIXED** |

#### MEDIUM Confidence (Upstream Template Advisory Notes)

* **Upstream `gsd-assets-v1.6.0.zip` template prose citations**: `_orchestrator.agent.md` (`.github/skills/install-harness/SKILL.md` and `harness-architecture.instructions.md` citations), `_stage.agent.md` (`templates/policies/workflow-policies.md.tmpl` citation), `_ship.agent.md` (`schemas/tool-telemetry-event.schema.json` field-alignment note), `shipment-reconcile/SKILL.md` (`templates/agents/_ship.agent.md.tmpl` citation), `workflow-policies.md` (historical v1.37.0 changelog note), and `pr-lifecycle/SKILL.md` (Step 5a 4-check vs 5-check prose) originate verbatim in upstream GSD v1.6.0 templates (`targeted_checks.agent_skill_references` explicitly classifies `install-harness` as `KNOWN_TEMPLATE`). Left aligned with upstream templates to avoid unnecessary template divergence.

### Auto-Remediation Summary

* Findings auto-fixed: 10 consensus groups (covering 25 individual reviewer items), plus `TUNE-006` PA-2 launcher preservation and post-v1.6.0 gate/hook hygiene
* Findings remaining for operator opt-in/review: 4 (`TUNE-007` `.backlogit/` root migration preview, `TUNE-008` `kbdb` pack opt-in, `TUNE-009` workspace-authored subagents, `TUNE-010` `P-DYN-001`)
* False positives dismissed: 1 (`.gsd/staging/` ephemeral scratch file flagged by Anchor Reviewer; installed `.github/instructions/capability-pack-enforcement.instructions.md` already omits `kbdb`)

### Verification Result: PASS WITH WARNINGS

* `.\.bin\gsd.exe verify-workspace --workspace . --gsd-home .gsd/gsd-assets-v1.6.0.zip --json`: **EXIT 0** (`blockers: []`, `strict_schema_blockers: []`, `unresolved: []`, `targeted_checks`: 12/12 PASS; sole warning is informational `backlogit-engine-version-unverified` on debug build `v1.13.0-27-g76f7b144-debug`).
* `cargo test --test contract_start_launcher --test contract_start_launcher_failure --test contract_start_sh_launcher --test integration_installer`: **56/56 PASS** (2 ignored).
* Pre-update backups saved to gitignored `.gsd/backups/2026-11-19/`. Committed on dedicated branch `chore/gsd-v1.6.0-migration` targeting `main` via PR (preserving Phase 0c branch-safety policy: never commit or push directly to `main`).
