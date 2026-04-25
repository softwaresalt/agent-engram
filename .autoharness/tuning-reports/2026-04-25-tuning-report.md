---
type: tuning-report
generated_at: "2026-04-25T08:31:00-07:00"
workspace: "D:\\Source\\GitHub\\agent-engram"
autoharness_home: "C:\\Users\\derek.williams\\AppData\\Roaming\\uv\\tools\\autoharness\\Lib\\site-packages\\autoharness\\data"
manifest_installed_at: "2025-07-23"
previous_tune: "2026-04-24"
---

# Harness Tuning Report — 2026-04-25

## Drift Summary

| Category | Count | Description |
|----------|-------|-------------|
| P1 — Degrading | 2 | suffix_map spike/shipment swapped, shipment-reconcile lock-path bug |
| P2 — Growth | 2 | Profile inaccuracies, compound library quality |
| P3 — Cosmetic | 1 | Continuous learning unused by configuration |
| Flags | 1 | .mcp.json workspace paths incorrect + embedded API key (not harness-managed) |

## Composition

| Aspect | Previous | Current |
|--------|----------|---------|
| Preset | full | full (no change) |
| Primary stack pack | rust-mcp-daemon | rust-mcp-daemon (no change) |
| Stack packs | (unchanged) | rust-mcp-daemon, rust-surrealdb-embedded, rust-tree-sitter, github-actions-ci |
| Install layers | (unchanged) | foundation, instructions, workflow, review, runtime, backlog, knowledge, overlays |
| Capability packs | (unchanged) | agent-intercom, agent-engram, backlogit, strict-safety, release-observability, continuous-learning, adversarial-review |

## Profile Changes Since Last Tune

