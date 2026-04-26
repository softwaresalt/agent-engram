---
title: "Repository configuration and policy compliance cleanup"
source: "docs/decisions/2026-04-25-repo-config-policy-cleanup-deliberation.md"
stash_ids:
  - 4CE7A279
  - stash-001-rebase-merge
  - stash-002-mcp-json-paths
  - stash-003-tavily-key
---

# Repository Configuration and Policy Compliance Cleanup

## Problem Frame

Four accumulated repository hygiene items need resolution:

1. **Process violation (4CE7A279)**: Shipment 007-S was committed directly to
   `main` without a feature branch or PR. Existing policies (Constitution
   Principle XI, P-009) mandate branch-per-feature and merge-commit-only, but
   there is no explicit policy enforcing feature branch creation before the first
   commit. The Ship agent needs a clear gate.

2. **Rebase merge enabled (stash-001)**: GitHub repository settings still allow
   rebase merging, violating P-009. This is a manual GitHub UI change.

3. **`.mcp.json` paths (stash-002)**: The local `.mcp.json` (gitignored,
   untracked) has stale workspace paths (`D:\\GitHub\\agent-engram`) that should
   be `D:\\Source\\GitHub\\agent-engram`. No committed template exists for
   onboarding.

4. **Tavily API key (stash-003)**: The local `.mcp.json` contains a plaintext
   Tavily API key in the URL. Since `.mcp.json` is gitignored, this is not a
   committed secret, but it should use an environment variable for hygiene.

## Requirements Trace

| Requirement | Source | Implementation Unit |
|---|---|---|
| Prevent direct-to-main commits by Ship | 4CE7A279 | Unit 2 |
| Disable rebase merge in GitHub settings | stash-001 | Unit 1 |
| Fix local `.mcp.json` workspace paths | stash-002 | Unit 3 |
| Externalize Tavily API key from `.mcp.json` | stash-003 | Unit 3 |
| Provide onboarding template for `.mcp.json` | implied | Unit 3 |

## Implementation Units

### Unit 1: Disable rebase merge in GitHub repository settings

**Domain**: config (manual operator action)
**Execution posture**: operator-guided
**Files affected**: None (GitHub UI change)

The operator must navigate to GitHub Settings → General → Pull Requests and
uncheck "Allow rebase merging." This cannot be automated by the agent and must
complete before Unit 2 can note the settings change in P-009.

**Acceptance criteria**:
- `gh api /repos/softwaresalt/agent-engram --jq '.allow_rebase_merge'` returns `false`

### Unit 2: Add P-010 branch creation enforcement policy

**Domain**: docs/policy
**Execution posture**: direct edit
**Files affected**: `.github/policies/workflow-policies.md` (1 file)

Add a new policy P-010 "Feature Branch Creation Before First Commit" to the
workflow policy registry:

- **Applies to**: `ship`
- **Gate point**: Ship Step 1 pre-flight (before any file modification or commit)
- **Statement**: Ship MUST verify it is operating on a dedicated feature branch
  before creating any commit. Direct commits to `main` or the default branch are
  forbidden.
- **Precondition**: `git branch --show-current` returns a branch name other than
  `main` (or the configured default branch).
- **Violation action**: Halt. Create a feature branch from `main` before
  proceeding. Broadcast a P-005 violation event.

Also add a compliance action note to the P-009 section documenting the requirement
to disable `allow_rebase_merge` in GitHub settings (pending operator completion of
Unit 1, tracked as 033.001-T).

Update the amendment log with version bump.

**Note on Ship enforcement**: The Ship agent reads `workflow-policies.md` at each
declared gate point. Adding P-010 to the registry IS the enforcement mechanism —
Ship will pick it up at pre-flight. The Ship agent definition is
autoharness-generated and should not be modified directly; the policy registry is
the authoritative enforcement surface.

**Acceptance criteria**:
- `workflow-policies.md` contains P-010 with Applies To, Gate Point, Statement, Precondition, Postcondition, and Violation Action fields
- P-009 section includes compliance action note documenting the requirement to disable `allow_rebase_merge` (pending operator action in 033.001-T)
- Amendment log updated
- `Select-String -Path ".github/policies/workflow-policies.md" -Pattern "P-010"` finds the policy

**Tests**: Manual review — policy files are markdown, not code.

### Unit 3: Create `.mcp.json.example` and fix local MCP configuration

**Domain**: config
**Execution posture**: direct edit
**Files affected**: `.mcp.json.example` (new, committed), `.mcp.json` (local-only, untracked)

Create a committed `.mcp.json.example` template that:

- Shows the correct structure for all MCP server entries (engram, backlogit,
  context7, microsoft-docs, agent-intercom, tavily, github)
