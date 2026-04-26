---
source_plan: docs/exec-plans/2026-04-25-repo-config-policy-cleanup-plan.md
shipment: 012-S
chore: 033-C
shipped: true
merge_sha: 866fc175c9c4272c525fd60ae069431dbe2dfe32
---

# Decided Plan — Repository Config & Policy Compliance (012-S)

## Final Decisions

| Decision | Rationale |
|---|---|
| Add P-010 (branch creation gate) | Existing policies gap — no explicit gate on feature branch creation before first commit. Policy registry is the enforcement surface (Ship reads it at pre-flight). |
| P-009 compliance note | Documents pending operator action (disable rebase merge); separate from P-010. |
| Create `.mcp.json.example` | Committed template for onboarding; prevents hardcoded paths from leaking in config commits. |
| Externalize Tavily key via `${TAVILY_API_KEY}` | Standard env var pattern for local secrets in gitignored files. |
| Combine stash-002 + stash-003 into one task (033.003-T) | Both touch the same file/domain; splitting would violate width isolation. |
| 033.001-T blocked — excluded from 012-S manifest | GitHub Settings change cannot be automated by agent (security policy). Tracked in stash `8897FD50`. |

## Delivered Units

| Unit | Task | Status | Key File |
|---|---|---|---|
| Unit 1 | Disable rebase merge | 🔴 blocked (operator action) | GitHub Settings |
| Unit 2 | Add P-010 + P-009 note | ✅ done | `.github/policies/workflow-policies.md` v1.5.0 |
| Unit 3 | `.mcp.json.example` + local fix | ✅ done | `.mcp.json.example` |

## Verification Results

- P-010 present in `workflow-policies.md` ✅
- `.mcp.json.example` tracked, no API keys, no hardcoded paths ✅
- `allow_rebase_merge` still `true` (Unit 1 pending) ⏳
- 6 rounds Copilot review; 43 threads resolved; CI green ✅
- Merge commit: `866fc175` (PR #28, 2026-04-26)

## Rejected Alternatives

- Modify autoharness-generated ship agent definition directly — rejected (policy registry is the right surface)
- Separate tasks for stash-002 and stash-003 — rejected (same file, same domain)