| Metric | Previous | Current | Delta |
|--------|----------|---------|-------|
| Source files (src/*.rs) | 87 | 87 | 0 |
| Test files | 102 | 102 | 0 |
| Tracked .md files | 487 | 718 | +47% (methodology: git-tracked only) |
| ADRs | 17 | 17 | 0 |
| New directories | — | docs/archive/ | Compacted artifacts |
| New files | — | .gitmodules, .mcp.json | Submodule + MCP config |
| New dev-dep | — | rayon (≥1.0, <1.11) | Parallel test execution |

## Checksum Scan

| Status | Count |
|--------|-------|
| Present (all artifacts) | 73 |
| Preserved (non-managed) | 7 |
| Missing | 0 |
| User-modified | 0 |
| Ignored | 0 |

## Healthy Checks (No Action Needed)

- ✅ All 73 manifest artifacts exist in workspace
- ✅ All 7 preserved artifacts exist in workspace
- ✅ Plan-hardening pipeline coherent (impl-plan → stage → plan-harden → plan-review)
- ✅ Gitignore has `.*.lock` / `**/.*.lock` without blocking `Cargo.lock`
- ✅ All 6 scripts present (acquire/release lock × 2, search × 2)
- ✅ Continuous-learning directory structure present (observations/, instincts/, learned/)
- ✅ Circuit-breaker and concurrency instructions installed and referenced
- ✅ Agent-intercom weaving coherent
- ✅ Agent-engram weaving coherent
- ✅ Backlogit weaving coherent
- ✅ Strict-safety weaving coherent
- ✅ Release-observability weaving coherent
- ✅ Adversarial-review weaving coherent
- ✅ Agent-native-parity-reviewer installed
- ✅ Backlog tool unchanged (backlogit) — no migration needed
- ✅ No deprecated agents lingering in .github/agents/
- ✅ No markdownlint config expected (profile: markdownlint: false)

## Approved Local Overlays (Do Not Regenerate)

| File | Delta | Reason |
|------|-------|--------|
| `.github/copilot-instructions.md` | +18.8% | Workspace-specific search strategy and session memory requirements |
| `.github/instructions/backlogit.instructions.md` | +43.5% | Shipment reconciliation rules |
| `.github/instructions/backlog-integration.instructions.md` | +13.2% | Extended shipment operations |
| `.github/instructions/commit-message.instructions.md` | +17.5% | Workspace-specific commit scopes |
| `AGENTS.md` | +1.3% | Workspace-specific quality gates (3 vs template's 4) |

## Proposed Changes (ordered by priority)

### P1 — Degrading (Applied)

#### TUNE-001: suffix_map spike/shipment values swapped

**Artifacts**: `.autoharness/config.yaml`, `.backlogit/config.yml`
**Issue**: Config had `spike: "S"` and `shipment: "SH"`, but actual queue files use `-S` suffix for shipments (008-S.md, 010-S.md, 011-S.md). The schema defaults are `spike: "SP"` and `shipment: "S"`. The values were swapped.
**Impact**: Future harness-generated artifacts consuming `{{SUFFIX_SHIPMENT}}` or `{{SUFFIX_SPIKE}}` would produce incorrect file paths. The backlogit runtime config (`.backlogit/config.yaml`) already uses the correct `suffix: -S` for shipments, so actual backlogit operations were not affected.
**Resolution**: Corrected to `spike: "SP"`, `shipment: "S"` in both config files. Backups at `.autoharness/backups/2026-04-25/`.

#### TUNE-002: shipment-reconcile lock-path bug + missing Model Routing

**Artifact**: `.github/skills/shipment-reconcile/SKILL.md`
**Issue**: Installed file had `.backlogit/queue/{shipment_id}.S.md` in lock paths, producing double-suffix paths like `004-S.S.md`. Template correctly uses `{shipment_id}.md`. Also missing `## Model Routing` section.
**Impact**: Ship agents using shipment-reconcile would attempt to lock/release non-existent file paths, weakening the concurrency gate.
**Resolution**: Regenerated from template with full variable substitution. Lock paths now correctly use `{shipment_id}.md`. Backup at `.autoharness/backups/2026-04-25/`.

### P2 — Growth (Applied)

#### TUNE-003: Profile inaccuracies and staleness

**Artifact**: `.autoharness/workspace-profile.yaml`
**Issue**: Multiple inaccuracies accumulated since last profile generation:
- `tempfile` listed as dev-dependency but is a production dependency
- tree-sitter language list included Kotlin and Markdown (no matching crates)
- Markdown file count used on-disk methodology (487); now uses git-tracked (718)
- Missing docs/archive/ directory, .gitmodules, .mcp.json
- Missing rayon dev-dependency pin
**Resolution**: Applied targeted fixes. Backups at `.autoharness/backups/2026-04-25/`.

### P2 — Growth (Deferred, Learning-Driven)

#### TUNE-004: Compound library quality — compound-refresh candidate

**Source**: learning-driven (Step 1.8.1)
**Issue**: Compound library grew from 0 to 15 entries since last tune. Category concentration:
- workflow-issues: 5 entries (33%) — highest
- build-errors: 4 entries (27%)
- test-failures: 3 entries (20%)
- best-practices: 2 entries (13%)
- concurrency-issues: 1 entry (7%)

Most entries have incomplete frontmatter (missing severity, tags). Two workflow-issues entries about ship-shipment behavior are already mitigated by the shipment-reconcile skill.
**Recommendation**: Invoke `compound-refresh` skill to consolidate, tag, and mark stale entries. No harness artifact change needed — this is a knowledge maintenance task.

### P3 — Cosmetic (Noted)

#### TUNE-005: Continuous learning unused by configuration

**Issue**: The `continuous-learning` capability pack is enabled with `capture_hooks: false` and `environment_adapter: "none"`. Zero observations, instincts, or learned artifacts exist. This is expected given the configuration — automatic capture is disabled and manual capture has not yet been invoked.
**Recommendation**: No action needed. Consider invoking the `observe` skill manually after sessions that surface recurring patterns.

## Flags (Not Harness-Managed)

### FLAG-001: .mcp.json workspace paths + embedded API key

**File**: `.mcp.json` (not in harness manifest — user-created config)
**Issues**:
1. `ENGRAM_WORKSPACE` and `BACKLOGIT_WORKSPACE` point to `D:\GitHub\agent-engram` — should be `D:\Source\GitHub\agent-engram`
2. Tavily server entry contains an embedded API key in the URL
**Impact**: MCP servers may target wrong workspace or fail to connect. Embedded API key is a security concern if committed.
**Resolution**: Not harness-managed. Flagged for operator attention.

## Learning Signals

| Source | Finding |
|--------|---------|
| Compound library | 15 entries (was 0). Category concentration: workflow-issues 33%. Two entries mitigated by shipment-reconcile. Quality: most entries missing severity/tags. |
| Continuous-learning | Empty by design (capture disabled). No instincts to promote. |
| Closure artifacts | 6 closure reports, 10+ compacted memory files. No recurring patterns detected across closures. |
| Prior tuning reports | 2 reports (2026-04-14, 2026-04-24). No recurring proposals — all prior proposals fully resolved. |

## Verification

| Check | Result |
|-------|--------|
| Template variable sweep | PASS — 0 unresolved `{{UPPERCASE}}` variables in regenerated artifact |
| shipment-reconcile SKILL.md | 160L (template 161L, matches after line-ending normalization) |
| suffix_map config consistency | PASS — config.yaml and config.yml both use spike: SP, shipment: S |
| Lock-path correctness | PASS — no `.S.md` double-suffix patterns in regenerated file |
| All manifest artifacts present | PASS — 73/73 |
| Preserved artifacts present | PASS — 7/7 |
| Backups created | 4 files in `.autoharness/backups/2026-04-25/` |

## Recommendation

This was a focused tune addressing correctness issues (suffix swap, lock-path bug) and profile staleness. The harness is structurally sound — no template-level content drift detected beyond the shipment-reconcile regeneration. Key improvements:

1. **suffix_map corrected** — future harness operations will use correct shipment/spike suffixes
2. **shipment-reconcile lock-path fixed** — Ship concurrency gate now targets correct file paths
3. **Profile refreshed** — dependency classification, language list, and file counts now accurate

### Next Tuning Triggers

- After CozoDB migration completes (changes backend, likely profile + stack-pack drift)
- After compound-refresh consolidates the 15-entry library
- When continuous-learning begins capturing observations
- Monthly cadence: target 2026-05-25

### Operator Action Items

- [ ] Fix `.mcp.json` workspace paths (`D:\GitHub\` → `D:\Source\GitHub\`)
- [ ] Remove or externalize Tavily API key from `.mcp.json`
- [ ] Consider invoking `compound-refresh` to consolidate/tag compound library entries

Generated by autoharness tune-harness skill