- Uses placeholder paths (`<WORKSPACE_ROOT>`) instead of hardcoded absolute paths
- Uses `${TAVILY_API_KEY}` environment variable reference instead of a plaintext key

Fix the local `.mcp.json`:

- Update `ENGRAM_WORKSPACE` from `D:\\GitHub\\agent-engram` to `D:\\Source\\GitHub\\agent-engram`
- Update `BACKLOGIT_WORKSPACE` from `D:\\GitHub\\agent-engram` to `D:\\Source\\GitHub\\agent-engram`
- Replace the Tavily URL's hardcoded API key with a `${TAVILY_API_KEY}` env var
  reference in the URL

**Acceptance criteria**:
- `.mcp.json.example` is tracked: `git ls-files .mcp.json.example` returns the file
- `.mcp.json.example` contains zero API keys: `Select-String -Path ".mcp.json.example" -Pattern "tvly-"` finds nothing
- `.mcp.json.example` contains zero hardcoded absolute paths: no `D:\\` or `C:\\` strings
- Local `.mcp.json` `ENGRAM_WORKSPACE` and `BACKLOGIT_WORKSPACE` contain `D:\\Source\\GitHub\\agent-engram`
- Local `.mcp.json` Tavily URL contains `${TAVILY_API_KEY}` (or an env var wrapper script) instead of a plaintext key

**Post-edit verification**: After fixing local `.mcp.json`, verify MCP
connectivity by confirming the engram and backlogit servers are reachable from
the IDE or CLI.

## Dependency Graph

```
Unit 1 (disable rebase merge) ──► Unit 2 (add P-010 + P-009 note)
                                          │
Unit 3 (config template + local fix) ─────┘ (parallel with Unit 2)
```

Unit 1 must complete before Unit 2 can note the settings change in P-009.
Unit 3 is independent and can execute in parallel with Units 1-2.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Add P-010 rather than strengthening existing policy | Existing policies (Constitution, P-009) address merge strategy and release-unit completion but do not explicitly gate branch creation. A dedicated policy provides a clear enforcement point. |
| Create `.mcp.json.example` rather than just fixing local file | A committed template enables onboarding and documents the expected MCP configuration without committing secrets. |
| Externalize Tavily key via env var | Environment variables are the standard approach for local secrets in gitignored config files. |
| Combine stash-002 and stash-003 into one unit | Both touch `.mcp.json` and the example template. Splitting them would violate width isolation (both are config domain, same file). |

## Risks and Caveats

| Risk | Likelihood | Mitigation |
|---|---|---|
| `.mcp.json` path change breaks MCP servers | Low | Verify paths exist; test MCP connectivity after edit |
| Operator forgets to disable rebase merge | Low | PR description includes verification command |
| P-010 too restrictive for hotfix scenarios | Low | Policy documents operator-assisted fallback and `skip_policy: P-010` escape with explicit approval |

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | No | Policy docs and config template only |
| Security, auth, permission, or compliance-sensitive | No | Tavily key is already untracked; externalization is hygiene not security remediation |
| Migration, backfill, destructive data/config action | No | No data migration |
| External integration, operator checkpoint | No | The GitHub UI toggle (Unit 1) is a one-time reversible settings change, not a deployment gate or external integration checkpoint. It is documented in the plan and verifiable via `gh api`. |
| High runtime, rollout, or rollback risk | No | Zero runtime impact |

**Requires plan hardening: no**

## Constitution Check

| Principle | Applies? | Compliance |
|---|---|---|
| I. Safety-First Rust | No | No Rust code changes |
| II. Test-First Development | No | No code changes requiring tests; policy and config only |
| III. Workspace Isolation | Yes | All file edits within workspace root |
| IV. CLI Containment | Yes | All file operations within cwd tree |
| V. Structured Observability | Yes | Changes documented in PR and workflow policies |
| VI. Single Responsibility | Yes | No new dependencies added |
| VII. Destructive Approval | N/A | No destructive commands |
| VIII. Safety Modes | N/A | Low blast radius; no elevated-risk work |
| IX. Git-Friendly Persistence | Yes | Markdown with YAML frontmatter conventions followed |
| X. Context Efficiency | N/A | No tool response changes |
| XI. Merge Commit Only | Yes | PR will use merge commit; rebase merge being disabled (Unit 1) |

## Runtime Verification and Closure

No runtime surfaces are changed by this work. All changes are documentation,
policy, and local configuration.

**Post-merge verification**:
- Confirm `gh api /repos/softwaresalt/agent-engram --jq '.allow_rebase_merge'` returns `false`
- Confirm `.mcp.json.example` is present in the repository
- Confirm `workflow-policies.md` contains P-010
