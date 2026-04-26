---
title: "Repository configuration and policy compliance cleanup"
description: "Deliberation on grouping 4 task-shaped stash entries into a single covering feature for repo config hygiene and PR merge policy enforcement"
topic: "Group A — repo config and policy compliance cleanup"
depth: "lightweight"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts: []
tags:
  - repo-config
  - policy-compliance
  - mcp-json
  - branch-protection
stash_ids:
  - 4CE7A279
  - stash-001-rebase-merge
  - stash-002-mcp-json-paths
  - stash-003-tavily-key
---

## Problem Frame

Four task-shaped stash entries have accumulated around repository configuration
hygiene and workflow policy compliance. None involve application code changes.
All were flagged during the autoharness v1.3.0 tune closure
(`docs/closure/2026-04-25-autoharness-tune-v1.3.0-closure.md`) or from a prior
shipment process violation.

The problem: these are small, independent config/policy fixes that individually
do not warrant full feature pipelines. Grouped together they form a coherent
"repo hygiene" release unit that can ship as a single PR.

### Entries

| Stash ID | Priority | Kind | Summary |
|---|---|---|---|
| `4CE7A279` | high | task | Enforce branch protection: Ship must create feature branches, use pr-lifecycle, treat bypass warnings as hard stops |
| `stash-001-rebase-merge` | medium | task | Disable `allow_rebase_merge` in GitHub repo settings (P-009 compliance) |
| `stash-002-mcp-json-paths` | low | task | Fix `.mcp.json` workspace paths from `D:\\GitHub\\` to `D:\\Source\\GitHub\\` |
| `stash-003-tavily-key` | medium | task | Remove or externalize Tavily API key from `.mcp.json` (secret in committed config) |

## Research Findings

### `.mcp.json` current state

- `ENGRAM_WORKSPACE` points to `D:\\GitHub\\agent-engram` — incorrect (should be `D:\\Source\\GitHub\\agent-engram`)
- `BACKLOGIT_WORKSPACE` points to `D:\\GitHub\\agent-engram` — same issue
- Tavily MCP server URL contains a plaintext API key: `tvly-dev-***REDACTED***` (key has been rotated)
- Both issues were flagged as known follow-ups in the v1.3.0 tune closure

### Branch protection / merge strategy

- Constitution Principle XI (P-009) mandates merge commits only; squash and rebase merge are forbidden
- The `stash-001-rebase-merge` entry specifically targets the GitHub Settings UI change
- The `4CE7A279` entry documents a process violation where shipment 007-S was committed directly to main without a branch or PR

### Compound library

No relevant prior learnings found for these topics.

## Options Evaluated

### Option A: Single covering feature — all 4 items together

**Description**: Group all 4 entries under one covering feature titled
"Repository configuration and policy compliance cleanup." Ship as one PR.

- **Pros**: Minimizes shipment overhead; all items are small and non-code; natural single-PR scope
- **Cons**: Mixes two domains (PR policy vs .mcp.json config) — but blast radius is low for both
- **Effort**: ~4 tasks × 2h = 8h
- **Risk**: Low — all changes are config/documentation level

### Option B: Split into two focused features

**Description**: Create two separate features:
1. "PR merge policy enforcement" (4CE7A279 + stash-001-rebase-merge) — 2 tasks
2. ".mcp.json configuration hygiene" (stash-002 + stash-003) — 2 tasks

- **Pros**: Cleaner domain separation; each feature is tightly scoped
- **Cons**: Doubles the shipment overhead for 4 tiny tasks; creates 2 PRs for work that could be one
- **Effort**: 2 + 2 tasks = same 8h total but with 2× pipeline overhead
- **Risk**: Low for both

## Trade-off Comparison

| Criterion | Option A (single feature) | Option B (two features) |
|---|---|---|
| Coherence | Good — shared "repo hygiene" theme | Better — domain-pure groupings |
| Pipeline overhead | 1 shipment, 1 PR | 2 shipments, 2 PRs |
| Blast radius | Low | Low |
| Review complexity | Single small PR | Two trivially small PRs |
| Time to ship | Faster (single pipeline) | Slower (2× pipeline) |

## Decision

**Chosen: Option A — single covering feature.**

Rationale: All 4 items are small, non-code config/policy fixes. The domain
overlap (repo-level hygiene) is strong enough to justify a single covering
feature. Splitting into two features doubles pipeline overhead for no meaningful
risk reduction. The combined scope still fits comfortably within a single PR
and review cycle.

**Covering feature title**: "Repository configuration and policy compliance cleanup"

**Confirmed task scope**:
1. Document branch protection enforcement policy and add guardrails
2. Disable `allow_rebase_merge` in GitHub repository settings
3. Fix `.mcp.json` workspace paths
4. Externalize Tavily API key from `.mcp.json`

## Rejected Alternatives

**Option B** was rejected because the pipeline overhead of 2 shipments and 2 PRs
is disproportionate to the size and risk of the work. Domain separation offers
no meaningful benefit when all items are low-blast-radius config changes.

## Unresolved Questions

- The Tavily API key externalization approach needs to be determined during
  planning: environment variable, separate untracked config, or vault reference?
- The branch protection documentation task may overlap with existing workflow
  policies in `.github/policies/workflow-policies.md` — check during planning.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| `.mcp.json` path change breaks local dev setup | Verify paths exist before committing; document in PR |
| Tavily key removal breaks search capability | Use env var fallback so the key is still usable locally |
| Branch protection doc is redundant with existing policies | Check existing docs first; extend rather than duplicate |
